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
-- with zero special-casing and nothing written to disk. This tool now
-- only REPORTS which pairs are multi-valued, for a human/hook message --
-- reducing a governed predicate's alternatives to one canonical value is
-- real, not-yet-built work (governed-machine arbitration, see the
-- tracking issue) that belongs at materialization/read time, not here.
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

import DMML.Ast (Literal (..), NodeRef (nodeRefSegments), Value (..))
import DMML.Materialize (WorldSnapshot (..), applyCommits, currentValue, mergeSnapshots)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

readListFile :: FilePath -> IO [FilePath]
readListFile p = do
  contents <- readFile p
  pure [l | l <- lines contents, not (null l)]

-- | A "new since merge-base" file list is real diff output over ALL
-- commits/*.dmml additions -- commits AND machines both, whatever an
-- agent minted. Classify-then-skip: a machine parses and validates like
-- anything else, it just contributes no facts to the snapshot.
materializeFiles :: Text -> [FilePath] -> IO WorldSnapshot
materializeFiles label paths = do
  srcs <- mapM TIO.readFile paths
  let classified = zip paths (map classify srcs)
  case [(p, e) | (p, Left e) <- classified] of
    ((p, e) : _) -> error (p <> ":\n" <> e)
    [] -> pure (applyCommits label [stmt | (_, Right (Just stmt)) <- classified])
  where
    classify src = case parseCommitSurface src of
      Right stmt -> Right (Just stmt)
      Left commitErr -> case parseMachineSurface src of
        Right _ -> Right Nothing
        Left _ -> Left (errorBundlePretty commitErr)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [mineListPath, peerListPath, _outDir, mineLabel, peerLabel] -> do
      mineFiles <- readListFile mineListPath
      peerFiles <- readListFile peerListPath
      mineSnap <- materializeFiles (T.pack mineLabel) mineFiles
      peerSnap <- materializeFiles (T.pack peerLabel) peerFiles
      let merged = mergeSnapshots mineSnap peerSnap
      -- Report, don't mint: every (subject, predicate) pair with more
      -- than one live alternative after the merge is real, surfaced
      -- divergence -- but it's already IN the merged commit history
      -- (each side's own file already exists on disk), nothing new to
      -- write.
      let reallyDivergent =
            [ (k, vs)
            | (k, _) <- Map.toList (snapshotFacts merged)
            , let vs = currentValue k merged
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
