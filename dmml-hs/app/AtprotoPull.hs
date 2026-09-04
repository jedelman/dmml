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
-- and its follow-on cleanup entry. Cursor correctness does not depend on
-- @listRecords@' own return order (observed newest-first against a real
-- PDS, but not a documented guarantee): every record in the collection
-- is paged in (bounded, see MAX_PAGES below) and filtered/sorted
-- client-side by rkey, which is a real atproto TID -- lexicographically
-- ordered by creation time, safe to compare as plain 'Text'.
--
-- Real, disclosed limit: refetches the WHOLE collection on every run
-- (paginating via listRecords' own cursor, capped at 'maxPages') rather
-- than asking the PDS to resume after a server-side cursor -- correct,
-- not efficient. Fine at today's real scale; worth revisiting if a
-- collection ever grows large enough for this to matter.
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

import qualified Data.Aeson as Aeson
import qualified Data.Aeson.Key as AK
import qualified Data.Aeson.KeyMap as KM
import Data.List (sortOn)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import DMML.Atproto (listRecords, resolveDidToPdsEndpoint, resolveHandle)
import System.Directory (createDirectoryIfMissing, doesFileExist)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import System.FilePath ((</>))
import System.IO (hPutStrLn, stderr)

maxPages :: Int
maxPages = 50

data PulledRecord = PulledRecord
  { prRkey :: Text
  , prDmml :: Text
  }

main :: IO ()
main = do
  args <- getArgs
  case args of
    [peerIdentifier, collection, cursorFile, outDir] -> do
      didResult <-
        if "did:" `T.isPrefixOf` T.pack peerIdentifier
          then pure (Right (T.pack peerIdentifier))
          else resolveHandle (T.pack peerIdentifier)
      case didResult of
        Left err -> hPutStrLn stderr ("resolveHandle failed: " <> show err) >> exitFailure
        Right did -> do
          pdsResult <- resolveDidToPdsEndpoint did
          case pdsResult of
            Left err -> hPutStrLn stderr ("resolveDidToPdsEndpoint failed: " <> show err) >> exitFailure
            Right pdsEndpoint -> do
              haveCursorFile <- doesFileExist cursorFile
              storedCursor <- if haveCursorFile then TIO.readFile cursorFile else pure ""
              allRecords <- pageAll pdsEndpoint did (T.pack collection) Nothing maxPages
              let new =
                    sortOn prRkey
                      [ r
                      | r <- allRecords
                      , prRkey r > T.strip storedCursor
                      ]
              createDirectoryIfMissing True outDir
              paths <- mapM (writeOne outDir) new
              mapM_ putStrLn paths
              -- Deliberately does NOT advance cursorFile itself: this
              -- process has no idea whether the caller will actually
              -- accept this batch (validate-commit could reject any
              -- file in it). Writing the advanced cursor here, before
              -- that's known, would permanently skip a rejected batch
              -- instead of letting the caller retry it -- silently
              -- breaking the same all-or-nothing guarantee broker.sh
              -- already has for the git-fetch path. Instead: write the
              -- candidate next cursor next to the output files, and
              -- leave persisting it to whoever actually incorporates
              -- this batch (see atproto-broker.sh).
              case new of
                [] -> pure ()
                _ -> TIO.writeFile (outDir </> "next-cursor") (prRkey (last new))
    _ ->
      hPutStrLn
        stderr
        "usage: atproto-pull <peer-handle-or-did> <collection> <cursor-file> <out-dir>\n\
        \  prints one newly-materialized .dmml file path per line (empty if nothing new)"
        >> exitFailure

writeOne :: FilePath -> PulledRecord -> IO FilePath
writeOne outDir r = do
  let path = outDir </> T.unpack (prRkey r) <> ".dmml"
  TIO.writeFile path (prDmml r)
  pure path

-- | Retries this many times, per page, before giving up on that one
-- page specifically -- separate from, and on top of, 'DMML.Atproto.
-- runCurl''s own curl-level @--retry@ (which only covers one already-
-- launched curl process's transient failures; this covers the case
-- where 'listRecords' itself returns a hard 'Left', e.g. after curl's
-- own retries are exhausted).
pageRetries :: Int
pageRetries = 3

-- | Real, disclosed fix (jedelman/dmml#7): a single page failing used
-- to call 'exitFailure' unconditionally, discarding every already-
-- fetched page from this same run -- on a genuinely flaky link (the
-- kind this was built against), a drop on page 30 of a 50-page pull
-- meant starting completely over, every time, rather than resuming.
-- Now: retry the failing page a bounded number of times, and if it
-- still fails, stop paging and return whatever was already
-- successfully fetched, instead of throwing it all away. This is safe
-- because 'main' never advances @cursor-file@ unconditionally either
-- way (see this module's own top-of-file doc comment) -- a partial
-- batch here just means a smaller, still-correct set of new files for
-- the caller to validate and incorporate; the NEXT run picks up
-- wherever the stored cursor actually left off, same as it always did.
pageAll :: Text -> Text -> Text -> Maybe Text -> Int -> IO [PulledRecord]
pageAll _ _ _ _ 0 = pure []
pageAll pdsEndpoint did collection cursor pagesLeft = do
  result <- fetchPageWithRetries pageRetries
  case result of
    Nothing -> pure []
    Just v -> do
      let records = extractRecords v
          nextCursor = extractCursor v
      rest <- case nextCursor of
        Just c | not (null records) -> pageAll pdsEndpoint did collection (Just c) (pagesLeft - 1)
        _ -> pure []
      pure (records ++ rest)
  where
    fetchPageWithRetries :: Int -> IO (Maybe Aeson.Value)
    fetchPageWithRetries attemptsLeft = do
      result <- listRecords pdsEndpoint did collection cursor
      case result of
        Right v -> pure (Just v)
        Left err
          | attemptsLeft > 1 -> do
              hPutStrLn
                stderr
                ( "atproto-pull: listRecords failed (retrying, "
                    <> show (attemptsLeft - 1)
                    <> " attempt(s) left): "
                    <> show err
                )
              fetchPageWithRetries (attemptsLeft - 1)
          | otherwise -> do
              hPutStrLn
                stderr
                ( "atproto-pull: listRecords failed after "
                    <> show pageRetries
                    <> " attempt(s), stopping pagination here and returning "
                    <> "what was already fetched: "
                    <> show err
                )
              pure Nothing

extractCursor :: Aeson.Value -> Maybe Text
extractCursor (Aeson.Object o) = case KM.lookup "cursor" o of
  Just (Aeson.String s) -> Just s
  _ -> Nothing
extractCursor _ = Nothing

extractRecords :: Aeson.Value -> [PulledRecord]
extractRecords (Aeson.Object o) = case KM.lookup "records" o of
  Just (Aeson.Array arr) -> [r | Just r <- map recordFromValue (toListArr arr)]
  _ -> []
  where
    toListArr = foldr (:) []
extractRecords _ = []

-- | A record with no @dmml@ field (the legacy @produces@/N-Quads
-- encoding, or a pure retraction with neither) is silently skipped here,
-- not an error -- this pull path only materializes the modern text
-- encoding; a real, disclosed exclusion, matching what the rest of
-- dmml-hs already treats as legacy (see @DMML.Ast@'s own doc comments).
recordFromValue :: Aeson.Value -> Maybe PulledRecord
recordFromValue (Aeson.Object o) = do
  Aeson.String uri <- KM.lookup "uri" o
  Aeson.Object value <- KM.lookup "value" o
  Aeson.String dmml <- KM.lookup (AK.fromText "dmml") value
  let rkey = last (T.splitOn "/" uri)
  pure (PulledRecord rkey dmml)
recordFromValue _ = Nothing
