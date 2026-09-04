{-# LANGUAGE OverloadedStrings #-}

-- | CLI: checks that every @consumes@ citation's cid, across a real set
-- of @.dmml@ files, is consistent -- jedelman/dmml#6. @DMML.
-- CitationIntegrity@'s own doc comment explains exactly what "observed"
-- means and why. Every file given on argv is treated as independently
-- read (its own real 'DMML.LocalIdentity.localFileRef' seeds the
-- ledger) -- so a citation naming one of these same files must match
-- its recomputed cid exactly; a citation to some uri not in this batch
-- falls back to first-citation-wins, same real, disclosed limit the
-- retired Rust crate had. Exit 0 (consistent) or 1 (reports the first
-- mismatch found).
--
-- Usage: check-citations <file.dmml> [<file.dmml> ...]
module Main (main) where

import qualified Data.ByteString as BS
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt)
import DMML.CitationIntegrity
  ( CitationError (..)
  , checkCommits
  , emptyCidLedger
  , seedObserved
  )
import DMML.LocalIdentity (localFileRef)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> putStrLn "usage: check-citations <file.dmml> [<file.dmml> ...]" >> exitFailure
    paths -> do
      contents <- mapM BS.readFile paths
      let seeded = foldr seedObserved emptyCidLedger [localFileRef p c | (p, c) <- zip paths contents]
      commits <- mapM (classify . decodeUtf8Pair) (zip paths contents)
      case checkCommits seeded [c | Just c <- commits] of
        Right _ -> putStrLn "check-citations: OK -- every consumes citation is consistent"
        Left (CitationCidMismatch uri expected actual) -> do
          putStrLn "check-citations: CITATION MISMATCH"
          putStrLn ("  uri: " <> T.unpack uri)
          putStrLn ("  already on record: " <> T.unpack expected)
          putStrLn ("  this citation claims: " <> T.unpack actual)
          exitFailure
  where
    decodeUtf8Pair (path, bs) = (path, TE.decodeUtf8 bs)

    classify :: (FilePath, T.Text) -> IO (Maybe CommitStmt)
    classify (path, src) = case parseCommitSurface src of
      Right c -> pure (Just c)
      Left commitErr -> case parseMachineSurface src of
        Right _ -> pure Nothing
        Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure
