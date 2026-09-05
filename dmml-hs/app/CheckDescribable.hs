{-# LANGUAGE OverloadedStrings #-}

-- | CLI: reports every subject in a materialized world that has NO
-- reachable description -- neither a direct naming fact nor a
-- reachable governing machine (`DMML.Describability`). The discipline
-- check "the director is officially out of the picture" (SPEC.md
-- §19.4) actually needs: nothing else in this codebase enforces that
-- every describable thing in a world really IS describable.
--
-- Usage: check-describable [--naming-predicate <pred>]... <file.dmml> [<file.dmml> ...]
--   Default naming predicates: name, epithet (this session's own
--   convention -- override if a world uses different vocabulary).
--   Exits 0 (every subject describable) or 1 (reports every
--   undescribable subject found).
module Main (main) where

import qualified Data.ByteString as BS
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt)
import DMML.Describability (DescribabilityReport (..), checkDescribability)
import DMML.Materialize (applyCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

main :: IO ()
main = do
  args <- getArgs
  case parseArgs args [] [] of
    Nothing -> usage >> exitFailure
    Just (namingPreds, paths) -> do
      let namingPredicates = if null namingPreds then ["name", "epithet"] else namingPreds
      contents <- mapM BS.readFile paths
      commits <- mapM (classify . decodeUtf8Pair) (zip paths contents)
      let snap = applyCommits "world" [c | Just c <- commits]
          report = checkDescribability namingPredicates snap
      case undescribableSubjects report of
        [] ->
          putStrLn
            ( "check-describable: OK -- all "
                <> show (length (describableSubjects report))
                <> " subject(s) describable"
            )
        undescribable -> do
          putStrLn "check-describable: UNDESCRIBABLE SUBJECTS FOUND"
          mapM_ (\s -> putStrLn ("  - " <> T.unpack s)) undescribable
          exitFailure
  where
    usage =
      putStrLn
        "usage: check-describable [--naming-predicate <pred>]... <file.dmml> [<file.dmml> ...]"

    parseArgs [] preds paths
      | null paths = Nothing
      | otherwise = Just (reverse preds, reverse paths)
    parseArgs ("--naming-predicate" : p : rest) preds paths = parseArgs rest (T.pack p : preds) paths
    parseArgs (path : rest) preds paths = parseArgs rest preds (path : paths)

    decodeUtf8Pair (path, bs) = (path, TE.decodeUtf8 bs)

    classify :: (FilePath, Text) -> IO (Maybe CommitStmt)
    classify (path, src) = case parseCommitSurface src of
      Right c -> pure (Just c)
      Left commitErr -> case parseMachineSurface src of
        Right _ -> pure Nothing
        Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure
