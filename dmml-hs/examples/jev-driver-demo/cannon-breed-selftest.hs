{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED self-test for "DMML.Recombine" -- the recombinant
-- half of @app\/Cannon.hs@.
--
-- Unlike @spawn-fire-selftest.hs@, which had to run against hand-copied
-- interface stubs of "DMML.Surface" and "DMML.Retroconsistency" because
-- no megaparsec was reachable in that sandbox, this one runs against
-- the genuine, unmodified modules -- including the real
-- 'DMML.Surface.parseMachineSurface'. So the round-trip that selftest
-- explicitly listed as UNVERIFIED ("whether 'renderFiredMachine's
-- output actually re-parses") is checked here for real, on every
-- offspring this module can produce.
--
-- Build and run from @dmml-hs\/@:
--
-- > ghc -isrc -iexamples/jev-driver-demo -o \/tmp\/breed-selftest \\
-- >     examples\/jev-driver-demo\/cannon-breed-selftest.hs && \/tmp\/breed-selftest
--
-- Every check is a structural property that must hold of EVERY
-- offspring, not a golden-output comparison -- golden text would pin
-- the renderer, which 'DMML.Fire' already has its own tests for, and
-- would say nothing about whether a crossover produced a machine that
-- can actually run.
module Main (main) where

import Control.Monad (forM_, unless)
import Data.IORef (IORef, modifyIORef', newIORef, readIORef)
import Data.List (nub, sort)
import Data.Text (Text)
import qualified Data.Text as T
import System.Exit (exitFailure)

import DMML.Ast
import DMML.Fire (renderFiredMachine)
import DMML.Recombine
import DMML.Surface (parseMachineSurface)

-- Harness ------------------------------------------------------------------

check :: IORef [Text] -> Text -> Bool -> IO ()
check failures label ok
  | ok = putStrLn ("  ok   " <> T.unpack label)
  | otherwise = do
      putStrLn ("  FAIL " <> T.unpack label)
      modifyIORef' failures (label :)

sp :: Span
sp = Span "selftest"

nr :: Text -> NodeRef
nr = NodeRef . T.splitOn "/"

-- Fixtures -----------------------------------------------------------------

-- Parent A: a vault, verbatim in shape with what `cannon vault` fires --
-- two states, one guarded breach, a self-derived sigil literal.
vault :: MachineStmt
vault =
  MachineStmt
    { machineNode = nr "room/wVault"
    , machineStates = [StateDecl "sealed" sp, StateDecl "open" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "breach"
            , transitionParams = []
            , transitionFrom = Just "sealed"
            , transitionTo = Just "open"
            , transitionGuards =
                [ guardFact "room/wForge" "cleared" "mark/yes"
                , guardFact "vault/keyring" "has" "key/gold"
                ]
            , transitionEffects =
                [ assertSelf "cleared" "mark/yes"
                , assertSelf "yields" "sigil/wVault"
                , assertSelf "state" "open"
                , EffectRetract TermSelf [] (PredIdent "state") Nothing
                ]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

-- Parent B: the entry fork -- a DIFFERENT state vocabulary from A's, two
-- transitions sharing one lifecycle lock. Crossing these two is the case
-- naive state unioning gets wrong.
fork :: MachineStmt
fork =
  MachineStmt
    { machineNode = nr "fork/main"
    , machineStates = [StateDecl "unchosen" sp, StateDecl "chosen" sp]
    , machineTransitions = [go "goLeft" "path/west", go "goRight" "path/east"]
    , machineSpan = sp
    }
  where
    go name frontier =
      TransitionDecl
        { transitionIdent = name
        , transitionParams = []
        , transitionFrom = Just "unchosen"
        , transitionTo = Just "chosen"
        , transitionGuards = [guardFact "room/entry" "cleared" "mark/yes"]
        , transitionEffects =
            [ EffectAssert (TermNode frontier) (PredIdent "cleared") (EffectValueTerm (TermNode "mark/yes"))
            , assertSelf "state" "chosen"
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }

-- Parent D: a binder-driven machine -- the case that produced corpses.
-- Its guard BINDS @?rock@ and its effects SPEND that same rock, so the
-- guard is not a test the machine passes, it is a query supplying a
-- witness. Chimera takes A's guards and B's effects; before the repair,
-- crossing this in as B handed the offspring effects naming a @?rock@
-- nothing bound, and 'DMML.Fire' refuses at the first unresolvable
-- effect term whatever the world looks like. Also carries a formal param
-- used by nothing, the other half of the same corpse.
digger :: MachineStmt
digger =
  MachineStmt
    { machineNode = nr "crew/digger"
    , machineStates = [StateDecl "idle" sp, StateDecl "working" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "take"
            , transitionParams = ["unused"]
            , transitionFrom = Just "idle"
            , transitionTo = Just "working"
            , transitionGuards = [bindFact "rock" "in" "quarry/north"]
            , transitionEffects =
                [ EffectAssert TermSelf (PredIdent "took") (EffectValueTerm (TermBind "rock"))
                , EffectRetract (TermBind "rock") [] (PredIdent "in") (Just (EffectValueTerm (TermNode "quarry/north")))
                , assertSelf "state" "working"
                , EffectRetract TermSelf [] (PredIdent "state") Nothing
                ]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

-- Parent E: an offspring that already CARRIES the corpse -- effects
-- naming @?rock@ with no guard to bind it, plus a param used by nothing.
-- This is what generation 1 looked like in the measured run, and what
-- Union and Splice then copied forward unchanged for eleven generations.
-- Without a fixture in this shape the inheritance checks are vacuous:
-- crossing two SOUND parents can't test pruning.
orphaned :: MachineStmt
orphaned =
  MachineStmt
    { machineNode = nr "room/g1"
    , machineStates = [StateDecl "gathering" sp, StateDecl "spent" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "fall"
            , transitionParams = ["unit"]
            , transitionFrom = Just "gathering"
            , transitionTo = Just "spent"
            , transitionGuards = []
            , transitionEffects =
                [ EffectAssert TermSelf (PredIdent "took") (EffectValueTerm (TermBind "rock"))
                , EffectRetract (TermBind "rock") [] (PredIdent "in") (Just (EffectValueTerm (TermNode "quarry/north")))
                , assertSelf "state" "spent"
                , EffectRetract TermSelf [] (PredIdent "state") Nothing
                ]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

-- @guard ?v `pred` obj@ -- a QUERY, as against guardFact's test.
bindFact :: Text -> Text -> Text -> GuardClause
bindFact v predicate obj =
  GuardClause
    { guardNegated = False
    , guardExists =
        ExistsExpr
          { existsPattern = Pattern (TermBind v) [PatternHop predicate (TermNode obj)]
          , existsSpan = sp
          }
    , guardSpan = sp
    }

-- Parent C: a THREE-state machine whose state idents collide with A's,
-- to exercise the surplus-state path and its collision rename at once.
-- @sealed@ and @open@ align positionally onto A's; the third state is
-- itself called @open@, which must NOT silently merge with A's @open@.
longLived :: MachineStmt
longLived =
  MachineStmt
    { machineNode = nr "room/kiln"
    , machineStates = [StateDecl "cold" sp, StateDecl "lit" sp, StateDecl "open" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "light"
            , transitionParams = []
            , transitionFrom = Just "cold"
            , transitionTo = Just "lit"
            , transitionGuards = [guardFact "room/entry" "cleared" "mark/yes"]
            , transitionEffects = [assertSelf "state" "lit"]
            , transitionSpan = sp
            }
        , TransitionDecl
            { transitionIdent = "burnThrough"
            , transitionParams = []
            , transitionFrom = Just "lit"
            , transitionTo = Just "open"
            , transitionGuards = []
            , transitionEffects = [assertSelf "yields" "ash/kiln", assertSelf "state" "open"]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

guardFact :: Text -> Text -> Text -> GuardClause
guardFact anchor predicate obj =
  GuardClause
    { guardNegated = False
    , guardExists =
        ExistsExpr
          { existsPattern = Pattern (TermNode anchor) [PatternHop predicate (TermNode obj)]
          , existsSpan = sp
          }
    , guardSpan = sp
    }

assertSelf :: Text -> Text -> Effect
assertSelf predicate obj =
  EffectAssert TermSelf (PredIdent predicate) (EffectValueTerm (TermNode obj))

-- Properties ---------------------------------------------------------------

-- | Every state a transition travels between must be declared.
statesDeclared :: MachineStmt -> Bool
statesDeclared m =
  and
    [ s `elem` declared
    | t <- machineTransitions m
    , Just s <- [transitionFrom t, transitionTo t]
    ]
  where
    declared = map stateIdent (machineStates m)

-- | The regression this file exists for. A transition names its
-- destination TWICE -- on its @from -> to@ line and in the lifecycle
-- effect @assert self \`state\` X@ that records the arrival as a fact.
-- The first cut of 'breed' remapped only the first, so a bred offspring
-- travelled to @open@ while asserting @state chosen@: a state it did
-- not declare, from which nothing could ever transition out. Parses
-- fine, renders fine, dead on first fire.
lifecycleAgrees :: MachineStmt -> Bool
lifecycleAgrees m =
  and
    [ Just s == transitionTo t
    | t <- machineTransitions m
    , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode s)) <-
        transitionEffects t
    ]

identsUnique :: MachineStmt -> Bool
identsUnique m = length idents == length (nub idents)
  where
    idents = map transitionIdent (machineTransitions m)

-- | Every node literal anywhere in a machine, as written.
nodeLiterals :: MachineStmt -> [Text]
nodeLiterals m = concatMap ofTransition (machineTransitions m)
  where
    ofTransition t =
      concatMap ofGuard (transitionGuards t) ++ concatMap ofEffect (transitionEffects t)
    ofGuard g =
      let p = existsPattern (guardExists g)
       in ofTerm (patternAnchor p) ++ concatMap (ofTerm . hopTerm) (patternHops p)
    ofTerm (TermNode n) = [n]
    ofTerm _ = []
    ofValue (EffectValueTerm t) = ofTerm t
    ofValue _ = []
    ofEffect e
      | isLifecycle e = [] -- a state ident, not a node
    ofEffect (EffectAssert subj _ v) = ofTerm subj ++ ofValue v
    ofEffect (EffectRetract subj hops _ mv) =
      ofTerm subj ++ concatMap (ofTerm . hopTerm) hops ++ maybe [] ofValue mv
    ofEffect (EffectSpawn n ref) = ofTerm n ++ [T.intercalate "/" (nodeRefSegments ref)]
    ofEffect (EffectGraft target source) = ofTerm target ++ ofTerm source
    isLifecycle (EffectAssert TermSelf (PredIdent "state") _) = True
    isLifecycle (EffectRetract TermSelf _ (PredIdent "state") _) = True
    isLifecycle _ = False

-- | No parent's own last segment may survive in an offspring's node
-- literals -- an inherited @sigil\/wVault@ would have the child yielding
-- its parent's prize.
noParentIdentity :: [Text] -> MachineStmt -> Bool
noParentIdentity parentSegs m =
  not (or [piece `elem` parentSegs | lit <- nodeLiterals m, piece <- pieces lit])
  where
    pieces = concatMap (T.splitOn ".") . T.splitOn "/"

-- | Render, re-parse, compare. Spans differ (the parser records real
-- pointers where 'breed' stamped one span), so compare span-erased.
roundTrips :: MachineStmt -> Bool
roundTrips m = case parseMachineSurface (renderFiredMachine m) of
  Left _ -> False
  Right back -> erase back == erase m
  where
    z = Span ""
    erase x =
      x
        { machineSpan = z
        , machineStates = [s {stateSpan = z} | s <- machineStates x]
        , machineTransitions = map eraseT (machineTransitions x)
        }
    eraseT t =
      t
        { transitionSpan = z
        , transitionGuards =
            [ g {guardSpan = z, guardExists = (guardExists g) {existsSpan = z}}
            | g <- transitionGuards t
            ]
        }

worldEffects :: TransitionDecl -> [Effect]
worldEffects = filter (not . lc) . transitionEffects
  where
    lc (EffectAssert TermSelf (PredIdent "state") _) = True
    lc (EffectRetract TermSelf _ (PredIdent "state") _) = True
    lc _ = False

main :: IO ()
main = do
  failures <- newIORef []
  let ok = check failures

  -- Every offspring this module can produce from every pairing of the
  -- three fixtures, in both orientations. The universal properties are
  -- asserted over all of them, not over a chosen example.
  let pairs = [(x, y) | x <- [vault, fork, longLived], y <- [vault, fork, longLived]]
      pool = concat [map snd (breedPool sp (nr "room/bred") x y) | (x, y) <- pairs]
      parentSegs = ["wVault", "main", "kiln"]

  putStrLn ("universal properties over " <> show (length pool) <> " bred offspring")
  ok "premise: both parents already agree with themselves on lifecycle" $
    all lifecycleAgrees [vault, fork, longLived]
  ok "every offspring declares every state it travels between" $ all statesDeclared pool
  ok "every offspring's `assert self state X` agrees with its own to-state" $
    all lifecycleAgrees pool
  ok "every offspring has unique transition idents" $ all identsUnique pool
  ok "no offspring carries a parent's own identity in a node literal" $
    all (noParentIdentity parentSegs) pool
  ok "every offspring round-trips through the real parseMachineSurface" $ all roundTrips pool
  ok "no offspring is empty of transitions" $ all (not . null . machineTransitions) pool

  putStrLn "state alignment"
  case breed sp Union (nr "room/x") vault fork of
    Left e -> ok ("union vault x fork breeds: " <> T.pack (show e)) False
    Right m -> do
      ok "offspring's state space is parent A's, not a union of both" $
        map stateIdent (machineStates m) == ["sealed", "open"]
      ok "B's transitions are rebased onto A's lifecycle" $
        all (\t -> transitionFrom t == Just "sealed" && transitionTo t == Just "open")
          (machineTransitions m)
      ok "all three transitions survive the cross" $ length (machineTransitions m) == 3

  case breed sp Union (nr "room/x") vault longLived of
    Left e -> ok ("union vault x kiln breeds: " <> T.pack (show e)) False
    Right m -> do
      -- C's `cold`/`lit` align onto A's `sealed`/`open`; C's third state
      -- is itself named `open`, which already exists, so it must be
      -- renamed rather than merged.
      ok "a longer-lived parent EXTENDS the lifecycle instead of forking it" $
        map stateIdent (machineStates m) == ["sealed", "open", "open_2"]
      ok "the surplus state is reachable from the aligned one" $
        [(transitionFrom t, transitionTo t) | t <- machineTransitions m, transitionIdent t == "burnThrough"]
          == [(Just "open", Just "open_2")]

  putStrLn "collision renaming"
  case breed sp Union (nr "room/x") vault vault of
    Left e -> ok ("self-cross breeds: " <> T.pack (show e)) False
    Right m ->
      ok "crossing a machine with itself keeps BOTH transitions, renamed" $
        map transitionIdent (machineTransitions m) == ["breach", "breach_2"]

  putStrLn "chimera: the crossover no graft can do"
  case breed sp Chimera (nr "room/x") vault fork of
    Left e -> ok ("chimera breeds: " <> T.pack (show e)) False
    Right m -> case machineTransitions m of
      (t : _) -> do
        ok "the chimeric transition keeps A's identity and lifecycle" $
          transitionIdent t == "breach" && transitionTo t == Just "open"
        ok "it keeps A's guards -- what the vault REQUIRED" $
          transitionGuards t == transitionGuards (head (machineTransitions vault))
        ok "and takes B's world effects -- what the fork YIELDED" $
          worldEffects t == worldEffects (head (machineTransitions fork))
        ok "so it is a shape neither parent had" $
          worldEffects t /= worldEffects (head (machineTransitions vault))
      [] -> ok "chimera produced a transition" False

  putStrLn "orientation is not symmetric"
  case (breed sp Chimera (nr "room/x") vault fork, breed sp Chimera (nr "room/x") fork vault) of
    (Right l, Right r) -> ok "swapping the parents breeds a different offspring" $ l /= r
    _ -> ok "both orientations breed" False

  putStrLn "splice"
  case breed sp (Splice 1) (nr "room/x") vault fork of
    Left e -> ok ("splice breeds: " <> T.pack (show e)) False
    Right m ->
      ok "Splice 1 takes A's first transition and B's tail" $
        map transitionIdent (machineTransitions m) == ["breach", "goRight"]
  ok "an out-of-range splice point is refused, not silently clamped" $
    breed sp (Splice 9) (nr "room/x") vault fork == Left (SpliceOutOfRange 9 1)

  putStrLn "reanchor"
  case reanchor "room/entry" <$> breed sp Union (nr "room/x") vault fork of
    Left e -> ok ("reanchor breeds: " <> T.pack (show e)) False
    Right m -> do
      let anchors =
            [ n
            | t <- machineTransitions m
            , g <- transitionGuards t
            , let p = existsPattern (guardExists g)
            , [PatternHop "cleared" _] <- [patternHops p]
            , TermNode n <- [patternAnchor p]
            ]
      ok "every lineage guard is re-rooted onto the named anchor" $
        nub anchors == ["room/entry"]
      ok "the non-lineage guard is untouched" $
        or
          [ patternHops (existsPattern (guardExists g)) == [PatternHop "has" (TermNode "key/gold")]
          | t <- machineTransitions m
          , g <- transitionGuards t
          ]
      ok "re-anchoring still round-trips" $ roundTrips m

  putStrLn "orphan binders: an effect cannot outlive the guard that bound it"
  case breed sp Chimera (nr "room/x") vault digger of
    Left e -> ok ("chimera vault x digger breeds: " <> T.pack (show e)) False
    Right m -> case machineTransitions m of
      (t : _) -> do
        ok "no offspring transition names a binder nothing supplies" $
          all (null . orphanBinders) (machineTransitions m)
        -- The repair is to CARRY the query, not to drop the effect --
        -- dropping would reduce chimera to lifecycle whenever B is
        -- binder-driven, deleting the mode's whole point.
        ok "the guard that BINDS ?rock travels with the effects that spend it" $
          bindFact "rock" "in" "quarry/north" `elem` transitionGuards t
        ok "and A's own conditions are all still there" $
          all (`elem` transitionGuards t) (transitionGuards (head (machineTransitions vault)))
        ok "so the offspring really does take a rock -- B's consequence survived" $
          EffectAssert TermSelf (PredIdent "took") (EffectValueTerm (TermBind "rock"))
            `elem` transitionEffects t
        ok "the repaired offspring still round-trips" $ roundTrips m
      [] -> ok "chimera produced a transition" False

  -- Chimera keeps A's params, so the param sweep is only testable with
  -- the parameterized machine on the A side.
  ok "a formal param nothing references is dropped" $
    case breed sp Chimera (nr "room/x") digger vault of
      Right m -> all (null . transitionParams) (machineTransitions m)
      Left _ -> False
  ok "the fixture really does carry a param to drop" $
    transitionParams (head (machineTransitions digger)) == ["unused"]

  -- The measured failure was not the injection, it was the INHERITANCE:
  -- parent A of each generation is the last offspring, so one orphan bred
  -- in generation 0 rode every later cross forever. 9 of 12 machines in a
  -- real 20-round run were dead this way.
  putStrLn "orphans are not inherited down a lineage"
  let lineage 0 m = Right m
      lineage n m = breed sp Chimera (nr "room/x") m digger >>= lineage (n - 1 :: Int)
  case lineage 4 orphaned of
    Left e -> ok ("four generations breed: " <> T.pack (show e)) False
    Right m -> do
      ok "after four generations, still no orphan anywhere" $
        all (null . orphanBinders) (machineTransitions m)
      ok "and still no param standing for nothing" $
        all (null . transitionParams) (machineTransitions m)

  -- Union and Splice never INJECT an orphan; they propagate one. Both
  -- must therefore also prune, or an orphaned parent poisons them.
  putStrLn "union and splice prune what they inherit"
  ok "the fixture really is orphaned to begin with" $
    orphanBinders (head (machineTransitions orphaned)) == ["rock"]
  forM_ [(Union, "union" :: Text), (Splice 1, "splice1")] $ \(mode, label) ->
    case breed sp mode (nr "room/x") orphaned vault of
      Left e -> ok (label <> " breeds: " <> T.pack (show e)) False
      Right m -> do
        ok (label <> " carries no orphan through") $
          all (null . orphanBinders) (machineTransitions m)
        ok (label <> " drops the param that went with it") $
          all (null . transitionParams) (machineTransitions m)

  putStrLn "pool"
  let candidates = breedPool sp (nr "room/bred") vault fork
  ok "a single pair fires a frontier, not one shot" $ length candidates >= 4
  ok "every candidate has its own node" $
    let ns = [machineNode m | (_, m) <- candidates] in length (nub ns) == length ns
  ok "candidates are structurally distinct (dedup ran)" $
    let shapes = [m {machineNode = nr "room/bred"} | (_, m) <- candidates]
     in length (nub shapes) == length shapes

  putStrLn ""
  bad <- readIORef failures
  if null bad
    then putStrLn "cannon-breed-selftest: all checks passed"
    else do
      putStrLn ("cannon-breed-selftest: " <> show (length bad) <> " FAILED:")
      forM_ (sort bad) (putStrLn . ("  - " <>) . T.unpack)
      exitFailure
  unless (null bad) exitFailure
