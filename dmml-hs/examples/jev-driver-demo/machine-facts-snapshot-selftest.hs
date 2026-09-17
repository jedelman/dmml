{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED phase-2 test: 'DMML.MachineFacts.decodeMachineFromSnapshot'
-- against a genuine 'DMML.Materialize.WorldSnapshot' built by applying
-- 'encodeMachine's output as real, separate single-fact commits (never
-- one commit asserting the same (subject, predicate) key twice -- the
-- one hard constraint this whole design fits inside, per
-- DMML.MachineFacts's own doc comment) -- not a hand-built fact list,
-- which the phase-1 test already covered. Reuses the same
-- exercise-every-branch machine as that test. Compiled and run for
-- real; needs only DMML.Ast/DMML.Materialize/DMML.MachineFacts, no
-- stubs, same as the phase-1 test.
module Main (main) where

import Data.List (sortOn)
import Data.Text (Text)
import qualified Data.Text as T
import System.Exit (exitFailure)

import DMML.Ast
import DMML.MachineFacts (decodeMachineFromSnapshot, encodeMachine)
import DMML.Materialize (applyCommits)

sp :: Span
sp = Span "/test"

nodeRef :: Text -> NodeRef
nodeRef = NodeRef . T.splitOn "/"

original :: MachineStmt
original =
  MachineStmt
    { machineNode = nodeRef "forest/warden"
    , machineStates = [StateDecl "watching" sp, StateDecl "acted" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "intervene"
            , transitionParams = ["target", "reason"]
            , transitionFrom = Just "watching"
            , transitionTo = Just "acted"
            , transitionGuards =
                [ GuardClause
                    { guardNegated = True
                    , guardExists =
                        ExistsExpr
                          { existsPattern =
                              Pattern
                                { patternAnchor = TermSelf
                                , patternHops =
                                    [ PatternHop {hopPredicate = "sanctioned", hopTerm = TermParam "target"}
                                    , PatternHop {hopPredicate = "status", hopTerm = TermNode "status/forbidden"}
                                    ]
                                }
                          , existsSpan = sp
                          }
                    , guardSpan = sp
                    }
                ]
            , transitionEffects =
                [ EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "acted"))
                , EffectRetract
                    (TermParam "target")
                    [PatternHop {hopPredicate = "guardedBy", hopTerm = TermSelf}]
                    (PredIdent "status")
                    (Just (EffectValueLiteral (LitString "at-risk")))
                , EffectSpawn (TermParam "reason") (nodeRef "template/incident")
                ]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

-- | One commit per fact -- the real constraint (never the same
-- (subject, predicate) key twice in one commit) this whole design has
-- to respect, faced for real here rather than assumed away.
factToCommit :: FactStmt -> CommitStmt
factToCommit f =
  CommitStmt
    { commitVerb = "encode"
    , commitItems = [ItemFact f]
    , commitRefs = mempty
    , commitSpan = sp
    }

blank :: Span
blank = Span ""

stripSpans :: MachineStmt -> MachineStmt
stripSpans m =
  m
    { machineSpan = blank
    , machineStates = [s {stateSpan = blank} | s <- machineStates m]
    , machineTransitions = map stripTransition (machineTransitions m)
    }
  where
    stripTransition t =
      t
        { transitionSpan = blank
        , transitionParams = sortOn id (transitionParams t)
        , transitionGuards = map stripGuard (transitionGuards t)
        }
    stripGuard g =
      g
        { guardSpan = blank
        , guardExists = (guardExists g) {existsSpan = blank}
        }

sameStructure :: MachineStmt -> MachineStmt -> Bool
sameStructure a0 b0 =
  let a = stripSpans a0
      b = stripSpans b0
   in machineNode a == machineNode b
        && sortOn stateIdent (machineStates a) == sortOn stateIdent (machineStates b)
        && sortOn transitionIdent (machineTransitions a) == sortOn transitionIdent (machineTransitions b)

main :: IO ()
main = do
  let facts = encodeMachine original
      commits = map factToCommit facts
      snap = applyCommits "encode" commits
  putStrLn ("applied " ++ show (length commits) ++ " single-fact commits to build the snapshot")
  case decodeMachineFromSnapshot (machineNode original) snap of
    Left err -> putStrLn ("FAIL: decode-from-snapshot error: " ++ show err) >> exitFailure
    Right decoded
      | sameStructure decoded original ->
          putStrLn "ok   machine decoded from a REAL WorldSnapshot (built via one commit per fact) matches the original"
      | otherwise -> do
          putStrLn "FAIL: decoded-from-snapshot machine does not match original (spans/order normalized, shown below)"
          putStrLn ("  original: " ++ show (stripSpans original))
          putStrLn ("  decoded:  " ++ show (stripSpans decoded))
          exitFailure
