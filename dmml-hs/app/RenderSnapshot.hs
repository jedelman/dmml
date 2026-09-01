{-# LANGUAGE OverloadedStrings #-}

-- | CLI: parse one or more .dmml files (Surface-syntax commits OR
-- machines, one top-level item per file) IN ORDER and print the
-- materialized WorldSnapshot's rendering after applying all of them.
-- Machine files are validated (so a bad one still fails the whole run)
-- but contribute no facts to the snapshot -- only commits do; a
-- machine's own behavior isn't executed here (no guard evaluator, see
-- DMML.Materialize.applyContests's own doc comment). Built for
-- compliance-surface-informed/, compliance-world-assembly/, and
-- sync-spike/'s dispatch scripts to generate real "world so far"
-- context from a real chain of files, rather than a copy-pasted static
-- string that could drift from the actual seed content.
module Main (main) where

import System.Environment (getArgs)
import System.Exit (exitFailure)
import qualified Data.Text.IO as TIO
import Text.Megaparsec (errorBundlePretty)

import DMML.Materialize (applyCommits, applyContests, renderSnapshot)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

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
          let stmts = [stmt | (_, Right (Just stmt)) <- results]
          TIO.putStr (renderSnapshot (applyContests (applyCommits stmts)))
  where
    -- Right (Just stmt): a valid commit, contributes facts.
    -- Right Nothing: a valid machine, contributes nothing to the snapshot.
    -- Left err: neither parses.
    classify src = case parseCommitSurface src of
      Right stmt -> Right (Just stmt)
      Left commitErr -> case parseMachineSurface src of
        Right _ -> Right Nothing
        Left _ -> Left (errorBundlePretty commitErr)
