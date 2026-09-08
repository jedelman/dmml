{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ForeignFunctionInterface #-}
{-# LANGUAGE ScopedTypeVariables #-}
{-# LANGUAGE TypeApplications #-}

-- | The real Android bridge surface for everything built 2026-09-07/08
-- on top of a live JNI environment -- git (via 'DMML.Jgit'), atproto
-- sync (via 'DMML.Atproto'), and BYOK authoring (via 'DMML.Llm').
-- 'DMML.JniBridge' already covers the ORIGINAL, JVM-object-free v1
-- surface (render\/actions\/fire -- pure Haskell logic, plain
-- 'Foreign.C.String.CString' in and out, no real @JNIEnv*@ needed at
-- all). Everything in THIS module is different: each function needs a
-- live @JNIEnv*@ to call real Java objects (JGit's @Git@, OkHttp's
-- @OkHttpClient@) the same way the desktop CLI does via an embedded
-- JVM -- the only difference on Android is WHERE that @JNIEnv*@ comes
-- from, which is exactly what 'DMML.Jni.JvmEnvironment'\'s 'UpcallJvm'
-- constructor exists to abstract away (see its own doc comment). Every
-- function here takes the upcall's real @JNIEnv*@ as its own first
-- parameter -- ordinary JNI calling convention -- and does
-- @withJvm (UpcallJvm envPtr) $ \\jvm -> ...@ internally; NONE of them
-- create or destroy a JVM, unlike every desktop entry point.
--
-- Binding: these are plain, unmangled C symbol names (@android_jgit_commit@,
-- not @Java_org_..._nativeJgitCommit@) -- deliberately, since this
-- module was written before any real Android app package\/class name
-- existed to mangle against. The intended binding mechanism is
-- @JNI_OnLoad@\'s @RegisterNatives@ (see @cbits\/android_onload.c@),
-- which maps an arbitrary C symbol to a declared Kotlin @external fun@
-- by an explicit table, not by name-mangling -- the same shape this
-- project's own August \"hatter\"-pattern reference used. A real
-- Android app is free to rename the Kotlin-side class\/methods in that
-- table without touching a line here.
--
-- Every exported function wraps its entire body in 'Control.Exception.try'
-- and marshals ANY failure back as a plain @\"ERROR: ...\"@-prefixed
-- 'CString' -- same discipline 'DMML.JniBridge' already established,
-- required here for the identical reason (no process isolation on this
-- path; an uncaught exception crossing the FFI boundary is undefined
-- behavior, not a clean nonzero exit).
--
-- Broker orchestration: @android_broker_incorporate@, below, ports
-- @written-world@'s own @cli\/app\/Broker.hs@ @incorporate@ (validate +
-- git commit + divergence report + checkpoint fold as one unit,
-- itself the real Haskell replacement for
-- @sync-spike\/broker\/atproto-broker.sh@'s bash+subprocess
-- orchestration -- subprocess spawning doesn't work on Android at
-- all) onto 'UpcallJvm', the same adaptation every other function in
-- this module already makes for its own desktop counterpart. Ported
-- from @written-world@ commit 317d179 (branch
-- @claude\/written-world-dmml-enrichment-257mkv@) 2026-09-08 -- a
-- separate repo\/language boundary from this one, so this is a real
-- adaptation, not a shared import; kept behavior-equivalent
-- (including the original's own @jgitResolve ... \"HEAD:commits\"@
-- hardcoding regardless of the actual @commitsDir@ argument, and its
-- checkpoint-folds-only-when-commitsDir-is-literally-\"commits\"
-- restriction) with one real, necessary difference: the desktop
-- original prints its divergence report to stdout and calls
-- 'System.Exit.exitFailure' on validation rejection -- neither works
-- across an FFI boundary with no attached console, so this version
-- returns the divergence report as structured JSON and a rejection as
-- a normal @Left@, marshaled the same @\"ERROR: ...\"@ way as
-- everything else here.
module DMML.AndroidBridge
  ( android_jgit_commit
  , android_atproto_resolve
  , android_atproto_pull
  , android_atproto_create_session
  , android_atproto_create_record
  , android_llm_chat_complete
  , android_broker_incorporate
  , jgitCommitBridge
  , atprotoResolveBridge
  , atprotoPullBridge
  , atprotoCreateSessionBridge
  , atprotoCreateRecordBridge
  , llmChatCompleteBridge
  , brokerIncorporateBridge
  ) where

import Control.Exception (SomeException, catch, displayException, try)
import Data.Aeson (object, (.=))
import qualified Data.Aeson as Aeson
import qualified Data.ByteString.Lazy as BL
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Time.Clock (getCurrentTime)
import Data.Time.Format (defaultTimeLocale, formatTime)
import Foreign.C.String (CString, newCString, peekCString)

import DMML.Ast (Literal (..), MachineStmt (machineNode), NodeRef (nodeRefSegments), Value (..))
import DMML.Atproto
  ( AtprotoError
  , Session (..)
  , commitRecord
  , createRecord
  , createSession
  , pullNewRecords
  , resolveDidToPdsEndpoint
  , resolveHandle
  )
import DMML.Checkpoint (resolveAndFoldCheckpoint)
import DMML.Governance (applyGovernance)
import DMML.Jni (JNIEnvPtr, JvmEnvironment (UpcallJvm), JvmHandle, withJvm)
import DMML.Jgit (JGit, jgitAddFilepattern, jgitCommit, jgitOpen, jgitResolve, revCommitName)
import DMML.Llm (chatComplete)
import DMML.Materialize (WorldSnapshot (..), applyCommits, currentValue, mergeSnapshots)
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import System.Directory (createDirectoryIfMissing, doesFileExist, listDirectory)
import System.FilePath (takeDirectory, takeExtension, (</>))
import Text.Megaparsec (errorBundlePretty)
import qualified Data.Text.IO as TIO

foreign export ccall android_jgit_commit :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString
foreign export ccall android_atproto_resolve :: JNIEnvPtr -> CString -> IO CString
foreign export ccall android_atproto_pull :: JNIEnvPtr -> CString -> CString -> CString -> IO CString
foreign export ccall android_atproto_create_session :: JNIEnvPtr -> CString -> CString -> CString -> IO CString
foreign export ccall android_atproto_create_record :: JNIEnvPtr -> CString -> CString -> CString -> CString -> CString -> CString -> IO CString
foreign export ccall android_llm_chat_complete :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString
foreign export ccall android_broker_incorporate :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString

-- | Writes @content@ to @repoDir\/relPath@ (creating parent directories
-- as needed), @git add@s and commits it via 'DMML.Jgit' against the
-- upcall's own live @JNIEnv*@ -- the exact same operation
-- @written-world fire@\'s @jgitAddCommit@ performs on desktop, just
-- reached through 'UpcallJvm' instead of 'DMML.Jgit.withEmbeddedJvm'.
-- Returns @\"OK:\<commit-sha\>\"@ on success.
jgitCommitBridge :: JNIEnvPtr -> FilePath -> FilePath -> Text -> String -> IO (Either String String)
jgitCommitBridge envPtr repoDir relPath content message =
  withJvm (UpcallJvm envPtr) $ \jvm -> do
    let path = repoDir </> relPath
    createDirectoryIfMissing True (takeDirectory path)
    TIO.writeFile path content
    git <- jgitOpen jvm repoDir
    jgitAddFilepattern jvm git relPath
    rev <- jgitCommit jvm git message
    sha <- revCommitName jvm rev
    pure (Right ("OK:" <> sha))

-- | Resolve a handle-or-DID all the way to its real PDS endpoint --
-- the same two-step chain @atproto-resolve@ does on desktop. Returns
-- JSON @{\"did\":...,\"pdsEndpoint\":...}@.
atprotoResolveBridge :: JNIEnvPtr -> Text -> IO (Either String Text)
atprotoResolveBridge envPtr identifier =
  withJvm (UpcallJvm envPtr) $ \jvm -> do
    didResult <-
      if "did:" `T.isPrefixOf` identifier
        then pure (Right identifier)
        else resolveHandle jvm identifier
    case didResult of
      Left err -> pure (Left (show err))
      Right did -> do
        pdsResult <- resolveDidToPdsEndpoint jvm did
        case pdsResult of
          Left err -> pure (Left (show err))
          Right pdsEndpoint ->
            pure (Right (jsonText (object ["did" .= did, "pdsEndpoint" .= pdsEndpoint])))

-- | Pull a peer's new commit records since @storedCursor@ -- the same
-- 'DMML.Atproto.pullNewRecords' the desktop broker uses. Returns JSON
-- @{\"nextCursor\":...|null,\"records\":[{\"rkey\":...,\"dmml\":...}]}@;
-- the caller (Kotlin) is responsible for validating\/writing\/
-- committing each record, same division of labor as
-- @written-world@\'s own @Broker.hs@.
atprotoPullBridge :: JNIEnvPtr -> Text -> Text -> Text -> IO (Either String Text)
atprotoPullBridge envPtr peerIdentifier collection storedCursor =
  withJvm (UpcallJvm envPtr) $ \jvm -> do
    result <- pullNewRecords jvm peerIdentifier collection storedCursor
    pure $ case result of
      Left err -> Left (show err)
      Right (nextCursor, records) ->
        Right
          ( jsonText
              ( object
                  [ "nextCursor" .= nextCursor
                  , "records" .= [object ["rkey" .= rkey, "dmml" .= dmml] | (rkey, dmml) <- records]
                  ]
              )
          )

-- | Authenticate against a resolved PDS endpoint. Returns JSON
-- @{\"did\":...,\"accessJwt\":...,\"pdsEndpoint\":...}@ -- the caller
-- holds onto this for subsequent 'atprotoCreateRecordBridge' calls
-- (no session object crosses the FFI boundary, just its three plain
-- fields).
atprotoCreateSessionBridge :: JNIEnvPtr -> Text -> Text -> Text -> IO (Either String Text)
atprotoCreateSessionBridge envPtr pdsEndpoint identifier password =
  withJvm (UpcallJvm envPtr) $ \jvm -> do
    result <- createSession jvm pdsEndpoint identifier password
    pure $ case result of
      Left err -> Left (show err)
      Right session ->
        Right
          ( jsonText
              ( object
                  [ "did" .= sessionDid session
                  , "accessJwt" .= sessionAccessJwt session
                  , "pdsEndpoint" .= sessionPdsEndpoint session
                  ]
              )
          )

-- | Publish one DMML commit into the caller's own repo, matching the
-- real, existing @org.jason-edelman.writtenworld.commit@ lexicon
-- ('DMML.Atproto.commitRecord') -- @createdAt@ is stamped here (UTC,
-- ISO-8601), same as @atproto-publish@ does on desktop, so the caller
-- doesn't need a working wall-clock API of its own on the Kotlin side.
-- Returns the created record's @at://@ URI.
atprotoCreateRecordBridge :: JNIEnvPtr -> Text -> Text -> Text -> Text -> Text -> Text -> IO (Either String Text)
atprotoCreateRecordBridge envPtr pdsEndpoint did accessJwt collection predicate dmmlText =
  withJvm (UpcallJvm envPtr) $ \jvm -> do
    now <- getCurrentTime
    let createdAt = T.pack (formatTime defaultTimeLocale "%Y-%m-%dT%H:%M:%S%QZ" now)
        session = Session {sessionDid = did, sessionAccessJwt = accessJwt, sessionPdsEndpoint = pdsEndpoint}
        record = commitRecord predicate dmmlText createdAt
    result <- createRecord jvm session collection record
    pure (either (Left . show) Right result)

-- | One BYOK chat completion via 'DMML.Llm.chatComplete' -- the same
-- call @written-world-author@ makes on desktop, reached through
-- 'UpcallJvm'. Returns the raw assistant content; the caller validates
-- it as real DMML itself (this bridge does no DMML-specific
-- validation, matching 'DMML.Llm' proper -- that stays the caller's
-- job on both platforms).
llmChatCompleteBridge :: JNIEnvPtr -> Text -> Text -> Text -> Text -> IO (Either String Text)
llmChatCompleteBridge envPtr apiKey model systemPrompt userPrompt =
  withJvm (UpcallJvm envPtr) $ \jvm -> do
    result <- chatComplete jvm apiKey model systemPrompt userPrompt
    pure (either (Left . show) Right result)

-- | Port of @Broker.hs@'s @collection@ -- the one real lexicon this
-- project has.
brokerCollection :: Text
brokerCollection = "org.jason-edelman.writtenworld.commit"

-- | Pulls a peer's new commit records (via 'DMML.Atproto.pullNewRecords',
-- same as 'atprotoPullBridge') and, if there are any, validates the
-- whole batch (all-or-nothing, same policy as the desktop original),
-- writes each as a @.dmml@ file under @repoDir\/commitsDir@, updates
-- the cursor file, commits via JGit, computes real cross-player
-- divergence, and folds the checkpoint chain (only when @commitsDir@
-- is literally @\"commits\"@, same restriction the original has).
-- Returns JSON:
-- @{\"incorporatedCount\":N,\"nextCursor\":...,\"commitSha\":...,
--   \"divergences\":[{\"subject\":...,\"predicate\":...,
--     \"options\":[{\"label\":...,\"value\":...}]}],
--   \"checkpoint\":{\"foldedCount\":N,\"path\":...}|null}@,
-- or (nothing new) @{\"incorporatedCount\":0,\"message\":\"nothing new\"}@.
-- A validation rejection or any other failure is a normal 'Left'
-- (marshaled as an @\"ERROR: ...\"@-prefixed 'CString' by the caller),
-- never a silent partial commit -- same all-or-nothing guarantee as
-- the desktop original's @failWith@.
brokerIncorporateBridge :: JNIEnvPtr -> FilePath -> Text -> FilePath -> FilePath -> IO (Either String Text)
brokerIncorporateBridge envPtr repoDir peerIdentifier cursorFile commitsDir =
  withJvm (UpcallJvm envPtr) $ \jvm -> do
    let cursorPath = repoDir </> cursorFile
    haveCursorFile <- doesFileExist cursorPath
    storedCursor <- if haveCursorFile then T.strip <$> TIO.readFile cursorPath else pure ""
    pullResult <- pullNewRecords jvm peerIdentifier brokerCollection storedCursor
    case pullResult of
      Left err -> pure (Left (show (err :: AtprotoError)))
      Right (nextCursor, []) ->
        pure (Right (jsonText (object ["incorporatedCount" .= (0 :: Int), "message" .= ("nothing new" :: Text), "nextCursor" .= nextCursor])))
      Right (nextCursor, newRecords) -> do
        let badRkeys = [rkey | (rkey, dmml) <- newRecords, Just _ <- [shapeError dmml]]
        if not (null badRkeys)
          then pure (Left ("REJECTED -- at least one new record failed validation, not incorporating any of this batch: " <> T.unpack (T.intercalate ", " badRkeys)))
          else do
            let commitsPath = repoDir </> commitsDir
            createDirectoryIfMissing True commitsPath

            -- Snapshot MY OWN existing commits BEFORE incorporating
            -- anything new -- same reasoning as the desktop original.
            mineFilesBefore <- listDmmlFiles commitsPath

            let peerPaths = [commitsPath </> T.unpack rkey <> ".dmml" | (rkey, _) <- newRecords]
            mapM_ (\(rkey, dmml) -> TIO.writeFile (commitsPath </> T.unpack rkey <> ".dmml") dmml) newRecords
            case nextCursor of
              Nothing -> pure ()
              Just c -> TIO.writeFile cursorPath c

            git <- jgitOpen jvm repoDir
            -- Same hardcoded "HEAD:commits" as the desktop original,
            -- regardless of the actual commitsDir argument -- kept
            -- behavior-equivalent, not fixed here (see module haddock).
            preCommitTreeSha <- jgitResolve jvm git "HEAD:commits"

            jgitAddFilepattern jvm git commitsDir
            jgitAddFilepattern jvm git cursorFile
            rev <- jgitCommit jvm git ("atproto: incorporate " <> show (length peerPaths) <> " new commit(s) from " <> T.unpack peerIdentifier)
            commitSha <- revCommitName jvm rev

            divergences <- computeDivergence mineFilesBefore peerPaths

            checkpointResult <-
              if commitsDir == "commits"
                then Just <$> foldCheckpointBridge jvm git preCommitTreeSha peerPaths
                else pure Nothing

            pure
              ( Right
                  ( jsonText
                      ( object
                          [ "incorporatedCount" .= length peerPaths
                          , "nextCursor" .= nextCursor
                          , "commitSha" .= commitSha
                          , "divergences" .= divergences
                          , "checkpoint" .= checkpointResult
                          ]
                      )
                  )
              )
  where
    shapeError :: Text -> Maybe String
    shapeError src = case parseCommitSurface src of
      Right _ -> Nothing
      Left commitErr -> case parseMachineSurface src of
        Right _ -> Nothing
        Left _ -> Just (errorBundlePretty commitErr)

listDmmlFiles :: FilePath -> IO [FilePath]
listDmmlFiles dir = do
  entries <- listDirectory dir
  pure [dir </> e | e <- entries, takeExtension e == ".dmml"]

-- | Port of @Broker.hs@'s @reportDivergence@ -- same real divergence
-- computation ('DMML.Materialize.applyCommits'\/@mergeSnapshots@\/
-- @currentValue@, 'DMML.Governance.applyGovernance'), returning
-- structured JSON-encodable data instead of @putStrLn@ing it (there is
-- no console on the far side of this FFI boundary to print to).
computeDivergence :: [FilePath] -> [FilePath] -> IO [Aeson.Value]
computeDivergence minePaths peerPaths = do
  (mineSnap, mineMachines) <- materializeFiles "mine" minePaths
  (peerSnap, peerMachines) <- materializeFiles "peer" peerPaths
  let merged = mergeSnapshots mineSnap peerSnap
      machines = Map.union mineMachines peerMachines
      governed = applyGovernance machines merged
      reallyDivergent =
        [ (k, vs)
        | (k, _) <- Map.toList (snapshotFacts governed)
        , let vs = currentValue k governed
        , length vs > 1
        ]
  pure [report k vs | (k, vs) <- reallyDivergent]
  where
    materializeFiles :: Text -> [FilePath] -> IO (WorldSnapshot, Map Text MachineStmt)
    materializeFiles label paths = do
      srcs <- mapM TIO.readFile paths
      let classified = zip paths (map classify srcs)
      case [(p, e) | (p, Left e) <- classified] of
        ((p, e) : _) -> ioError (userError (p <> ":\n" <> e))
        [] ->
          let commits = [c | (_, Right (Left c)) <- classified]
              machines = [m | (_, Right (Right m)) <- classified]
              machineMap = Map.fromList [(nodeRefText (machineNode m), m) | m <- machines]
           in pure (applyCommits label commits, machineMap)

    classify src = case parseCommitSurface src of
      Right stmt -> Right (Left stmt)
      Left commitErr -> case parseMachineSurface src of
        Right machine -> Right (Right machine)
        Left _ -> Left (errorBundlePretty commitErr)

    nodeRefText = T.intercalate "/" . nodeRefSegments

    report (subj, pred_) opts =
      object
        [ "subject" .= subj
        , "predicate" .= pred_
        , "options" .= [object ["label" .= label, "value" .= renderValue v] | (label, v) <- opts]
        ]

    renderValue (ValueNode n) = T.intercalate "/" (nodeRefSegments n)
    renderValue (ValueLiteral (LitString s)) = "\"" <> s <> "\""
    renderValue (ValueLiteral (LitNumber n)) = n
    renderValue (ValueLiteral (LitBoolean b)) = if b then "true" else "false"

-- | Port of @Broker.hs@'s @foldCheckpoint@ -- same lookup-by-pre-
-- incorporation-tree-sha shape, non-fatal on failure (the sync itself
-- already succeeded; next run self-heals), returning the result as
-- JSON instead of printing it.
foldCheckpointBridge :: JvmHandle -> JGit -> Maybe String -> [FilePath] -> IO Aeson.Value
foldCheckpointBridge jvm git preCommitTreeSha newFiles =
  attempt `catch` \(e :: SomeException) ->
    pure (object ["error" .= ("checkpoint-fold failed (non-fatal -- sync already succeeded): " <> show e)])
  where
    attempt = do
      createDirectoryIfMissing True "checkpoints"
      mNewTreeSha <- jgitResolve jvm git "HEAD:commits"
      case mNewTreeSha of
        Nothing -> pure (object ["error" .= ("could not resolve commits/ tree at HEAD -- skipping checkpoint" :: Text)])
        Just newTreeSha -> do
          (outPath, foldedCount) <- resolveAndFoldCheckpoint "checkpoints" "commits" preCommitTreeSha newTreeSha newFiles
          jgitAddFilepattern jvm git outPath
          _ <-
            jgitCommit
              jvm
              git
              ("checkpoint: " <> newTreeSha <> " (" <> show foldedCount <> " new file(s) folded in)")
          pure (object ["foldedCount" .= foldedCount, "path" .= outPath])

android_broker_incorporate :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString
android_broker_incorporate envPtr repoDirC peerC cursorC commitsC = guardedRun $ do
  repoDir <- peekCString repoDirC
  peerIdentifier <- T.pack <$> peekCString peerC
  cursorFile <- peekCString cursorC
  commitsDir <- peekCString commitsC
  marshalText =<< brokerIncorporateBridge envPtr repoDir peerIdentifier cursorFile commitsDir

jsonText :: Aeson.Value -> Text
jsonText = TE.decodeUtf8 . BL.toStrict . Aeson.encode

marshalStr :: Either String String -> IO CString
marshalStr (Left err) = newCString ("ERROR: " <> err)
marshalStr (Right ok) = newCString ok

marshalText :: Either String Text -> IO CString
marshalText (Left err) = newCString ("ERROR: " <> err)
marshalText (Right ok) = newCString (T.unpack ok)

android_jgit_commit :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString
android_jgit_commit envPtr repoDirC relPathC contentC msgC = guardedRun $ do
  repoDir <- peekCString repoDirC
  relPath <- peekCString relPathC
  content <- T.pack <$> peekCString contentC
  message <- peekCString msgC
  marshalStr =<< jgitCommitBridge envPtr repoDir relPath content message

android_atproto_resolve :: JNIEnvPtr -> CString -> IO CString
android_atproto_resolve envPtr identC = guardedRun $ do
  identifier <- T.pack <$> peekCString identC
  marshalText =<< atprotoResolveBridge envPtr identifier

android_atproto_pull :: JNIEnvPtr -> CString -> CString -> CString -> IO CString
android_atproto_pull envPtr peerC collC cursorC = guardedRun $ do
  peerIdentifier <- T.pack <$> peekCString peerC
  collection <- T.pack <$> peekCString collC
  storedCursor <- T.pack <$> peekCString cursorC
  marshalText =<< atprotoPullBridge envPtr peerIdentifier collection storedCursor

android_atproto_create_session :: JNIEnvPtr -> CString -> CString -> CString -> IO CString
android_atproto_create_session envPtr pdsC identC passC = guardedRun $ do
  pdsEndpoint <- T.pack <$> peekCString pdsC
  identifier <- T.pack <$> peekCString identC
  password <- T.pack <$> peekCString passC
  marshalText =<< atprotoCreateSessionBridge envPtr pdsEndpoint identifier password

android_atproto_create_record :: JNIEnvPtr -> CString -> CString -> CString -> CString -> CString -> CString -> IO CString
android_atproto_create_record envPtr pdsC didC jwtC collC predC dmmlC = guardedRun $ do
  pdsEndpoint <- T.pack <$> peekCString pdsC
  did <- T.pack <$> peekCString didC
  accessJwt <- T.pack <$> peekCString jwtC
  collection <- T.pack <$> peekCString collC
  predicate <- T.pack <$> peekCString predC
  dmmlText <- T.pack <$> peekCString dmmlC
  marshalText =<< atprotoCreateRecordBridge envPtr pdsEndpoint did accessJwt collection predicate dmmlText

android_llm_chat_complete :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString
android_llm_chat_complete envPtr keyC modelC sysC userC = guardedRun $ do
  apiKey <- T.pack <$> peekCString keyC
  model <- T.pack <$> peekCString modelC
  systemPrompt <- T.pack <$> peekCString sysC
  userPrompt <- T.pack <$> peekCString userC
  marshalText =<< llmChatCompleteBridge envPtr apiKey model systemPrompt userPrompt

-- | Same exception backstop as 'DMML.JniBridge.guardedRun' -- kept as
-- a separate copy rather than a shared import since 'DMML.JniBridge'
-- exports its own version narrowly and this module has no other
-- dependency on that module.
guardedRun :: IO CString -> IO CString
guardedRun act = do
  result <- try @SomeException act
  case result of
    Right cstr -> pure cstr
    Left e -> newCString ("ERROR: " <> displayException e)
