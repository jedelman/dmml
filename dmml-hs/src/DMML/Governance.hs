{-# LANGUAGE OverloadedStrings #-}

-- | Governed-vs-ungoverned arbitration for a (subject, predicate) pair
-- holding several live alternatives (see 'DMML.Materialize.Alternatives').
-- This is the piece that replaces the old reactive @Contest@-minting
-- mechanism per the 2026-09-02 redesign: nothing mints anything here,
-- this module only decides whether reduction to one canonical value is
-- even possible right now.
--
-- Reuses the real production crate's governance idiom faithfully
-- ('dmml-runtime/src/machine.rs'\'s @machines_for_verb@ -- @owner
-- equips machine@ + @machine trigger \"verb\"@), reinterpreted: the
-- disputed pair's subject is the owner, and the disputed predicate's
-- name is the verb. A machine equipped this way on the subject, with a
-- matching @trigger@, is that pair's declared governor.
--
-- HONEST SCOPE LIMIT, not silently glossed over: 'arbitrate' can only
-- actually validate a resolution for the @state@ predicate today,
-- because 'DMML.Ast.Effect' is still hardcoded to
-- @(self, \"state\", ident)@ -- there is no way for a transition to
-- assert any other predicate's value yet. A governed non-@state@
-- predicate is correctly FOUND (its governing machine is real, located)
-- but cannot yet be ARBITRATED -- that gap is real, disclosed, and
-- tracked as Phase A1/A2 (typed commits + Effect generalization),
-- jedelman/dmml#1. This module does not pretend otherwise.
module DMML.Governance
  ( findGoverningMachine
  , GovernedOutcome (..)
  , arbitrate
  , applyGovernance
  ) where

import Data.List (foldl')
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast
import DMML.Guard (EvalContext (..), mayFire)
import DMML.Materialize (WorldSnapshot (..), collapseToOne, currentValue)

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

-- | Which machine, if any, governs a (subject, predicate) pair. An
-- ordinary application-code lookup, not a guard walk -- @trigger@'s
-- literal-string value is fine here (matches the real crate's own
-- encoding), unlike a fact 'DMML.Guard.evalExists' needs to traverse.
findGoverningMachine :: (Text, Text) -> WorldSnapshot -> Maybe Text
findGoverningMachine (subj, pred_) snap =
  case
    [ machineText
    | (_label, ValueNode m) <- currentValue (subj, "equips") snap
    , let machineText = nodeRefText m
    , (_label2, ValueLiteral (LitString v)) <- currentValue (machineText, "trigger") snap
    , v == pred_
    ]
  of
    (m : _) -> Just m
    [] -> Nothing

data GovernedOutcome
  = -- | One live alternative is a legal transition outcome on the
    -- governing machine, given the world's OTHER current facts (e.g. a
    -- real @witnessedBy@) -- its provenance label and value.
    Resolved Text Value
  | -- | A governing machine was found, but no live alternative currently
    -- validates against any of its transitions (or more than one does,
    -- ambiguously -- never silently picked either way).
    StillPending
  | -- | No governing machine at all for this pair.
    Ungoverned
  deriving (Eq, Show)

-- | Arbitrates one (subject, predicate) pair's live alternatives against
-- its governing machine, if any exists and if the predicate is @state@
-- (see this module's own doc comment for why that restriction is real
-- and not yet lifted).
arbitrate :: Map Text MachineStmt -> (Text, Text) -> WorldSnapshot -> GovernedOutcome
arbitrate machines key@(_subj, pred_) snap =
  case findGoverningMachine key snap of
    Nothing -> Ungoverned
    Just machineNode
      | pred_ /= "state" -> StillPending
      | otherwise -> case Map.lookup machineNode machines of
          Nothing -> StillPending
          -- self is the GOVERNING MACHINE's own node, not the disputed
          -- pair's subject -- a transition's guards (e.g. `self
          -- witnessedBy npc/keeper`) are about the machine's own facts
          -- (contest/x . witnessedBy = ...), never the disputed
          -- subject's (shrine/threshold's). Real bug, caught by
          -- GovernanceDemo.hs's Resolved case actually failing before
          -- this fix -- every guard silently evaluated against the
          -- wrong node's facts, and passed for the wrong reason (or, as
          -- here, failed for the wrong reason).
          Just machine -> case validated machineNode machine of
            [(label, v)] -> Resolved label v
            _ -> StillPending
  where
    alternatives = currentValue key snap
    validated machineNode machine =
      [ (label, v)
      | (label, v) <- alternatives
      , ValueNode targetIdent <- [v]
      , let targetText = nodeRefText targetIdent
            ctx = EvalContext {ctxSelfNode = machineNode, ctxParams = Map.empty}
      , TransitionDecl {transitionIdent = tident} <- machineTransitions machine
      , Just (True, effects, _to) <- [mayFire machine tident ctx snap]
      , EffectAssert assertedIdent <- effects
      , assertedIdent == targetText
      ]

-- | Applies governance-aware arbitration across every currently multi-
-- valued (subject, predicate) pair in a snapshot. A pair that
-- 'arbitrate's to 'Resolved' gets collapsed to that one value; every
-- other pair (single-valued already, 'Ungoverned', or 'StillPending')
-- is left exactly as it is -- multi-valued or not, nothing forced. This
-- is the direct replacement for the old reactive @applyContests@: same
-- job (decide what a snapshot's facts settle to), entirely different
-- mechanism (governed arbitration over already-collision-free
-- alternatives, not a hand-simulated single guard shape).
applyGovernance :: Map Text MachineStmt -> WorldSnapshot -> WorldSnapshot
applyGovernance machines snap =
  foldl' resolveOne snap (Map.keys (snapshotFacts snap))
  where
    resolveOne s key = case arbitrate machines key s of
      Resolved label v -> collapseToOne key label v s
      _ -> s
