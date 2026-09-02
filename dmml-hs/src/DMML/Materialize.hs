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
  , ContestedEntry (..)
  , emptySnapshot
  , applyCommit
  , applyCommits
  , markContested
  , resolveContested
  , applyContests
  , renderSnapshot
  ) where

import Data.List (foldl', sortOn)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast

-- | Two or more values independently asserted for the same (subject,
-- predicate) by branches that never saw each other's change -- per
-- written-world's own "corruption as content" principle (SPEC.md §12),
-- this is never silently collapsed to one value. Each option carries a
-- provenance label (whichever commit/branch asserted it) so both sides
-- stay attributable, not anonymous.
newtype ContestedEntry = ContestedEntry {contestedOptions :: [(Text, Value)]}
  deriving (Eq, Show)

data WorldSnapshot = WorldSnapshot
  { snapshotDeclared :: Map Text DeclKind
  , snapshotFacts :: Map (Text, Text) Value
  , -- | A contested (subject, predicate) is NEVER also present in
    -- 'snapshotFacts' -- there is no single "current" value to report
    -- for it, on purpose. See 'markContested'/'resolveContested'.
    snapshotContested :: Map (Text, Text) ContestedEntry
  }
  deriving (Eq, Show)

emptySnapshot :: WorldSnapshot
emptySnapshot = WorldSnapshot Map.empty Map.empty Map.empty

-- | Marks a (subject, predicate) as contested -- moves it out of
-- 'snapshotFacts' (if a value happened to be there) and records every
-- live option with its provenance. Never called by ordinary sequential
-- commit application ('applyCommit') -- an ordinary later commit
-- overwriting an earlier one for the same pair is ordinary, correct
-- append-only history (SPEC.md/UpdateInput's own established rule),
-- not a contest. This is only ever called by something that has
-- independently determined real divergence (two branches, neither
-- aware of the other) -- see dmml-hs/app/CheckDivergence.hs.
--
-- MERGES into any options already recorded for this key rather than
-- replacing them -- a real bug, found running the worktree-sync
-- endurance test, not hypothesized: a pair left unresolved after one
-- Contest can go on to be disputed again by a LATER, entirely separate
-- Contest (nobody ever witnessed a resolution in between), and a plain
-- overwrite here silently dropped the first Contest's options from the
-- rendered snapshot the moment the second one was processed -- exactly
-- the "silently resolve/drop information" move SPEC.md sec12 forbids,
-- just relocated from resolving-a-value to dropping-an-option. Options
-- already present (identical label AND value) aren't duplicated if the
-- same divergence is somehow reprocessed.
markContested :: (Text, Text) -> [(Text, Value)] -> WorldSnapshot -> WorldSnapshot
markContested key options snap =
  snap
    { snapshotFacts = Map.delete key (snapshotFacts snap)
    , snapshotContested = Map.insertWith mergeEntries key (ContestedEntry options) (snapshotContested snap)
    }
  where
    mergeEntries (ContestedEntry newOpts) (ContestedEntry existingOpts) =
      ContestedEntry (existingOpts ++ [o | o <- newOpts, o `notElem` existingOpts])

-- | Resolves a contest: removes it from 'snapshotContested', sets the
-- agreed value in 'snapshotFacts'. Callers are responsible for having
-- actually verified the resolution is legitimate (per whatever
-- machine/guard governs that specific contest) before calling this --
-- this function itself does no verification, it only applies an
-- already-legitimate outcome.
resolveContested :: (Text, Text) -> Value -> WorldSnapshot -> WorldSnapshot
resolveContested key value snap =
  snap
    { snapshotContested = Map.delete key (snapshotContested snap)
    , snapshotFacts = Map.insert key value (snapshotFacts snap)
    }

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

-- | Second pass, run after 'applyCommits': finds every @Contest@-typed
-- node (minted by @dmml-hs/app/CheckDivergence.hs@'s divergence-to-
-- content step) and enforces it. A contest whose recorded state is
-- still @"contested"@ overrides whatever the naive fold above put in
-- 'snapshotFacts' for its disputed pair -- moves it to
-- 'snapshotContested' instead, per SPEC.md §12: never silently resolve
-- to whichever value happened to be applied last.
--
-- A contest recorded as @"resolved"@ is trusted ONLY if the contest
-- node itself carries a real @witnessedBy npc/keeper@ fact -- this is a
-- hand-simulation of the ONE guard the minted contest machine actually
-- has (@self \`witnessedBy\` npc\/keeper@), not a general machine-guard
-- evaluator (dmml-hs doesn't have one yet -- see written-world/dev-
-- journal/2026-08-31-dmml-runtime-migration-scope.md's Phase 2). An
-- unwitnessed "resolved" claim is not honored -- the pair stays
-- contested with its original options, the same as if no resolution
-- had been attempted at all.
applyContests :: WorldSnapshot -> WorldSnapshot
applyContests snap = foldl' processContest snap contestNodes
  where
    facts = snapshotFacts snap

    contestNodes =
      [ subj
      | ((subj, p), ValueNode ty) <- Map.toList facts
      , p == "a"
      , nodeRefSegments ty == ["Contest"]
      ]

    getStr key = case Map.lookup key facts of
      Just (ValueLiteral (LitString s)) -> Just s
      _ -> Nothing

    -- | For fields that are guard-relevant (subject/predicate/state) --
    -- these are minted as bare node refs, never quoted string literals,
    -- so a guard's EXISTS walk can actually traverse them (see
    -- DMML.Guard / dev-journal/2026-09-02-machines-as-facts-generic-
    -- guard-evaluator.md's Opus-review finding: the real Rust crate's
    -- crepe loader refuses to walk a literal-valued fact at all, and
    -- Jason's own call was "eliminate string literals for guards --
    -- only symbols"). Only 'optionNSource' provenance labels stay
    -- 'getStr' -- free-text attribution, never guard-checked.
    getNode key = case Map.lookup key facts of
      Just (ValueNode n) -> Just (nodeRefText n)
      _ -> Nothing

    isWitnessedByKeeper contestNode =
      Map.lookup (contestNode, "witnessedBy") facts == Just (ValueNode (NodeRef ["npc", "keeper"]))

    collectOptions contestNode =
      [ (label, v)
      | i <- [1 .. 10 :: Int] -- small fixed bound -- good enough for a spike, not a general list encoding
      , Just v <- [Map.lookup (contestNode, "option" <> T.pack (show i) <> "Value") facts]
      , Just label <- [getStr (contestNode, "option" <> T.pack (show i) <> "Source")]
      ]

    processContest s contestNode =
      case (getNode (contestNode, "subject"), getNode (contestNode, "predicate"), getNode (contestNode, "state")) of
        (Just _, Just _, Just "resolved") | isWitnessedByKeeper contestNode ->
          s -- legitimately witnessed resolution: trust the naive fold's own value for this pair
        (Just origSubj, Just origPred, _) ->
          -- still contested, or an unwitnessed "resolved" claim -- either way, not settled
          markContested (origSubj, origPred) (collectOptions contestNode) s
        _ -> s

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
-- Every contested pair is surfaced explicitly, with every live option
-- and its provenance -- never collapsed to a single value, never
-- silently omitted. An agent (or a player) reading this snapshot sees
-- the dispute exist, the same way "corruption as content" means a
-- player finds out about drift instead of it being quietly resolved
-- out of sight.
renderSnapshot :: WorldSnapshot -> Text
renderSnapshot snap =
  T.unlines $
    ["Declared predicates:"]
      ++ ["  " <> kindWord k <> " " <> name | (name, k) <- sortOn fst (Map.toList (snapshotDeclared snap))]
      ++ ["", "Current facts:"]
      ++ [ "  " <> subj <> " . " <> pred_ <> " = " <> renderValue v
         | ((subj, pred_), v) <- sortOn fst (Map.toList (snapshotFacts snap))
         ]
      ++ contestedLines
  where
    contestedLines
      | Map.null (snapshotContested snap) = []
      | otherwise =
          ["", "CONTESTED -- not settled, both/all options are live:"]
            ++ concat
              [ ("  " <> subj <> " . " <> pred_ <> " is disputed:")
                  : ["    - " <> label <> " asserts " <> renderValue v | (label, v) <- contestedOptions entry]
              | ((subj, pred_), entry) <- sortOn fst (Map.toList (snapshotContested snap))
              ]
