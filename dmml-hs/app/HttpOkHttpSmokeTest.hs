{-# LANGUAGE OverloadedStrings #-}

-- | Ad hoc, scratch smoke test for the OkHttp rewrite of DMML.Http --
-- proves a real HTTP GET and POST go out through okhttp3.* via JNI
-- upcall primitives and come back with a real 200 body, not just that
-- the code type-checks and links. Not part of the permanent test
-- suite; exercised once during the 2026-09-08 OkHttp migration to
-- satisfy this project's "verify for real" discipline, same reasoning
-- as JgitSmokeTest/JniBridgeSmokeTest.
module Main (main) where

import qualified Data.Aeson as Aeson
import qualified Data.ByteString.Lazy.Char8 as BLC
import DMML.Http (HttpError (..), getJson, postJson)
import DMML.Jni (JvmEnvironment (..), withJvm)
import System.Environment (getArgs)
import System.Exit (exitFailure)

report :: String -> Either HttpError BLC.ByteString -> IO ()
report label result = case result of
  Right body -> do
    putStrLn (label ++ ": OK, real 2xx response body:")
    BLC.putStrLn body
  Left (HttpTransportFailed err) -> do
    putStrLn (label ++ ": TRANSPORT FAILURE: " ++ show err)
    exitFailure
  Left (HttpFailed code body) -> do
    putStrLn (label ++ ": HTTP FAILURE: " ++ show code)
    BLC.putStrLn body
    exitFailure

main :: IO ()
main = do
  args <- getArgs
  case args of
    [classpath] -> withJvm (EmbeddedJvm classpath) $ \jvm -> do
      putStrLn "HttpOkHttpSmokeTest: GET https://httpbin.org/get via okhttp3.*"
      getResult <- getJson jvm "https://httpbin.org/get" [("dmml", "okhttp-smoke")]
      report "GET" getResult
      putStrLn "HttpOkHttpSmokeTest: POST https://httpbin.org/post via okhttp3.*"
      postResult <-
        postJson
          jvm
          "https://httpbin.org/post"
          [("Authorization", "Bearer dmml-smoke-token")]
          (Aeson.object ["hello" Aeson..= ("okhttp" :: String), "n" Aeson..= (42 :: Int)])
      report "POST" postResult
    _ -> do
      putStrLn "usage: http-okhttp-smoke-test <classpath-incl-okhttp+okio+kotlin-stdlib jars>"
      exitFailure
