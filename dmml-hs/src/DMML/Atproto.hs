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
-- @curl@ binary either. Now goes through @java.net.http.HttpClient@,
-- the same embedded\/upcalled JVM 'DMML.Jgit' already needs for git
-- operations -- one canonical transport for both platforms, no new
-- dependency (the JDK's HTTP client ships in @java.base@, no extra
-- jar on the classpath). Every public function here now takes a
-- 'JvmHandle' as its first argument as a result.
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

import Control.Concurrent (threadDelay)
import Data.Aeson (Value, (.:), (.=))
import qualified Data.Aeson as Aeson
import Data.Aeson.Key (fromText)
import qualified Data.Aeson.Types as AesonT (parseEither)
import Data.Bits (shiftR, (.&.))
import qualified Data.ByteString.Lazy as BL
import Data.Char (intToDigit, isAscii, isAlphaNum, ord)
import Data.List (isPrefixOf)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Word (Word8)

import DMML.Jni
  ( JRef
  , JvmHandle (..)
  , c_callIntMethod0
  , c_callObjectMethod0
  , c_callObjectMethod1Obj
  , c_callObjectMethod2Obj
  , c_callObjectMethod2Str
  , c_callStaticObjectMethod0
  , c_callStaticObjectMethod1Long
  , c_callStaticObjectMethod1Obj
  , describeAndClearException
  , findClass
  , hsStringToJString
  , jStringToHsString
  , methodId
  , staticMethodId
  )

data AtprotoError
  = HttpTransportFailed Text
  -- ^ The request never got a real HTTP response at all (DNS, connect
  -- refused/timeout, a Java exception out of @HttpClient.send@ -- see
  -- 'describeAndClearException'). Distinct from 'HttpFailed', a real
  -- non-2xx response.
  | HttpFailed Int BL.ByteString
  -- ^ A real HTTP response with a non-2xx status; the body is whatever
  -- the server actually sent back, not discarded.
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

-- | RFC 3986 unreserved-char percent-encoding over a string's UTF-8
-- bytes -- the same encoding @curl --data-urlencode@ produced, now done
-- directly since there's no @curl@ to delegate to. A DID
-- (@did:plc:...@) needs its @:@ encoded to survive as a single query
-- value, which is exactly what this does and a naive ASCII-only
-- encoder would get wrong for anything past 7-bit input.
percentEncode :: Text -> String
percentEncode = concatMap encodeByte . BL.unpack . BL.fromStrict . TE.encodeUtf8
  where
    encodeByte :: Word8 -> String
    encodeByte b
      | isUnreserved b = [toEnum (fromIntegral b)]
      | otherwise = '%' : [hexDigit (b `shiftR` 4), hexDigit (b .&. 0x0F)]
    isUnreserved b =
      let c = toEnum (fromIntegral b) :: Char
       in isAscii c && (isAlphaNum c || c `elem` ("-._~" :: String))
    hexDigit n = intToDigit' (fromIntegral n)
    intToDigit' n = if n < 10 then intToDigit n else toEnum (ord 'A' + n - 10)

-- | Build a @java.net.URI@ from an already-fully-formed URL string.
-- Real signature: @URI.create(String) -> "(Ljava/lang/String;)Ljava/net/URI;"@ (static).
buildUri :: JRef -> String -> IO JRef
buildUri env urlStr = do
  uriCls <- findClass env "java/net/URI"
  createM <- staticMethodId env uriCls "create" "(Ljava/lang/String;)Ljava/net/URI;"
  urlJStr <- hsStringToJString env urlStr
  c_callStaticObjectMethod1Obj env uriCls createM urlJStr

-- | Real signature: @HttpRequest.newBuilder(URI) -> "(Ljava/net/URI;)Ljava/net/http/HttpRequest$Builder;"@ (static).
newRequestBuilder :: JRef -> JRef -> IO JRef
newRequestBuilder env uriObj = do
  reqCls <- findClass env "java/net/http/HttpRequest"
  newBuilderM <- staticMethodId env reqCls "newBuilder" "(Ljava/net/URI;)Ljava/net/http/HttpRequest$Builder;"
  c_callStaticObjectMethod1Obj env reqCls newBuilderM uriObj

-- | Real signature: @HttpRequest.Builder.header(String,String) -> "(Ljava/lang/String;Ljava/lang/String;)Ljava/net/http/HttpRequest$Builder;"@.
addHeader :: JRef -> JRef -> String -> String -> IO JRef
addHeader env builder k v = do
  builderCls <- findClass env "java/net/http/HttpRequest$Builder"
  headerM <- methodId env builderCls "header" "(Ljava/lang/String;Ljava/lang/String;)Ljava/net/http/HttpRequest$Builder;"
  kJ <- hsStringToJString env k
  vJ <- hsStringToJString env v
  c_callObjectMethod2Str env builder headerM kJ vJ

