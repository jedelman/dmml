{-# LANGUAGE OverloadedStrings #-}

-- | A generic @EXISTS(pattern)@ guard evaluator and transition-firing
-- check, faithful to the real production crate's semantics
-- (@dmml/src/machine.rs@'s @eval_exists@/@eval_guard@/@eval_guards@/
-- @resolve_transition@/@may_fire@, @dmml/src/datalog_guard.rs@'s
-- documented quirks) but operating over 'DMML.Materialize.WorldSnapshot'
-- rather than the real crate's oxigraph-backed @Materialized@. Design
-- reviewed twice by an independent pass before being built (see
-- written-world/dev-journal/2026-09-02-machines-as-facts-generic-guard-
-- evaluator.md); both real blockers that review found are already
-- resolved upstream of this module:
--
--   * literal-valued facts never participate in a walk (the real
--     crate's crepe loader refuses to walk one at all -- confirmed via
--     its own @non_node_value_fails_the_walk@ test) -- 'objectAsNodeText'
--     is partial over 'ValueNode' for exactly this reason, and since
--     Phase A's literal-elimination convention nothing guard-relevant is
--     minted as a literal anymore anyway (see @CheckDivergence.hs@'s
--     mint site and @Materialize.hs@'s own doc comment).
--   * the multi-valued 'DMML.Materialize.Alternatives' fact store
--     (collision-free mints, same day) means 'factsForPredicate' fans
--     out over EVERY live alternative for a (subject, predicate) pair,
--     not just one -- an ungoverned, still-disputed fact is not a
--     special case to exclude from a walk, it's simply more candidates
--     to try, the same way an unbound @?var@ already fans out.
--
-- This module answers "is this transition legal to fire right now,"
-- nothing more -- it does not itself apply any effect. Reducing several
-- live alternatives to one canonical value (governed-machine arbitration)
-- is the caller's job (see 'mayFire'\'s returned 'Effect' list), not
-- this module's.
module DMML.Guard
  ( EvalContext (..)
  , resolveTerm
  , evalExists
  , evalGuard
  , evalGuards
  , resolveTransition
  , lookupTransition
  , mayFire
  ) where

import Data.List (find, foldl', nub)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast
import DMML.Materialize (Alternatives (..), WorldSnapshot (..))

-- | Which node is @self@, and current transition-parameter (@$param@)
-- bindings for this evaluation.
data EvalContext = EvalContext
  { ctxSelfNode :: Text
  , ctxParams :: Map Text Text
  }

-- | Resolves a 'PatternTerm' to a concrete node string, or 'Nothing' if
-- it's existentially open (a @?var@, or a @$param@ 'EvalContext' has no
-- binding for). @?var@ NEVER binds, at any position, including a second
-- occurrence of the same name within one pattern -- deliberate, per
-- @MACHINE_SPEC.md@'s "Multi-hop patterns and \`?vars\`": a real
-- unification variable is what @$param@ is for.
resolveTerm :: PatternTerm -> EvalContext -> Maybe Text
resolveTerm TermSelf ctx = Just (ctxSelfNode ctx)
resolveTerm (TermParam name) ctx = Map.lookup name (ctxParams ctx)
resolveTerm (TermVar _) _ = Nothing
resolveTerm (TermNode n) _ = Just n

nodeRefText :: NodeRef -> Text
nodeRefText = T.intercalate "/" . nodeRefSegments

-- | Only a node-valued fact can be walked through -- see this module's
-- own doc comment for why.
objectAsNodeText :: Value -> Maybe Text
objectAsNodeText (ValueNode n) = Just (nodeRefText n)
objectAsNodeText (ValueLiteral _) = Nothing

-- | Every (subject, object-as-text) pair currently live for one
-- predicate, fanned out over every alternative each pair holds (the
-- fact store is multi-valued -- see this module's own doc comment).
factsForPredicate :: Text -> WorldSnapshot -> [(Text, Text)]
factsForPredicate p snap =
  [ (subj, objText)
  | ((subj, p'), alts) <- Map.toList (snapshotFacts snap)
  , p' == p
  , (_label, v) <- alternativeValues alts
  , Just objText <- [objectAsNodeText v]
  ]

-- | Walks one hop from a set of current candidate node-texts. An
-- existential hop (a @?var@, or an unbound @$param@) fans out to every
-- matching object; a bound hop (@self@, a bound @$param@, or a literal
-- node ident) filters to exactly that value. Mirrors
-- @machine::walk_pattern@ exactly.
stepHop :: EvalContext -> WorldSnapshot -> [Text] -> PatternHop -> [Text]
stepHop ctx snap currentNodes hop =
  [ objText
  | subj <- currentNodes
  , (factSubj, objText) <- factsForPredicate (hopPredicate hop) snap
  , factSubj == subj
  , maybe True (== objText) (resolveTerm (hopTerm hop) ctx)
  ]

-- | @EXISTS(pattern)@: true iff at least one walk from the anchor
-- through every hop, in order, lands on a real, currently-asserted
-- chain of facts. An unbound anchor (@?var@) starts from every node
-- that's ever been a fact subject; a bound anchor starts from exactly
-- that one node. Terminates for any well-formed pattern: the fold is
-- over a finite hop list, and each step's candidate set is bounded by
-- the (finite) fact store -- no cycle risk even against a cyclic graph.
evalExists :: Pattern -> EvalContext -> WorldSnapshot -> Bool
evalExists pattern ctx snap =
  not (null (foldl' (stepHop ctx snap) startNodes (patternHops pattern)))
  where
    startNodes = case resolveTerm (patternAnchor pattern) ctx of
      Just n -> [n]
      Nothing -> nub [subj | (subj, _pred) <- Map.keys (snapshotFacts snap)]

-- | One 'GuardClause': its @EXISTS@ result, XORed with @negated@.
evalGuard :: GuardClause -> EvalContext -> WorldSnapshot -> Bool
evalGuard g ctx snap = evalExists (existsPattern (guardExists g)) ctx snap /= guardNegated g

-- | A full guard list -- plain conjunction: every guard must hold.
evalGuards :: [GuardClause] -> EvalContext -> WorldSnapshot -> Bool
evalGuards gs ctx snap = all (\g -> evalGuard g ctx snap) gs

-- | Resolves a 'TransitionDecl': prepends the implicit
-- @(self, state, from)@ guard a @from -> to@ transition sugars into.
-- No-op when 'transitionFrom' is 'Nothing'.
resolveTransition :: TransitionDecl -> ([GuardClause], [Effect])
resolveTransition decl =
  (maybe (transitionGuards decl) (: transitionGuards decl) implicitGuard, transitionEffects decl)
  where
    implicitGuard = do
      fromState <- transitionFrom decl
      pure
        GuardClause
          { guardNegated = False
          , guardExists =
              ExistsExpr
                { existsPattern =
                    Pattern
                      { patternAnchor = TermSelf
                      , patternHops = [PatternHop {hopPredicate = "state", hopTerm = TermNode fromState}]
                      }
                , existsSpan = transitionSpan decl
                }
          , guardSpan = transitionSpan decl
          }

-- | Finds one named transition on a machine, if declared.
lookupTransition :: MachineStmt -> Text -> Maybe TransitionDecl
lookupTransition machine ident = find ((== ident) . transitionIdent) (machineTransitions machine)

-- | Whether @ident@'s transition may fire right now, given @ctx@ and
-- @snap@. 'Nothing' if no such transition is declared on this machine --
-- distinct from @Just (False, _, _)@ ("declared, but blocked"). On
-- success, also returns the transition's own effects and target state
-- (@transitionTo@) -- the caller needs these to actually apply the
-- firing; re-deriving them via a second 'lookupTransition' call would be
-- redundant.
mayFire :: MachineStmt -> Text -> EvalContext -> WorldSnapshot -> Maybe (Bool, [Effect], Maybe Text)
mayFire machine ident ctx snap = do
  decl <- lookupTransition machine ident
  let (guards, effects) = resolveTransition decl
  pure (evalGuards guards ctx snap, effects, transitionTo decl)
