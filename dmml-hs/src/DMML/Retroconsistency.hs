{-# LANGUAGE OverloadedStrings #-}

-- | Retroconsistency: given a transition's guards, what real facts
-- would need to exist for it to have legitimately already fired — and
-- are they actually there? Jason, 2026-09-02: "if a forest is
-- depleted, there must be someone who depleted it... if these facts
-- aren't in the graph, we should be able to fill them in in a commit
-- as long as it's consistent."
--
-- DMML-first check (this repo's own standing rule,
-- @written-world/CLAUDE.md@'s "DMML first"): this needs NO new grammar
-- or interpreter primitive. A guard's @EXISTS(pattern)@ already says,
-- precisely and structurally, exactly what must be true — retro-
-- consistency is a different ALGORITHM over that same, already-real
-- data ('DMML.Ast.Pattern'\/'PatternHop'\/'PatternTerm'), not new
-- content-authoring surface. Concretely: 'DMML.Guard.evalExists' walks
-- a pattern FORWARD (do real facts satisfy it?) via 'DMML.Guard.stepHop'
-- (private there); this module walks the SAME pattern shape the same
-- direction, but where a hop finds no matching fact, it SYNTHESIZES one
-- instead of failing — the walk itself, not the grammar, is what's new.
--
-- Deliberately scoped to what's actually determinate, not guessed at:
--
--   * A guard whose pattern anchor is unbound (@?var@) is refused, not
--     silently picked for — "some existing node should retroactively
--     gain this fact" has no principled answer without inventing one,
--     and this project has already rejected silently-resolve moves
--     like that once (the old reactive-Contest-minting mechanism,
--     jedelman/dmml#1).
--   * A negated guard that's currently BLOCKED (the forbidden pattern
--     genuinely exists) is refused — minting more facts can only make
--     an EXISTS pattern MORE likely to hold, never retract one, so
--     there is no fact this module could add to fix that; a real
--     modeling problem, not something to paper over.
--   * A multi-hop pattern IS handled (the whole chain gets synthesized,
--     each hop building on the last) — this covers a single machine's
--     own guard chain, but NOT chaining across separate machines\/
--     relations (Jason's quarry example: the stone's own destination is
--     a different fact on a different node's own governance, not a hop
--     in the quarry's own pattern). That's a real, disclosed follow-up:
--     run this module again, recursively, on whatever machine (if any)
--     governs each freshly-implied node, until nothing new is implied.
--     Not built here — see @written-world@'s own dev-journal entry for
--     this feature for why that's a fixpoint over this primitive, not a
--     new primitive itself.
--
-- Consistency, precisely: at the DATA level, minting is ALWAYS safe by
-- construction here — 'DMML.Materialize.applyCommit' is collision-free
-- (every independent assert just adds an alternative, nothing is ever
-- overwritten), so an implied fact can never structurally conflict with
-- anything already asserted. What this module cannot check is
-- DOMAIN/narrative plausibility (is "npc/whoever" a SENSIBLE actor to
-- have depleted this specific forest) — same boundary this project
-- already draws elsewhere (deterministic code decides WHAT'S missing
-- precisely; an authoring agent decides WHO, using that precise gap as
-- its prompt, never inventing the gap itself).
module DMML.Retroconsistency
  ( ImpliedFact (..)
  , RetroResult (..)
  , retroconsistency
  , renderImpliedCommit
  ) where

import Data.List (foldl', nub)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast
import DMML.Guard (EvalContext (..), evalGuard, lookupTransition)
import DMML.Materialize (WorldSnapshot, currentValue)

-- | One fact retroconsistency proposes minting to make a guard hold.
-- Always node-valued -- a guard pattern only ever walks node-valued
-- facts to begin with (literal-valued facts never participate in a
-- walk, see 'DMML.Guard''s own doc comment), so nothing this module
-- synthesizes could be a literal either.
data ImpliedFact = ImpliedFact
  { impliedSubject :: Text
  , impliedPredicate :: Text
  , impliedTarget :: Text
  }
  deriving (Eq, Show)

data RetroResult
  = -- | Every guard already holds -- nothing to imply.
    AlreadyConsistent
  | -- | These facts, minted, would make every currently-unsatisfied
    -- guard hold. Order matters when a pattern has more than one hop:
    -- each later fact was synthesized assuming the earlier ones in the
    -- SAME list already exist.
    Implied [ImpliedFact]
  | -- | A guard can't be reconciled by minting anything -- see this
    -- module's own doc comment for the two real cases (unbound anchor,
    -- blocked negation).
    Irreconcilable Text
  deriving (Eq, Show)

resolveTerm :: PatternTerm -> EvalContext -> Maybe Text
resolveTerm TermSelf ctx = Just (ctxSelfNode ctx)
resolveTerm (TermParam name) ctx = Map.lookup name (ctxParams ctx)
resolveTerm (TermVar _) _ = Nothing
resolveTerm (TermNode n) _ = Just n

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

-- | Every node-valued target currently live for one (subject,
-- predicate) pair -- fans out over every live alternative, same as
-- 'DMML.Guard.factsForPredicate' does, since an ungoverned or still-
-- disputed pair is still real, current, matchable fact content.
targetsFor :: Text -> Text -> WorldSnapshot -> [Text]
targetsFor subj pred_ snap =
  [nodeRefText n | (_label, ValueNode n) <- currentValue (subj, pred_) snap]

-- | A deterministic, meaningful fresh node for an unconstrained
-- existential hop -- content-addressed on (subject, predicate) rather
-- than a counter, so two independent retroconsistency runs against the
-- same unresolved pair converge on the SAME proposed node instead of
-- minting two different placeholders for what's really one gap. No
-- hyphens (node ref segments can't contain them, confirmed the hard
-- way earlier this session -- EntropySidecar.hs's own node-naming hit
-- this first).
freshNodeFor :: Text -> Text -> Text
freshNodeFor subj pred_ = "retro/" <> T.map slashToUnderscore subj <> "_" <> pred_
  where
    slashToUnderscore c = if c == '/' then '_' else c

-- | Walks one guard's hop chain from an already-resolved anchor,
-- synthesizing an 'ImpliedFact' for the first hop with no matching real
-- fact and continuing the walk from whatever that hop's (real or
-- freshly-synthesized) target is. Terminates for the same reason
-- 'DMML.Guard.evalExists' does -- a finite hop list, one step per hop.
walkAndImply :: EvalContext -> WorldSnapshot -> Text -> [PatternHop] -> [ImpliedFact]
walkAndImply _ctx _snap _current [] = []
walkAndImply ctx snap current (hop : rest) =
  case [t | t <- targetsFor current (hopPredicate hop) snap, maybe True (== t) boundTarget] of
    (existing : _) -> walkAndImply ctx snap existing rest
    [] ->
      let target = maybe (freshNodeFor current (hopPredicate hop)) id boundTarget
       in ImpliedFact current (hopPredicate hop) target : walkAndImply ctx snap target rest
  where
    boundTarget = resolveTerm (hopTerm hop) ctx

-- | The whole answer for one named transition: walks every AUTHOR-
-- WRITTEN guard (deliberately 'transitionGuards', NOT
-- 'resolveTransition''s own resolved list) -- short-circuiting to
-- 'Irreconcilable' the moment one can't be fixed by minting, otherwise
-- accumulating every implied fact across every currently-unsatisfied
-- guard. 'Nothing' if no such transition is declared -- same "declared
-- vs. not" distinction 'DMML.Guard.mayFire' already draws, for the same
-- reason.
--
-- REAL BUG, found by actually running the forest-depletion example,
-- not by reasoning alone: 'resolveTransition' prepends an IMPLICIT
-- @(self, "state", from)@ guard for a @from -> to@ transition (see its
-- own doc comment) -- this module's first version used that resolved
-- list and, for a forest already asserted @depleted@, dutifully tried
-- to imply the missing @pristine@ state too, since that guard wasn't
-- currently satisfied EITHER. That's backwards, not just noisy: once a
-- machine has moved to its target state, the FROM state must NOT
-- presently hold -- that's what makes it the state being transitioned
-- OUT of, not a real domain precondition retroconsistency could ever
-- legitimately backfill without contradicting the very fact already
-- asserted. The implicit from-state check answers "can this fire
-- RIGHT NOW," a present-tense question this module was never asking;
-- only a transition's own author-written guards (real domain
-- preconditions, like "someone harvested this") are the kind of thing
-- retroconsistency exists to imply.
retroconsistency :: MachineStmt -> Text -> EvalContext -> WorldSnapshot -> Maybe RetroResult
retroconsistency machine ident ctx snap = do
  decl <- lookupTransition machine ident
  let guards = transitionGuards decl
  pure (foldl' step AlreadyConsistent guards)
  where
    step (Irreconcilable msg) _ = Irreconcilable msg
    step acc g
      | evalGuard g ctx snap = acc -- already holds, positive or negated alike -- nothing to imply
      | guardNegated g =
          Irreconcilable
            "negated guard is blocked by a real, currently-asserted fact -- retroconsistency can only add facts, never retract one"
      | otherwise = case resolveTerm (patternAnchor pat) ctx of
          Nothing ->
            Irreconcilable
              "guard's pattern anchor is unbound (?var) -- which existing node should retroactively gain this fact is genuinely underdetermined, not guessed at"
          Just anchorText -> combine acc (Implied (walkAndImply ctx snap anchorText (patternHops pat)))
      where
        pat = existsPattern (guardExists g)
    combine AlreadyConsistent r = r
    combine (Implied xs) (Implied ys) = Implied (xs ++ ys)
    combine (Implied xs) AlreadyConsistent = Implied xs
    combine (Irreconcilable m) _ = Irreconcilable m
    combine _ (Irreconcilable m) = Irreconcilable m

-- | Renders a list of implied facts as a real, parseable DMML Surface
-- commit -- "fill them in in a commit," literally. Declares every
-- predicate it uses (harmless if already declared elsewhere, same
-- idiom 'EntropySidecar.hs''s own @mintAlert@ already uses) since this
-- module has no way to know what the target repo has already declared.
renderImpliedCommit :: Text -> [ImpliedFact] -> Text
renderImpliedCommit verb facts =
  T.unlines $
    ["commit " <> verb]
      ++ ["  declare relation " <> p | p <- nub (map impliedPredicate facts)]
      ++ ["  " <> impliedSubject f <> " `" <> impliedPredicate f <> "` " <> impliedTarget f | f <- facts]
