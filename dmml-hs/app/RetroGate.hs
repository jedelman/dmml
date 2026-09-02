{-# LANGUAGE OverloadedStrings #-}

-- | CLI: gates one candidate retro commit against the FULL current
-- tree, not just the one transition it was generated to satisfy.
-- "let's gate retro commits on a consistent tree" (Jason, 2026-09-02).
-- Real companion to 'DMML.Retroconsistency.gateConsistentTree' -- see
-- that function's own doc comment for exactly what "consistent" means
-- here (no currently-satisfied negated guard, anywhere in the known
-- machine set, gets newly blocked) and what it deliberately does not
-- check ($param-dependent guards, domain/narrative plausibility).
--
-- Usage:
--   retro-gate <candidate.dmml> <world-file.dmml> [<world-file.dmml> ...]
--     candidate.dmml:  the proposed retro commit (must parse as a
--                      commit, not a machine).
--     world-file.dmml...: every commit AND machine file that makes up
--                      the current tree -- the caller's job to pass the
--                      real, current set (same responsibility
--                      render-snapshot's own callers already have).
--
-- Exit 0 (GateOk) or 1 (GateBroken, with which guard(s) it would break)
-- -- meant to sit in front of actually committing a retro commit, the
-- same role pre-merge-commit plays for ordinary merges, just invoked
-- explicitly rather than as a git hook (retro commits aren't produced
-- at merge time).
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure, exitSuccess)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt, MachineStmt (..), NodeRef (nodeRefSegments))
import DMML.Materialize (applyCommit, applyCommits)
import DMML.Retroconsistency (BrokenGuard (..), GateResult (..), gateConsistentTree)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

classify :: FilePath -> Text -> IO (Either CommitStmt MachineStmt)
classify path src = case parseCommitSurface src of
  Right c -> pure (Left c)
  Left commitErr -> case parseMachineSurface src of
    Right m -> pure (Right m)
    Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure

main :: IO ()
main = do
  args <- getArgs
  case args of
    (candidatePath : worldPaths@(_ : _)) -> do
      candidateSrc <- TIO.readFile candidatePath
      candidate <- case parseCommitSurface candidateSrc of
        Right c -> pure c
        Left e -> putStrLn (candidatePath <> ": candidate must parse as a commit, not a machine:\n" <> errorBundlePretty e) >> exitFailure

      worldSrcs <- mapM TIO.readFile worldPaths
      classified <- mapM (uncurry classify) (zip worldPaths worldSrcs)
      let commits = [c | Left c <- classified]
          machineList = [m | Right m <- classified]
          machines = Map.fromList [(nodeRefText (machineNode m), m) | m <- machineList]
          before = applyCommits "world" commits
          after = applyCommit "candidate" before candidate

      case gateConsistentTree machines before after of
        GateOk -> do
          putStrLn "retro-gate: OK -- no currently-satisfied negated guard is broken by this candidate"
          exitSuccess
        GateBroken broken -> do
          putStrLn "retro-gate: REJECTED -- this candidate would break the following guard(s):"
          mapM_
            (\b -> putStrLn ("  " <> T.unpack (brokenMachine b) <> "'s " <> T.unpack (brokenTransition b) <> " (predicate " <> T.unpack (brokenPredicate b) <> ")"))
            broken
          exitFailure
    _ ->
      putStrLn "usage: retro-gate <candidate.dmml> <world-file.dmml> [<world-file.dmml> ...]"
        >> exitFailure
