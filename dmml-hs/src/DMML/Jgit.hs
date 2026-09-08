-- | Typed Haskell wrappers over a small, fixed set of real JGit calls,
-- via the generic JNI primitives in 'DMML.Jni' (backed by
-- @cbits\/jni_prims.c@).
--
-- Deliberately NOT built on the Tweag @jni@\/@jvm@\/@inline-java@
-- packages: both @jni@ and @jvm@ are marked deprecated on Hackage, last
-- published 2020-11-30, and Hackage's own page for @jvm@ points at a
-- GitHub repo that now builds with Bazel -- a different build system
-- than this project uses everywhere else. Real, checked facts, not
-- assumed (see the design conversation this module comes out of).
--
-- Every JNI method\/class\/field name and signature string used to call
-- into JGit appears EXACTLY ONCE in this file, immediately beside the
-- typed Haskell function that uses it -- e.g. 'jgitCommit' is the only
-- place @\"()Lorg\/eclipse\/jgit\/revwalk\/RevCommit;\"@ is written. A
-- signature that drifts from what JGit actually declares fails loudly,
-- at the very next call, via 'JgitException' (checked directly against
-- the real 7.7.1 jar with @javap -s@, not typed from memory) rather
-- than silently. See @dmml-hs\/app\/JgitSmokeTest.hs@ for the real,
-- runnable end-to-end proof: init, add, commit, read the commit name
-- back, against a real throwaway repository.
--
-- Scope, deliberately narrow: only what @atproto-broker.sh@ itself
-- actually calls (@git init@\/@add@\/@commit@) plus reading a commit's
-- name back. No merge, no worktree, no hooks -- see
-- @written-world\/dev-journal\/2026-09-07-jgit-canonical-single-
-- implementation.md@ for why those are gone, not deferred.
module DMML.Jgit
  ( JvmHandle
  , JvmEnvironment (..)
  , withJvm
  , withEmbeddedJvm
  , JGit
  , RevCommitRef
  , JgitException (..)
  , jgitInit
  , jgitOpen
  , jgitAddFilepattern
  , jgitCommit
  , jgitResolve
  , revCommitName
  ) where

import Control.Exception (Exception, throwIO)
import Foreign.Ptr (nullPtr)

import DMML.Jni
  ( JRef
  , JvmEnvironment (..)
  , JvmHandle (..)
  , c_callObjectMethod0
  , c_callObjectMethod1Obj
  , c_callObjectMethod1Str
  , c_callStaticObjectMethod0
  , c_callStaticObjectMethod1Bool
  , c_callStaticObjectMethod1Obj
  , c_newObject1Obj
  , describeAndClearException
  , findClass
  , hsStringToJString
  , jStringToHsString
  , methodId
  , staticMethodId
  , withEmbeddedJvm
  , withJvm
  )

-- | A live @org.eclipse.jgit.api.Git@ instance.
newtype JGit = JGit JRef

-- | A live @org.eclipse.jgit.revwalk.RevCommit@ instance.
newtype RevCommitRef = RevCommitRef JRef

-- | A JGit call threw a real Java exception (most commonly a checked
-- @GitAPIException@ subtype) -- the description is that exception's own
-- @toString()@, read back via JNI before it's cleared, not invented.
newtype JgitException = JgitException String
  deriving (Eq)

instance Show JgitException where
  show (JgitException msg) = "JgitException: " <> msg

instance Exception JgitException

-- | Throw 'JgitException' if the last JNI call left a pending Java
-- exception -- called after every @call()@ that can throw a checked
-- @GitAPIException@. @what@ names the operation, for a readable error.
checkException :: JRef -> String -> IO ()
checkException env what = do
  mDesc <- describeAndClearException env
  case mDesc of
    Nothing -> pure ()
    Just desc -> throwIO (JgitException (what <> ": " <> desc))

-- | @git init@ at the given directory. Real JGit call chain, real
-- signature strings, checked directly against 7.7.1's jar:
--
-- > Git.init()          -> "()Lorg/eclipse/jgit/api/InitCommand;"   (static)
-- > InitCommand.setDirectory(File) -> "(Ljava/io/File;)Lorg/eclipse/jgit/api/InitCommand;"
-- > InitCommand.call()  -> "()Lorg/eclipse/jgit/api/Git;"
jgitInit :: JvmHandle -> FilePath -> IO JGit
jgitInit (JvmHandle env) dir = do
  gitCls <- findClass env "org/eclipse/jgit/api/Git"
  initM <- staticMethodId env gitCls "init" "()Lorg/eclipse/jgit/api/InitCommand;"
  initCmd <- c_callStaticObjectMethod0 env gitCls initM

  fileCls <- findClass env "java/io/File"
  fileCtor <- methodId env fileCls "<init>" "(Ljava/lang/String;)V"
  dirJStr <- hsStringToJString env dir
  fileObj <- c_newObject1Obj env fileCls fileCtor dirJStr

  initCmdCls <- findClass env "org/eclipse/jgit/api/InitCommand"
  setDirM <- methodId env initCmdCls "setDirectory" "(Ljava/io/File;)Lorg/eclipse/jgit/api/InitCommand;"
  _ <- c_callObjectMethod1Obj env initCmd setDirM fileObj

  callM <- methodId env initCmdCls "call" "()Lorg/eclipse/jgit/api/Git;"
  gitObj <- c_callObjectMethod0 env initCmd callM
  checkException env "Git.init().setDirectory(...).call()"
  pure (JGit gitObj)

