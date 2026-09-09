{-# LANGUAGE OverloadedStrings #-}

-- | Generic JSON-over-HTTPS transport via bundled OkHttp
-- (@okhttp3.*@), through the same embedded\/upcalled JVM 'DMML.Jgit'
-- already needs for git. Originally written against
-- @java.net.http.HttpClient@ (the JDK 11+ API); rewritten 2026-09-08
-- after a real on-device probe (@dalvikvm@ loading
-- @java.net.http.HttpClient@ via reflection) confirmed that class is
-- simply not part of Android\/ART's core library on any API level --
-- not a missing permission or a proguard strip, the class does not
-- exist there at all. OkHttp is a plain JVM library with no
-- Android-core dependency, so it works identically on desktop and
-- Android once its jar is on the relevant classpath (see
-- 'DMML.Jgit''s own precedent for JGit).
--
-- Deliberately simple, per the "bundle okhttp, keep it simple"
-- instruction this rewrite was done under: no per-request timeout
-- override (OkHttp's own defaults -- 10s connect\/read\/write -- are
-- used via a single default-constructed @OkHttpClient@ per call,
-- matching the original's "new client per call" policy); no explicit
-- @Content-Type@ header is set separately from the request body --
-- OkHttp derives it from the 'okhttp3.RequestBody''s
-- 'okhttp3.MediaType' automatically, which is the normal OkHttp idiom
-- and *more* correct than the old code's manual duplication (the old
-- @java.net.http@ code had to set @Content-Type@ by hand because
-- @HttpRequest@'s body publisher carries no media type of its own).
module DMML.Http
  ( HttpError (..)
  , oneRequest
  , withRetry
  , getJson
  , postJson
  , postJsonDpop
  ) where

import Control.Concurrent (threadDelay)
import Data.Aeson (Value)
import qualified Data.Aeson as Aeson
import Data.Bits (shiftR, (.&.))
import qualified Data.ByteString.Lazy as BL
import Data.Char (intToDigit, isAscii, isAlphaNum, ord)
import Data.List (isInfixOf)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Data.Word (Word8)
import Foreign.Ptr (nullPtr)

import DMML.Dpop (createProofUpcall)
import DMML.Jni
  ( JRef
  , JvmHandle (..)
  , c_callIntMethod0
  , c_callObjectMethod0
  , c_callObjectMethod1Obj
  , c_callObjectMethod1Str
  , c_callObjectMethod2Str
  , c_callStaticObjectMethod1Obj
  , c_callStaticObjectMethod2Obj
  , c_newObject0
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
  -- refused/timeout, a Java exception out of @Call.execute@ -- see
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

-- | Real signature: @new OkHttpClient()@ -- confirmed via @javap@ as a
-- real public no-arg constructor. A new client per call, deliberately
-- (see module haddock) -- matches the original's own "new client per
-- call" policy, not a regression.
newOkHttpClient :: JRef -> IO JRef
newOkHttpClient env = do
  clientCls <- findClass env "okhttp3/OkHttpClient"
  ctor <- methodId env clientCls "<init>" "()V"
  c_newObject0 env clientCls ctor

-- | Real signature: @new Request.Builder()@ -- confirmed via @javap@
-- as a real public no-arg constructor.
newRequestBuilder :: JRef -> IO JRef
newRequestBuilder env = do
  builderCls <- findClass env "okhttp3/Request$Builder"
  ctor <- methodId env builderCls "<init>" "()V"
  c_newObject0 env builderCls ctor

-- | Real signature: @Request.Builder.url(String) ->
-- "(Ljava/lang/String;)Lokhttp3/Request$Builder;"@.
setUrl :: JRef -> JRef -> String -> IO JRef
setUrl env builder url = do
  builderCls <- findClass env "okhttp3/Request$Builder"
  urlM <- methodId env builderCls "url" "(Ljava/lang/String;)Lokhttp3/Request$Builder;"
  urlJ <- hsStringToJString env url
  c_callObjectMethod1Obj env builder urlM urlJ

-- | Real signature: @Request.Builder.header(String,String) ->
-- "(Ljava/lang/String;Ljava/lang/String;)Lokhttp3/Request$Builder;"@.
addHeader :: JRef -> JRef -> String -> String -> IO JRef
addHeader env builder k v = do
  builderCls <- findClass env "okhttp3/Request$Builder"
  headerM <- methodId env builderCls "header" "(Ljava/lang/String;Ljava/lang/String;)Lokhttp3/Request$Builder;"
  kJ <- hsStringToJString env k
  vJ <- hsStringToJString env v
  c_callObjectMethod2Str env builder headerM kJ vJ

-- | Real signature: @Request.Builder.get() ->
-- "()Lokhttp3/Request$Builder;"@.
setGet :: JRef -> JRef -> IO JRef
setGet env builder = do
  builderCls <- findClass env "okhttp3/Request$Builder"
  getM <- methodId env builderCls "get" "()Lokhttp3/Request$Builder;"
  c_callObjectMethod0 env builder getM

-- | Real signature: @Request.Builder.post(RequestBody) ->
-- "(Lokhttp3/RequestBody;)Lokhttp3/Request$Builder;"@.
setPost :: JRef -> JRef -> JRef -> IO JRef
setPost env builder body = do
  builderCls <- findClass env "okhttp3/Request$Builder"
  postM <- methodId env builderCls "post" "(Lokhttp3/RequestBody;)Lokhttp3/Request$Builder;"
  c_callObjectMethod1Obj env builder postM body

-- | Real signature: @Request.Builder.build() -> "()Lokhttp3/Request;"@.
buildRequest :: JRef -> JRef -> IO JRef
buildRequest env builder = do
  builderCls <- findClass env "okhttp3/Request$Builder"
  buildM <- methodId env builderCls "build" "()Lokhttp3/Request;"
  c_callObjectMethod0 env builder buildM

-- | Real signature: @MediaType.parse(String) ->
-- "(Ljava/lang/String;)Lokhttp3/MediaType;"@ (static, confirmed via
-- @javap@ callable directly -- not behind Kotlin's @Companion@
-- indirection).
mediaTypeParse :: JRef -> String -> IO JRef
mediaTypeParse env mime = do
  mtCls <- findClass env "okhttp3/MediaType"
  parseM <- staticMethodId env mtCls "parse" "(Ljava/lang/String;)Lokhttp3/MediaType;"
  mimeJ <- hsStringToJString env mime
  c_callStaticObjectMethod1Obj env mtCls parseM mimeJ

-- | Real signature: @RequestBody.create(String,MediaType) ->
-- "(Ljava/lang/String;Lokhttp3/MediaType;)Lokhttp3/RequestBody;"@
-- (static, confirmed via @javap@ callable directly).
requestBodyCreate :: JRef -> String -> JRef -> IO JRef
requestBodyCreate env body mediaType = do
  rbCls <- findClass env "okhttp3/RequestBody"
  createM <- staticMethodId env rbCls "create" "(Ljava/lang/String;Lokhttp3/MediaType;)Lokhttp3/RequestBody;"
  bodyJ <- hsStringToJString env body
  c_callStaticObjectMethod2Obj env rbCls createM bodyJ mediaType

-- | Real signature: @OkHttpClient.newCall(Request) ->
-- "(Lokhttp3/Request;)Lokhttp3/Call;"@.
newCall :: JRef -> JRef -> JRef -> IO JRef
newCall env client req = do
  clientCls <- findClass env "okhttp3/OkHttpClient"
  newCallM <- methodId env clientCls "newCall" "(Lokhttp3/Request;)Lokhttp3/Call;"
  c_callObjectMethod1Obj env client newCallM req

-- | Real signature: @Call.execute() -> "()Lokhttp3/Response;"@. Throws
-- checked @IOException@ -- checked by the caller via
-- 'describeAndClearException', not here.
executeCall :: JRef -> JRef -> IO JRef
executeCall env call = do
  callCls <- findClass env "okhttp3/Call"
  executeM <- methodId env callCls "execute" "()Lokhttp3/Response;"
  c_callObjectMethod0 env call executeM

-- | Real signature: @Response.code() -> "()I"@.
getStatusCode :: JRef -> JRef -> IO Int
getStatusCode env resp = do
  respCls <- findClass env "okhttp3/Response"
  codeM <- methodId env respCls "code" "()I"
  fromIntegral <$> c_callIntMethod0 env resp codeM

-- | Real signatures: @Response.body() -> "()Lokhttp3/ResponseBody;"@,
-- @ResponseBody.string() -> "()Ljava/lang/String;"@ (throws checked
-- @IOException@). No explicit @ResponseBody.close()@ -- these are
-- one-shot, whole-body-buffered calls (matching the original's own
-- @HttpResponse.BodyHandlers.ofString@ behaviour), and @string()@
-- itself closes the underlying source once fully consumed.
getBody :: JRef -> JRef -> IO String
getBody env resp = do
  respCls <- findClass env "okhttp3/Response"
  bodyM <- methodId env respCls "body" "()Lokhttp3/ResponseBody;"
  bodyObj <- c_callObjectMethod0 env resp bodyM
  rbCls <- findClass env "okhttp3/ResponseBody"
  stringM <- methodId env rbCls "string" "()Ljava/lang/String;"
  strObj <- c_callObjectMethod0 env bodyObj stringM
  jStringToHsString env strObj

-- | Real signature: @Response.header(String) ->
-- "(Ljava/lang/String;)Ljava/lang/String;"@ -- nullable, per OkHttp's
-- own contract (returns @null@, not an exception, when the named
-- header isn't present). Used to read the @DPoP-Nonce@ response
-- header for 'oneRequestDpop''s nonce retry -- the same real-server
-- behavior 'AtprotoOAuthClient.kt''s own PAR\/token-endpoint DPoP
-- handling already discovered and handled, now needed here too for
-- resource-server (PDS) requests.
getResponseHeader :: JRef -> JRef -> String -> IO (Maybe String)
getResponseHeader env resp name = do
  respCls <- findClass env "okhttp3/Response"
  headerM <- methodId env respCls "header" "(Ljava/lang/String;)Ljava/lang/String;"
  nameJ <- hsStringToJString env name
  result <- c_callObjectMethod1Str env resp headerM nameJ
  if result == nullPtr then pure Nothing else Just <$> jStringToHsString env result

-- | One real HTTP call, no retry -- 'withRetry' wraps this for the
-- transport-level-failure retry curl used to do via @--retry
-- --retry-connrefused@. @method@ is @\"GET\"@ or @\"POST\"@;
-- @maybeBody@ is the raw request body for POST, ignored for GET;
-- @extraHeaders@ are added after any bearer token/auth header a
-- caller supplies. @Content-Type@ for a POST body is set by OkHttp
-- itself from the 'okhttp3.RequestBody''s media type, not added here.
oneRequest :: JvmHandle -> String -> String -> Maybe (String, [(String, String)]) -> [(String, String)] -> IO (Either HttpError (Int, BL.ByteString))
oneRequest (JvmHandle env) method url maybeBody extra = do
  b0 <- newRequestBuilder env
  b1 <- setUrl env b0 url
  b2 <- case method of
    "GET" -> setGet env b1
    "POST" -> do
      let body = maybe "" fst maybeBody
      mediaType <- mediaTypeParse env "application/json"
      rb <- requestBodyCreate env body mediaType
      setPost env b1 rb
    other -> ioError (userError ("DMML.Http: unsupported method " <> other))
  b3 <- foldHeaders b2 (maybe [] snd maybeBody ++ extra)
  req <- buildRequest env b3
  client <- newOkHttpClient env
  call <- newCall env client req
  resp <- executeCall env call
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

-- | A DPoP-authenticated POST -- real atproto requirement (see
-- 'DMML.Dpop''s own module haddock): every authenticated resource-
-- server (PDS) request needs its own fresh @Authorization: DPoP
-- \<token\>@ + @DPoP: \<proof-jwt\>@ pair, not a reusable @Bearer@
-- header the way 'postJson' sends one. @accessToken@ is a real,
-- DPoP-bound access token from a completed atproto OAuth login (see
-- @OAuthTokenStore.kt@) -- NOT the plain @Bearer@-style JWT
-- 'DMML.Atproto.createSession' (the app-password flow) produces,
-- which this function was never meant for.
--
-- Handles the real DPoP-nonce retry every DPoP-protected endpoint can
-- require (first attempt: 400\/401 with a @DPoP-Nonce@ response
-- header; retried once with that nonce folded into a freshly re-signed
-- proof) -- the exact same real-server behavior
-- @AtprotoOAuthClient.kt@'s own PAR\/token-endpoint calls already
-- discovered and handled on 2026-09-09, now needed again here for
-- resource-server requests specifically, since the PDS enforces its
-- own nonce independently of the authorization server's.
postJsonDpop :: JvmHandle -> String -> Text -> Value -> IO (Either HttpError BL.ByteString)
postJsonDpop jvm url accessToken bodyValue = do
  let bodyStr = T.unpack (TE.decodeUtf8 (BL.toStrict (Aeson.encode bodyValue)))
  result <- withRetry (oneRequestDpop jvm "POST" url bodyStr accessToken)
  pure (snd <$> result)

-- | Real DPoP request/response cycle, with one nonce retry -- see
-- 'postJsonDpop''s own doc comment for why this exists as a separate
-- function from 'oneRequest' rather than a variant of it (a
-- DPoP-bound request's headers can't be finalized -- specifically the
-- @DPoP@ proof header itself -- until AFTER a possible nonce-carrying
-- failure response is seen, which 'oneRequest'\'s single-pass shape
-- has no way to express).
oneRequestDpop :: JvmHandle -> String -> String -> String -> Text -> IO (Either HttpError (Int, BL.ByteString))
oneRequestDpop (JvmHandle env) method url bodyStr accessToken = attempt Nothing
  where
    attempt :: Maybe Text -> IO (Either HttpError (Int, BL.ByteString))
    attempt mNonce = do
      proof <- createProofUpcall env (T.pack method) (T.pack url) (maybe "" id mNonce) accessToken
      b0 <- newRequestBuilder env
      b1 <- setUrl env b0 url
      b2 <- case method of
        "GET" -> setGet env b1
        "POST" -> do
          mediaType <- mediaTypeParse env "application/json"
          rb <- requestBodyCreate env bodyStr mediaType
          setPost env b1 rb
        other -> ioError (userError ("DMML.Http: unsupported method " <> other))
      b3 <- addHeader env b2 "Authorization" ("DPoP " <> T.unpack accessToken)
      b4 <- addHeader env b3 "DPoP" (T.unpack proof)
      req <- buildRequest env b4
      client <- newOkHttpClient env
      call <- newCall env client req
      resp <- executeCall env call
      mErr <- describeAndClearException env
      case mErr of
        Just err -> pure (Left (HttpTransportFailed (T.pack err)))
        Nothing -> do
          code <- getStatusCode env resp
          body <- getBody env resp
          let bodyBytes = BL.fromStrict (TE.encodeUtf8 (T.pack body))
          nonceHeader <- getResponseHeader env resp "DPoP-Nonce"
          case (mNonce, nonceHeader) of
            (Nothing, Just newNonce) | "use_dpop_nonce" `isInfixOf` body ->
              attempt (Just (T.pack newNonce))
            _ ->
              if code >= 200 && code < 300
                then pure (Right (code, bodyBytes))
                else pure (Left (HttpFailed code bodyBytes))
