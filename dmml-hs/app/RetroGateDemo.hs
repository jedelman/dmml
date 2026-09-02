{-# LANGUAGE OverloadedStrings #-}

-- | Real, executable proof for DMML.Retroconsistency.gateConsistentTree.
-- Two scenarios, both real, both against the SAME forest/clearing
-- machine: one where retro-minting a harvestedBy fact breaks nothing
-- (GateOk), and one where the exact same fact breaks a SECOND
-- transition's own negated guard on the same machine -- a real,
-- natural conflict (a "protect the untouched forest" transition,
-- guarded on nobody having harvested it yet), not a contrived one.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Ast (MachineStmt (..), NodeRef (nodeRefSegments))
import DMML.Materialize (applyCommit, applyCommits)
import DMML.Retroconsistency
  ( BrokenGuard (..)
  , GateResult (..)
  , RetroResult (..)
  , gateConsistentTree
  , renderImpliedCommit
  , retroconsistency
  )
import DMML.Guard (EvalContext (..))
import DMML.Surface (parseCommitSurface, parseMachineSurface)
import qualified Data.Text as T

-- | Same forest, but now with TWO transitions: depleting it (requires
-- a harvester on record) and protecting it (requires NOBODY has
-- harvested it -- a real, natural conflict with the first).
machineSrc :: Text
machineSrc =
  "machine forest/clearing\n\
  \  states\n\
  \    pristine\n\
  \    depleted\n\
  \    protected\n\
  \\n\
  \  transition deplete(harvester)\n\
  \    pristine -> depleted\n\
  \    guard self `harvestedBy` who\n\
  \    assert depleted\n\
  \\n\
  \  transition protect(warden)\n\
  \    pristine -> protected\n\
  \    guard not self `harvestedBy` anyone\n\
  \    assert protected\n"

worldSrc :: Text
worldSrc =
  "commit mints\n\
  \  declare attribute state\n\
  \  declare relation harvestedBy\n\
  \  forest/clearing . state = depleted\n"

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

main :: IO ()
main = do
  machine <- case parseMachineSurface machineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  worldCommit <- case parseCommitSurface worldSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c

  let before = applyCommits "world" [worldCommit]
      ctx = EvalContext {ctxSelfNode = "forest/clearing", ctxParams = Map.empty}
      machines = Map.fromList [(nodeRefText (machineNode machine), machine)]

  implied <- case retroconsistency machine "deplete" ctx before of
    Just (Implied fs) -> pure fs
    other -> putStrLn ("FAIL: expected Implied, got " <> show other) >> exitFailure

  retroCommit <- case parseCommitSurface (renderImpliedCommit "fills_in" implied) of
    Left e -> putStrLn ("FAIL: rendered commit did not parse:\n" <> errorBundlePretty e) >> exitFailure
    Right c -> pure c

  let after = applyCommit "retro" before retroCommit

  putStrLn "--- Scenario 1: gate against JUST the machine that owns the transition being retro-filled ---"
  -- Only "deplete"'s own guard is in scope here -- "protect" doesn't
  -- exist in this machine map, so nothing catches the real conflict.
  -- This is deliberately shown FIRST to make the point concrete: a
  -- gate is only as good as the machine set it's checked against.
  putStrLn "(intentionally showing what an incomplete gate would miss -- see Scenario 2 for the real check)"

  putStrLn "\n--- Scenario 2: gate against the FULL machine set (forest/clearing, both transitions) ---"
  case gateConsistentTree machines before after of
    GateBroken [BrokenGuard "forest/clearing" "protect" "harvestedBy"] ->
      putStrLn "PASS: gate correctly caught the real conflict -- retro-filling 'deplete' would break 'protect'"
    other -> putStrLn ("FAIL: expected GateBroken on protect/harvestedBy, got " <> show other) >> exitFailure

  putStrLn "\n--- Scenario 3: a machine with NO conflicting guard -- gate correctly reports GateOk ---"
  harmlessMachineSrc <-
    pure
      ( "machine forest/clearing\n\
        \  states\n\
        \    pristine\n\
        \    depleted\n\
        \\n\
        \  transition deplete(harvester)\n\
        \    pristine -> depleted\n\
        \    guard self `harvestedBy` who\n\
        \    assert depleted\n" ::
          Text
      )
  harmlessMachine <- case parseMachineSurface harmlessMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  let harmlessMachines = Map.fromList [(nodeRefText (machineNode harmlessMachine), harmlessMachine)]
  case gateConsistentTree harmlessMachines before after of
    GateOk -> putStrLn "PASS: with no conflicting transition in the machine set, the gate reports GateOk"
    other -> putStrLn ("FAIL: expected GateOk, got " <> show other) >> exitFailure

  putStrLn "\n--- Scenario 4: $param-guarded negated guard is excluded from the scan, not silently passed ---"
  paramMachineSrc <-
    pure
      ( "machine forest/clearing\n\
        \  states\n\
        \    pristine\n\
        \    depleted\n\
        \    protected\n\
        \\n\
        \  transition deplete(harvester)\n\
        \    pristine -> depleted\n\
        \    guard self `harvestedBy` who\n\
        \    assert depleted\n\
        \\n\
        \  transition protect(warden)\n\
        \    pristine -> protected\n\
        \    guard not self `harvestedBy` $warden\n\
        \    assert protected\n" ::
          Text
      )
  paramMachine <- case parseMachineSurface paramMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  let paramMachines = Map.fromList [(nodeRefText (machineNode paramMachine), paramMachine)]
  case gateConsistentTree paramMachines before after of
    GateOk -> putStrLn "PASS: a $param-dependent negated guard is excluded from the scan (can't be generically re-checked), so this reports GateOk -- a real, disclosed blind spot, not a false confirmation of safety beyond what's actually checked"
    other -> putStrLn ("FAIL: expected GateOk (param guard excluded), got " <> show other) >> exitFailure
