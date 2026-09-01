{-# LANGUAGE OverloadedStrings #-}

-- | CLI: parse a .dmml file (Surface-syntax commits, one after another
-- -- see the caveat in Main below) and print the materialized
-- WorldSnapshot's rendering. Built for compliance-surface-informed/'s
-- dispatch script to generate real "world so far" context from
-- examples/shrine-genesis.dmml, rather than a copy-pasted static string
-- that could drift from the actual seed file.
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
    [path] -> do
      src <- TIO.readFile path
      case parseCommitSurface src of
        Left err -> putStrLn (errorBundlePretty err) >> exitFailure
        Right stmt -> TIO.putStr (renderSnapshot (applyCommits [stmt]))
    _ -> putStrLn "usage: render-snapshot <file.dmml>" >> exitFailure
