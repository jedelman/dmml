{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED self-test for @?binder@ guard variables
-- ('DMML.Ast.TermBind', 'DMML.Guard.evalGuardsBinding') -- the change
-- that stopped throwing away the witness a guard already found.
--
-- The scenario is a QUARRY, deliberately, because that is the case the
-- feature exists for. Before this, a guard could ask "is there a rock?"
-- but could not tell an effect WHICH rock, so nothing could actually be
-- spent. With it, a machine can take one, and the quarry can run out --
-- and depletion is not a flag anyone maintains, it is the guard simply
-- failing to find a witness any more. That is scarcity expressed as an
-- external relation rather than as internal state, which is the whole
-- reason it is worth having ("gate on external relations, not internal
-- state", 2026-09-18).
--
-- Runs against the genuine modules -- no stubs, real
-- 'DMML.Surface.parseMachineSurface', real
-- 'DMML.Materialize.applyIdentifiedCommits', real 'DMML.Fire.fireTransition'.
--
-- Build and run from @dmml-hs\/@:
--
-- > ghc -isrc -iexamples/jev-driver-demo -o \/tmp\/gb \\
-- >     examples\/jev-driver-demo\/guard-binding-selftest.hs && \/tmp\/gb
module Main (main) where

import Control.Monad (forM_)
import Data.IORef (IORef, modifyIORef', newIORef, readIORef)
import Data.List (sort)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import System.Exit (exitFailure)

import DMML.Ast
import DMML.Fire
import DMML.Guard
import DMML.Materialize (IdentifiedCommit (..), WorldSnapshot, applyIdentifiedCommits)
import DMML.Surface (parseMachineSurface)

sp :: Span
sp = Span "guard-binding-selftest"

nodeRef :: Text -> NodeRef
nodeRef = NodeRef . T.splitOn "/"

check :: IORef [Text] -> Text -> Bool -> IO ()
check failures label ok
  | ok = putStrLn ("  ok   " <> T.unpack label)
  | otherwise = putStrLn ("  FAIL " <> T.unpack label) >> modifyIORef' failures (label :)

-- World ---------------------------------------------------------------------

-- A retract needs real 'StrongRef' provenance (see
-- 'DMML.Fire.FireRetractNoProvenance'); minted deterministically per
-- fact, the same way fire-from-facts-selftest does, since there is no
-- file here to hash.
factCommit :: Int -> FactStmt -> IdentifiedCommit
factCommit n f =
  IdentifiedCommit
    { icRef = StrongRef ("test://fact/" <> T.pack (show n)) ("c" <> T.pack (show n)) sp
    , icCommit = CommitStmt "quarry" [ItemFact f] mempty sp
    }

fact :: Text -> Text -> Text -> FactStmt
fact s p o = FactStmt (nodeRef s) (PredIdent p) (ValueNode (nodeRef o)) sp

world :: [FactStmt] -> WorldSnapshot
world fs = applyIdentifiedCommits "world" (zipWith factCommit [0 ..] fs)

-- Every rock is its OWN SUBJECT (@rock\/1 `in` quarry\/north@) rather
-- than an alternative under one key (@quarry\/north `holds` rock\/1@).
-- Both express "the quarry has these rocks"; only the first survives
-- contact with the grammar, because a commit cannot assert the same
-- (subject, predicate) pair twice, so fifty rocks the second way would
-- be fifty separate commits. It also makes taking one unambiguous at
-- the retract, which the alternative shape is not.
quarryOf :: [Text] -> [FactStmt]
quarryOf rocks = [fact r "in" "quarry/north" | r <- rocks]

machineOf :: Text -> MachineStmt
machineOf src = case parseMachineSurface src of
  Left _ -> error "selftest fixture does not parse"
  Right m -> m

-- Machines ------------------------------------------------------------------

-- Take a rock: the guard finds one, the effect spends THAT one.
digger :: MachineStmt
digger =
  machineOf
    "machine crew/digger\n\
    \  states\n\
    \    idle\n\
    \    working\n\
    \\n\
    \  transition take()\n\
    \    idle -> working\n\
    \    guard ?rock `in` quarry/north\n\
    \    assert self `took` ?rock\n\
    \    retract ?rock `in` quarry/north\n\
    \    assert self `state` working\n\
    \    retract self `state`\n"

-- Two guards, threaded: the first binds a rock, the second filters that
-- SAME rock. Several rocks in the quarry, one of them rich.
picky :: MachineStmt
picky =
  machineOf
    "machine crew/picky\n\
    \  states\n\
    \    idle\n\
    \    working\n\
    \\n\
    \  transition take()\n\
    \    idle -> working\n\
    \    guard ?rock `in` quarry/north\n\
    \    guard ?rock `grade` ore/rich\n\
    \    assert self `took` ?rock\n\
    \    assert self `state` working\n\
    \    retract self `state`\n"

ctxFor :: Text -> EvalContext
ctxFor n = EvalContext {ctxSelfNode = n, ctxParams = Map.empty, ctxBindings = Map.empty}

guardsOf :: MachineStmt -> [GuardClause]
guardsOf m = case machineTransitions m of
  (t : _) -> transitionGuards t
  [] -> []

took :: [ResolvedEffect] -> [Text]
took effects =
  sort
    [ T.intercalate "/" (nodeRefSegments n)
    | ResolvedAssert f <- effects
    , rfPredicate f == PredIdent "took"
    , ValueNode n <- [rfValue f]
    ]

retracted :: [ResolvedEffect] -> [Text]
retracted effects = sort [subj | ResolvedRetract subj (PredIdent "in") _ _ <- effects]

main :: IO ()
main = do
  failures <- newIORef []
  let ok = check failures

  putStrLn "surface"
  ok "?binder parses, and renders back as itself" $
    renderFiredMachine digger == renderFiredMachine (machineOf (renderFiredMachine digger))
  ok "the machine really contains a TermBind, not a bareword TermVar" $
    binderNames (existsPattern (guardExists (head (guardsOf digger)))) == ["rock"]

  putStrLn "one witness: the guard binds it and the effect spends it"
  let oneRock = world (quarryOf ["rock/1"] ++ [fact "crew/digger" "state" "idle"])
  case fireTransition (Map.fromList [("crew/digger", digger)]) digger "take" (ctxFor "crew/digger") oneRock of
    Left e -> ok ("digger fires with one rock: " <> T.pack (show e)) False
    Right effects -> do
      ok "the effect asserts the rock the guard actually found" $ took effects == ["rock/1"]
      ok "and retracts that same rock from the quarry" $ retracted effects == ["rock/1"]

  putStrLn "depletion is guard failure, not a flag"
  let emptyQuarry = world [fact "crew/digger" "state" "idle"]
  ok "an empty quarry blocks the transition -- nothing tracks 'empty'" $
    case fireTransition (Map.fromList [("crew/digger", digger)]) digger "take" (ctxFor "crew/digger") emptyQuarry of
      Left FireBlocked -> True
      _ -> False

  putStrLn "several witnesses: refuse, and name them"
  let manyRocks = world (quarryOf ["rock/1", "rock/2", "rock/3"] ++ [fact "crew/digger" "state" "idle"])
  ok "three rocks refuse rather than silently picking one" $
    case fireTransition (Map.fromList [("crew/digger", digger)]) digger "take" (ctxFor "crew/digger") manyRocks of
      Left (FireGuardError (GuardAmbiguousBinding v cands)) ->
        v == "rock" && sort cands == ["rock/1", "rock/2", "rock/3"]
      _ -> False
  ok "the refusal carries every candidate -- a question, not a dead end" $
    case evalGuardsBinding (guardsOf digger) (ctxFor "crew/digger") manyRocks of
      Left (GuardAmbiguousBinding _ cands) -> length cands == 3
      _ -> False

  putStrLn "bindings thread from one guard to the next"
  let graded =
        world
          ( quarryOf ["rock/1", "rock/2", "rock/3"]
              ++ [ fact "rock/2" "grade" "ore/rich"
                 , fact "crew/picky" "state" "idle"
                 ]
          )
  case fireTransition (Map.fromList [("crew/picky", picky)]) picky "take" (ctxFor "crew/picky") graded of
    Left e -> ok ("picky fires: " <> T.pack (show e)) False
    Right effects ->
      ok "a second guard on the SAME ?rock narrows three candidates to one" $
        took effects == ["rock/2"]

  putStrLn "a repeated ?name must agree with itself"
  let loopy =
        machineOf
          "machine crew/loop\n\
          \  states\n\
          \    idle\n\
          \    working\n\
          \\n\
          \  transition go()\n\
          \    idle -> working\n\
          \    guard ?x `links` ?x\n\
          \    assert self `state` working\n\
          \    retract self `state`\n"
      selfLoop = world [fact "node/a" "links" "node/a", fact "crew/loop" "state" "idle"]
      noLoop = world [fact "node/a" "links" "node/b", fact "crew/loop" "state" "idle"]
  ok "a self-loop satisfies `?x `links` ?x`" $
    evalGuardsBinding (guardsOf loopy) (ctxFor "crew/loop") selfLoop == Right (True, Map.fromList [("x", "node/a")])
  ok "a -> b does NOT, because the two ?x must be the same node" $
    evalGuardsBinding (guardsOf loopy) (ctxFor "crew/loop") noLoop == Right (False, Map.empty)

  putStrLn "a binder in a negated guard is refused, not quietly ignored"
  let negated =
        machineOf
          "machine crew/neg\n\
          \  states\n\
          \    idle\n\
          \    working\n\
          \\n\
          \  transition go()\n\
          \    idle -> working\n\
          \    guard not ?r `in` quarry/north\n\
          \    assert self `state` working\n\
          \    retract self `state`\n"
  ok "nothing can be bound from the absence of a fact" $
    case evalGuardsBinding (guardsOf negated) (ctxFor "crew/neg") oneRock of
      Left (GuardBinderInNegatedGuard "r") -> True
      _ -> False

  putStrLn "the old bareword is untouched"
  let barish =
        machineOf
          "machine crew/bare\n\
          \  states\n\
          \    idle\n\
          \    working\n\
          \\n\
          \  transition go()\n\
          \    idle -> working\n\
          \    guard quarry/north `sort` granite\n\
          \    assert self `state` working\n\
          \    retract self `state`\n"
      anySort = world [fact "quarry/north" "sort" "stone/basalt", fact "crew/bare" "state" "idle"]
  ok "a slash-free bareword is still a TermVar, binding nothing" $
    null (binderNames (existsPattern (guardExists (head (guardsOf barish)))))
  ok "and still matches anything, exactly as before this change" $
    evalGuardsBinding (guardsOf barish) (ctxFor "crew/bare") anySort == Right (True, Map.empty)
  ok "a machine with no binders evaluates identically to the old path" $
    evalGuards (guardsOf barish) (ctxFor "crew/bare") anySort
      == either (const False) fst (evalGuardsBinding (guardsOf barish) (ctxFor "crew/bare") anySort)

  putStrLn ""
  bad <- readIORef failures
  if null bad
    then putStrLn "guard-binding-selftest: all checks passed"
    else do
      putStrLn ("guard-binding-selftest: " <> show (length bad) <> " FAILED:")
      forM_ (sort bad) (putStrLn . ("  - " <>) . T.unpack)
      exitFailure