-- | Real signature: @HttpRequest.Builder.GET() -> "()Ljava/net/http/HttpRequest$Builder;"@.
setGet :: JRef -> JRef -> IO JRef
setGet env builder = do
  builderCls <- findClass env "java/net/http/HttpRequest$Builder"
  getM <- methodId env builderCls "GET" "()Ljava/net/http/HttpRequest$Builder;"
  c_callObjectMethod0 env builder getM

-- | Real signature: @HttpRequest.Builder.POST(BodyPublisher) -> "(Ljava/net/http/HttpRequest$BodyPublisher;)Ljava/net/http/HttpRequest$Builder;"@.
setPost :: JRef -> JRef -> JRef -> IO JRef
setPost env builder bodyPublisher = do
  builderCls <- findClass env "java/net/http/HttpRequest$Builder"
  postM <- methodId env builderCls "POST" "(Ljava/net/http/HttpRequest$BodyPublisher;)Ljava/net/http/HttpRequest$Builder;"
  c_callObjectMethod1Obj env builder postM bodyPublisher

-- | Real signatures: @HttpRequest.Builder.timeout(Duration) -> "(Ljava/time/Duration;)Ljava/net/http/HttpRequest$Builder;"@,
-- @Duration.ofSeconds(long) -> "(J)Ljava/time/Duration;"@ (static). Sets
-- an upper bound on the WHOLE request, matching curl's own
-- @--max-time@ (this module doesn't separately model curl's
-- connect-vs-total distinction -- one timeout, applied to the total,
-- is close enough for the small JSON calls this module makes).
setTimeoutSeconds :: JRef -> JRef -> Int -> IO JRef
setTimeoutSeconds env builder secs = do
  builderCls <- findClass env "java/net/http/HttpRequest$Builder"
  timeoutM <- methodId env builderCls "timeout" "(Ljava/time/Duration;)Ljava/net/http/HttpRequest$Builder;"
  durCls <- findClass env "java/time/Duration"
  ofSecondsM <- staticMethodId env durCls "ofSeconds" "(J)Ljava/time/Duration;"
  durObj <- c_callStaticObjectMethod1Long env durCls ofSecondsM (fromIntegral secs)
  c_callObjectMethod1Obj env builder timeoutM durObj

-- | Real signature: @HttpRequest.Builder.build() -> "()Ljava/net/http/HttpRequest;"@.
buildRequest :: JRef -> JRef -> IO JRef
buildRequest env builder = do
  builderCls <- findClass env "java/net/http/HttpRequest$Builder"
  buildM <- methodId env builderCls "build" "()Ljava/net/http/HttpRequest;"
  c_callObjectMethod0 env builder buildM

-- | Real signature: @HttpRequest.BodyPublishers.ofString(String) -> "(Ljava/lang/String;)Ljava/net/http/HttpRequest$BodyPublisher;"@ (static).
bodyPublisherOfString :: JRef -> String -> IO JRef
bodyPublisherOfString env s = do
  bpCls <- findClass env "java/net/http/HttpRequest$BodyPublishers"
  ofStringM <- staticMethodId env bpCls "ofString" "(Ljava/lang/String;)Ljava/net/http/HttpRequest$BodyPublisher;"
  sJ <- hsStringToJString env s
  c_callStaticObjectMethod1Obj env bpCls ofStringM sJ

-- | Real signature: @HttpClient.newHttpClient() -> "()Ljava/net/http/HttpClient;"@ (static).
-- A new client per call, deliberately -- these are small, infrequent
-- XRPC calls, not a hot path worth the complexity of threading a
-- shared client through every caller.
newHttpClient :: JRef -> IO JRef
newHttpClient env = do
  clientCls <- findClass env "java/net/http/HttpClient"
  newClientM <- staticMethodId env clientCls "newHttpClient" "()Ljava/net/http/HttpClient;"
  c_callStaticObjectMethod0 env clientCls newClientM

-- | Real signature: @HttpResponse.BodyHandlers.ofString() -> "()Ljava/net/http/HttpResponse$BodyHandler;"@ (static).
bodyHandlerOfString :: JRef -> IO JRef
bodyHandlerOfString env = do
  bhCls <- findClass env "java/net/http/HttpResponse$BodyHandlers"
  ofStringM <- staticMethodId env bhCls "ofString" "()Ljava/net/http/HttpResponse$BodyHandler;"
  c_callStaticObjectMethod0 env bhCls ofStringM

-- | Real signature: @HttpClient.send(HttpRequest, BodyHandler) -> "(Ljava/net/http/HttpRequest;Ljava/net/http/HttpResponse$BodyHandler;)Ljava/net/http/HttpResponse;"@.
-- Throws checked @IOException@\/@InterruptedException@ -- checked by
-- the caller via 'describeAndClearException', not here.
sendRequest :: JRef -> JRef -> JRef -> JRef -> IO JRef
sendRequest env client req bh = do
  clientCls <- findClass env "java/net/http/HttpClient"
  sendM <-
    methodId
      env
      clientCls
      "send"
      "(Ljava/net/http/HttpRequest;Ljava/net/http/HttpResponse$BodyHandler;)Ljava/net/http/HttpResponse;"
  c_callObjectMethod2Obj env client sendM req bh

