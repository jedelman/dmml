{-# LANGUAGE OverloadedStrings #-}

-- | DPoP (RFC 9449) proof generation, reached via JNI upcall into the
-- Android app's own 'org.jasonedelman.writtenworld.oauth.DpopKeyManager'
-- -- deliberately NOT reimplemented in Haskell. The DPoP private key
-- lives in Android's own Keystore, generated non-exportable on
-- purpose (that's the whole point: the server never needs to trust
-- Haskell, or this process, with the raw key material, only with the
-- ability to ask Android to sign one proof at a time). Re-deriving
-- Keystore access + ECDSA signing + JWK/JWT construction as raw JNI
-- primitive calls in Haskell would just be a second, more fragile
-- copy of 'DpopKeyManager.kt''s already-proven logic (proven for
-- real: it's the exact same code the 2026-09-09 OAuth login used to
-- complete a real login with a real access token) -- calling back into
-- it via the SAME generic upcall machinery 'DMML.Jgit'/'DMML.Http'
-- already use for JGit/OkHttp objects is the established pattern for
-- exactly this shape of need.
--
-- Android-only: 'org.jasonedelman.writtenworld.oauth.DpopKeyManager'
-- is only ever loaded inside the real Android app's classpath: this
-- module's functions will fail (a real 'DMML.Http.HttpTransportFailed'-
-- shaped error, via the same 'DMML.Jni.describeAndClearException'
-- path everything else here uses) if called under an 'EmbeddedJvm'
-- (desktop CLI) that never loaded that class -- not silently, not
-- assumed to be fine, a real disclosed platform boundary.
module DMML.Dpop
  ( createProofUpcall
  ) where

import Data.Text (Text)
import qualified Data.Text as T

import DMML.Jni
  ( JNIEnvPtr
  , c_callStaticObjectMethod4Obj
  , findClass
  , hsStringToJString
  , jStringToHsString
  , staticMethodId
  )

-- | Calls @DpopKeyManager.createProofForNative(htm, htu, nonce,
-- accessToken)@ -- see that function's own doc comment for why it
-- exists as a separate, non-default-args overload rather than calling
-- @createProof@ directly. Pass @\"\"@ for @nonce@\/@accessToken@ when
-- there is none (the Kotlin side treats an empty string as absent,
-- the same sentinel convention, not a real @null@ jstring crossing
-- the upcall boundary).
createProofUpcall :: JNIEnvPtr -> Text -> Text -> Text -> Text -> IO Text
createProofUpcall env htm htu nonce accessToken = do
  cls <- findClass env "org/jasonedelman/writtenworld/oauth/DpopKeyManager"
  m <-
    staticMethodId
      env
      cls
      "createProofForNative"
      "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;"
  htmJ <- hsStringToJString env (T.unpack htm)
  htuJ <- hsStringToJString env (T.unpack htu)
  nonceJ <- hsStringToJString env (T.unpack nonce)
  tokenJ <- hsStringToJString env (T.unpack accessToken)
  resultRef <- c_callStaticObjectMethod4Obj env cls m htmJ htuJ nonceJ tokenJ
  T.pack <$> jStringToHsString env resultRef
