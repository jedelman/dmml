{-# LANGUAGE OverloadedStrings #-}

-- | CLI shape-validator for one .dmml (Surface-syntax) file. Built for
-- the peer-to-peer git broker spike (written-world/sync-spike/) --
-- exits 0 and prints nothing on success, exits 1 and prints a real
-- parse error on failure, so a shell script can gate a merge on it.
--
-- Scope, stated plainly: this checks SHAPE only (does the file parse to
-- a valid CommitStmt) via DMML.Surface -- it does NOT check semantic
-- validity against the accumulated world state (self-declaration,
-- duplicate-fact-across-commits, consumed-cid-actually-exists). Those
-- live in the Rust crate's validate.rs/interpret.rs, not yet ported to
-- dmml-hs (see dmml-hs/../written-world/dev-journal/2026-08-31-dmml-
-- runtime-migration-scope.md's Phase 2). A broker built on this alone
-- catches malformed commits, not unsafe-but-well-formed ones.
module Main (main) where

import System.Environment (getArgs)
import System.Exit (exitFailure, exitSuccess)
import qualified Data.Text.IO as TIO
import Text.Megaparsec (errorBundlePretty)

import DMML.Surface (parseCommitSurface)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [path] -> do
      src <- TIO.readFile path
      case parseCommitSurface src of
        Left err -> do
          putStrLn (path <> ": REJECTED")
          putStrLn (errorBundlePretty err)
          exitFailure
        Right _ -> exitSuccess
    _ -> do
      putStrLn "usage: validate-commit <file.dmml>"
      exitFailure
