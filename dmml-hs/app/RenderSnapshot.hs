{-# LANGUAGE OverloadedStrings #-}

-- | CLI: parse one or more .dmml files (Surface-syntax commits OR
-- machines, one top-level item per file) IN ORDER and print the
-- materialized WorldSnapshot's rendering after applying all of them.
-- Built for compliance-surface-informed/, compliance-world-assembly/,
-- and sync-spike/'s dispatch scripts to generate real "world so far"
-- context from a real chain of files, rather than a copy-pasted static
-- string that could drift from the actual seed content.
--
-- REWORKED 2026-09-02 (Phase D3, jedelman/dmml#1): machines are no
-- longer discarded after validation. Every parsed machine is kept
-- (keyed by its own node) and DMML.Governance.applyGovernance is run
-- over the materialized snapshot before rendering -- a governed pair
-- that validates gets collapsed to its one canonical value, exactly
-- like any other real-world read now would; an ungoverned or still-
-- pending pair renders every live alternative, same as before.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import System.Environment (getArgs)
import System.Exit (exitFailure)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt, MachineStmt (..), NodeRef (nodeRefSegments))
import DMML.Governance (applyGovernance)
import DMML.Materialize (applyCommits, renderSnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> putStrLn "usage: render-snapshot <file.dmml> [<file.dmml> ...]" >> exitFailure
    paths -> do
      srcs <- mapM TIO.readFile paths
      let results = zip paths (map classify srcs)
      case [(p, e) | (p, Left e) <- results] of
        ((p, e) : _) -> putStrLn (p <> ":\n" <> e) >> exitFailure
        [] -> do
          let stmts = [c | (_, Right (Left c)) <- results]
              machines = [m | (_, Right (Right m)) <- results]
              machineMap = Map.fromList [(nodeRefText (machineNode m), m) | m <- machines]
          -- No cross-branch divergence to attribute here -- one ordered,
          -- single-author file list, not two independently-labeled
          -- sides -- so a fixed batch label is fine.
          let snap = applyGovernance machineMap (applyCommits "world" stmts)
          TIO.putStr (renderSnapshot snap)
  where
    classify :: Text -> Either String (Either CommitStmt MachineStmt)
    classify src = case parseCommitSurface src of
      Right stmt -> Right (Left stmt)
      Left commitErr -> case parseMachineSurface src of
        Right machine -> Right (Right machine)
        Left _ -> Left (errorBundlePretty commitErr)
