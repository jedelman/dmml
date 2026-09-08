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
-- Transport rewritten 2026-09-07 (@written-world@'s
-- @dev-journal/2026-09-07-jgit-canonical-single-implementation.md@):
-- this used to shell out to @curl@ via "System.Process" -- a real,
-- disclosed limit from the day it was built, since Android has no
-- @curl@ binary either. Now goes through the JVM, the same
-- embedded\/upcalled JVM 'DMML.Jgit' already needs for git operations
-- -- one canonical transport for both platforms. Every public
-- function here now takes a 'JvmHandle' as its first argument as a
-- result.
--
-- The low-level HTTP-over-JNI plumbing itself moved out to 'DMML.Http'
-- on 2026-09-08, the moment a second real consumer needed it
-- ('DMML.Llm''s OpenRouter calls) -- this module now just builds
-- request/response shapes; 'DMML.Http' itself was rewritten the same
-- day from @java.net.http.HttpClient@ to bundled OkHttp, after a real
-- on-device probe found @java.net.http.HttpClient@ does not exist on
-- Android\/ART at all (see 'DMML.Http''s module haddock). OkHttp is a
-- real jar dependency now, not a JDK-bundled freebie.
-- atproto-shaped URLs/bodies on top of that generic transport.
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
  , pullNewRecords
  ) where

import Data.Aeson (Value, (.:), (.=))
import qualified Data.Aeson as Aeson
import Data.Aeson.Key (fromText)
import qualified Data.Aeson.KeyMap as KM
import qualified Data.Aeson.Types as AesonT (parseEither)
import qualified Data.ByteString.Lazy as BL
import Data.List (isPrefixOf, sortOn)
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Http (HttpError (..), getJson, postJson)
import DMML.Jni (JvmHandle)

data AtprotoError
  = TransportError HttpError
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

runGet :: JvmHandle -> String -> [(String, String)] -> IO (Either AtprotoError BL.ByteString)
runGet jvm baseUrl queryParams = either (Left . TransportError) Right <$> getJson jvm baseUrl queryParams

runPostJson :: JvmHandle -> Text -> Text -> Maybe Text -> Value -> IO (Either AtprotoError BL.ByteString)
runPostJson jvm pdsEndpoint xrpcPath maybeToken bodyValue = do
  let url = T.unpack pdsEndpoint ++ T.unpack xrpcPath
      authHeaders = maybe [] (\tok -> [("Authorization", "Bearer " <> T.unpack tok)]) maybeToken
  either (Left . TransportError) Right <$> postJson jvm url authHeaders bodyValue

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
resolveHandle :: JvmHandle -> Text -> IO (Either AtprotoError Text)
resolveHandle jvm handle = do
  result <- runGet jvm "https://public.api.bsky.app/xrpc/com.atproto.identity.resolveHandle" [("handle", T.unpack handle)]
  pure $ do
    body <- result
    v <- parseJson body
    field "did" v body

-- | Resolve a DID to its declared atproto PDS service endpoint. Only
-- did:plc is implemented (via @plc.directory@) -- did:web is a real,
-- disclosed gap, not handled.
resolveDidToPdsEndpoint :: JvmHandle -> Text -> IO (Either AtprotoError Text)
resolveDidToPdsEndpoint jvm did
  | "did:plc:" `isPrefixOf` T.unpack did = do
      result <- runGet jvm ("https://plc.directory/" ++ T.unpack did) []
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
createSession :: JvmHandle -> Text -> Text -> Text -> IO (Either AtprotoError Session)
createSession jvm pdsEndpoint identifier password = do
  result <-
    runPostJson
      jvm
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

-- | Write one record into the caller's own repo (the DID inside
-- 'Session'). Returns the created record's @at://@ URI.
createRecord :: JvmHandle -> Session -> Text -> Value -> IO (Either AtprotoError Text)
createRecord jvm session collection recordValue = do
  result <-
    runPostJson
      jvm
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
deleteRecord :: JvmHandle -> Session -> Text -> Text -> IO (Either AtprotoError ())
deleteRecord jvm session collection rkey = do
  result <-
    runPostJson
      jvm
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
listRecords :: JvmHandle -> Text -> Text -> Text -> Maybe Text -> IO (Either AtprotoError Value)
listRecords jvm pdsEndpoint repoDid collection cursor = do
  result <-
    runGet
      jvm
      (T.unpack pdsEndpoint ++ "/xrpc/com.atproto.repo.listRecords")
      ( [("repo", T.unpack repoDid), ("collection", T.unpack collection)]
          ++ maybe [] (\c -> [("cursor", T.unpack c)]) cursor
      )
  pure (result >>= parseJson)

