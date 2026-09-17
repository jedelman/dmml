{-# LANGUAGE OverloadedStrings #-}

-- | CLI: flags every bare (slash-free) guard-pattern term, across a
-- real set of machine files, whose text collides with a state name
-- some machine in that same set actually declares --
-- @DMML.GuardLiterals@'s own doc comment explains why this exists and
-- what real, repeated bug it catches. Non-blocking: exit 0 either way
-- is NOT what this does (mirrors @check-declared@'s own convention of
-- exit 1 when it finds something), but finding something here is a
-- warning to read, not proof the content is wrong -- see the module
-- doc for why false positives are possible by design.
--
-- Usage: check-guard-literals <machine.dmml> [<machine.dmml> ...]
module Main (main) where

import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (NodeRef, nodeRefSegments)
import DMML.GuardLiterals (SuspiciousTerm (..), suspiciousGuardTerms)
import DMML.Surface (parseMachineSurface)

nodeRefText :: NodeRef -> T.Text
nodeRefText = T.intercalate "/" . nodeRefSegments

main :: IO ()
main = do
  args <- getArgs
  case args of
    [] -> putStrLn "usage: check-guard-literals <machine.dmml> [<machine.dmml> ...]" >> exitFailure
    paths -> do
      machines <- mapM parseOne paths
      let hits = suspiciousGuardTerms machines
      if null hits
        then putStrLn "check-guard-literals: OK -- no bare guard term collides with a declared state name"
        else do
          putStrLn "check-guard-literals: SUSPICIOUS bare guard terms found:"
          mapM_ printHit hits
          exitFailure
  where
    parseOne path = do
      src <- TIO.readFile path
      case parseMachineSurface src of
        Right m -> pure m
        Left err -> putStrLn (path <> ":\n" <> errorBundlePretty err) >> exitFailure >> error "unreachable"

    printHit st =
      putStrLn $
        "  "
          <> T.unpack (nodeRefText (stMachine st))
          <> "'s `"
          <> T.unpack (stTransition st)
          <> "` guards on bare `"
          <> T.unpack (stVar st)
          <> "` -- that has no `/`, so it's an EXISTENTIAL VARIABLE (matches ANY value), not a literal "
          <> "match, even though "
          <> T.unpack (T.intercalate ", " (map nodeRefText (stDeclaredBy st)))
          <> " declares a state named exactly that. If a literal match was intended, guard on a "
          <> "separate, multi-segment, externally-checkable fact instead (see the dmml-authoring skill's "
          <> "first documented landmine)."
