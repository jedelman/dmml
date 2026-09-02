{-# LANGUAGE OverloadedStrings #-}

-- | Real, executable proof for DMML.Retroconsistency.fixpointRetroconsistency
-- -- Jason's quarry example, the "cross machine chaining" half: a quarry
-- is asserted quarried with no `quarriedBy` AND no `deliversTo` fact,
-- and the delivery target (warehouse/central) is itself a real,
-- separately-declared machine with its OWN unsatisfied precondition
-- (needs a clerk on record before it can be "receiving"). One
-- fixpointRetroconsistency call should surface BOTH machines' gaps, in
-- order, each gated against the full set.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt (..), NodeRef (nodeRefSegments))
import DMML.Materialize (applyCommits)
import DMML.Retroconsistency (ChainResult (..), ChainStep (..), ImpliedFact (..), fixpointRetroconsistency)
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import qualified Data.Text as T

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

quarryMachineSrc :: Text
quarryMachineSrc =
  "machine quarry/east\n\
  \  states\n\
  \    intact\n\
  \    quarried\n\
  \\n\
  \  transition extract(quarrier)\n\
  \    intact -> quarried\n\
  \    guard self `quarriedBy` who\n\
  \    guard self `deliversTo` warehouse/central\n\
  \    assert quarried\n"

-- | A REAL, separate machine, governing a DIFFERENT node
-- (warehouse/central), with its own unrelated precondition. Nothing
-- about quarry/east's own transition mentions this guard at all -- it
-- only becomes relevant because the quarry's OWN implied
-- `deliversTo warehouse/central` fact points at a node this machine
-- happens to govern.
warehouseMachineSrc :: Text
warehouseMachineSrc =
  "machine warehouse/central\n\
  \  states\n\
  \    empty\n\
  \    receiving\n\
  \\n\
  \  transition open(clerk)\n\
  \    empty -> receiving\n\
  \    guard self `staffedBy` clerk2\n\
  \    assert receiving\n"

worldSrc :: Text
worldSrc =
  "commit mints\n\
  \  declare attribute state\n\
  \  declare relation quarriedBy\n\
  \  declare relation deliversTo\n\
  \  declare relation staffedBy\n\
  \  quarry/east . state = quarried\n\
  \  warehouse/central . state = receiving\n"