-- | Resolve a peer (handle or DID), page through its
-- @org.jason-edelman.writtenworld.commit@ collection (bounded at 50
-- pages, same limit @atproto-pull@ always used), and return every
-- record whose rkey (a real atproto TID, lexicographically ordered by
-- creation time) sorts strictly after @storedCursor@ -- sorted
-- ascending, plus the candidate next cursor (the last returned rkey,
-- or 'Nothing' if nothing new). Extracted from @app/AtprotoPull.hs@
-- (which now just calls this) so the broker orchestration
-- (@written-world@'s @cli/app/Broker.hs@) can pull records as a direct
-- in-process call too, with no subprocess spawning -- required for
-- Android, and simpler on desktop besides. Retries a failing page up
-- to 3 times before giving up on it and returning whatever was already
-- fetched (jedelman/dmml#7's fix, preserved here).
--
-- Does NOT write anything to disk or advance any cursor itself -- same
-- discipline @atproto-pull@ always had: the caller decides whether this
-- batch is actually incorporated (e.g. after validation) before
-- persisting the returned cursor anywhere.
pullNewRecords :: JvmHandle -> Text -> Text -> Text -> IO (Either AtprotoError (Maybe Text, [(Text, Text)]))
pullNewRecords jvm peerIdentifier collection storedCursor = do
  didResult <-
    if "did:" `T.isPrefixOf` peerIdentifier
      then pure (Right peerIdentifier)
      else resolveHandle jvm peerIdentifier
  case didResult of
    Left err -> pure (Left err)
    Right did -> do
      pdsResult <- resolveDidToPdsEndpoint jvm did
      case pdsResult of
        Left err -> pure (Left err)
        Right pdsEndpoint -> do
          allRecords <- pageAll pdsEndpoint did Nothing maxPages
          let new = sortOn fst [r | r@(rkey, _) <- allRecords, rkey > storedCursor]
          pure (Right (if null new then Nothing else Just (fst (last new)), new))
  where
    maxPages :: Int
    maxPages = 50

    pageRetries :: Int
    pageRetries = 3

    pageAll :: Text -> Text -> Maybe Text -> Int -> IO [(Text, Text)]
    pageAll _ _ _ 0 = pure []
    pageAll pdsEndpoint did cursor pagesLeft = do
      result <- fetchPageWithRetries pdsEndpoint did cursor pageRetries
      case result of
        Nothing -> pure []
        Just v -> do
          let records = extractRecords v
              nextCursor = extractCursor v
          rest <- case nextCursor of
            Just _ | not (null records) -> pageAll pdsEndpoint did nextCursor (pagesLeft - 1)
            _ -> pure []
          pure (records ++ rest)

    fetchPageWithRetries :: Text -> Text -> Maybe Text -> Int -> IO (Maybe Value)
    fetchPageWithRetries pdsEndpoint did cursor attemptsLeft = do
      result <- listRecords jvm pdsEndpoint did collection cursor
      case result of
        Right v -> pure (Just v)
        Left _ | attemptsLeft > 1 -> fetchPageWithRetries pdsEndpoint did cursor (attemptsLeft - 1)
        Left _ -> pure Nothing

    extractCursor :: Value -> Maybe Text
    extractCursor (Aeson.Object o) = case KM.lookup "cursor" o of
      Just (Aeson.String s) -> Just s
      _ -> Nothing
    extractCursor _ = Nothing

    extractRecords :: Value -> [(Text, Text)]
    extractRecords (Aeson.Object o) = case KM.lookup "records" o of
      Just (Aeson.Array arr) -> [r | Just r <- map recordFromValue (foldr (:) [] arr)]
      _ -> []
    extractRecords _ = []

    recordFromValue :: Value -> Maybe (Text, Text)
    recordFromValue (Aeson.Object o) = do
      Aeson.String uri <- KM.lookup "uri" o
      Aeson.Object value <- KM.lookup "value" o
      Aeson.String dmml <- KM.lookup (fromText "dmml") value
      let rkey = last (T.splitOn "/" uri)
      pure (rkey, dmml)
    recordFromValue _ = Nothing

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
