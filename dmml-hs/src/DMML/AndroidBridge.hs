{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ForeignFunctionInterface #-}
{-# LANGUAGE ScopedTypeVariables #-}

-- | The real Android bridge surface for everything built 2026-09-07/08
-- on top of a live JNI environment -- git (via 'DMML.Jgit'), atproto
-- sync (via 'DMML.Atproto'), and BYOK authoring (via 'DMML.Llm').
-- 'DMML.JniBridge' already covers the ORIGINAL, JVM-object-free v1
-- surface (render\/actions\/fire -- pure Haskell logic, plain
-- 'Foreign.C.String.CString' in and out, no real @JNIEnv*@ needed at
-- all). Everything in THIS module is different: each function needs a
-- live @JNIEnv*@ to call real Java objects (JGit's @Git@, @java.net.
-- http.HttpClient@) the same way the desktop CLI does via an embedded
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
-- NOT YET COVERED here: written-world's own broker orchestration
-- (@cli\/app\/Broker.hs@'s @incorporate@ -- validate + git commit +
-- divergence report + checkpoint fold as one unit) lives in
-- @written-world@, not @dmml-hs@, and needs its own, parallel Android
-- entry point built the same way once an app exists to call it. This
-- module covers the PRIMITIVES that orchestration is built from
-- (commit, pull, chat) -- proven individually, not yet wired together
-- behind one Android call.
module DMML.AndroidBridge
  ( android_jgit_commit
  , android_atproto_resolve
  , android_atproto_pull
  , android_atproto_create_session
  , android_atproto_create_record
  , android_llm_chat_complete
  , jgitCommitBridge
  , atprotoResolveBridge
  , atprotoPullBridge
  , atprotoCreateSessionBridge
  , atprotoCreateRecordBridge
  , llmChatCompleteBridge
  ) where

import Control.Exception (SomeException, displayException, try)
import Data.Aeson (object, (.=))
import qualified Data.Aeson as Aeson
import qualified Data.ByteString.Lazy as BL
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Time.Clock (getCurrentTime)
import Data.Time.Format (defaultTimeLocale, formatTime)
import Foreign.C.String (CString, newCString, peekCString)

import DMML.Atproto
  ( Session (..)
  , commitRecord
  , createRecord
  , createSession
  , pullNewRecords
  , resolveDidToPdsEndpoint
  , resolveHandle
  )
import DMML.Jni (JNIEnvPtr, JvmEnvironment (UpcallJvm), withJvm)
import DMML.Jgit (jgitAddFilepattern, jgitCommit, jgitOpen, revCommitName)
import DMML.Llm (chatComplete)
import System.Directory (createDirectoryIfMissing)
import System.FilePath (takeDirectory, (</>))
import qualified Data.Text.IO as TIO

foreign export ccall android_jgit_commit :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString
foreign export ccall android_atproto_resolve :: JNIEnvPtr -> CString -> IO CString
foreign export ccall android_atproto_pull :: JNIEnvPtr -> CString -> CString -> CString -> IO CString
foreign export ccall android_atproto_create_session :: JNIEnvPtr -> CString -> CString -> CString -> IO CString
foreign export ccall android_atproto_create_record :: JNIEnvPtr -> CString -> CString -> CString -> CString -> CString -> CString -> IO CString
foreign export ccall android_llm_chat_complete :: JNIEnvPtr -> CString -> CString -> CString -> CString -> IO CString

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
