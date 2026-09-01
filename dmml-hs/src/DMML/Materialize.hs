{-# LANGUAGE OverloadedStrings #-}

-- | A minimal materializer: applies a sequence of already-validated
-- 'CommitStmt's in order and produces a queryable snapshot of current
-- world state -- what's declared, and the latest value per (subject,
-- predicate). This is deliberately NOT a port of the real crate's
-- @interpret::Materialized@: no real cid/uri-based provenance, no
-- @reachable_from@ graph-scoping from a root commit, @consumes@ is
-- interpreted operationally (retract the referenced (subject,
-- predicate) from current state) rather than resolved against a real
-- citation graph. Built for one purpose -- rendering "the world so
-- far" as agent-readable context, so a compliance scenario can ask an
-- agent to author something that's actually CONSISTENT with existing
-- state, instead of authoring blind -- not as a stand-in for the real
-- interpreter. See written-world/dev-journal/2026-08-31-dmml-runtime-
-- migration-scope.md's Phase 2 for what a real port still needs.
module DMML.Materialize
  ( WorldSnapshot (..)
  , emptySnapshot
  , applyCommit
  , applyCommits
  , renderSnapshot
  ) where

import Data.List (foldl', sortOn)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast

data WorldSnapshot = WorldSnapshot
  { snapshotDeclared :: Map Text DeclKind
  , snapshotFacts :: Map (Text, Text) Value
  }
  deriving (Eq, Show)

emptySnapshot :: WorldSnapshot
emptySnapshot = WorldSnapshot Map.empty Map.empty

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

predText :: PredicateRef -> Text
predText RdfType = "a"
predText (PredIdent t) = t

applyCommit :: WorldSnapshot -> CommitStmt -> WorldSnapshot
applyCommit snap stmt = foldl' applyItem snap (commitItems stmt)
  where
    applyItem s (ItemDeclare d) =
      s {snapshotDeclared = Map.insert (declareIdent d) (declareKind d) (snapshotDeclared s)}
    applyItem s (ItemFact f) =
      s {snapshotFacts = Map.insert (nodeRefText (factSubject f), predText (factPredicate f)) (factValue f) (snapshotFacts s)}
    applyItem s (ItemConsumes cb) = foldl' applyConsume s (consumesEntries cb)

    applyConsume s (ConsumeFact fc) =
      s {snapshotFacts = Map.delete (nodeRefText (factConsumeSubject fc), factConsumePredicate fc) (snapshotFacts s)}
    applyConsume s (ConsumeStrong _) = s

applyCommits :: [CommitStmt] -> WorldSnapshot
applyCommits = foldl' applyCommit emptySnapshot

renderValue :: Value -> Text
renderValue (ValueNode n) = nodeRefText n
renderValue (ValueLiteral (LitString s)) = "\"" <> s <> "\""
renderValue (ValueLiteral (LitNumber n)) = n
renderValue (ValueLiteral (LitBoolean b)) = if b then "true" else "false"

kindWord :: DeclKind -> Text
kindWord DeclRelation = "relation"
kindWord DeclAttribute = "attribute"

-- | Renders a snapshot as plain text, in the same dot-field idiom the
-- Surface syntax itself uses for facts -- so handing this to an agent
-- as context reads consistently with what it's being asked to author.
renderSnapshot :: WorldSnapshot -> Text
renderSnapshot snap =
  T.unlines $
    ["Declared predicates:"]
      ++ ["  " <> kindWord k <> " " <> name | (name, k) <- sortOn fst (Map.toList (snapshotDeclared snap))]
      ++ ["", "Current facts:"]
      ++ [ "  " <> subj <> " . " <> pred_ <> " = " <> renderValue v
         | ((subj, pred_), v) <- sortOn fst (Map.toList (snapshotFacts snap))
         ]
