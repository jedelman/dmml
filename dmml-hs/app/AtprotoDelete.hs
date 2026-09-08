{-# LANGUAGE OverloadedStrings #-}

-- | Deletes one record, by rkey, from the caller's own repo. Built
-- 2026-09-04 to clean up a real, disclosed mistake: an invalid test
-- commit published while verifying 'DMML.Atproto.createRecord' had no
-- way to be removed. Needs ATPROTO_APP_PASSWORD, same as atproto-publish.
module Main (main) where

import qualified Data.Text as T
import DMML.Atproto (createSession, deleteRecord, resolveDidToPdsEndpoint, resolveHandle)
import DMML.Jni (JvmEnvironment (..), JvmHandle, withJvm)
import System.Environment (getArgs, lookupEnv)
import System.Exit (exitFailure)
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = do
  args <- getArgs
  classpath <- maybe "." id <$> lookupEnv "DMML_JGIT_CLASSPATH"
  withJvm (EmbeddedJvm classpath) $ \jvm ->
    case args of
      [identifierStr, collectionStr, rkeyStr] -> do
        maybePassword <- lookupEnv "ATPROTO_APP_PASSWORD"
        case maybePassword of
          Nothing -> hPutStrLn stderr "ATPROTO_APP_PASSWORD must be set" >> exitFailure
          Just password -> run jvm (T.pack identifierStr) (T.pack collectionStr) (T.pack rkeyStr) (T.pack password)
      _ -> hPutStrLn stderr "usage: atproto-delete <handle-or-did> <collection> <rkey>" >> exitFailure

run :: JvmHandle -> T.Text -> T.Text -> T.Text -> T.Text -> IO ()
run jvm identifier collection rkey password = do
  didResult <-
    if "did:" `T.isPrefixOf` identifier
      then pure (Right identifier)
      else resolveHandle jvm identifier
  case didResult of
    Left err -> hPutStrLn stderr ("resolveHandle failed: " <> show err) >> exitFailure
    Right did -> do
      pdsResult <- resolveDidToPdsEndpoint jvm did
      case pdsResult of
        Left err -> hPutStrLn stderr ("resolveDidToPdsEndpoint failed: " <> show err) >> exitFailure
        Right pdsEndpoint -> do
          sessionResult <- createSession jvm pdsEndpoint identifier password
          case sessionResult of
            Left err -> hPutStrLn stderr ("createSession failed: " <> show err) >> exitFailure
            Right session -> do
              deleteResult <- deleteRecord jvm session collection rkey
              case deleteResult of
                Left err -> hPutStrLn stderr ("deleteRecord failed: " <> show err) >> exitFailure
                Right () -> putStrLn ("deleted: " <> T.unpack rkey)
