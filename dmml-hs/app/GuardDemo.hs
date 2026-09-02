{-# LANGUAGE OverloadedStrings #-}

-- | Real, executable smoke test for DMML.Guard -- parses a real machine
-- and real commits (Surface syntax, not hand-built AST), materializes a
-- WorldSnapshot, and checks mayFire against both a satisfied and an
-- unsatisfied guard. Dogfooded, not just typechecked: this is exactly
-- the "prove it before trusting it" discipline this session's design
-- work has applied everywhere else.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Guard (EvalContext (..), mayFire)
import DMML.Materialize (applyCommits)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

machineSrc :: Text
machineSrc =
  "machine contest/x\n\
  \  states\n\
  \    contested\n\
  \    resolved\n\
  \\n\
  \  transition resolve(witness)\n\
  \    contested -> resolved\n\
  \    guard self `witnessedBy` npc/keeper\n\
  \    assert resolved\n"

unwitnessedSrc :: Text
unwitnessedSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  contest/x . state = contested\n"

witnessedSrc :: Text
witnessedSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  declare relation witnessedBy\n\
  \  contest/x . state = contested\n\
  \  contest/x `witnessedBy` npc/keeper\n"

main :: IO ()
main = do
  machine <- case parseMachineSurface machineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  let ctx = EvalContext {ctxSelfNode = "contest/x", ctxParams = Map.empty}

  -- Case 1: contested but unwitnessed -- resolve must be blocked.
  unwitnessedCommit <- case parseCommitSurface unwitnessedSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c
  let unwitnessedSnap = applyCommits "seed" [unwitnessedCommit]
  case mayFire machine "resolve" ctx unwitnessedSnap of
    Nothing -> putStrLn "FAIL: resolve transition not found" >> exitFailure
    Just (True, _, _) -> putStrLn "FAIL: resolve fired without a witness" >> exitFailure
    Just (False, _, _) -> putStrLn "PASS: resolve correctly blocked, no witnessedBy fact"

  -- Case 2: contested AND witnessed -- resolve must be legal.
  witnessedCommit <- case parseCommitSurface witnessedSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c
  let witnessedSnap = applyCommits "seed" [witnessedCommit]
  case mayFire machine "resolve" ctx witnessedSnap of
    Nothing -> putStrLn "FAIL: resolve transition not found" >> exitFailure
    Just (False, _, _) -> putStrLn "FAIL: resolve blocked despite a real witness" >> exitFailure
    Just (True, effects, toState) ->
      putStrLn ("PASS: resolve correctly fires, effects=" <> show effects <> ", to=" <> show toState)
