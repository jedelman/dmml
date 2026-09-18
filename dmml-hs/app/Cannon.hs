{-# LANGUAGE OverloadedStrings #-}

-- | The cannon: a generator that FIRES ARCHITECTURE. It constructs
-- candidate dungeon rooms as real 'DMML.Ast' values -- never as text --
-- and renders them via 'DMML.Fire.renderFiredMachine' (the exact
-- inverse of 'DMML.Surface.parseMachineSurface', proven to round-trip).
-- Because every emitted machine is a well-typed 'MachineStmt' by
-- construction, the cannon is STRUCTURALLY INCAPABLE of producing
-- ungrammatical DMML -- the formal-grammar guarantee discussed at
-- length: generativity enters here (the generator), grammatical
-- legality is free (AST construction), semantic soundness is still the
-- guard's job downstream (fire-transition filters), and the choice of
-- which generated architecture becomes real stays Jev's.
--
-- The architectural LANGUAGE is a small set of room variants that
-- differ in their RELATIONSHIPS, not their surface shape -- what a room
-- requires to breach and what it yields when breached. That relational
-- variety is what turns a frontier into a real choice for the chooser
-- downstream (build a forge to enable vaults later, spend a move on a
-- spur's treasure now, or push a hall forward), rather than a forced
-- single-option chain.
--
-- Usage:
--   cannon <hall|forge|vault|spur> <newNode> <parentNode>
--   cannon fork <forkNode> <parentNode> <leftFrontier> <rightFrontier>
--
-- A room's variant is its RELATIONSHIP to the rest of the dungeon. A
-- fork is the sharper architectural element: one machine with two
-- mutually-exclusive transitions (a single unchosen -> chosen
-- lifecycle lock, so exactly one ever fires), each clearing a
-- DIFFERENT onward frontier. Choosing one strands the other subtree
-- forever -- the one place a chooser downstream faces a real, permanent
-- decision about the dungeon's shape, not just a forced next step.
--
-- Prints one `machine` block to stdout. The machine's own initial
-- `state` fact is NOT emitted here (a machine's current state is
-- mutable world data, not structural definition -- the caller seeds it,
-- same as every other fact-native machine in this project).
module Main (main) where

import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Environment (getArgs)
import System.Exit (exitFailure)

import DMML.Ast
import DMML.Fire (renderFiredMachine)

sp :: Span
sp = Span "cannon"

nr :: Text -> NodeRef
nr = NodeRef . T.splitOn "/"

-- A guard `anchor `pred` obj` over multi-segment literal nodes.
guardFact :: Text -> Text -> Text -> GuardClause
guardFact anchor predicate obj =
  GuardClause
    { guardNegated = False
    , guardExists =
        ExistsExpr
          { existsPattern =
              Pattern
                { patternAnchor = TermNode anchor
                , patternHops = [PatternHop predicate (TermNode obj)]
                }
          , existsSpan = sp
          }
    , guardSpan = sp
    }

-- assert self `pred` obj  (obj a multi-segment literal node)
assertSelf :: Text -> Text -> Effect
assertSelf predicate obj =
  EffectAssert TermSelf (PredIdent predicate) (EffectValueTerm (TermNode obj))

-- assert <node> `pred` obj  (targets some other subject)
assertNode :: Text -> Text -> Text -> Effect
assertNode subj predicate obj =
  EffectAssert (TermNode subj) (PredIdent predicate) (EffectValueTerm (TermNode obj))

lastSeg :: Text -> Text
lastSeg = last . T.splitOn "/"

data Variant = Hall | Forge | Vault | Spur

parseVariant :: String -> Maybe Variant
parseVariant "hall" = Just Hall
parseVariant "forge" = Just Forge
parseVariant "vault" = Just Vault
parseVariant "spur" = Just Spur
parseVariant _ = Nothing

-- | Build a room machine. Every variant shares the same spine -- a
-- `breach` transition, sealed -> open, gated on its parent being
-- cleared, that clears itself so its own children become reachable.
-- The variants diverge only in the RELATIONSHIP layer: extra guards
-- (what else must hold) and extra effects (what breaching yields).
mkRoom :: Variant -> Text -> Text -> MachineStmt
mkRoom variant newNode parent =
  MachineStmt
    { machineNode = nr newNode
    , machineStates = [StateDecl "sealed" sp, StateDecl "open" sp]
    , machineTransitions = [breach]
    , machineSpan = sp
    }
  where
    (extraGuards, extraEffects) = case variant of
      Hall -> ([], [])
      -- forge: no extra requirement; yields a key that vaults need.
      Forge -> ([], [assertNode "vault/keyring" "has" "key/gold"])
      -- vault: requires a key to already exist; yields a sigil.
      Vault -> ([guardFact "vault/keyring" "has" "key/gold"], [assertSelf "yields" ("sigil/" <> lastSeg newNode)])
      -- spur: no extra requirement; yields treasure but clears nothing
      -- onward (see below -- a spur does NOT clear itself, so it is a
      -- genuine dead end: reward at the cost of a foreclosed path).
      Spur -> ([], [assertSelf "yields" ("relic/" <> lastSeg newNode)])
    -- Every variant except a spur clears itself, opening its children.
    clearSelf = case variant of
      Spur -> []
      _ -> [assertSelf "cleared" "mark/yes"]
    breach =
      TransitionDecl
        { transitionIdent = "breach"
        , transitionParams = []
        , transitionFrom = Just "sealed"
        , transitionTo = Just "open"
        , transitionGuards = guardFact parent "cleared" "mark/yes" : extraGuards
        , transitionEffects =
            clearSelf
              ++ extraEffects
              ++ [ EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "open"))
                 , EffectRetract TermSelf [] (PredIdent "state") Nothing
                 ]
        , transitionSpan = sp
        }

