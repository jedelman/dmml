{-# LANGUAGE OverloadedStrings #-}

-- | atproto-native replacement for @broker.sh@'s git-fetch step: pulls a
-- peer's new @org.jason-edelman.writtenworld.commit@ records (since a
-- stored per-peer cursor) and materializes each one's @dmml@ field as a
-- local @.dmml@ file, named by the record's own rkey -- the same shape
-- @check-divergence@ already expects for its peer-file-list argument
-- (see @DMML.Fire@/@CheckDivergence.hs@; this produces that list's
-- *contents*, it doesn't change what consumes it).
--
-- Design: @written-world/dev-journal/2026-09-04-atproto-discovery-no-knot-needed.md@
-- and its follow-on cleanup entry.
--
-- Thin CLI wrapper as of 2026-09-07: all the actual DID/PDS resolution,
-- pagination, retry, and cursor-filtering logic now lives in
-- 'DMML.Atproto.pullNewRecords' -- extracted so @written-world@'s
-- broker orchestration (@cli/app/Broker.hs@) can pull records as a
-- direct in-process call too, with no subprocess spawning. This binary
-- just writes what that function returns to disk.
--
-- This process never advances @cursor-file@ itself -- it has no way to
-- know whether the caller will actually accept this batch (a file could
-- still fail @validate-commit@). It writes the *candidate* next cursor
-- to @out-dir\/next-cursor@ instead; persisting it to @cursor-file@ is
-- the caller's job, only once the batch is actually incorporated (see
-- @atproto-broker.sh@). Advancing it unconditionally here would
-- silently and permanently skip a rejected batch instead of letting the
-- caller retry it.
module Main (main) where

import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import DMML.Atproto (pullNewRecords)
import DMML.Jni (withEmbeddedJvm)
import System.Directory (createDirectoryIfMissing, doesFileExist)
import System.Environment (getArgs, lookupEnv)
import System.Exit (exitFailure)
import System.FilePath ((</>))
import System.IO (hPutStrLn, stderr)

main :: IO ()
main = do
  args <- getArgs
  classpath <- maybe "." id <$> lookupEnv "DMML_JGIT_CLASSPATH"
  withEmbeddedJvm classpath $ \jvm ->
    case args of
      [peerIdentifier, collection, cursorFile, outDir] -> do
        haveCursorFile <- doesFileExist cursorFile
        storedCursor <- if haveCursorFile then T.strip <$> TIO.readFile cursorFile else pure ""
        result <- pullNewRecords jvm (T.pack peerIdentifier) (T.pack collection) storedCursor
        case result of
          Left err -> hPutStrLn stderr ("pullNewRecords failed: " <> show err) >> exitFailure
          Right (nextCursor, new) -> do
            createDirectoryIfMissing True outDir
            paths <- mapM (writeOne outDir) new
            mapM_ putStrLn paths
            case nextCursor of
              Nothing -> pure ()
              Just c -> TIO.writeFile (outDir </> "next-cursor") c
      _ ->
        hPutStrLn
          stderr
          "usage: atproto-pull <peer-handle-or-did> <collection> <cursor-file> <out-dir>\n\
          \  prints one newly-materialized .dmml file path per line (empty if nothing new)"
          >> exitFailure

writeOne :: FilePath -> (Text, Text) -> IO FilePath
writeOne outDir (rkey, dmml) = do
  let path = outDir </> T.unpack rkey <> ".dmml"
  TIO.writeFile path dmml
  pure path