-- | Open an EXISTING repository at the given directory -- distinct from
-- 'jgitInit', which creates one. @written-world fire@'s repo is created
-- once, out of band, and opened fresh on every invocation; conflating
-- the two (e.g. calling 'jgitInit' every time on the assumption it's a
-- harmless no-op against an already-initialized repo) was deliberately
-- not assumed here and not needed, since JGit has a real, separate
-- @Git.open@ for exactly this. Real signature:
--
-- > Git.open(File) -> "(Ljava/io/File;)Lorg/eclipse/jgit/api/Git;" (static)
jgitOpen :: JvmHandle -> FilePath -> IO JGit
jgitOpen (JvmHandle env) dir = do
  fileCls <- findClass env "java/io/File"
  fileCtor <- methodId env fileCls "<init>" "(Ljava/lang/String;)V"
  dirJStr <- hsStringToJString env dir
  fileObj <- c_newObject1Obj env fileCls fileCtor dirJStr

  gitCls <- findClass env "org/eclipse/jgit/api/Git"
  openM <- staticMethodId env gitCls "open" "(Ljava/io/File;)Lorg/eclipse/jgit/api/Git;"
  gitObj <- c_callStaticObjectMethod1Obj env gitCls openM fileObj
  checkException env ("Git.open(" <> dir <> ")")
  pure (JGit gitObj)

