{-# LANGUAGE OverloadedStrings #-}

-- | Reports real cross-branch divergence -- never mints anything.
--
-- REWORKED 2026-09-02 per Jason's "collision-free mints" redesign
-- (written-world/dev-journal/2026-09-02-machines-as-facts-generic-guard-
-- evaluator.md's follow-on conversation, jedelman/dmml#1): the previous
-- version of this file minted a @Contest@ fact-commit plus a bespoke
-- resolution machine on every detected divergence -- exactly the
-- "guard should not be minting contests" move Jason called out directly.
-- Mints are collision-free now (DMML.Materialize's 'Alternatives'):
-- applying both sides' commits and unioning the two snapshots via
-- 'mergeSnapshots' IS the whole mechanism -- every (subject, predicate)
-- pair both sides genuinely diverge on ends up multi-valued automatically,
-- with zero special-casing and nothing written to disk.
--
-- REWORKED AGAIN, same day (Phase D3): machines are no longer discarded.
-- Every parsed machine from EITHER side is kept and
-- DMML.Governance.applyGovernance runs over the merged snapshot before
-- reporting -- a pair that's genuinely governed and already validates
-- (e.g. a real witnessed resolve already sitting in one side's history)
-- is correctly reported as settled, not as live divergence. Only a
-- still-multi-valued pair after governance is real, unresolved
-- divergence worth surfacing.
--
-- Usage: check-divergence <mine-list-file> <peer-list-file> <output-dir> <mine-label> <peer-label>
--   Each list file: one .dmml path per line (may be empty).
--   <output-dir> is accepted but unused -- kept so existing callers
--   (post-merge, broker.sh) don't need an argv shape change.
-- Always exits 0 -- divergence was never a reason to block a merge, and
-- still isn't; it's just no longer a reason to write new files either.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt, Literal (..), MachineStmt (..), NodeRef (nodeRefSegments), Value (..))
import DMML.Governance (applyGovernance)
import DMML.Materialize (WorldSnapshot (..), applyCommits, currentValue, mergeSnapshots)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

readListFile :: FilePath -> IO [FilePath]
readListFile p = do
  contents <- readFile p
  pure [l | l <- lines contents, not (null l)]

-- | A "new since merge-base" file list is real diff output over ALL
-- commits/*.dmml additions -- commits AND machines both, whatever an
-- agent minted. Returns both: commits materialize into facts, machines
-- are retained (keyed by their own node) for governance lookup.
materializeFiles :: Text -> [FilePath] -> IO (WorldSnapshot, Map.Map Text MachineStmt)
materializeFiles label paths = do
  srcs <- mapM TIO.readFile paths
  let classified = zip paths (map classify srcs)
  case [(p, e) | (p, Left e) <- classified] of
    ((p, e) : _) -> error (p <> ":\n" <> e)
    [] ->
      let commits = [c | (_, Right (Left c)) <- classified]
          machines = [m | (_, Right (Right m)) <- classified]
          machineMap = Map.fromList [(nodeRefText (machineNode m), m) | m <- machines]
       in pure (applyCommits label commits, machineMap)
  where
    classify :: Text -> Either String (Either CommitStmt MachineStmt)
    classify src = case parseCommitSurface src of
      Right stmt -> Right (Left stmt)
      Left commitErr -> case parseMachineSurface src of
        Right machine -> Right (Right machine)
        Left _ -> Left (errorBundlePretty commitErr)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [mineListPath, peerListPath, _outDir, mineLabel, peerLabel] -> do
      mineFiles <- readListFile mineListPath
      peerFiles <- readListFile peerListPath
      (mineSnap, mineMachines) <- materializeFiles (T.pack mineLabel) mineFiles
      (peerSnap, peerMachines) <- materializeFiles (T.pack peerLabel) peerFiles
      let merged = mergeSnapshots mineSnap peerSnap
          machines = Map.union mineMachines peerMachines
          governed = applyGovernance machines merged
      -- Report, don't mint: every (subject, predicate) pair STILL
      -- multi-valued after governance is real, unresolved divergence --
      -- but it's already IN the merged commit history (each side's own
      -- file already exists on disk), nothing new to write.
      let reallyDivergent =
            [ (k, vs)
            | (k, _) <- Map.toList (snapshotFacts governed)
            , let vs = currentValue k governed
            , length vs > 1
            ]
      if null reallyDivergent
        then putStrLn "no divergence"
        else mapM_ (report mineLabel peerLabel) reallyDivergent
    _ -> putStrLn "usage: check-divergence <mine-list-file> <peer-list-file> <output-dir> <mine-label> <peer-label>" >> exitFailure
  where
    report :: String -> String -> ((Text, Text), [(Text, Value)]) -> IO ()
    report _mineLabel _peerLabel ((subj, pred_), opts) = do
      putStrLn ("DIVERGENCE (live, unresolved): " <> T.unpack subj <> " . " <> T.unpack pred_)
      mapM_ (\(label, v) -> putStrLn ("  " <> T.unpack label <> " asserts " <> T.unpack (renderValue v))) opts

    renderValue :: Value -> Text
    renderValue (ValueNode n) = T.intercalate "/" (nodeRefSegments n)
    renderValue (ValueLiteral (LitString s)) = "\"" <> s <> "\""
    renderValue (ValueLiteral (LitNumber n)) = n
    renderValue (ValueLiteral (LitBoolean b)) = if b then "true" else "false"
