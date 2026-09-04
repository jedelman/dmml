{-# LANGUAGE OverloadedStrings #-}

-- | Publishes a rendered DMML commit (the text `DMML.Fire.renderFiredCommit`
-- already produces) as a real atproto record, into the caller's OWN repo,
-- under the existing @org.jason-edelman.writtenworld.commit@ lexicon.
--
-- Live-verified 2026-09-04 against a real account: Jason confirmed the
-- sandbox's @ATPROTO_APP_PASSWORD@ pairs with the @claude.jason-
-- edelman.org@ handle (the identity worker described in that repo's own
-- CLAUDE.md). Resolved handle -> DID -> PDS endpoint, opened a session,
-- and \`createRecord\`ed a real test commit
-- (@atproto/writePathVerification@, predicate @mints@); confirmed via a
-- separate \`listRecords\` read that it came back byte-for-byte
-- identical. See
-- @written-world/dev-journal/2026-09-04-atproto-discovery-no-knot-needed.md@
-- for the full design context.
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Data.Time.Clock (getCurrentTime)
import Data.Time.Format (defaultTimeLocale, formatTime)
import DMML.Atproto
  ( commitRecord
  , createRecord
  , createSession
  , resolveDidToPdsEndpoint
  , resolveHandle
  )
import System.Environment (getArgs, lookupEnv)
import System.Exit (exitFailure)
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [identifierStr, predicateStr, dmmlPath] -> do
      maybePassword <- lookupEnv "ATPROTO_APP_PASSWORD"
      case maybePassword of
        Nothing -> do
          hPutStrLn stderr "ATPROTO_APP_PASSWORD must be set (an atproto app password, never the real account password)"
          exitFailure
        Just password -> do
          dmmlText <- TIO.readFile dmmlPath
          run (T.pack identifierStr) (T.pack predicateStr) dmmlText (T.pack password)
    _ ->
      hPutStrLn
        stderr
        "usage: atproto-publish <handle-or-did> <predicate> <commit.dmml> (needs ATPROTO_APP_PASSWORD)"
        >> exitFailure

run :: T.Text -> T.Text -> T.Text -> T.Text -> IO ()
run identifier predicate dmmlText password = do
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
              now <- getCurrentTime
              let createdAt = T.pack (formatTime defaultTimeLocale "%Y-%m-%dT%H:%M:%S%QZ" now)
                  record = commitRecord predicate dmmlText createdAt
              publishResult <- createRecord session "org.jason-edelman.writtenworld.commit" record
              case publishResult of
                Left err -> hPutStrLn stderr ("createRecord failed: " <> show err) >> exitFailure
                Right uri -> putStrLn ("published: " <> T.unpack uri)
