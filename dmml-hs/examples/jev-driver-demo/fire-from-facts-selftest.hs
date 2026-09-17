{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED phase-2 integration test: fires a real transition
-- on a machine that exists ENTIRELY as live facts in a real
-- 'DMML.Materialize.WorldSnapshot' (built from
-- 'DMML.MachineFacts.encodeMachine's output via
-- 'DMML.Materialize.applyIdentifiedCommits', one commit per fact, with
-- real -- if synthetic -- 'DMML.Ast.StrongRef' provenance so the
-- transition's retract effect has something real to cite) via the new
-- 'DMML.Fire.fireTransitionFromFacts'. Checks the result against
-- firing the identical, hand-authored 'MachineStmt' directly through
-- the ordinary 'DMML.Fire.fireTransition' -- the actual closure claim:
-- a fact-sourced machine fires indistinguishably from a hand-authored
-- one.
--
-- Compiled and run for real (needs the same interface-only
-- Surface/Retroconsistency stubs 'spawn-fire-selftest.hs' already
-- uses and explains -- see that file's own doc comment for exactly
-- what is and isn't real about this sandbox's build).
--
-- Real bug this test caught before it shipped, worth recording: the
-- first version forgot to assert the machine's own initial `state`
-- fact (`smithy/furnace \`state\` idle`) alongside the world fact its
-- guard needs -- exactly the landmine `.claude/skills/dmml-authoring`
-- already documents ("A freshly minted machine needs its own initial
-- state fact"). Confirmed it wasn't a decode bug by firing the
-- ORIGINAL, non-decoded machine against the same incomplete snapshot
-- first: it refused identically, proving the bug was in the test's
-- snapshot setup, not in decodeMachineFromSnapshot.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import System.Exit (exitFailure)

import DMML.Ast
import DMML.Fire
import DMML.Guard (EvalContext (..))
import DMML.MachineFacts (encodeMachine)
import DMML.Materialize (IdentifiedCommit (..), applyIdentifiedCommits)

sp :: Span
sp = Span "/test"

nodeRef :: Text -> NodeRef
nodeRef = NodeRef . T.splitOn "/"

-- | Same shape as cascade-demo's furnace, authored once here and
-- reconstructed entirely from facts before firing.
furnace :: MachineStmt
furnace =
  MachineStmt
    { machineNode = nodeRef "smithy/furnace"
    , machineStates = [StateDecl "idle" sp, StateDecl "smelted" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "smelt"
            , transitionParams = ["ore"]
            , transitionFrom = Just "idle"
            , transitionTo = Just "smelted"
            , transitionGuards =
                [GuardClause False (ExistsExpr (Pattern TermSelf [PatternHop "stocked" (TermParam "ore")]) sp) sp]
            , transitionEffects =
                [ EffectAssert (TermParam "ore") (PredIdent "refinedInto") (EffectValueTerm (TermNode "ingot/batch1"))
                , EffectRetract TermSelf [] (PredIdent "state") Nothing
                , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "smelted"))
                ]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

-- | A real retract needs a real 'StrongRef' to cite
-- ('DMML.Fire.FireRetractNoProvenance'). @app/FireTransition.hs@
-- sources one from a real file's content hash
-- ('DMML.LocalIdentity.localFileRef'); this test has no file, so it
-- mints one deterministically per fact instead -- satisfies the
-- identical contract ('DMML.Ast.StrongRef's own doc comment: "never
-- parsed or validated beyond non-emptiness"), nothing about
-- 'DMML.Fire' cares that it isn't a real hash.
factToIdentifiedCommit :: Int -> FactStmt -> IdentifiedCommit
factToIdentifiedCommit n f =
  IdentifiedCommit
    { icRef = StrongRef {strongRefUri = "test://fact/" <> T.pack (show n), strongRefCid = "c" <> T.pack (show n), strongRefSpan = sp}
    , icCommit = CommitStmt {commitVerb = "encode", commitItems = [ItemFact f], commitRefs = mempty, commitSpan = sp}
    }

worldFact :: NodeRef -> Text -> Value -> FactStmt
worldFact subj p v = FactStmt {factSubject = subj, factPredicate = PredIdent p, factValue = v, factSpan = sp}

main :: IO ()
main = do
  let allFacts =
        encodeMachine furnace
          ++ [ worldFact (nodeRef "smithy/furnace") "stocked" (ValueNode (nodeRef "ore/raw1"))
             , worldFact (nodeRef "smithy/furnace") "state" (ValueNode (nodeRef "idle"))
             ]
      commits = zipWith factToIdentifiedCommit [0 ..] allFacts
      snap = applyIdentifiedCommits "world" commits
      machines = Map.fromList [("smithy/furnace", furnace)]
      ctx = EvalContext {ctxSelfNode = "smithy/furnace", ctxParams = Map.fromList [("ore", "ore/raw1")]}

  -- The real closure claim: firing the fact-sourced machine produces
  -- the SAME commit as firing the hand-authored one, against the same
  -- snapshot.
  let viaOriginal = fireTransition machines furnace "smelt" ctx snap
      viaFacts = fireTransitionFromFacts machines (nodeRef "smithy/furnace") "smelt" ctx snap

  case (viaOriginal, viaFacts) of
    (Left err, _) -> putStrLn ("FAIL: firing the hand-authored machine itself refused: " ++ show err) >> exitFailure
    (_, Left err) -> putStrLn ("FAIL: fireTransitionFromFacts refused: " ++ show err) >> exitFailure
    (Right originalEffects, Right factsEffects)
      | renderFiredCommit "smelts" originalEffects == renderFiredCommit "smelts" factsEffects -> do
          putStrLn "ok   firing a machine decoded ENTIRELY from live snapshot facts produces the IDENTICAL commit"
          putStrLn "     to firing the hand-authored machine directly:"
          putStrLn "--- rendered commit ---"
          putStrLn (T.unpack (renderFiredCommit "smelts" factsEffects))
          putStrLn "--- end ---"
      | otherwise -> do
          putStrLn "FAIL: fact-sourced and hand-authored firings produced DIFFERENT commits"
          putStrLn ("  via original: " ++ T.unpack (renderFiredCommit "smelts" originalEffects))
          putStrLn ("  via facts:    " ++ T.unpack (renderFiredCommit "smelts" factsEffects))
          exitFailure
