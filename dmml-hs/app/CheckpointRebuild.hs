{-# LANGUAGE BangPatterns #-}
{-# LANGUAGE OverloadedStrings #-}

-- | CLI: folds ONLY the new commits/*.dmml files a single merge just
-- introduced into a parent checkpoint's raw materialized facts, and
-- writes the result as a new checkpoint -- O(new files this merge
-- introduces), never a full replay from genesis. This is the mechanism
-- 'sync-spike/broker/hooks/pre-merge-commit' calls to compute and stage
-- a checkpoint into the SAME merge commit it's already creating for the
-- real content, per @written-world@'s
-- @dev-journal/2026-09-02-checkpoint-per-commit.md@.
--
-- Deliberately writes the RAW (pre-'DMML.Governance.applyGovernance')
-- snapshot -- see 'DMML.Checkpoint''s own doc comment for why. Deriving
-- the governed/rendered view from a checkpoint is a separate, later
-- step (today, still 'sync-spike/broker/rebuild-cache.sh''s job).
--
-- Usage:
--   checkpoint-rebuild <tree-sha> <output.json> <parent.json|none> <newfile.dmml> [...]
--     tree-sha:   the resulting commits/ tree hash this checkpoint will
--                 be keyed by (the caller's job to compute -- this CLI
--                 never touches git itself).
--     output.json: where to write the new checkpoint.
--     parent.json|none: the checkpoint to fold new files into, or the
--                 literal "none" for a from-scratch fold (bootstrap: no
--                 parent checkpoint exists yet, e.g. the first merge
--                 after genesis).
--     newfile.dmml...: the files THIS merge introduces, and ONLY those
--                 -- passing every file in commits/ here defeats the
--                 entire point; the caller (pre-merge-commit) is
--                 responsible for scoping this correctly, same
--                 responsibility it already has for shape-validation.
--                 May freely include machine-definition files mixed in
--                 with real commits -- see 'foldFiles' below for why
--                 those are silently skipped, not an error.
module Main (main) where

import qualified Data.ByteString.Lazy as BL
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Exit (exitFailure)
import System.Environment (getArgs)
import Text.Megaparsec (errorBundlePretty)

import DMML.Checkpoint
  ( checkpointToSnapshot
  , decodeCheckpoint
  , encodeCheckpoint
  , snapshotToCheckpoint
  )
import DMML.Materialize (applyCommit, emptySnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

main :: IO ()
main = do
  args <- getArgs
  case args of
    (treeSha : outPath : parentArg : newFiles) -> do
      parentSnap <- case parentArg of
        "none" -> pure emptySnapshot
        path -> do
          raw <- BL.readFile path
          case decodeCheckpoint raw of
            Nothing -> putStrLn ("checkpoint-rebuild: failed to decode parent checkpoint " <> path) >> exitFailure
            Just ck -> pure (checkpointToSnapshot ck)
      (newSnap, foldedCount) <- foldFiles parentSnap 0 newFiles
      let out = snapshotToCheckpoint (T.pack treeSha) newSnap
          skippedCount = length newFiles - foldedCount
      BL.writeFile outPath (encodeCheckpoint out)
      putStrLn
        ( "checkpoint-rebuild: wrote "
            <> outPath
            <> " (tree "
            <> treeSha
            <> ", "
            <> show (length newFiles)
            <> " new file(s) given, "
            <> show foldedCount
            <> " folded as commits, "
            <> show skippedCount
            <> " skipped as machine defs, over parent "
            <> parentArg
            <> ")"
        )
    _ ->
      putStrLn "usage: checkpoint-rebuild <tree-sha> <output.json> <parent.json|none> <newfile.dmml> [...]"
        >> exitFailure
  where
    -- CORRECTED after a real endurance-scale run found this wrong (not
    -- just reasoned about): a machine-definition file showing up in the
    -- new-files list is the NORMAL case, not a caller error -- both the
    -- bootstrap fold (seed content always includes machine files
    -- alongside real commits) and, more importantly, ordinary steady-
    -- state operation (an agent minting a brand-new machine mid-run is
    -- completely routine authored content, confirmed happening for real
    -- in a 40-commit worktree-sync run) can put one in this file list.
    -- Machines simply don't belong in the raw fact checkpoint at all
    -- (DMML.Checkpoint's own doc comment) -- silently skipping one here
    -- is the correct behavior, not information loss: nothing about a
    -- machine file was ever going to be folded into this snapshot
    -- regardless of how it arrived. A genuine parse failure (neither a
    -- commit nor a machine) stays a hard error -- real malformed
    -- content, which pre-merge-commit's own shape validation should
    -- already have caught before this ever runs, kept here as a safety
    -- net, not the expected path.
    foldFiles snap !n [] = pure (snap, n)
    foldFiles snap !n (path : rest) = do
      src <- TIO.readFile path
      case parseCommitSurface src of
        Right stmt -> foldFiles (applyCommit "merge" snap stmt) (n + 1) rest
        Left commitErr -> case parseMachineSurface src of
          Right _ -> foldFiles snap n rest
          Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure
