{-# LANGUAGE OverloadedStrings #-}

-- | Real, executable smoke test for DMML.Governance -- exercises all
-- three GovernedOutcome cases against real parsed Surface syntax, not
-- hand-built AST. Dogfooded, matching this session's own discipline.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Governance (GovernedOutcome (..), applyGovernance, arbitrate)
import DMML.Materialize (WorldSnapshot, applyCommits, currentValue, mergeSnapshots)
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

-- | Two independently-asserted alternatives for shrine/threshold.state,
-- no governance equipped at all -- the ungoverned case.
ungovernedSrc, ungovernedPeerSrc :: Text
ungovernedSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  shrine/threshold . state = stirring\n"
ungovernedPeerSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  shrine/threshold . state = sealed\n"

-- | Same two-alternative shape, but contest/x IS equipped as governor
-- of shrine/threshold's "state" predicate -- and unwitnessed. Neither
-- alternative should validate.
pendingSrc, pendingPeerSrc :: Text
pendingSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  declare relation equips\n\
  \  declare attribute trigger\n\
  \  shrine/threshold . state = stirring\n\
  \  shrine/threshold `equips` contest/x\n\
  \  contest/x . trigger = \"state\"\n\
  \  contest/x . state = contested\n"
pendingPeerSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  shrine/threshold . state = sealed\n"

-- | Same as pending, but with a real witnessedBy fact, AND "resolved"
-- is itself one of the live alternatives -- arbitrate only ever picks
-- among already-live alternatives, it never invents a new value.
resolvedSrc, resolvedPeerSrc :: Text
resolvedSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  declare relation equips\n\
  \  declare relation witnessedBy\n\
  \  declare attribute trigger\n\
  \  shrine/threshold . state = stirring\n\
  \  shrine/threshold `equips` contest/x\n\
  \  contest/x . trigger = \"state\"\n\
  \  contest/x . state = contested\n\
  \  contest/x `witnessedBy` npc/keeper\n"
resolvedPeerSrc =
  "commit raises\n\
  \  declare relation state\n\
  \  shrine/threshold . state = resolved\n"

-- | Phase A2: a governed predicate that ISN'T "state" -- npc/y's own
-- role, governed by contest/z, whose OWN internal state stays a
-- completely separate concept (contested -> trickster, unrelated in
-- name to "role"). Proves arbitrate's value-match never actually
-- depended on the disputed predicate being named "state" -- the
-- earlier restriction was an overcautious guard, not a structural
-- necessity.
nonStateMachineSrc :: Text
nonStateMachineSrc =
  "machine contest/z\n\
  \  states\n\
  \    contested\n\
  \    trickster\n\
  \\n\
  \  transition resolve(witness)\n\
  \    contested -> trickster\n\
  \    guard self `witnessedBy` npc/keeper\n\
  \    assert trickster\n"

nonStateSrc, nonStatePeerSrc :: Text
nonStateSrc =
  "commit raises\n\
  \  declare relation role\n\
  \  declare relation equips\n\
  \  declare relation witnessedBy\n\
  \  declare attribute trigger\n\
  \  npc/y . role = guide\n\
  \  npc/y `equips` contest/z\n\
  \  contest/z . trigger = \"role\"\n\
  \  contest/z . state = contested\n\
  \  contest/z `witnessedBy` npc/keeper\n"
nonStatePeerSrc =
  "commit raises\n\
  \  declare relation role\n\
  \  npc/y . role = trickster\n"

buildSnap :: Text -> Text -> IO WorldSnapshot
buildSnap mineSrc peerSrc = do
  mineC <- parseC mineSrc
  peerC <- parseC peerSrc
  pure (mergeSnapshots (applyCommits "mine" [mineC]) (applyCommits "peer" [peerC]))
  where
    parseC src = case parseCommitSurface src of
      Left e -> putStrLn (errorBundlePretty e) >> exitFailure
      Right c -> pure c

main :: IO ()
main = do
  machine <- case parseMachineSurface machineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  let machines = Map.fromList [("contest/x", machine)]
      key = ("shrine/threshold", "state")

  ungovernedSnap <- buildSnap ungovernedSrc ungovernedPeerSrc
  check "Ungoverned" Ungoverned (arbitrate machines key ungovernedSnap)

  pendingSnap <- buildSnap pendingSrc pendingPeerSrc
  check "StillPending (governed, unwitnessed)" StillPending (arbitrate machines key pendingSnap)

  resolvedSnap <- buildSnap resolvedSrc resolvedPeerSrc
  case arbitrate machines key resolvedSnap of
    Resolved label v ->
      putStrLn ("PASS: Resolved (governed, witnessed, one alternative matches effect) -- " <> show label <> " " <> show v)
    other -> putStrLn ("FAIL: expected Resolved, got " <> show other) >> exitFailure

  -- applyGovernance must actually collapse this pair to one value --
  -- and leave the ungoverned/pending snapshots untouched.
  let collapsed = applyGovernance machines resolvedSnap
  case currentValue key collapsed of
    [_single] -> putStrLn "PASS: applyGovernance collapsed the resolved pair to one live value"
    other -> putStrLn ("FAIL: applyGovernance left " <> show (length other) <> " alternatives, expected 1") >> exitFailure

  let untouchedUngoverned = applyGovernance machines ungovernedSnap
  if length (currentValue key untouchedUngoverned) == length (currentValue key ungovernedSnap)
    then putStrLn "PASS: applyGovernance left the ungoverned pair untouched"
    else putStrLn "FAIL: applyGovernance changed an ungoverned pair" >> exitFailure

  let untouchedPending = applyGovernance machines pendingSnap
  if length (currentValue key untouchedPending) == length (currentValue key pendingSnap)
    then putStrLn "PASS: applyGovernance left the still-pending pair untouched"
    else putStrLn "FAIL: applyGovernance changed a still-pending pair" >> exitFailure

  -- Phase A2: governance on a NON-"state" predicate (npc/y . role),
  -- governed by contest/z whose own internal state field is a
  -- completely separate concept. Proves arbitrate's value-match never
  -- depended on the disputed predicate being named "state".
  nonStateMachine <- case parseMachineSurface nonStateMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  let machines' = Map.insert "contest/z" nonStateMachine machines
      nonStateKey = ("npc/y", "role")
  nonStateSnap <- buildSnap nonStateSrc nonStatePeerSrc
  case arbitrate machines' nonStateKey nonStateSnap of
    Resolved label v ->
      putStrLn ("PASS: Resolved (non-state predicate, governed, witnessed) -- " <> show label <> " " <> show v)
    other -> putStrLn ("FAIL: expected Resolved for non-state predicate, got " <> show other) >> exitFailure

  let nonStateCollapsed = applyGovernance machines' nonStateSnap
  case currentValue nonStateKey nonStateCollapsed of
    [_single] -> putStrLn "PASS: applyGovernance collapsed the non-state pair to one live value"
    other -> putStrLn ("FAIL: applyGovernance left " <> show (length other) <> " alternatives for non-state pair, expected 1") >> exitFailure
  where
    check label expected actual
      | actual == expected = putStrLn ("PASS: " <> label)
      | otherwise = putStrLn ("FAIL: " <> label <> " -- expected " <> show expected <> ", got " <> show actual) >> exitFailure
