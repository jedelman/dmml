{-# LANGUAGE ForeignFunctionInterface #-}

-- | Generic, reusable JNI plumbing -- JVM lifecycle, class\/method
-- lookup, and calls by shape (arity + arg\/return kind) -- shared by
-- every typed wrapper module that needs to call into the JVM
-- ('DMML.Jgit' for JGit, the JVM-backed HTTP client this module was
-- split out to let 'DMML.Atproto' share). Extracted from what was
-- originally all inside 'DMML.Jgit' the moment a second, unrelated
-- consumer (real HTTP calls via @java.net.http@, replacing
-- 'DMML.Atproto''s @curl@-shelling -- see @written-world@'s
-- @dev-journal/2026-09-07-jgit-canonical-single-implementation.md@)
-- needed the exact same primitives -- not duplicated a second time.
--
-- Nothing here is specific to any one JVM class or method; each typed
-- wrapper module still holds its own JNI signature strings exactly
-- once, right beside the Haskell function that uses them, per
-- 'DMML.Jgit''s own documented discipline.
module DMML.Jni
  ( JRef
  , JNIEnvPtr
  , JVMPtr
  , JvmHandle (..)
  , JvmEnvironment (..)
  , withJvm
  , withEmbeddedJvm
  , findClass
  , methodId
  , staticMethodId
  , hsStringToJString
  , jStringToHsString
  , describeAndClearException
    -- * Raw JNI call primitives, by shape
  , c_newObject0
  , c_newObject1Obj
  , c_callStaticObjectMethod0
  , c_callStaticObjectMethod1Obj
  , c_callStaticObjectMethod1Bool
  , c_callStaticObjectMethod1Long
  , c_callStaticObjectMethod2Obj
  , c_callObjectMethod0
  , c_callObjectMethod1Obj
  , c_callObjectMethod1Str
  , c_callObjectMethod2Str
  , c_callObjectMethod2Obj
  , c_callIntMethod0
  ) where

import Control.Exception (bracket)
import Foreign.C.String (CString, newCString, peekCString, withCString)
import Foreign.C.Types (CInt (..), CLLong (..), CUChar (..))
import Foreign.Marshal.Alloc (alloca, free)
import Foreign.Ptr (Ptr, nullPtr)
import Foreign.Storable (peek)

-- | An opaque JNI handle -- @jobject@\/@jclass@\/@jmethodID@\/@JNIEnv*@
-- are all pointer-sized opaque values at the C ABI level on every JVM
-- this project targets (HotSpot on desktop, ART's own JNI-compatible
-- layer on Android); never dereferenced from Haskell, only threaded
-- back into the @hs_jni_*@ primitives that produced or expect them.
type JRef = Ptr ()

type JNIEnvPtr = JRef
type JVMPtr = JRef

-- | A live embedded JVM's environment pointer, valid only inside
-- 'withEmbeddedJvm'. Constructor exported for other modules in this
-- package to pattern-match on ('DMML.Jgit', the HTTP client) -- an
-- external consumer only ever sees the type via those modules' own,
-- narrower export lists.
newtype JvmHandle = JvmHandle JNIEnvPtr

-- | The one real platform incompatibility every module built on top of
-- 'JvmHandle' (@DMML.Jgit@, @DMML.Http@, @DMML.Llm@, and anything
-- built on those) needs abstracted away, added 2026-09-08 once the
-- Android side of "one canonical implementation" was finally being
-- built rather than just disclosed as a gap. The two cases:
--
-- * 'EmbeddedJvm': desktop\/CLI. Nothing hosts a JVM for us, so we
--   create one ourselves via the JNI Invocation API
--   ('JNI_CreateJavaVM') and own its whole lifecycle -- must destroy
--   it when done. This is exactly 'withEmbeddedJvm' below, unchanged.
-- * 'UpcallJvm': Android. The JVM already exists (it's what's hosting
--   the Kotlin app that called into this native library in the first
--   place) -- a JNI-exported Haskell function receives that JVM's real
--   @JNIEnv*@ as its own first parameter, per ordinary JNI calling
--   convention, whether the export is bound by @Java_pkg_Class_method@
--   name-mangling or via @JNI_OnLoad@\'s @RegisterNatives@ (this
--   project's 'DMML.AndroidBridge' uses the latter, see its own doc
--   comment). Wrapping that pointer is the WHOLE job -- calling
--   'JNI_CreateJavaVM' would be wrong (a process hosts exactly one
--   JVM) and there is nothing to destroy afterward either: the caller
--   (Android\/ART) owns that JVM's entire lifecycle, not us.
--
-- Every consumer downstream of this type should go through 'withJvm',
-- not call 'withEmbeddedJvm' directly -- that's the actual point of
-- this abstraction existing: the exact same 'DMML.Jgit'\/'DMML.Atproto'\/
-- 'DMML.Llm' call sites work unmodified under either environment, since
-- none of them ever see how the 'JvmHandle' they were given came to be.
data JvmEnvironment
  = EmbeddedJvm FilePath
  -- ^ Desktop\/CLI: the classpath to embed a fresh JVM with.
  | UpcallJvm JNIEnvPtr
  -- ^ Android: the real @JNIEnv*@ the upcall was invoked with. Never
  -- created or destroyed by this module -- purely borrowed for the
  -- duration of @action@.
  deriving (Show)

