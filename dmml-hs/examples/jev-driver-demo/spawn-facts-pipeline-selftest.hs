{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED test of the full "spawn produces something
-- immediately fireable, no Surface round-trip" pipeline: fires a
-- spawner machine's transition to mint a new machine instance, checks
-- 'DMML.Fire.renderFiredCommits' produces one commit per encoded fact
-- (never a duplicate (subject, predicate) key inside any single one --
-- the one hard constraint this whole design fits inside), then --
-- independently of that rendered text, using the same direct-fact
-- path 'fire-from-facts-selftest.hs' already proved -- applies the
-- spawned machine's OWN encoded facts and fires ITS transition too.
-- Two real machines fired in sequence, the second of which never
-- existed as anything but facts the first machine's own firing
-- produced.
--
-- Compiled and run for real against the same interface-only
-- Surface\/Retroconsistency stubs the other Fire.hs-touching
-- self-tests already use and explain.
module Main (main) where

import Data.List (isPrefixOf)
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

templateMachine :: MachineStmt
templateMachine =
  MachineStmt
    { machineNode = nodeRef "template/sapling"
    , machineStates = [StateDecl "growing" sp, StateDecl "grown" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "grow"
            , transitionParams = []
            , transitionFrom = Nothing
            , transitionTo = Nothing
            , transitionGuards = [GuardClause False (ExistsExpr (Pattern TermSelf [PatternHop "watered" (TermNode "yes/1")]) sp) sp]
            , transitionEffects = [EffectAssert TermSelf (PredIdent "grown") (EffectValueLiteral (LitBoolean True))]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

spawnerMachine :: MachineStmt
spawnerMachine =
  MachineStmt
    { machineNode = nodeRef "forest/mother"
    , machineStates = [StateDecl "idle" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "seed"
            , transitionParams = ["name"]
            , transitionFrom = Nothing
            , transitionTo = Nothing
            , transitionGuards = []
            , transitionEffects = [EffectSpawn (TermParam "name") (nodeRef "template/sapling")]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

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
  let machines = Map.fromList [("forest/mother", spawnerMachine), ("template/sapling", templateMachine)]
      ctx = EvalContext {ctxSelfNode = "forest/mother", ctxParams = Map.fromList [("name", "forest/sapling1")]}

  spawnedFacts <- case fireTransition machines spawnerMachine "seed" ctx (applyIdentifiedCommits "empty" []) of
    Left err -> putStrLn ("FAIL: seed refused: " ++ show err) >> exitFailure
    Right [ResolvedSpawn spawned] -> do
      putStrLn "ok   spawner produced one ResolvedSpawn"

      -- 1. Structural check on renderFiredCommits: one commit per
      -- encoded fact, no duplicate (subject, predicate) key within any
      -- single commit's own text.
      let commits = renderFiredCommits "seeds" [ResolvedSpawn spawned]
          encoded = encodeMachine spawned
      if length commits == length encoded
        then putStrLn ("ok   renderFiredCommits produced " ++ show (length commits) ++ " commits, one per encoded fact")
        else putStrLn ("FAIL: expected " ++ show (length encoded) ++ " commits, got " ++ show (length commits)) >> exitFailure

      if all noDuplicateFactLine commits
        then putStrLn "ok   no commit contains more than one fact line (the one hard constraint, respected structurally)"
        else putStrLn "FAIL: some commit contains more than one fact line" >> exitFailure

      pure encoded
    Right other -> putStrLn ("FAIL: unexpected resolved effects: " ++ show other) >> exitFailure

  -- 2. The real pipeline claim: apply the SPAWNED machine's own
  -- encoded facts (the direct-fact path, not the rendered text) plus
  -- the one world fact its guard needs, and fire ITS transition --
  -- a machine that never existed as anything but facts the first
  -- machine's firing produced.
  let allFacts = spawnedFacts ++ [worldFact (nodeRef "forest/sapling1") "watered" (ValueNode (nodeRef "yes/1"))]
      commits2 = zipWith factToIdentifiedCommit [0 ..] allFacts
      snap = applyIdentifiedCommits "world" commits2

  case fireTransitionFromFacts machines (nodeRef "forest/sapling1") "grow" (EvalContext {ctxSelfNode = "forest/sapling1", ctxParams = Map.empty}) snap of
    Left err -> putStrLn ("FAIL: firing the SPAWNED machine's own transition refused: " ++ show err) >> exitFailure
    Right effects -> do
      putStrLn "ok   fired a transition on a machine that never existed as anything but facts a PRIOR firing produced"
      putStrLn (T.unpack (renderFiredCommit "grows" effects))
  where
    noDuplicateFactLine commitText =
      let ls = filter (\l -> "  " `isPrefixOf` T.unpack l && not ("  declare" `isPrefixOf` T.unpack l)) (T.lines commitText)
       in length ls <= 1
