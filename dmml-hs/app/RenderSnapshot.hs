{-# LANGUAGE OverloadedStrings #-}

-- | CLI: parse one or more .dmml files (Surface-syntax commits, each
-- file one commit) IN ORDER and print the materialized WorldSnapshot's
-- rendering after applying all of them. Built for
-- compliance-surface-informed/ and compliance-world-assembly/'s
-- dispatch scripts to generate real "world so far" context from a real
-- chain of files, rather than a copy-pasted static string that could
-- drift from the actual seed content.
module Main (main) where

import System.Environment (getArgs)
import System.Exit (exitFailure)
import qualified Data.Text.IO as TIO
import Text.Megaparsec (errorBundlePretty)

import DMML.Materialize (applyCommits, renderSnapshot)
import DMML.Surface (parseCommitSurface)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> putStrLn "usage: render-snapshot <file.dmml> [<file.dmml> ...]" >> exitFailure
    paths -> do
      srcs <- mapM TIO.readFile paths
      let parsed = zip paths (map parseCommitSurface srcs)
      case [(p, e) | (p, Left e) <- parsed] of
        ((p, e) : _) -> putStrLn (p <> ":\n" <> errorBundlePretty e) >> exitFailure
        [] -> do
          let stmts = [stmt | (_, Right stmt) <- parsed]
          TIO.putStr (renderSnapshot (applyCommits stmts))
