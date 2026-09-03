{-# LANGUAGE OverloadedStrings #-}

-- | CLI: checks that every fact's predicate, across a set of real
-- commits\/machines, was actually declared somewhere in that same set
-- -- @DMML.SelfDeclaration@'s own doc comment explains why this exists
-- and what real bug it closes. Exit 0 (fully declared) or 1 (lists
-- every undeclared predicate found).
--
-- Usage: check-declared <file.dmml> [<file.dmml> ...]
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (CommitStmt)
import DMML.Materialize (applyCommits)
import DMML.SelfDeclaration (undeclaredPredicates)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> putStrLn "usage: check-declared <file.dmml> [<file.dmml> ...]" >> exitFailure
    paths -> do
      srcs <- mapM TIO.readFile paths
      commits <- mapM classify (zip paths srcs)
      let snap = applyCommits "world" [c | Just c <- commits]
          undeclared = undeclaredPredicates snap
      if null undeclared
        then putStrLn "check-declared: OK -- every used predicate is declared"
        else do
          putStrLn "check-declared: UNDECLARED predicates found:"
          mapM_ (\p -> putStrLn ("  " <> T.unpack p)) undeclared
          exitFailure
  where
    -- 'Nothing' for a real machine file (nothing to materialize there);
    -- a genuine parse failure is still a hard error, same as every
    -- other CLI in this project.
    classify :: (FilePath, T.Text) -> IO (Maybe CommitStmt)
    classify (path, src) = case parseCommitSurface src of
      Right c -> pure (Just c)
      Left commitErr -> case parseMachineSurface src of
        Right _ -> pure Nothing
        Left _ -> putStrLn (path <> ":\n" <> errorBundlePretty commitErr) >> exitFailure