-- | Runs @action@ with a valid 'JvmHandle' for either environment,
-- handling create\/destroy lifecycle when (and only when) this code
-- actually owns it ('EmbeddedJvm') and doing nothing extra otherwise
-- ('UpcallJvm') -- see 'JvmEnvironment'\'s own doc comment for why
-- those are the only two cases and why they need different lifecycle
-- handling at all.
withJvm :: JvmEnvironment -> (JvmHandle -> IO a) -> IO a
withJvm (EmbeddedJvm classpath) action = withEmbeddedJvm classpath action
withJvm (UpcallJvm envPtr) action = action (JvmHandle envPtr)

-- Desktop\/CLI only. Embeds a fresh JVM via the JNI Invocation API,
-- runs the action with a valid 'JvmHandle', tears the JVM down after --
-- exactly the mechanism dmml-hs\/spikes\/jvm-embed\/ proved works from a
-- real GHC-compiled binary. Prefer 'withJvm' (with an 'EmbeddedJvm')
-- at new call sites -- this is kept as the underlying primitive, not
-- because it should be called directly going forward.
withEmbeddedJvm :: FilePath -> (JvmHandle -> IO a) -> IO a
withEmbeddedJvm classpath action =
  alloca $ \jvmPtrPtr ->
    withCString classpath $ \cClasspath ->
      bracket
        (do
          env <- c_createJvm jvmPtrPtr cClasspath
          if env == nullPtr
            then ioError (userError "DMML.Jni: JNI_CreateJavaVM failed")
            else pure env)
        (const (peek jvmPtrPtr >>= c_destroyJvm))
        (action . JvmHandle)

foreign import ccall safe "hs_jgit_create_jvm"
  c_createJvm :: Ptr JVMPtr -> CString -> IO JNIEnvPtr

foreign import ccall safe "hs_jgit_destroy_jvm"
  c_destroyJvm :: JVMPtr -> IO ()

foreign import ccall unsafe "hs_jni_find_class"
  c_findClass :: JNIEnvPtr -> CString -> IO JRef

foreign import ccall unsafe "hs_jni_get_method_id"
  c_getMethodId :: JNIEnvPtr -> JRef -> CString -> CString -> IO JRef

foreign import ccall unsafe "hs_jni_get_static_method_id"
  c_getStaticMethodId :: JNIEnvPtr -> JRef -> CString -> CString -> IO JRef

foreign import ccall safe "hs_jni_new_object_0"
  c_newObject0 :: JNIEnvPtr -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_new_object_1obj"
  c_newObject1Obj :: JNIEnvPtr -> JRef -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_static_object_method_0"
  c_callStaticObjectMethod0 :: JNIEnvPtr -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_static_object_method_1obj"
  c_callStaticObjectMethod1Obj :: JNIEnvPtr -> JRef -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_static_object_method_1bool"
  c_callStaticObjectMethod1Bool :: JNIEnvPtr -> JRef -> JRef -> CUChar -> IO JRef