-- | A fork: one machine, two mutually-exclusive transitions sharing a
-- single unchosen -> chosen lock. Each clears a different onward
-- frontier node; the from->to lifecycle (not a negated guard, which the
-- retroconsistency gate would refuse) is what makes them exclusive.
mkFork :: Text -> Text -> Text -> Text -> MachineStmt
mkFork forkNode parent leftFrontier rightFrontier =
  MachineStmt
    { machineNode = nr forkNode
    , machineStates = [StateDecl "unchosen" sp, StateDecl "chosen" sp]
    , machineTransitions = [go "goLeft" leftFrontier, go "goRight" rightFrontier]
    , machineSpan = sp
    }
  where
    go name frontier =
      TransitionDecl
        { transitionIdent = name
        , transitionParams = []
        , transitionFrom = Just "unchosen"
        , transitionTo = Just "chosen"
        , transitionGuards = [guardFact parent "cleared" "mark/yes"]
        , transitionEffects =
            [ assertNode frontier "cleared" "mark/yes"
            , EffectAssert TermSelf (PredIdent "state") (EffectValueTerm (TermNode "chosen"))
            , EffectRetract TermSelf [] (PredIdent "state") Nothing
            ]
        , transitionSpan = sp
        }

main :: IO ()
main = do
  args <- getArgs
  case args of
    ["fork", forkNode, parent, left, right] ->
      TIO.putStr (renderFiredMachine (mkFork (T.pack forkNode) (T.pack parent) (T.pack left) (T.pack right)))
    [v, newNode, parent] ->
      case parseVariant v of
        Just variant -> TIO.putStr (renderFiredMachine (mkRoom variant (T.pack newNode) (T.pack parent)))
        Nothing -> putStrLn ("cannon: unknown variant " <> v <> " (want hall|forge|vault|spur)") >> exitFailure
    _ ->
      putStrLn "usage: cannon <hall|forge|vault|spur> <newNode> <parentNode>"
        >> putStrLn "       cannon fork <forkNode> <parentNode> <leftFrontier> <rightFrontier>"
        >> exitFailure