-- | Real signature: @HttpResponse.statusCode() -> "()I"@.
getStatusCode :: JRef -> JRef -> IO Int
getStatusCode env resp = do
  respCls <- findClass env "java/net/http/HttpResponse"
  statusM <- methodId env respCls "statusCode" "()I"
  fromIntegral <$> c_callIntMethod0 env resp statusM

-- | Real signature: @HttpResponse.body() -> "()Ljava/lang/Object;"@ (the
-- descriptor is erased to @Object@ regardless of the generic type
-- parameter -- confirmed via @javap@, not assumed; the real runtime
-- type is @String@ here because 'bodyHandlerOfString' was used).
getBody :: JRef -> JRef -> IO String
getBody env resp = do
  respCls <- findClass env "java/net/http/HttpResponse"
  bodyM <- methodId env respCls "body" "()Ljava/lang/Object;"
  bodyObj <- c_callObjectMethod0 env resp bodyM
  jStringToHsString env bodyObj

-- | One real HTTP call, no retry -- 'withRetry' wraps this for the
-- transport-level-failure retry curl used to do via @--retry
-- --retry-connrefused@. @method@ is @\"GET\"@ or @\"POST\"@;
-- @maybeBody@ is the raw request body for POST, ignored for GET;
-- @extraHeaders@ are added after @Content-Type@ (always set for a
-- POST body) and any bearer token.
oneRequest :: JvmHandle -> String -> String -> Maybe (String, [(String, String)]) -> [(String, String)] -> IO (Either AtprotoError (Int, BL.ByteString))
oneRequest (JvmHandle env) method url maybeBody extra = do
  uriObj <- buildUri env url
  b0 <- newRequestBuilder env uriObj
  b1 <- case method of
    "GET" -> setGet env b0
    "POST" -> do
      let body = maybe "" fst maybeBody
      bp <- bodyPublisherOfString env body
      b0' <- setPost env b0 bp
      addHeader env b0' "Content-Type" "application/json"
    other -> ioError (userError ("DMML.Atproto: unsupported method " <> other))
  b2 <- foldHeaders b1 (maybe [] snd maybeBody ++ extra)
  b3 <- setTimeoutSeconds env b2 60
  req <- buildRequest env b3
  client <- newHttpClient env
  bh <- bodyHandlerOfString env
  resp <- sendRequest env client req bh
  mErr <- describeAndClearException env
  case mErr of
    Just err -> pure (Left (HttpTransportFailed (T.pack err)))
    Nothing -> do
      code <- getStatusCode env resp
      body <- getBody env resp
      let bodyBytes = BL.fromStrict (TE.encodeUtf8 (T.pack body))
      if code >= 200 && code < 300
        then pure (Right (code, bodyBytes))
        else pure (Left (HttpFailed code bodyBytes))
  where
    foldHeaders b [] = pure b
    foldHeaders b ((k, v) : rest) = do
      b' <- addHeader env b k v
      foldHeaders b' rest

-- | Retry only a transport-level failure ('HttpTransportFailed') --
-- matching curl's own @--retry@ semantics of retrying a connection
-- drop\/refuse but never a real HTTP 4xx\/5xx (those are application
-- errors, not transport flakiness, and retrying one wouldn't help).
-- 3 attempts total, 2s between, same numbers @curlNetworkFlags@ used.
withRetry :: IO (Either AtprotoError a) -> IO (Either AtprotoError a)
withRetry action = go (3 :: Int)
  where
    go attemptsLeft = do
      result <- action
      case result of
        Left (HttpTransportFailed _) | attemptsLeft > 1 -> do
          threadDelay (2 * 1000 * 1000)
          go (attemptsLeft - 1)
        other -> pure other

runGet :: JvmHandle -> String -> [(String, String)] -> IO (Either AtprotoError BL.ByteString)
runGet jvm baseUrl queryParams = do
  let url = baseUrl ++ if null queryParams then "" else "?" ++ intercalateAmp [percentEncode (T.pack k) ++ "=" ++ percentEncode (T.pack v) | (k, v) <- queryParams]
  result <- withRetry (oneRequest jvm "GET" url Nothing [])
  pure (snd <$> result)
  where
    intercalateAmp = foldr1 (\a b -> a ++ "&" ++ b)

runPostJson :: JvmHandle -> Text -> Text -> Maybe Text -> Value -> IO (Either AtprotoError BL.ByteString)
runPostJson jvm pdsEndpoint xrpcPath maybeToken bodyValue = do
  let url = T.unpack pdsEndpoint ++ T.unpack xrpcPath
      bodyStr = T.unpack (TE.decodeUtf8 (BL.toStrict (Aeson.encode bodyValue)))
      authHeaders = maybe [] (\tok -> [("Authorization", "Bearer " <> T.unpack tok)]) maybeToken
  result <- withRetry (oneRequest jvm "POST" url (Just (bodyStr, [])) authHeaders)
  pure (snd <$> result)

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
