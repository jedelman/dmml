{-# LANGUAGE OverloadedStrings #-}

-- | Standalone, runnable self-test for DMML.SpawnCycles, built and
-- actually EXECUTED (not just type-checked) in a sandbox that has GHC
-- but not the project's third-party deps (megaparsec, aeson), which
-- Surface.hs/FromJson.hs need but SpawnCycles.hs and its one real
-- dependency (Ast.hs) do not. Constructs MachineStmt values directly
-- (no parser involved) covering: no cycle, a direct self-spawn, and an
-- indirect A->B->A cycle. Exits nonzero if any check fails.
--
-- Not part of the cabal package -- a throwaway verification script,
-- run once as: ghc -isrc:examples/jev-driver-demo
--   examples/jev-driver-demo/spawn-cycles-selftest.hs && ./spawn-cycles-selftest
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import System.Exit (exitFailure)

import DMML.Ast
import DMML.SpawnCycles (detectSpawnCycles)

sp :: Span
sp = Span "/test"

nodeRef :: Text -> NodeRef
nodeRef t = NodeRef [t]

-- | A machine with one transition that (maybe) spawns a named template.
mkMachine :: Text -> Maybe Text -> MachineStmt
mkMachine name mSpawnTarget =
  MachineStmt
    { machineNode = nodeRef name
    , machineStates = [StateDecl "idle" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = "go"
            , transitionParams = []
            , transitionFrom = Just "idle"
            , transitionTo = Just "idle"
            , transitionGuards = []
            , transitionEffects = case mSpawnTarget of
                Nothing -> [EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "idle"))]
                Just target -> [EffectSpawn (TermParam "n") (nodeRef target)]
            , transitionSpan = sp
            }
        ]
    , machineSpan = sp
    }

check :: (Eq a, Show a) => String -> a -> a -> IO Bool
check label expected actual
  | expected == actual = putStrLn ("ok   " ++ label) >> pure True
  | otherwise =
      putStrLn ("FAIL " ++ label ++ ": expected " ++ show expected ++ ", got " ++ show actual) >> pure False

main :: IO ()
main = do
  let noCycleMachines = Map.fromList [("a", mkMachine "a" (Just "b")), ("b", mkMachine "b" Nothing)]
      selfSpawn = Map.fromList [("a", mkMachine "a" (Just "a"))]
      indirect = Map.fromList [("a", mkMachine "a" (Just "b")), ("b", mkMachine "b" (Just "a"))]
      threeCycle = Map.fromList [("a", mkMachine "a" (Just "b")), ("b", mkMachine "b" (Just "c")), ("c", mkMachine "c" (Just "a"))]

  results <-
    sequence
      [ check "acyclic A->B, no cycle reported" [] (detectSpawnCycles noCycleMachines)
      , check "direct self-spawn A->A" [["a", "a"]] (detectSpawnCycles selfSpawn)
      , check "indirect A->B->A collapses to one cycle" [["a", "b", "a"]] (detectSpawnCycles indirect)
      , check "three-node cycle A->B->C->A" [["a", "b", "c", "a"]] (detectSpawnCycles threeCycle)
      ]

  if and results
    then putStrLn "all spawn-cycle checks passed"
    else putStrLn "SOME CHECKS FAILED" >> exitFailure
