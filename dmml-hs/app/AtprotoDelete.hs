{-# LANGUAGE OverloadedStrings #-}

-- | Deletes one record, by rkey, from the caller's own repo. Built
-- 2026-09-04 to clean up a real, disclosed mistake: an invalid test
-- commit published while verifying 'DMML.Atproto.createRecord' had no
-- way to be removed. Needs ATPROTO_APP_PASSWORD, same as atproto-publish.
module Main (main) where

import qualified Data.Text as T
import DMML.Atproto (createSession, deleteRecord, resolveDidToPdsEndpoint, resolveHandle)
import System.Environment (getArgs, lookupEnv)
import System.Exit (exitFailure)
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [identifierStr, collectionStr, rkeyStr] -> do
      maybePassword <- lookupEnv "ATPROTO_APP_PASSWORD"
      case maybePassword of
        Nothing -> hPutStrLn stderr "ATPROTO_APP_PASSWORD must be set" >> exitFailure
        Just password -> run (T.pack identifierStr) (T.pack collectionStr) (T.pack rkeyStr) (T.pack password)
    _ -> hPutStrLn stderr "usage: atproto-delete <handle-or-did> <collection> <rkey>" >> exitFailure

run :: T.Text -> T.Text -> T.Text -> T.Text -> IO ()
run identifier collection rkey password = do
  didResult <-
    if "did:" `T.isPrefixOf` identifier
      then pure (Right identifier)
      else resolveHandle identifier
  case didResult of
    Left err -> hPutStrLn stderr ("resolveHandle failed: " <> show err) >> exitFailure
    Right did -> do
      pdsResult <- resolveDidToPdsEndpoint did
      case pdsResult of
        Left err -> hPutStrLn stderr ("resolveDidToPdsEndpoint failed: " <> show err) >> exitFailure
        Right pdsEndpoint -> do
          sessionResult <- createSession pdsEndpoint identifier password
          case sessionResult of
            Left err -> hPutStrLn stderr ("createSession failed: " <> show err) >> exitFailure
            Right session -> do
              deleteResult <- deleteRecord session collection rkey
              case deleteResult of
                Left err -> hPutStrLn stderr ("deleteRecord failed: " <> show err) >> exitFailure
                Right () -> putStrLn ("deleted: " <> T.unpack rkey)
