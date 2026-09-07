{-# LANGUAGE OverloadedStrings #-}

-- | Real-verifiable-today CLI: resolves a handle to a DID, then to its
-- actual PDS service endpoint, then (optionally) lists a collection --
-- exactly the chain confirmed live by hand 2026-09-04, see
-- @written-world/dev-journal/2026-09-04-atproto-discovery-no-knot-needed.md@.
-- Everything this binary does is unauthenticated -- no session, no
-- app password -- since resolution and public record reads never need
-- one.
module Main (main) where

import qualified Data.Aeson as Aeson
import qualified Data.ByteString.Lazy.Char8 as BLC
import Data.Text (Text)
import qualified Data.Text as T
import DMML.Atproto (listRecords, resolveDidToPdsEndpoint, resolveHandle)
import DMML.Jni (JvmHandle, withEmbeddedJvm)
import System.Environment (getArgs, lookupEnv)
import System.Exit (exitFailure)
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = do
  args <- getArgs
  classpath <- maybe "." id <$> lookupEnv "DMML_JGIT_CLASSPATH"
  withEmbeddedJvm classpath $ \jvm ->
    case args of
      [handle] -> run jvm (T.pack handle) Nothing
      [handle, collection] -> run jvm (T.pack handle) (Just (T.pack collection))
      _ -> do
        hPutStrLn stderr "usage: atproto-resolve <handle-or-did> [collection]"
        exitFailure

run :: JvmHandle -> Text -> Maybe Text -> IO ()
run jvm identifier maybeCollection = do
  didResult <-
    if "did:" `T.isPrefixOf` identifier
      then pure (Right identifier)
      else resolveHandle jvm identifier
  case didResult of
    Left err -> hPutStrLn stderr ("resolveHandle failed: " <> show err) >> exitFailure
    Right did -> do
      putStrLn ("did: " <> T.unpack did)
      pdsResult <- resolveDidToPdsEndpoint jvm did
      case pdsResult of
        Left err -> hPutStrLn stderr ("resolveDidToPdsEndpoint failed: " <> show err) >> exitFailure
        Right pdsEndpoint -> do
          putStrLn ("pds:  " <> T.unpack pdsEndpoint)
          case maybeCollection of
            Nothing -> pure ()
            Just collection -> do
              recordsResult <- listRecords jvm pdsEndpoint did collection Nothing
              case recordsResult of
                Left err -> hPutStrLn stderr ("listRecords failed: " <> show err) >> exitFailure
                Right value -> BLC.putStrLn (Aeson.encode value)
