{-# LANGUAGE OverloadedStrings #-}

-- | The real "trust step peer-to-peer needs that single-player didn't":
-- given the set of commits I've authored since the last common point
-- with a peer, and the set of commits THEY'VE authored since that same
-- point, does their delta touch any (subject, predicate) pair mine
-- also touches? If not, this is ordinary append-only growth -- safe to
-- merge, no real conflict exists because nothing overlaps. If so, both
-- sides asserted something about the same fact without either having
-- seen the other's change -- not a git merge conflict (each commit is
-- its own file, git merges those trivially), a SEMANTIC divergence
-- naive last-write-wins would paper over silently.
--
-- Usage: check-divergence <mine-list-file> <peer-list-file>
--   Each list file: one .dmml path per line (may be empty).
-- Exit 0 and prints "no divergence" if the two deltas don't overlap.
-- Exit 1 and prints every overlapping (subject, predicate) with both
-- sides' asserted values if they do.
module Main (main) where

import qualified Data.Map.Strict as Map
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure, exitSuccess)
import Text.Megaparsec (errorBundlePretty)

import DMML.Materialize (WorldSnapshot (..), applyCommits)
import DMML.Surface (parseCommitSurface)

readListFile :: FilePath -> IO [FilePath]
readListFile p = do
  contents <- readFile p
  pure [l | l <- lines contents, not (null l)]

materializeFiles :: [FilePath] -> IO WorldSnapshot
materializeFiles paths = do
  srcs <- mapM TIO.readFile paths
  let parsed = zip paths (map parseCommitSurface srcs)
  case [(p, e) | (p, Left e) <- parsed] of
    ((p, e) : _) -> error (p <> ":\n" <> errorBundlePretty e)
    [] -> pure (applyCommits [stmt | (_, Right stmt) <- parsed])

main :: IO ()
main = do
  args <- getArgs
  case args of
    [mineListPath, peerListPath] -> do
      mineFiles <- readListFile mineListPath
      peerFiles <- readListFile peerListPath
      mineSnap <- materializeFiles mineFiles
      peerSnap <- materializeFiles peerFiles
      reportOverlap mineSnap peerSnap
    _ -> putStrLn "usage: check-divergence <mine-list-file> <peer-list-file>" >> exitFailure

reportOverlap :: WorldSnapshot -> WorldSnapshot -> IO ()
reportOverlap mineSnap peerSnap = do
  let overlapKeys =
        [ k
        | k <- Map.keys (snapshotFacts mineSnap)
        , k `Map.member` snapshotFacts peerSnap
        ]
  if null overlapKeys
    then putStrLn "no divergence" >> exitSuccess
    else do
      putStrLn "DIVERGENCE -- both sides independently asserted:"
      mapM_ report overlapKeys
      exitFailure
  where
    report (subj, pred_) = do
      let mv = Map.lookup (subj, pred_) (snapshotFacts mineSnap)
          pv = Map.lookup (subj, pred_) (snapshotFacts peerSnap)
      putStrLn ("  " <> T.unpack subj <> " . " <> T.unpack pred_ <> ":")
      putStrLn ("    mine: " <> show mv)
      putStrLn ("    peer: " <> show pv)
