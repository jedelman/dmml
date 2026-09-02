{-# LANGUAGE OverloadedStrings #-}

-- | Real, executable proof for DMML.Retroconsistency -- Jason's own
-- example: a forest is asserted depleted with no witness of who
-- depleted it. Exercises the whole loop, not just the pure function:
-- compute what's implied, render it as a real DMML commit, PARSE that
-- commit back (round-trip validity, not just "the Haskell values look
-- right"), apply it to the snapshot, and confirm the machine's guard
-- now actually evaluates true where it didn't before.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import System.Exit (exitFailure)
import Text.Megaparsec (errorBundlePretty)

import DMML.Guard (EvalContext (..), evalGuards, lookupTransition, mayFire)
import DMML.Ast (transitionGuards)
import DMML.Materialize (applyCommits, currentValue)
import DMML.Retroconsistency (ImpliedFact (..), RetroResult (..), renderImpliedCommit, retroconsistency)
import DMML.Surface (parseCommitSurface, parseMachineSurface)

-- | The forest's own governing machine: depleting it requires SOME
-- harvester to be on record, via an existential hop (a bare single-
-- segment identifier in a pattern position reads as a 'TermVar' --
-- Surface.hs's own pPatternTerm doc comment -- named "who", not
-- "harvester" again, specifically so it's not confused with the
-- transition's OWN "harvester" $param below, a different binding
-- mechanism entirely). Exactly Jason's "if a forest is depleted, there
-- must be someone who depleted it," expressed as an ordinary guard, no
-- new grammar.
machineSrc :: Text
machineSrc =
  "machine forest/clearing\n\
  \  states\n\
  \    pristine\n\
  \    depleted\n\
  \\n\
  \  transition deplete(harvester)\n\
  \    pristine -> depleted\n\
  \    guard self `harvestedBy` who\n\
  \    assert depleted\n"

-- | The forest is asserted depleted directly -- as if by a demiurge
-- generator seeding backstory, or an agent minting the END state of
-- something without having authored its history -- with NO
-- harvestedBy fact anywhere. This is deliberately how the gap arises:
-- retroconsistency exists for exactly this shape of authored content.
worldSrc :: Text
worldSrc =
  "commit mints\n\
  \  declare attribute state\n\
  \  declare relation harvestedBy\n\
  \  forest/clearing . state = depleted\n"

-- | Anchor is a bare, single-segment identifier ("someone") -- reads as
-- an unbound TermVar, per Surface.hs's own pPatternTerm rule. No
-- existing node can be principled-ly chosen to retroactively gain
-- this fact, so retroconsistency must refuse this, not guess.
unboundAnchorMachineSrc :: Text
unboundAnchorMachineSrc =
  "machine river/bend\n\
  \  states\n\
  \    calm\n\
  \    flooded\n\
  \\n\
  \  transition arrive(traveler)\n\
  \    calm -> flooded\n\
  \    guard someone `witnesses` self\n\
  \    assert flooded\n"

negatedGuardMachineSrc :: Text
negatedGuardMachineSrc =
  "machine shrine/altar\n\
  \  states\n\
  \    cursed\n\
  \    blessed\n\
  \\n\
  \  transition bless(priest)\n\
  \    cursed -> blessed\n\
  \    guard not self `hasCurse` mark/dark\n\
  \    assert blessed\n"

-- | The curse is a real, currently-asserted fact -- retroconsistency
-- can only ADD facts, so there is nothing it could mint to make this
-- negated guard hold; must refuse, not silently ignore the blocker.
cursedWorldSrc :: Text
cursedWorldSrc =
  "commit mints\n\
  \  declare relation hasCurse\n\
  \  shrine/altar `hasCurse` mark/dark\n"

-- | Two-hop pattern, neither hop satisfied: self -> guardedBy -> ?warden
-- (existential, gets a fresh node), then that fresh node -> swornTo ->
-- lord/ashgrove (bound, a real literal node reference). Proves the
-- chain-building claim: the second implied fact's subject is the FIRST
-- implied fact's own freshly-minted target, not independently guessed.
chainMachineSrc :: Text
chainMachineSrc =
  "machine outpost/north\n\
  \  states\n\
  \    open\n\
  \    guarded\n\
  \\n\
  \  transition waylay(bandit)\n\
  \    open -> guarded\n\
  \    guard self `guardedBy` warden `swornTo` lord/ashgrove\n\
  \    assert guarded\n"

main :: IO ()
main = do
  machine <- case parseMachineSurface machineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  worldCommit <- case parseCommitSurface worldSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c

  let snap0 = applyCommits "world" [worldCommit]
      ctx = EvalContext {ctxSelfNode = "forest/clearing", ctxParams = Map.empty}

  -- Sanity: the transition should NOT be firing-eligible yet -- the
  -- guard genuinely isn't satisfied, this isn't a vacuous test.
  case mayFire machine "deplete" ctx snap0 of
    Just (False, _, _) -> putStrLn "PASS: deplete correctly blocked before retroconsistency (no harvestedBy fact yet)"
    other -> putStrLn ("FAIL: expected the guard blocked pre-retro, got " <> show other) >> exitFailure

  result <- case retroconsistency machine "deplete" ctx snap0 of
    Nothing -> putStrLn "FAIL: no such transition declared" >> exitFailure
    Just r -> pure r

  implied <- case result of
    Implied fs -> pure fs
    other -> putStrLn ("FAIL: expected Implied, got " <> show other) >> exitFailure

  case implied of
    [ImpliedFact "forest/clearing" "harvestedBy" target] ->
      putStrLn ("PASS: implied exactly one fact -- forest/clearing `harvestedBy` " <> T.unpack target)
    other -> putStrLn ("FAIL: expected exactly one implied harvestedBy fact, got " <> show other) >> exitFailure

  let rendered = renderImpliedCommit "fills_in" implied
  putStrLn "--- rendered commit ---"
  putStrLn (show rendered)

  retroCommit <- case parseCommitSurface rendered of
    Left e -> putStrLn ("FAIL: rendered commit did not parse:\n" <> errorBundlePretty e) >> exitFailure
    Right c -> pure c
  putStrLn "PASS: rendered commit re-parses as valid DMML Surface syntax"

  let snap1 = applyCommits "world" [worldCommit, retroCommit]

  -- The AUTHOR-WRITTEN guard (harvestedBy) is what retroconsistency
  -- cares about, and it now holds -- checked directly via the
  -- transition's own transitionGuards, not mayFire's full resolved
  -- list (see DMML.Retroconsistency's own doc comment on why those are
  -- different questions).
  decl <- case lookupTransition machine "deplete" of
    Nothing -> putStrLn "FAIL: no such transition" >> exitFailure
    Just d -> pure d
  if evalGuards (transitionGuards decl) ctx snap1
    then putStrLn "PASS: the author-written harvestedBy guard now holds after applying the implied commit"
    else putStrLn "FAIL: expected the author-written guard to hold post-retro" >> exitFailure

  -- mayFire still (correctly) reports False here -- NOT a bug: its
  -- implicit (self, state, pristine) from-state check is a genuinely
  -- separate, present-tense question ("is the machine in the right
  -- state to fire RIGHT NOW") that this test's world content never
  -- asserted an answer to either way, and legitimately shouldn't need
  -- to for retroconsistency's own purposes.
  case mayFire machine "deplete" ctx snap1 of
    Just (False, _, _) -> putStrLn "PASS (expected): mayFire still blocked -- its implicit from-state check is a different question, not retroconsistency's job"
    other -> putStrLn ("FAIL: expected mayFire still blocked on the unrelated implicit from-state check, got " <> show other) >> exitFailure

  case currentValue ("forest/clearing", "harvestedBy") snap1 of
    [_single] -> putStrLn "PASS: forest/clearing.harvestedBy is now a real, single live fact"
    other -> putStrLn ("FAIL: expected exactly one live harvestedBy alternative, got " <> show other) >> exitFailure

  -- Idempotence / already-consistent path: running retroconsistency
  -- again against the NOW-consistent snapshot should report nothing
  -- left to imply.
  case retroconsistency machine "deplete" ctx snap1 of
    Just AlreadyConsistent -> putStrLn "PASS: re-running retroconsistency on the now-consistent snapshot reports AlreadyConsistent"
    other -> putStrLn ("FAIL: expected AlreadyConsistent on the second run, got " <> show other) >> exitFailure

  putStrLn "\n--- refusal cases: unbound anchor ---"
  unboundMachine <- case parseMachineSurface unboundAnchorMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  let riverCtx = EvalContext {ctxSelfNode = "river/bend", ctxParams = Map.empty}
  case retroconsistency unboundMachine "arrive" riverCtx snap0 of
    Just (Irreconcilable msg) -> putStrLn ("PASS: unbound-anchor guard correctly refused -- " <> T.unpack msg)
    other -> putStrLn ("FAIL: expected Irreconcilable for an unbound anchor, got " <> show other) >> exitFailure

  putStrLn "\n--- refusal cases: blocked negated guard ---"
  negMachine <- case parseMachineSurface negatedGuardMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  cursedCommit <- case parseCommitSurface cursedWorldSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right c -> pure c
  let cursedSnap = applyCommits "world" [worldCommit, cursedCommit]
      altarCtx = EvalContext {ctxSelfNode = "shrine/altar", ctxParams = Map.empty}
  case retroconsistency negMachine "bless" altarCtx cursedSnap of
    Just (Irreconcilable msg) -> putStrLn ("PASS: blocked negated guard correctly refused -- " <> T.unpack msg)
    other -> putStrLn ("FAIL: expected Irreconcilable for a blocked negated guard, got " <> show other) >> exitFailure

  putStrLn "\n--- multi-hop chain: both hops missing get implied, in order ---"
  chainMachine <- case parseMachineSurface chainMachineSrc of
    Left e -> putStrLn (errorBundlePretty e) >> exitFailure
    Right m -> pure m
  let outpostCtx = EvalContext {ctxSelfNode = "outpost/north", ctxParams = Map.empty}
  case retroconsistency chainMachine "waylay" outpostCtx snap0 of
    Just (Implied [f1, f2]) ->
      if impliedSubject f1 == "outpost/north"
        && impliedPredicate f1 == "guardedBy"
        && impliedSubject f2 == impliedTarget f1
        && impliedPredicate f2 == "swornTo"
        && impliedTarget f2 == "lord/ashgrove"
        then putStrLn "PASS: multi-hop chain implied both facts in order, second one building on the first's fresh node"
        else putStrLn ("FAIL: chain facts present but wrong shape -- " <> show [f1, f2]) >> exitFailure
    other -> putStrLn ("FAIL: expected a 2-fact Implied chain, got " <> show other) >> exitFailure