main :: IO ()
main = do
  quarryMachine <- case parseMachineSurface quarryMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  warehouseMachine <- case parseMachineSurface warehouseMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  worldCommit <- case parseCommitSurface worldSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c

  let snap0 = applyCommits "world" [worldCommit]
      machines =
        Map.fromList
          [ (nodeRefText (machineNode quarryMachine), quarryMachine)
          , (nodeRefText (machineNode warehouseMachine), warehouseMachine)
          ]

  putStrLn "--- Scenario 1: full chain -- quarry's own gap surfaces warehouse's own gap too ---"
  case fixpointRetroconsistency machines "quarry/east" "extract" snap0 of
    ChainOk [step1, step2] -> do
      putStrLn ("PASS: chain resolved in 2 real steps")
      putStrLn ("  step 1: " <> show (stepMachine step1) <> "/" <> show (stepTransition step1) <> " implied " <> show (stepFacts step1))
      putStrLn ("  step 2: " <> show (stepMachine step2) <> "/" <> show (stepTransition step2) <> " implied " <> show (stepFacts step2))
      if stepMachine step1 == "quarry/east"
        && stepMachine step2 == "warehouse/central"
        && stepTransition step2 == "open"
        && [ImpliedFact "warehouse/central" "staffedBy" "retro/warehouse_central_staffedBy"] == stepFacts step2
        then putStrLn "PASS: step 2 is warehouse/central's OWN gap, discovered purely by chaining -- never asked about directly"
        else putStrLn "FAIL: chain steps present but wrong shape" >> exitFailure
    other -> putStrLn ("FAIL: expected a 2-step ChainOk, got " <> show other) >> exitFailure

  putStrLn "\n--- Scenario 2: without the warehouse machine declared, chaining correctly finds nothing more to chase ---"
  let quarryOnlyMachines = Map.fromList [(nodeRefText (machineNode quarryMachine), quarryMachine)]
  case fixpointRetroconsistency quarryOnlyMachines "quarry/east" "extract" snap0 of
    ChainOk [singleStep] ->
      if stepMachine singleStep == "quarry/east"
        then putStrLn "PASS: with no warehouse machine in scope, the chain correctly stops after quarry/east's own step"
        else putStrLn "FAIL: wrong single step" >> exitFailure
    other -> putStrLn ("FAIL: expected a 1-step ChainOk, got " <> show other) >> exitFailure

  putStrLn "\n--- Scenario 3: a freshly-minted node structurally cannot chain further (nothing governs a brand-new node) ---"
  -- forest/clearing's own harvestedBy target is a FRESH placeholder --
  -- no machine anywhere is named after it, so chaining correctly finds
  -- nothing, even with an otherwise-unrelated machine in scope.
  forestMachine <- case parseMachineSurface forestMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  forestWorld <- case parseCommitSurface forestWorldSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c
  let forestSnap = applyCommits "world" [forestWorld]
      forestMachines = Map.fromList [(nodeRefText (machineNode forestMachine), forestMachine)]
  case fixpointRetroconsistency forestMachines "forest/clearing" "deplete" forestSnap of
    ChainOk [onlyStep] ->
      if null [() | f <- stepFacts onlyStep, T.isPrefixOf "retro/" (impliedTarget f)]
        then putStrLn "FAIL: expected a fresh retro/ target" >> exitFailure
        else putStrLn "PASS: chain correctly stops at 1 step -- the fresh harvestedBy target matches no declared machine"
    other -> putStrLn ("FAIL: expected a 1-step ChainOk, got " <> show other) >> exitFailure

  putStrLn "\n--- Scenario 4: a real cycle (A needs B, B needs A) terminates instead of looping forever ---"
  cycleAMachine <- case parseMachineSurface cycleAMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  cycleBMachine <- case parseMachineSurface cycleBMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  cycleWorld <- case parseCommitSurface cycleWorldSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c
  let cycleSnap = applyCommits "world" [cycleWorld]
      cycleMachines =
        Map.fromList
          [ (nodeRefText (machineNode cycleAMachine), cycleAMachine)
          , (nodeRefText (machineNode cycleBMachine), cycleBMachine)
          ]
  case fixpointRetroconsistency cycleMachines "loop/a" "activate" cycleSnap of
    ChainOk steps -> putStrLn ("PASS: cyclic machine graph terminated cleanly, " <> show (length steps) <> " step(s), no infinite loop")
    other -> putStrLn ("FAIL: expected ChainOk (terminating), got " <> show other) >> exitFailure
  where
    forestMachineSrc =
      "machine forest/clearing\n\
      \  states\n\
      \    pristine\n\
      \    depleted\n\
      \\n\
      \  transition deplete(harvester)\n\
      \    pristine -> depleted\n\
      \    guard self `harvestedBy` who\n\
      \    assert depleted\n"
    forestWorldSrc =
      "commit mints\n\
      \  declare attribute state\n\
      \  declare relation harvestedBy\n\
      \  forest/clearing . state = depleted\n"
    cycleAMachineSrc =
      "machine loop/a\n\
      \  states\n\
      \    idle\n\
      \    active\n\
      \\n\
      \  transition activate(x)\n\
      \    idle -> active\n\
      \    guard self `linkedTo` loop/b\n\
      \    assert active\n"
    cycleBMachineSrc =
      "machine loop/b\n\
      \  states\n\
      \    idle\n\
      \    active\n\
      \\n\
      \  transition activate(x)\n\
      \    idle -> active\n\
      \    guard self `linkedTo` loop/a\n\
      \    assert active\n"
    cycleWorldSrc =
      "commit mints\n\
      \  declare attribute state\n\
      \  declare relation linkedTo\n\
      \  loop/a . state = active\n\
      \  loop/b . state = active\n"
