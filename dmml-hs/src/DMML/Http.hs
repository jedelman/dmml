{-# LANGUAGE OverloadedStrings #-}

-- | Generic JSON-over-HTTPS transport via @java.net.http.HttpClient@,
-- through the same embedded\/upcalled JVM 'DMML.Jgit' already needs for
-- git. Extracted from 'DMML.Atproto' 2026-09-08 the moment a second
-- real consumer appeared ('DMML.Llm''s OpenRouter calls, for the
-- in-product authoring agents' BYOK path) -- despite the low-level
-- signature-string wrappers below having originally been written for
-- atproto's own XRPC calls, nothing about
-- @HttpClient@\/@HttpRequest@\/@HttpResponse@ is atproto-specific. Same
-- extraction discipline as 'DMML.Jni' being pulled out of 'DMML.Jgit'
-- the day before for the identical reason.
module DMML.Http
  ( HttpError (..)
  , oneRequest
  , withRetry
  , getJson
  , postJson
  ) where

import Control.Concurrent (threadDelay)
import Data.Aeson (Value)
import qualified Data.Aeson as Aeson
import Data.Bits (shiftR, (.&.))
import qualified Data.ByteString.Lazy as BL
import Data.Char (intToDigit, isAscii, isAlphaNum, ord)
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

data HttpError
  = HttpTransportFailed Text
  -- ^ The request never got a real HTTP response at all (DNS, connect
  -- refused/timeout, a Java exception out of @HttpClient.send@ -- see
  -- 'describeAndClearException'). Distinct from 'HttpFailed', a real
  -- non-2xx response.
  | HttpFailed Int BL.ByteString
  -- ^ A real HTTP response with a non-2xx status; the body is whatever
  -- the server actually sent back, not discarded.
  deriving (Eq, Show)

-- | RFC 3986 unreserved-char percent-encoding over a string's UTF-8
-- bytes -- the same encoding @curl --data-urlencode@ produced.
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
-- @Duration.ofSeconds(long) -> "(J)Ljava/time/Duration;"@ (static).
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
-- calls, not a hot path worth threading a shared client through every
-- caller.
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
-- parameter -- confirmed via @javap@; the real runtime type is
-- @String@ here because 'bodyHandlerOfString' was used).
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
-- POST body) and any bearer token/auth header a caller supplies.
oneRequest :: JvmHandle -> String -> String -> Maybe (String, [(String, String)]) -> [(String, String)] -> IO (Either HttpError (Int, BL.ByteString))
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
    other -> ioError (userError ("DMML.Http: unsupported method " <> other))
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
-- 3 attempts total, 2s between.
withRetry :: IO (Either HttpError a) -> IO (Either HttpError a)
withRetry action = go (3 :: Int)
  where
    go attemptsLeft = do
      result <- action
      case result of
        Left (HttpTransportFailed _) | attemptsLeft > 1 -> do
          threadDelay (2 * 1000 * 1000)
          go (attemptsLeft - 1)
        other -> pure other

-- | A GET request with URL-encoded query params, retried on transport
-- failure. @baseUrl@ has no query string of its own.
getJson :: JvmHandle -> String -> [(String, String)] -> IO (Either HttpError BL.ByteString)
getJson jvm baseUrl queryParams = do
  let url = baseUrl ++ if null queryParams then "" else "?" ++ intercalateAmp [percentEncode (T.pack k) ++ "=" ++ percentEncode (T.pack v) | (k, v) <- queryParams]
  result <- withRetry (oneRequest jvm "GET" url Nothing [])
  pure (snd <$> result)
  where
    intercalateAmp = foldr1 (\a b -> a ++ "&" ++ b)

-- | A POST request with a JSON body and arbitrary extra headers (e.g.
-- an @Authorization@ bearer token), retried on transport failure.
postJson :: JvmHandle -> String -> [(String, String)] -> Value -> IO (Either HttpError BL.ByteString)
postJson jvm url headers bodyValue = do
  let bodyStr = T.unpack (TE.decodeUtf8 (BL.toStrict (Aeson.encode bodyValue)))
  result <- withRetry (oneRequest jvm "POST" url (Just (bodyStr, [])) headers)
  pure (snd <$> result)