foreign import ccall safe "hs_jni_call_static_object_method_1long"
  c_callStaticObjectMethod1Long :: JNIEnvPtr -> JRef -> JRef -> CLLong -> IO JRef

foreign import ccall safe "hs_jni_call_static_object_method_2obj"
  c_callStaticObjectMethod2Obj :: JNIEnvPtr -> JRef -> JRef -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_object_method_0"
  c_callObjectMethod0 :: JNIEnvPtr -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_object_method_1obj"
  c_callObjectMethod1Obj :: JNIEnvPtr -> JRef -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_object_method_1str"
  c_callObjectMethod1Str :: JNIEnvPtr -> JRef -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_object_method_2str"
  c_callObjectMethod2Str :: JNIEnvPtr -> JRef -> JRef -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_object_method_2obj"
  c_callObjectMethod2Obj :: JNIEnvPtr -> JRef -> JRef -> JRef -> JRef -> IO JRef

foreign import ccall safe "hs_jni_call_int_method_0"
  c_callIntMethod0 :: JNIEnvPtr -> JRef -> JRef -> IO CInt

foreign import ccall unsafe "hs_jni_new_string_utf"
  c_newStringUtf :: JNIEnvPtr -> CString -> IO JRef

foreign import ccall unsafe "hs_jni_get_string_utf_chars_copy"
  c_getStringUtfCharsCopy :: JNIEnvPtr -> JRef -> IO CString

foreign import ccall unsafe "hs_jni_exception_check"
  c_exceptionCheck :: JNIEnvPtr -> IO CUChar

foreign import ccall unsafe "hs_jni_describe_and_clear_exception"
  c_describeAndClearExceptionRaw :: JNIEnvPtr -> IO CString

-- | 'Just' a description of the pending Java exception (its own
-- @toString()@, read back via JNI) if one exists, clearing it; else
-- 'Nothing'. Non-throwing -- callers that want a Haskell exception
-- instead (like 'DMML.Jgit') build their own on top of this; callers
-- that want an @Either@ (like the HTTP client, matching
-- 'DMML.Atproto''s existing idiom) use it directly.
describeAndClearException :: JNIEnvPtr -> IO (Maybe String)
describeAndClearException env = do
  pending <- c_exceptionCheck env
  if pending == 0
    then pure Nothing
    else do
      descC <- c_describeAndClearExceptionRaw env
      if descC == nullPtr
        then pure (Just "(no description)")
        else Just <$> peekCString descC

hsStringToJString :: JNIEnvPtr -> String -> IO JRef
hsStringToJString env s = withCString s (c_newStringUtf env)

jStringToHsString :: JNIEnvPtr -> JRef -> IO String
jStringToHsString env jstr = do
  cstr <- c_getStringUtfCharsCopy env jstr
  if cstr == nullPtr
    then ioError (userError "DMML.Jni: GetStringUTFChars returned null")
    else do
      s <- peekCString cstr
      free cstr
      pure s

-- Small helpers so every call site reads as (env, class, name, sig)
-- instead of manual newCString plumbing -- these free their temporary
-- C strings themselves; the returned jclass/jmethodID handles are the
-- only things that outlive the call, per ordinary JNI rules.
findClass :: JNIEnvPtr -> String -> IO JRef
findClass env name = do
  cName <- newCString name
  cls <- c_findClass env cName
  free cName
  if cls == nullPtr
    then ioError (userError ("DMML.Jni: FindClass failed for " <> name))
    else pure cls

methodId :: JNIEnvPtr -> JRef -> String -> String -> IO JRef
methodId env cls name sig = do
  cName <- newCString name
  cSig <- newCString sig
  m <- c_getMethodId env cls cName cSig
  free cName
  free cSig
  if m == nullPtr
    then ioError (userError ("DMML.Jni: GetMethodID failed for " <> name <> " " <> sig))
    else pure m

staticMethodId :: JNIEnvPtr -> JRef -> String -> String -> IO JRef
staticMethodId env cls name sig = do
  cName <- newCString name
  cSig <- newCString sig
  m <- c_getStaticMethodId env cls cName cSig
  free cName
  free cSig
  if m == nullPtr
    then ioError (userError ("DMML.Jni: GetStaticMethodID failed for " <> name <> " " <> sig))
    else pure m
