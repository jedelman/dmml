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
-- Built and run for real (not just type-checked) in a sandbox with
-- GHC but no network access to Hackage (a TUF signature-verification
-- mismatch with this cabal-install version blocked fetching
-- megaparsec/aeson; disabling that verification to route around it
-- was correctly refused as security-weakening, not attempted). Fire.hs
-- itself imports DMML.Surface (needs megaparsec) and
-- DMML.Retroconsistency (imports megaparsec's errorBundlePretty
-- directly), neither buildable here -- so this ran against LOCAL,
-- interface-only STUBS of those two modules, with their real exported
-- type signatures copied verbatim from src/DMML/Surface.hs and
-- src/DMML/Retroconsistency.hs (confirmed by reading them directly,
-- not guessed). Every module actually exercised here by real
-- 'EffectSpawn' logic -- DMML.Ast, DMML.Guard, DMML.Materialize,
-- DMML.Fire itself -- is the genuine, unmodified-elsewhere source,
-- not a stub. What's UNVERIFIED: whether 'renderFiredMachine's output
-- actually re-parses through the real 'DMML.Surface.parseMachineSurface'
-- (megaparsec unavailable here to check), and the new Surface.hs
-- grammar addition for `spawn` itself (also uncompiled, see this
-- change's own PR description). Re-run this against the real build
-- once a toolchain with network access to Hackage is available, and
-- delete this doc-comment's caveats once it has been.
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
      ctx = EvalContext {ctxSelfNode = "forest/mother", ctxParams = Map.fromList [("name", "forest/sapling17")]}
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
      other -> putStrLn ("FAIL: unexpected resolved effects: " ++ show other) >> exitFailure

  -- Template-not-found should refuse cleanly, not crash.
  let badMachines = Map.fromList [("forest/mother", spawnerMachine)]
  case fireTransition badMachines spawnerMachine "seed" ctx emptySnapshot of
    Left (FireSpawnTemplateNotFound _ _) -> putStrLn "ok   missing template refuses with FireSpawnTemplateNotFound"
    other -> putStrLn ("FAIL: missing template case: " ++ show other) >> exitFailure
  where
    linesOf = T.lines