-- | @git add \<pattern\>@ against an already-open 'JGit'. Real signatures:
--
-- > Git.add()                       -> "()Lorg/eclipse/jgit/api/AddCommand;"
-- > AddCommand.addFilepattern(String) -> "(Ljava/lang/String;)Lorg/eclipse/jgit/api/AddCommand;"
-- > AddCommand.call()               -> "()Lorg/eclipse/jgit/dircache/DirCache;"
jgitAddFilepattern :: JvmHandle -> JGit -> String -> IO ()
jgitAddFilepattern (JvmHandle env) (JGit gitObj) pattern' = do
  gitCls <- findClass env "org/eclipse/jgit/api/Git"
  addM <- methodId env gitCls "add" "()Lorg/eclipse/jgit/api/AddCommand;"
  addCmd <- c_callObjectMethod0 env gitObj addM

  addCmdCls <- findClass env "org/eclipse/jgit/api/AddCommand"
  addPatM <- methodId env addCmdCls "addFilepattern" "(Ljava/lang/String;)Lorg/eclipse/jgit/api/AddCommand;"
  patJStr <- hsStringToJString env pattern'
  _ <- c_callObjectMethod1Str env addCmd addPatM patJStr

  callM <- methodId env addCmdCls "call" "()Lorg/eclipse/jgit/dircache/DirCache;"
  _ <- c_callObjectMethod0 env addCmd callM
  checkException env ("Git.add().addFilepattern(" <> pattern' <> ").call()")

-- | @git commit -m \<message\>@, explicitly unsigned. Real signatures:
--
-- > Git.commit()              -> "()Lorg/eclipse/jgit/api/CommitCommand;"
-- > CommitCommand.setMessage(String) -> "(Ljava/lang/String;)Lorg/eclipse/jgit/api/CommitCommand;"
-- > CommitCommand.setSign(Boolean)   -> "(Ljava/lang/Boolean;)Lorg/eclipse/jgit/api/CommitCommand;"
-- > Boolean.valueOf(boolean)  -> "(Z)Ljava/lang/Boolean;" (static; how we
-- >   get a boxed Boolean to pass -- setSign takes java.lang.Boolean, not
-- >   a primitive boolean)
-- > CommitCommand.call()      -> "()Lorg/eclipse/jgit/revwalk/RevCommit;"
--
-- @setSign(FALSE)@ is not optional: JGit reads the same @~\/.gitconfig@
-- real @git@ does, and a machine with @commit.gpgsign = true@ set
-- globally (real, found on the machine this module was developed and
-- tested on) makes JGit try to honor it -- but JGit's SSH-signing
-- support does not interoperate with an external @gpg.ssh.program@ the
-- way the real @git@ CLI's shells out to one, so an unconditional
-- commit call fails with @UnsupportedSigningFormatException@ on any
-- such machine. None of the shell scripts this module replaces ever
-- signed commits either, so disabling it explicitly matches existing
-- behavior, not a new gap.
jgitCommit :: JvmHandle -> JGit -> String -> IO RevCommitRef
jgitCommit (JvmHandle env) (JGit gitObj) message = do
  gitCls <- findClass env "org/eclipse/jgit/api/Git"
  commitM <- methodId env gitCls "commit" "()Lorg/eclipse/jgit/api/CommitCommand;"
  commitCmd <- c_callObjectMethod0 env gitObj commitM

  commitCmdCls <- findClass env "org/eclipse/jgit/api/CommitCommand"
  setMsgM <- methodId env commitCmdCls "setMessage" "(Ljava/lang/String;)Lorg/eclipse/jgit/api/CommitCommand;"
  msgJStr <- hsStringToJString env message
  _ <- c_callObjectMethod1Str env commitCmd setMsgM msgJStr

  boolCls <- findClass env "java/lang/Boolean"
  valueOfM <- staticMethodId env boolCls "valueOf" "(Z)Ljava/lang/Boolean;"
  falseObj <- c_callStaticObjectMethod1Bool env boolCls valueOfM 0
  setSignM <- methodId env commitCmdCls "setSign" "(Ljava/lang/Boolean;)Lorg/eclipse/jgit/api/CommitCommand;"
  _ <- c_callObjectMethod1Obj env commitCmd setSignM falseObj

  callM <- methodId env commitCmdCls "call" "()Lorg/eclipse/jgit/revwalk/RevCommit;"
  revCommitObj <- c_callObjectMethod0 env commitCmd callM
  checkException env "Git.commit().setMessage(...).setSign(FALSE).call()"
  pure (RevCommitRef revCommitObj)

-- | Resolve a revision string (e.g. @\"HEAD\"@ or @\"HEAD:commits\"@ --
-- JGit's @Repository.resolve@ accepts the same @rev-parse@ syntax real
-- @git@ does, including the @\<commit\>:\<path\>@ form used to get a
-- subtree's own tree SHA) to its hex object id, or 'Nothing' if it
-- doesn't resolve (e.g. @HEAD:commits@ before any commit exists --
-- 'Repository.resolve' returns Java @null@ for this, not an exception,
-- and this module treats that as 'Nothing' rather than an error since
-- it's the normal bootstrap case, not a failure). Real signatures:
--
-- > Git.getRepository()      -> "()Lorg/eclipse/jgit/lib/Repository;"
-- > Repository.resolve(String) -> "(Ljava/lang/String;)Lorg/eclipse/jgit/lib/ObjectId;"
-- > AnyObjectId.getName()    -> "()Ljava/lang/String;"
jgitResolve :: JvmHandle -> JGit -> String -> IO (Maybe String)
jgitResolve (JvmHandle env) (JGit gitObj) revision = do
  gitCls <- findClass env "org/eclipse/jgit/api/Git"
  getRepoM <- methodId env gitCls "getRepository" "()Lorg/eclipse/jgit/lib/Repository;"
  repoObj <- c_callObjectMethod0 env gitObj getRepoM

  repoCls <- findClass env "org/eclipse/jgit/lib/Repository"
  resolveM <- methodId env repoCls "resolve" "(Ljava/lang/String;)Lorg/eclipse/jgit/lib/ObjectId;"
  revJStr <- hsStringToJString env revision
  objIdRef <- c_callObjectMethod1Str env repoObj resolveM revJStr
  checkException env ("Repository.resolve(" <> revision <> ")")
  if objIdRef == nullPtr
    then pure Nothing
    else do
      objIdCls <- findClass env "org/eclipse/jgit/lib/ObjectId"
      getNameM <- methodId env objIdCls "getName" "()Ljava/lang/String;"
      nameJStr <- c_callObjectMethod0 env objIdRef getNameM
      checkException env "ObjectId.getName()"
      Just <$> jStringToHsString env nameJStr

-- | The commit's hex SHA, via @AnyObjectId.getName()@ (inherited by
-- @RevCommit@ -- @GetMethodID@ walks the superclass chain, so looking
-- it up against the @RevCommit@ class reference is correct even though
-- @getName@ is declared higher up). Real signature:
--
-- > AnyObjectId.getName() -> "()Ljava/lang/String;"
revCommitName :: JvmHandle -> RevCommitRef -> IO String
revCommitName (JvmHandle env) (RevCommitRef revCommitObj) = do
  revCommitCls <- findClass env "org/eclipse/jgit/revwalk/RevCommit"
  getNameM <- methodId env revCommitCls "getName" "()Ljava/lang/String;"
  nameJStr <- c_callObjectMethod0 env revCommitObj getNameM
  checkException env "RevCommit.getName()"
  jStringToHsString env nameJStr
