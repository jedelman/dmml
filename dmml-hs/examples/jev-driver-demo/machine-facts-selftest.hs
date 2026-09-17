{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED round-trip test for 'DMML.MachineFacts' -- builds a
-- MachineStmt exercising every branch (multiple states, a multi-hop
-- NEGATED guard, an assert, a retract WITH hops and a value, and a
-- spawn effect), encodes it to facts, decodes back, and checks the
-- result matches -- order-insensitively where order was never
-- semantically real (states, transitions, guards, effects at the
-- top level), exactly (order-sensitively) for hop sequences, which
-- DO carry real meaning. Compiled and run for real, same sandbox
-- constraints as this change's other self-tests (see their own doc
-- comments) -- DMML.MachineFacts needs only DMML.Ast, no stubs
-- required at all here.
module Main (main) where

import Data.List (sortOn)
import Data.Text (Text)
import qualified Data.Text as T
import System.Exit (exitFailure)

import DMML.Ast
import DMML.MachineFacts (decodeMachine, encodeMachine)

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

-- | Every 'Span' reset to a canonical value first -- provenance
-- metadata this module was never asked to round-trip (encode always
-- stamps '/machine-facts', decode never had the original's), so
-- comparing it would only ever measure that fact, not the encoding's
-- real correctness. Order-insensitive at the top level (states,
-- transitions, guards, effects -- content this module's own doc
-- comment says was never semantically ordered), exact equality
-- everywhere nested, which is where hop order actually lives and
-- actually matters.
blank :: Span
blank = Span ""

sameStructure :: MachineStmt -> MachineStmt -> Bool
sameStructure a0 b0 =
  let a = stripSpans a0
      b = stripSpans b0
   in machineNode a == machineNode b
        && sortOn stateIdent (machineStates a) == sortOn stateIdent (machineStates b)
        && sortOn transitionIdent (machineTransitions a) == sortOn transitionIdent (machineTransitions b)

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
        , transitionParams = sortOn id (transitionParams t) -- param BINDING is by name (a Map), never by position
        , transitionGuards = map stripGuard (transitionGuards t)
        }
    stripGuard g =
      g
        { guardSpan = blank
        , guardExists = (guardExists g) {existsSpan = blank}
        }

main :: IO ()
main = do
  let facts = encodeMachine original
  putStrLn ("encoded to " ++ show (length facts) ++ " facts")
  case decodeMachine (machineNode original) facts of
    Left err -> putStrLn ("FAIL: decode error: " ++ show err) >> exitFailure
    Right decoded
      | decoded == original -> putStrLn "ok   decoded machine is EXACTLY equal to the original (including all orders)"
      | sameStructure decoded original ->
          putStrLn "ok   decoded machine matches original content (state/transition order differs, as expected -- never semantically real)"
      | otherwise -> do
          putStrLn "FAIL: decoded machine does not match original (spans/order normalized, shown below)"
          putStrLn ("  original: " ++ show (stripSpans original))
          putStrLn ("  decoded:  " ++ show (stripSpans decoded))
          exitFailure
