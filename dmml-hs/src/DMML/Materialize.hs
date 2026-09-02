{-# LANGUAGE OverloadedStrings #-}

-- | A minimal materializer: applies a sequence of already-validated
-- 'CommitStmt's and produces a queryable snapshot of world state. This
-- is deliberately NOT a port of the real crate's @interpret::Materialized@
-- (no real cid/uri-based provenance, no @reachable_from@ graph-scoping,
-- @consumes@ is interpreted operationally). Built for one purpose --
-- rendering "the world so far" as agent-readable context.
--
-- REWORKED 2026-09-02 per Jason's "collision-free mints" redesign
-- (written-world/dev-journal/2026-09-02-machines-as-facts-generic-guard-
-- evaluator.md and the same day's follow-on design conversation): a
-- (subject, predicate) pair no longer holds one "current" value that a
-- second independent assert either overwrites or gets specially flagged
-- against. It holds every independently-asserted 'Alternatives' value,
-- always -- mints are collision-free at the data level, by construction.
-- There is no more @Contest@/@ContestedEntry@/@markContested@ special
-- case: a pair with one live alternative and a pair with several are the
-- same kind of thing, just a different count. Reducing many alternatives
-- to one canonical value is never this module's job -- that's governed-
-- machine arbitration (not yet built, see DMML.Guard and the tracking
-- issue jedelman/dmml#1) or, for an ungoverned predicate, simply doesn't
-- happen: per SPEC.md §12 and Jason's own framing ("piles of alternatives
-- are just a new territory to deterritorialize"), staying multi-valued
-- indefinitely is correct behavior, not a defect to cap.
module DMML.Materialize
  ( WorldSnapshot (..)
  , Alternatives (..)
  , emptySnapshot
  , applyCommit
  , applyCommits
  , mergeSnapshots
  , currentValue
  , renderSnapshot
  ) where

import Data.List (foldl', sortOn)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast

-- | Every value ever independently asserted for one (subject, predicate)
-- pair, each tagged with the provenance label of whichever materialization
-- batch asserted it (typically a branch or agent name -- supplied by the
-- caller, this module has no notion of branches). Deduped on VALUE only:
-- two different provenances independently asserting the SAME value is
-- agreement, not divergence, and shouldn't accumulate as if it were two
-- live options (a real false-positive this exact confusion caused before
-- a similar check existed in CheckDivergence.hs's own overlap filter).
newtype Alternatives = Alternatives {alternativeValues :: [(Text, Value)]}
  deriving (Eq, Show)

addAlternative :: Text -> Value -> Alternatives -> Alternatives
addAlternative label v (Alternatives existing)
  | any ((== v) . snd) existing = Alternatives existing
  | otherwise = Alternatives (existing ++ [(label, v)])

mergeAlternatives :: Alternatives -> Alternatives -> Alternatives
mergeAlternatives (Alternatives a) (Alternatives b) =
  foldl' (\acc (label, v) -> addAlternative label v acc) (Alternatives a) b

data WorldSnapshot = WorldSnapshot
  { snapshotDeclared :: Map Text DeclKind
  , snapshotFacts :: Map (Text, Text) Alternatives
  }
  deriving (Eq, Show)

emptySnapshot :: WorldSnapshot
emptySnapshot = WorldSnapshot Map.empty Map.empty

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

predText :: PredicateRef -> Text
predText RdfType = "a"
predText (PredIdent t) = t

-- | Applies one already-validated commit, tagging every fact it asserts
-- with @label@ (the whole materialization batch's provenance -- see
-- 'applyCommits'). A second independent assert for a pair already seen
-- ADDS an alternative via 'addAlternative' -- it never overwrites.
applyCommit :: Text -> WorldSnapshot -> CommitStmt -> WorldSnapshot
applyCommit label snap stmt = foldl' applyItem snap (commitItems stmt)
  where
    applyItem s (ItemDeclare d) =
      s {snapshotDeclared = Map.insert (declareIdent d) (declareKind d) (snapshotDeclared s)}
    applyItem s (ItemFact f) =
      let key = (nodeRefText (factSubject f), predText (factPredicate f))
       in s
            { snapshotFacts =
                Map.insertWith
                  (\_new old -> addAlternative label (factValue f) old)
                  key
                  (Alternatives [(label, factValue f)])
                  (snapshotFacts s)
            }
    applyItem s (ItemConsumes cb) = foldl' applyConsume s (consumesEntries cb)

    applyConsume s (ConsumeFact fc) =
      let key = (nodeRefText (factConsumeSubject fc), factConsumePredicate fc)
       in s {snapshotFacts = Map.delete key (snapshotFacts s)}
    applyConsume s (ConsumeStrong _) = s

-- | Materializes one batch of commits under a single provenance label --
-- e.g. one player's/peer's own new commits since a shared merge-base.
-- To combine two independently-labeled batches (the actual divergence-
-- detection case 'CheckDivergence.hs' needs), materialize each side
-- separately then 'mergeSnapshots' them -- never pass a mixed list here
-- expecting per-commit provenance, there isn't any.
applyCommits :: Text -> [CommitStmt] -> WorldSnapshot
applyCommits label = foldl' (applyCommit label) emptySnapshot

-- | Unions two independently-materialized snapshots. Every
-- (subject, predicate) pair the two sides agree on stays single-valued;
-- every pair they genuinely diverge on ends up with every live
-- alternative from both sides, deduped on value. This is the ENTIRE
-- mechanism for surfacing a real cross-branch divergence now -- no
-- separate detection-then-mint step needed, the union itself IS the
-- surfaced content.
mergeSnapshots :: WorldSnapshot -> WorldSnapshot -> WorldSnapshot
mergeSnapshots a b =
  WorldSnapshot
    { snapshotDeclared = Map.union (snapshotDeclared a) (snapshotDeclared b)
    , snapshotFacts = Map.unionWith mergeAlternatives (snapshotFacts a) (snapshotFacts b)
    }

-- | Every live alternative for one (subject, predicate) pair -- empty
-- (never asserted), one entry (agreed/uncontested), or several (live,
-- currently-unreduced divergence). The only way to read a fact's value.
currentValue :: (Text, Text) -> WorldSnapshot -> [(Text, Value)]
currentValue key snap = maybe [] alternativeValues (Map.lookup key (snapshotFacts snap))

renderValue :: Value -> Text
renderValue (ValueNode n) = nodeRefText n
renderValue (ValueLiteral (LitString s)) = "\"" <> s <> "\""
renderValue (ValueLiteral (LitNumber n)) = n
renderValue (ValueLiteral (LitBoolean b)) = if b then "true" else "false"

kindWord :: DeclKind -> Text
kindWord DeclRelation = "relation"
kindWord DeclAttribute = "attribute"

-- | Renders a snapshot as plain text. A pair with exactly one live
-- alternative renders as an ordinary fact; a pair with more than one
-- renders every option with its provenance -- same idiom either way,
-- no special "CONTESTED" framing anymore. Multiple live alternatives on
-- an ungoverned predicate isn't pathological (Jason, 2026-09-02: "piles
-- of alternatives are just a new territory to deterritorialize").
renderSnapshot :: WorldSnapshot -> Text
renderSnapshot snap =
  T.unlines $
    ["Declared predicates:"]
      ++ ["  " <> kindWord k <> " " <> name | (name, k) <- sortOn fst (Map.toList (snapshotDeclared snap))]
      ++ ["", "Facts:"]
      ++ concat
        [ renderPair subj pred_ (alternativeValues alts)
        | ((subj, pred_), alts) <- sortOn fst (Map.toList (snapshotFacts snap))
        ]
  where
    renderPair subj pred_ [(_, v)] = ["  " <> subj <> " . " <> pred_ <> " = " <> renderValue v]
    renderPair subj pred_ opts =
      ("  " <> subj <> " . " <> pred_ <> " -- " <> T.pack (show (length opts)) <> " live alternatives:")
        : ["    - " <> label <> " asserts " <> renderValue v | (label, v) <- opts]
