{-# LANGUAGE OverloadedStrings #-}

-- | Real, EXECUTED test of 'DMML.MachineFacts.candidateTransitions' --
-- builds a snapshot holding TWO distinct fact-native machines (never
-- hand-authored Surface text, never touching Fire.hs or its stubs --
-- this module is boot-package-only, same as the rest of
-- DMML.MachineFacts), confirms discovery finds exactly the transitions
-- both declare, and confirms a THIRD, hand-authored-in-Haskell-only
-- machine that was never applied to the snapshot at all is correctly
-- invisible -- discovery only ever sees what's actually IN the
-- snapshot, never anything a caller merely has lying around.
module Main (main) where

import Data.List (sort)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import System.Exit (exitFailure)

import DMML.Ast
import DMML.MachineFacts (candidateTransitions, encodeMachine)
import DMML.Materialize (IdentifiedCommit (..), applyIdentifiedCommits)

sp :: Span
sp = Span "/test"

nodeRef :: Text -> NodeRef
nodeRef = NodeRef . T.splitOn "/"

mkMachine :: Text -> [Text] -> MachineStmt
mkMachine name transitionNames =
  MachineStmt
    { machineNode = nodeRef name
    , machineStates = [StateDecl "idle" sp]
    , machineTransitions =
        [ TransitionDecl
            { transitionIdent = tn
            , transitionParams = []
            , transitionFrom = Nothing
            , transitionTo = Nothing
            , transitionGuards = []
            , transitionEffects = [EffectAssert TermSelf (PredIdent "touched") (EffectValueLiteral (LitBoolean True))]
            , transitionSpan = sp
            }
        | tn <- transitionNames
        ]
    , machineSpan = sp
    }

factToIdentifiedCommit :: Int -> FactStmt -> IdentifiedCommit
factToIdentifiedCommit n f =
  IdentifiedCommit
    { icRef = StrongRef {strongRefUri = "test://fact/" <> T.pack (show n), strongRefCid = "c" <> T.pack (show n), strongRefSpan = sp}
    , icCommit = CommitStmt {commitVerb = "encode", commitItems = [ItemFact f], commitRefs = mempty, commitSpan = sp}
    }

main :: IO ()
main = do
  let machineA = mkMachine "forest/oak" ["shed", "grow"]
      machineB = mkMachine "forest/pond" ["ripple"]
      neverApplied = mkMachine "forest/ghost" ["haunt"]

      allFacts = encodeMachine machineA ++ encodeMachine machineB
      snap = applyIdentifiedCommits "world" (zipWith factToIdentifiedCommit [0 ..] allFacts)

      found = [(T.intercalate "/" (nodeRefSegments (machineNode m)), transitionIdent t) | (m, t) <- candidateTransitions snap]
      expected = [("forest/oak", "shed"), ("forest/oak", "grow"), ("forest/pond", "ripple")]

  if sort found == sort expected
    then putStrLn ("ok   candidateTransitions found exactly the " ++ show (length expected) ++ " real transitions, sorted-equal: " ++ show (sort found))
    else do
      putStrLn "FAIL: candidateTransitions result doesn't match"
      putStrLn ("  expected: " ++ show (sort expected))
      putStrLn ("  found:    " ++ show (sort found))
      exitFailure

  if any ((== "forest/ghost") . fst) found
    then putStrLn "FAIL: discovery found a machine that was never applied to the snapshot at all" >> exitFailure
    else putStrLn "ok   the never-applied machine (forest/ghost) is correctly invisible to discovery"

  -- keep neverApplied "used" so GHC doesn't complain it's unreferenced
  print (machineNode neverApplied)
