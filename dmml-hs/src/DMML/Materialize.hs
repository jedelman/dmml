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
  , alternativeValues
  , IdentifiedCommit (..)
  , emptySnapshot
  , applyCommit
  , applyCommits
  , applyIdentifiedCommit
  , applyIdentifiedCommits
  , mergeSnapshots
  , currentValue
  , currentValueWithProvenance
  , collapseToOne
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
-- caller, this module has no notion of branches), and, since 2026-09-04
-- (Phase 3's retract-provenance fix, jedelman/dmml#4), an optional real
-- 'StrongRef' -- present only for a fact materialized via
-- 'applyIdentifiedCommit', 'Nothing' for one materialized the ordinary
-- 'applyCommit' way (unchanged, still the label-only, no-real-provenance
-- path every existing call site uses). Deduped on VALUE only: two
-- different provenances independently asserting the SAME value is
-- agreement, not divergence, and shouldn't accumulate as if it were two
-- live options (a real false-positive this exact confusion caused before
-- a similar check existed in CheckDivergence.hs's own overlap filter).
newtype Alternatives = Alternatives {alternativeEntries :: [(Text, Maybe StrongRef, Value)]}
  deriving (Eq, Show)

-- | Backward-compatible view dropping the (label, StrongRef?) provenance
-- down to just (label, value) -- every pre-2026-09-04 call site
-- (renderSnapshot, DMML.Guard, DMML.Governance, DMML.Retroconsistency,
-- every app/*Demo.hs) reads 'Alternatives' through this and needs no
-- change at all.
alternativeValues :: Alternatives -> [(Text, Value)]
alternativeValues (Alternatives xs) = [(label, v) | (label, _ref, v) <- xs]

addAlternative :: Text -> Maybe StrongRef -> Value -> Alternatives -> Alternatives
addAlternative label ref v (Alternatives existing)
  | any (\(_, _, v') -> v' == v) existing = Alternatives existing
  | otherwise = Alternatives (existing ++ [(label, ref, v)])

-- | Merging two independently-materialized sides never invents
-- provenance for a side that didn't have any -- whatever 'Maybe
-- StrongRef' each side's own alternative already carried survives
-- unchanged.
mergeAlternatives :: Alternatives -> Alternatives -> Alternatives
mergeAlternatives (Alternatives a) (Alternatives b) =
  foldl' (\acc (label, ref, v) -> addAlternative label ref v acc) (Alternatives a) b

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
-- with @label@ and, when there is one, a real @ref@ -- shared by
-- 'applyCommit' (always @Nothing@, unchanged behavior) and
-- 'applyIdentifiedCommit' (always @Just@ a real 'StrongRef'). A second
-- independent assert for a pair already seen ADDS an alternative via
-- 'addAlternative' -- it never overwrites.
--
-- FIXED 2026-09-04: applies every 'ItemConsumes' block BEFORE any
-- 'ItemFact' in the commit, regardless of their order in
-- 'commitItems' (which is always @declares ++ facts ++ consumes@, per
-- 'DMML.FromJson.commitStmtFromInput' -- fixed AST-construction order,
-- not source order). The old single left-to-right fold over
-- 'commitItems' applied facts, THEN consumes -- which meant a commit
-- that both asserts a NEW value and retracts the OLD value for the
-- SAME (subject, predicate) key (exactly what a real @from -> to@
-- transition needs: @assert awakened@ + @retract stirring@, both on
-- @self . state@) had its own freshly-asserted fact silently deleted
-- by its own consumes block, leaving that key with NO live value at
-- all. Real bug, caught by firing 'DMML.Fire''s own real retract path
-- for the first time against a genuinely two-effect transition (see
-- dev-journal/2026-09-04-retract-provenance-fix.md's own now-corrected
-- verification) -- not a hypothetical. @written-world/SPEC.md@ already
-- names the intended ordering explicitly: "ordering derives from
-- consumes -> produces" -- consumes is a precondition-retraction step,
-- produces adds new facts on top of it, never the reverse.
applyCommitWithRef :: Text -> Maybe StrongRef -> WorldSnapshot -> CommitStmt -> WorldSnapshot
applyCommitWithRef label ref snap stmt =
  foldl' applyFactItem (foldl' applyConsumeItem afterDeclares (commitItems stmt)) (commitItems stmt)
  where
    afterDeclares = foldl' applyDeclareItem snap (commitItems stmt)

    applyDeclareItem s (ItemDeclare d) =
      s {snapshotDeclared = Map.insert (declareIdent d) (declareKind d) (snapshotDeclared s)}
    applyDeclareItem s _ = s

    applyConsumeItem s (ItemConsumes cb) = foldl' applyConsume s (consumesEntries cb)
    applyConsumeItem s _ = s

    -- UPDATED 2026-09-04: now honors 'factConsumeObject' instead of
    -- ignoring it. 'Nothing' keeps the documented wildcard semantics
    -- (every live alternative for the key is removed); 'Just v' removes
    -- ONLY the one alternative whose value equals @v@, leaving every
    -- OTHER live alternative for that key untouched. This is what makes
    -- a value-qualified retract (@retract $target \`wardedBy\` self@,
    -- 'DMML.Ast.Effect'\'s optional retract value) actually
    -- discriminate which alternative gets consumed, rather than being
    -- carried through for display only -- see 'DMML.Fire.
    -- resolveSingleRetract''s matching 2026-09-04 update, which is what
    -- this was built for.
    applyConsume s (ConsumeFact fc) =
      let key = (nodeRefText (factConsumeSubject fc), factConsumePredicate fc)
       in case factConsumeObject fc of
            Nothing -> s {snapshotFacts = Map.delete key (snapshotFacts s)}
            Just v ->
              s
                { snapshotFacts =
                    Map.update
                      ( \(Alternatives xs) -> case filter (\(_, _, v') -> v' /= v) xs of
                          [] -> Nothing
                          remaining -> Just (Alternatives remaining)
                      )
                      key
                      (snapshotFacts s)
                }
    applyConsume s (ConsumeStrong _) = s

    applyFactItem s (ItemFact f) =
      let key = (nodeRefText (factSubject f), predText (factPredicate f))
       in s
            { snapshotFacts =
                Map.insertWith
                  (\_new old -> addAlternative label ref (factValue f) old)
                  key
                  (Alternatives [(label, ref, factValue f)])
                  (snapshotFacts s)
            }
    applyFactItem s _ = s

-- | Applies one already-validated commit, tagging every fact it asserts
-- with @label@ (the whole materialization batch's provenance -- see
-- 'applyCommits'). Never carries real @uri#cid@ provenance -- see
-- 'applyIdentifiedCommit' for the path that does.
applyCommit :: Text -> WorldSnapshot -> CommitStmt -> WorldSnapshot
applyCommit label = applyCommitWithRef label Nothing

-- | Materializes one batch of commits under a single provenance label --
-- e.g. one player's/peer's own new commits since a shared merge-base.
-- To combine two independently-labeled batches (the actual divergence-
-- detection case 'CheckDivergence.hs' needs), materialize each side
-- separately then 'mergeSnapshots' them -- never pass a mixed list here
-- expecting per-commit provenance, there isn't any.
applyCommits :: Text -> [CommitStmt] -> WorldSnapshot
applyCommits label = foldl' (applyCommit label) emptySnapshot

-- | One already-validated commit paired with the real 'StrongRef'
-- (@uri#cid@) identifying it -- mirrors the real Rust crate's own
-- @IdentifiedCommit { uri, cid, commit }@ (@dmml::interpret@), the same
-- shape it already uses when a caller DOES have real commit provenance
-- to hand. @dmml-hs@ has no substrate of its own to source one from
-- (no atproto, no git-object identity computed here) -- the caller
-- supplies it, however it got one (a real atproto CID, a git blob hash,
-- 'DMML.LocalIdentity''s file-content fingerprint, ...).
data IdentifiedCommit = IdentifiedCommit
  { icRef :: StrongRef
  , icCommit :: CommitStmt
  }
  deriving (Eq, Show)

-- | Like 'applyCommit', but tags every fact this commit asserts with its
-- own real 'StrongRef' too -- this is what lets 'DMML.Fire' later build
-- a real @consumes@ block instead of refusing to fire a retract effect
-- at all (jedelman/dmml#4).
applyIdentifiedCommit :: Text -> IdentifiedCommit -> WorldSnapshot -> WorldSnapshot
applyIdentifiedCommit label ic snap = applyCommitWithRef label (Just (icRef ic)) snap (icCommit ic)

-- | 'applyCommits''s identified-provenance counterpart.
applyIdentifiedCommits :: Text -> [IdentifiedCommit] -> WorldSnapshot
applyIdentifiedCommits label = foldl' (flip (applyIdentifiedCommit label)) emptySnapshot

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

-- | Like 'currentValue', but keeps whichever real 'StrongRef' provenance
-- is on record for each live alternative (only ever present for a fact
-- materialized via 'applyIdentifiedCommit' -- 'Nothing' for the ordinary,
-- label-only 'applyCommit' path). 'DMML.Fire' needs this to build a real
-- @consumes@ block when resolving a retract effect instead of
-- fabricating a citation.
currentValueWithProvenance :: (Text, Text) -> WorldSnapshot -> [(Text, Maybe StrongRef, Value)]
currentValueWithProvenance key snap = maybe [] alternativeEntries (Map.lookup key (snapshotFacts snap))

-- | Deliberately collapses a (subject, predicate) pair's live
-- alternatives down to one -- the ONE write operation in this module
-- that overwrites rather than adds. Never called by ordinary commit
-- application ('applyCommit') -- an ordinary mint is collision-free by
-- construction and never collapses anything on its own. This exists
-- specifically for governed-machine arbitration (see DMML.Governance's
-- 'arbitrate') to apply an already-validated outcome; it does no
-- validation itself, same division of responsibility as the old
-- 'resolveContested' this replaces. Always collapses to a provenance-
-- free (@Nothing@) alternative: arbitration reduces to a computed
-- OUTCOME, not to any one input alternative's own citation, so there is
-- no single real 'StrongRef' that would honestly describe the collapsed
-- result's own provenance.
collapseToOne :: (Text, Text) -> Text -> Value -> WorldSnapshot -> WorldSnapshot
collapseToOne key label v snap =
  snap {snapshotFacts = Map.insert key (Alternatives [(label, Nothing, v)]) (snapshotFacts snap)}

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
