{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE ScopedTypeVariables #-}

-- | A minimal atproto/XRPC client: handle/DID resolution, session auth,
-- and record read/write. Built 2026-09-04 per
-- @written-world/dev-journal/2026-09-04-atproto-discovery-no-knot-needed.md@ --
-- read that entry for the design reasoning (why there is no separate
-- "knot"/pointer-record layer here: DID resolution to a PDS endpoint is
-- already the whole discovery mechanism, since our payload is small
-- DMML text that lives directly in an ordinary atproto record).
--
-- Deliberate dependency choice: this shells out to @curl@ via
-- "System.Process" rather than pulling in an HTTP client library.
-- This sandbox has no HTTP client installed and no working path to
-- Hackage to fetch one (see @cabal.project.local@); shelling out also
-- avoids adding an HTTP+TLS dependency stack before it is actually
-- needed, matching Jason's own "don't weigh dmml-hs down" concern.
-- Real, disclosed limit: this will not carry to the Android JNI bridge
-- unmodified (no @curl@ binary there) -- a Phase F follow-up, not
-- solved here.
--
-- did:web resolution is NOT implemented -- only did:plc, via
-- @plc.directory@. A real, disclosed gap, not silently mishandled: a
-- did:web identifier will fail with 'UnsupportedDidMethod'.
module DMML.Atproto
  ( AtprotoError (..)
  , Session (..)
  , resolveHandle
  , resolveDidToPdsEndpoint
  , createSession
  , createRecord
  , deleteRecord
  , listRecords
  , commitRecord
  ) where

import Control.Concurrent (forkIO)
import Control.Concurrent.MVar (newEmptyMVar, putMVar, takeMVar)
import Control.Exception (SomeException, try)
import Data.Aeson (Value, (.:), (.=))
import qualified Data.Aeson as Aeson
import Data.Aeson.Key (fromText)
import qualified Data.Aeson.Types as AesonT (parseEither)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.List (isPrefixOf)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import System.Exit (ExitCode (..))
import System.IO (hClose)
import System.Process
  ( CreateProcess (..)
  , StdStream (..)
  , proc
  , waitForProcess
  , withCreateProcess
  )

data AtprotoError
  = CurlLaunchFailed Text
  | HttpFailed Int BL.ByteString
  -- ^ curl exit code (non-zero means an actual transport error; a real
  -- HTTP error status is reported the same way, since @--fail-with-body@
  -- makes curl exit non-zero on 4xx\/5xx while still capturing the body)
  -- and whatever body curl did manage to print.
  | ResponseNotJson BL.ByteString
  | ResponseMissingField Text BL.ByteString
  | UnsupportedDidMethod Text
  | NoAtprotoPdsService Value
  deriving (Eq, Show)

data Session = Session
  { sessionDid :: Text
  , sessionAccessJwt :: Text
  , sessionPdsEndpoint :: Text
  }
  deriving (Eq, Show)

-- | Run curl with an explicit argument list (never a shell string --
-- this is the whole point: arguments are passed as real argv entries,
-- so a DID or handle containing shell metacharacters can never be
-- interpreted by a shell, because there is no shell in this path at all).
-- | Fixed curl flags added 2026-09-04 (jedelman/dmml#7), after a real,
-- flaky-link environment (0.1Mbps, heavy packet loss) exposed that
-- 'runCurl' previously set no timeout of any kind -- a stalled TCP
-- connection would hang this process indefinitely rather than failing
-- fast. @--connect-timeout 15@: generous for a slow-but-alive link
-- (this project's XRPC calls are all small JSON, never a large
-- transfer, so a slow-to-connect-but-working link should still get
-- there); @--max-time 60@: same reasoning, an upper bound on the WHOLE
-- request rather than just connect, since a stall can happen
-- mid-transfer too. @--retry 3 --retry-delay 2 --retry-connrefused@:
-- curl's own built-in retry -- covers a transient drop\/reset\/refused-
-- connection without this module needing its own retry loop; does NOT
-- retry on a real HTTP 4xx\/5xx (curl's documented behavior -- those are
-- real application errors, not transport flakiness, and retrying one
-- wouldn't help). Every call in this module goes through 'runCurl', so
-- every XRPC call (resolve, session, read, write) gets this for free.
curlNetworkFlags :: [String]
curlNetworkFlags =
  [ "--connect-timeout"
  , "15"
  , "--max-time"
  , "60"
  , "--retry"
  , "3"
  , "--retry-delay"
  , "2"
  , "--retry-connrefused"
  ]

runCurl :: [String] -> IO (Either AtprotoError BL.ByteString)
runCurl args = do
  result <-
    try
      ( withCreateProcess
          (proc "curl" (["-sS", "--fail-with-body"] ++ curlNetworkFlags ++ args))
            { std_in = NoStream
            , std_out = CreatePipe
            , std_err = CreatePipe
            }
          $ \_ mout merr ph -> do
            case (mout, merr) of
              (Just outH, Just errH) -> do
                -- Strict reads, not lazy Data.ByteString.Lazy.hGetContents:
                -- a lazy read isn't actually forced by anything here, so
                -- an earlier version of this function closed both
                -- handles (and let the process exit) before the lazy
                -- ByteString's thunks had been demanded at all --
                -- "hGetBufSome: illegal operation (handle is closed)"
                -- the moment something downstream (e.g. Aeson.decode)
                -- finally forced it. BS.hGetContents is strict: fully
                -- read before this line returns, safe to close and
                -- to wait on the process afterward.
                --
                -- Read both pipes concurrently, not sequentially: curl
                -- writing enough to stderr while stdout's pipe buffer is
                -- also full (or vice versa) would otherwise deadlock --
                -- unlikely for the small JSON bodies this module expects,
                -- but cheap enough to rule out for real rather than
                -- assume away.
                outVar <- newEmptyMVar
                errVar <- newEmptyMVar
                _ <- forkIO (BS.hGetContents outH >>= putMVar outVar)
                _ <- forkIO (BS.hGetContents errH >>= putMVar errVar)
                out <- takeMVar outVar
                err <- takeMVar errVar
                code <- waitForProcess ph
                hClose outH
                hClose errH
                pure (code, BL.fromStrict out, BL.fromStrict err)
              _ -> error "unreachable: both std streams were requested as CreatePipe"
      )
  pure $ case result of
    Left (e :: SomeException) -> Left (CurlLaunchFailed (T.pack (show e)))
    Right (ExitSuccess, out, _err) -> Right out
    Right (ExitFailure code, out, err) ->
      Left (HttpFailed code (if BL.null out then err else out))

parseJson :: BL.ByteString -> Either AtprotoError Value
parseJson body = maybe (Left (ResponseNotJson body)) Right (Aeson.decode body)

field :: (Aeson.FromJSON a) => Text -> Value -> BL.ByteString -> Either AtprotoError a
field name v raw =
  case AesonT.parseEither (Aeson.withObject "response" (.: fromText name)) v of
    Left _ -> Left (ResponseMissingField name raw)
    Right a -> Right a

-- | Resolve a handle (e.g. @alice.bsky.social@) to a DID, via the
-- public, unauthenticated Bluesky resolver. Verified live 2026-09-04
-- against a real handle -- see the dev-journal entry.
resolveHandle :: Text -> IO (Either AtprotoError Text)
resolveHandle handle = do
  result <-
    runCurl
      [ "-G"
      , "https://public.api.bsky.app/xrpc/com.atproto.identity.resolveHandle"
      , "--data-urlencode"
      , "handle=" ++ T.unpack handle
      ]
  pure $ do
    body <- result
    v <- parseJson body
    field "did" v body

-- | Resolve a DID to its declared atproto PDS service endpoint. Only
-- did:plc is implemented (via @plc.directory@) -- did:web is a real,
-- disclosed gap, not handled.
resolveDidToPdsEndpoint :: Text -> IO (Either AtprotoError Text)
resolveDidToPdsEndpoint did
  | "did:plc:" `isPrefixOf` T.unpack did = do
      result <- runCurl ["https://plc.directory/" ++ T.unpack did]
      pure $ do
        body <- result
        v <- parseJson body
        services <- field "service" v body :: Either AtprotoError [Value]
        case [ ep
             | svc <- services
             , Right ty <- [AesonT.parseEither (Aeson.withObject "svc" (.: "type")) svc]
             , (ty :: Text) == "AtprotoPersonalDataServer"
             , Right ep <- [AesonT.parseEither (Aeson.withObject "svc" (.: "serviceEndpoint")) svc]
             ] of
          (ep : _) -> Right ep
          [] -> Left (NoAtprotoPdsService v)
  | otherwise = pure (Left (UnsupportedDidMethod did))

-- | Authenticate against a resolved PDS endpoint, producing a session
-- usable for 'createRecord'. @identifier@ is a handle or DID; the
-- password is an app password (never the account's real password --
-- standard atproto convention, unrelated to anything this module
-- enforces).
createSession :: Text -> Text -> Text -> IO (Either AtprotoError Session)
createSession pdsEndpoint identifier password = do
  result <-
    runCurlWithBody
      pdsEndpoint
      "/xrpc/com.atproto.server.createSession"
      Nothing
      (Aeson.object ["identifier" .= identifier, "password" .= password])
  pure $ do
    body <- result
    v <- parseJson body
    did <- field "did" v body
    accessJwt <- field "accessJwt" v body
    Right (Session did accessJwt pdsEndpoint)

-- | POST a JSON body to an XRPC endpoint, with an optional bearer token.
runCurlWithBody :: Text -> Text -> Maybe Text -> Value -> IO (Either AtprotoError BL.ByteString)
runCurlWithBody pdsEndpoint xrpcPath maybeToken bodyValue =
  runCurl
    ( [ "-X"
      , "POST"
      , T.unpack pdsEndpoint ++ T.unpack xrpcPath
      , "-H"
      , "Content-Type: application/json"
      , "--data-raw"
      -- decodeUtf8 (real Unicode text), NOT ByteString.Lazy.Char8's
      -- unpack (a naive byte-as-codepoint widening) -- Char8.unpack
      -- would mangle any non-ASCII DMML text (accented names, etc.)
      -- once curl re-encodes this String argument via the process
      -- locale, since it never actually decodes the UTF-8 aeson wrote.
      , T.unpack (TE.decodeUtf8 (BL.toStrict (Aeson.encode bodyValue)))
      ]
        ++ maybe [] (\tok -> ["-H", "Authorization: Bearer " ++ T.unpack tok]) maybeToken
    )

-- | Write one record into the caller's own repo (the DID inside
-- 'Session'). Returns the created record's @at://@ URI.
createRecord :: Session -> Text -> Value -> IO (Either AtprotoError Text)
createRecord session collection recordValue = do
  result <-
    runCurlWithBody
      (sessionPdsEndpoint session)
      "/xrpc/com.atproto.repo.createRecord"
      (Just (sessionAccessJwt session))
      ( Aeson.object
          [ "repo" .= sessionDid session
          , "collection" .= collection
          , "record" .= recordValue
          ]
      )
  pure $ do
    body <- result
    v <- parseJson body
    field "uri" v body

-- | Permanently delete one record from the caller's own repo, by rkey
-- (the last @\/@-separated segment of its @at://@ URI). Added
-- 2026-09-04 after a real, disclosed mistake: an invalid test commit
-- was published while verifying 'createRecord', with no way to remove
-- it -- and a record that can never be corrected or retracted is a
-- real gap for any write path, not just a convenience.
deleteRecord :: Session -> Text -> Text -> IO (Either AtprotoError ())
deleteRecord session collection rkey = do
  result <-
    runCurlWithBody
      (sessionPdsEndpoint session)
      "/xrpc/com.atproto.repo.deleteRecord"
      (Just (sessionAccessJwt session))
      ( Aeson.object
          [ "repo" .= sessionDid session
          , "collection" .= collection
          , "rkey" .= rkey
          ]
      )
  pure (() <$ result)

-- | List records in a collection. Unauthenticated -- works against any
-- public repo once its PDS endpoint is known, verified live 2026-09-04
-- (see the dev-journal entry). @cursor@ pages through results, same
-- convention @listRecords@ itself uses.
listRecords :: Text -> Text -> Text -> Maybe Text -> IO (Either AtprotoError Value)
listRecords pdsEndpoint repoDid collection cursor = do
  result <-
    runCurl
      ( [ "-G"
        , T.unpack pdsEndpoint ++ "/xrpc/com.atproto.repo.listRecords"
        , "--data-urlencode"
        , "repo=" ++ T.unpack repoDid
        , "--data-urlencode"
        , "collection=" ++ T.unpack collection
        ]
          ++ maybe [] (\c -> ["--data-urlencode", "cursor=" ++ T.unpack c]) cursor
      )
  pure (result >>= parseJson)

-- | Build a record value matching the real, existing
-- @org.jason-edelman.writtenworld.commit@ lexicon (@lexicons/org/
-- jason-edelman/writtenworld/commit.json@ in the @written-world@ repo):
-- @predicate@ and @createdAt@ are required; @dmml@ carries the literal
-- DMML source text `DMML.Fire.renderFiredCommit` already produces.
commitRecord :: Text -> Text -> Text -> Value
commitRecord predicate dmmlText createdAt =
  Aeson.object
    [ "predicate" .= predicate
    , "dmml" .= dmmlText
    , "createdAt" .= createdAt
    ]
