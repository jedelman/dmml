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
      newSnap <- foldFiles parentSnap newFiles
      let out = snapshotToCheckpoint (T.pack treeSha) newSnap
      BL.writeFile outPath (encodeCheckpoint out)
      putStrLn
        ( "checkpoint-rebuild: wrote "
            <> outPath
            <> " (tree "
            <> treeSha
            <> ", "
            <> show (length newFiles)
            <> " new file(s) folded in over parent "
            <> parentArg
            <> ")"
        )
    _ ->
      putStrLn "usage: checkpoint-rebuild <tree-sha> <output.json> <parent.json|none> <newfile.dmml> [...]"
        >> exitFailure
  where
    -- Every new file must be a real, already-validated commit -- a
    -- machine definition here is a real caller error (machines aren't
    -- folded into the fact checkpoint at all, see DMML.Checkpoint's own
    -- doc comment), not silently skipped, since silently dropping it
    -- would leave the checkpoint's caller believing it was accounted
    -- for.
    foldFiles snap [] = pure snap
    foldFiles snap (path : rest) = do
      src <- TIO.readFile path
      case parseCommitSurface src of
        Right stmt -> foldFiles (applyCommit "merge" snap stmt) rest
        Left commitErr -> case parseMachineSurface src of
          Right _ ->
            putStrLn (path <> ": checkpoint-rebuild only folds commits, not machine defs")
              >> exitFailure
          Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure
