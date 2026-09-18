{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED self-test for 'DMML.Ast.EffectSpawn' and
-- 'DMML.Fire.renderFiredMachine' -- constructs a spawner machine and a
-- template machine directly (no parser involved), fires the spawn
-- transition through the real 'DMML.Fire.fireTransition', and checks
-- both the resolved 'DMML.Fire.ResolvedSpawn' payload (correct new
-- node, template body copied verbatim) and the rendered Surface text
-- against the exact lines 'SURFACE.md'\'s documented machine grammar
-- would produce.
--
-- This was originally written in a sandbox with GHC but no reachable
-- Hackage, so 'DMML.Fire''s own imports of "DMML.Surface" (megaparsec)
-- and "DMML.Retroconsistency" could not be built, and it ran against
-- local interface-only STUBS of those two. It left one thing explicitly
-- unverified -- whether 'renderFiredMachine''s output actually
-- re-parses through the real 'DMML.Surface.parseMachineSurface' -- and
-- asked for a re-run against a real build.
--
-- That re-run happened 2026-09-18, on a box with megaparsec and aeson
-- from the distro rather than Hackage. No stubs: every module here is
-- the genuine source. And the gap is closed rather than merely
-- re-asserted -- the round-trip check below actually calls the real
-- parser on the rendered spawn output and compares the result
-- structurally, which is the check the caveat said was missing. The
-- `spawn` grammar addition in Surface.hs is exercised by exactly that
-- call, so it is no longer uncompiled or untested either.
module Main (main) where

import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Exit (exitFailure)

import DMML.Ast
import DMML.Fire
import DMML.Guard (EvalContext (..))
import DMML.Materialize (emptySnapshot)
import DMML.Surface (parseMachineSurface)

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
            , transitionFrom = Just "growing"
            , transitionTo = Just "grown"
            , transitionGuards = [GuardClause False (ExistsExpr (Pattern TermSelf [PatternHop "watered" (TermNode "yes/1")]) sp) sp]
            , transitionEffects =
                [ EffectRetract TermSelf [] (PredIdent "state") Nothing
                , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "grown"))
                ]
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

main :: IO ()
main = do
  let machines = Map.fromList [("forest/mother", spawnerMachine), ("template/sapling", templateMachine)]
      ctx = EvalContext {ctxSelfNode = "forest/mother", ctxParams = Map.fromList [("name", "forest/sapling17")], ctxBindings = Map.empty}
  case fireTransition machines spawnerMachine "seed" ctx emptySnapshot of
    Left err -> putStrLn ("FAIL: fire refused: " ++ show err) >> exitFailure
    Right effects -> case effects of
      [ResolvedSpawn spawned]
        | machineNode spawned == nodeRef "forest/sapling17"
        , machineStates spawned == machineStates templateMachine
        , length (machineTransitions spawned) == 1 -> do
            putStrLn "ok   spawn resolved to a fresh MachineStmt with correct node + copied body"
            let rendered = renderFiredMachine spawned
            TIO.putStrLn "--- rendered spawned machine ---"
            TIO.putStr rendered
            TIO.putStrLn "--- end ---"
            if "machine forest/sapling17" `elem` linesOf rendered
              && "  states" `elem` linesOf rendered
              && "    growing" `elem` linesOf rendered
              && "  transition grow()" `elem` linesOf rendered
              && "    growing -> grown" `elem` linesOf rendered
              && "    guard self `watered` yes/1" `elem` linesOf rendered
              && "    retract self `state`" `elem` linesOf rendered
              && "    assert self `state` grown" `elem` linesOf rendered
              then putStrLn "ok   rendered machine text matches expected SURFACE.md-shaped lines"
              else putStrLn "FAIL: rendered machine text missing expected lines" >> exitFailure
            -- The round-trip this file's original caveat listed as
            -- UNVERIFIED. Matching expected LINES only proves the
            -- renderer emits the text someone expected; it says nothing
            -- about whether the grammar can read it back. Spans differ
            -- (the parser records real pointers where this test stamped
            -- its own), so compare span-erased.
            case parseMachineSurface rendered of
              Left _ -> putStrLn "FAIL: rendered spawn output does not re-parse" >> exitFailure
              Right back
                | eraseSpans back == eraseSpans spawned ->
                    putStrLn "ok   rendered machine re-parses through the real parseMachineSurface, unchanged"
                | otherwise ->
                    putStrLn "FAIL: rendered spawn output re-parses to a DIFFERENT machine" >> exitFailure
      other -> putStrLn ("FAIL: unexpected resolved effects: " ++ show other) >> exitFailure

  -- Template-not-found should refuse cleanly, not crash.
  let badMachines = Map.fromList [("forest/mother", spawnerMachine)]
  case fireTransition badMachines spawnerMachine "seed" ctx emptySnapshot of
    Left (FireSpawnTemplateNotFound _ _) -> putStrLn "ok   missing template refuses with FireSpawnTemplateNotFound"
    other -> putStrLn ("FAIL: missing template case: " ++ show other) >> exitFailure
  where
    linesOf = T.lines

    eraseSpans m =
      m
        { machineSpan = z
        , machineStates = [st {stateSpan = z} | st <- machineStates m]
        , machineTransitions = map eraseT (machineTransitions m)
        }
      where
        z = Span ""
        eraseT t =
          t
            { transitionSpan = z
            , transitionGuards =
                [ g {guardSpan = z, guardExists = (guardExists g) {existsSpan = z}}
                | g <- transitionGuards t
                ]
            }

