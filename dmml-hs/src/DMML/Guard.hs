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
  , GuardError (..)
  , evalGuardsBinding
  , binderNames
  , resolveTransition
  , lookupTransition
  , mayFire
  , availableTransitions
  ) where

import Data.List (find, foldl', nub)
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast
import DMML.Materialize (WorldSnapshot (..), alternativeValues)

-- | Which node is @self@, and current transition-parameter (@$param@)
-- bindings for this evaluation.
data EvalContext = EvalContext
  { ctxSelfNode :: Text
  , ctxParams :: Map Text Text
  , -- | Bindings a guard already produced, visible to later guards in
    -- the same transition and to every effect. Empty at the start of an
    -- evaluation; 'evalGuardsBinding' fills it in left to right.
    ctxBindings :: Map Text Text
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
-- A binder resolves from a guard that already bound it, and FAILING
-- THAT from the caller's own params. That fallback is what makes an
-- ambiguous refusal answerable: the engine refuses and names the
-- candidates, the chooser picks one, and the caller passes it back in as
-- an ordinary @--param@ that pre-binds the binder to exactly that
-- witness. Without it, "narrow it with a --param" would be advice the
-- engine did not actually honour.
resolveTerm (TermBind v) ctx =
  case Map.lookup v (ctxBindings ctx) of
    Just x -> Just x
    Nothing -> Map.lookup v (ctxParams ctx)
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

-- | "What can @ctx@'s @self@ legally do right now" -- every
-- @(machineNode, transitionIdent)@ pair, across the given machine set,
-- whose guards currently hold. Not a new interpreter capability: 'mayFire'
-- already answers this for one named transition; this is the SAME
-- "new evaluation mode" §19.2 already named as the shape sense-machines
-- need ("walk every transition on every declared machine and collect
-- the effects of every one whose guard currently holds... a forward
-- pass reusing eval_guards/resolve_transition unchanged, just called in
-- a loop instead of by ident") -- applied here to surfacing available
-- ACTIONS to a player rather than to materializing perceived facts,
-- same underlying primitive either way. Deliberately takes the machine
-- SET to scan as a plain argument rather than deciding for itself which
-- machines are "in scope" (e.g. via @equips@) -- that's a caller
-- policy (a player's own equipped machines? every machine on co-located
-- entities? both?), not something this generic evaluator should decide.
availableTransitions :: Map Text MachineStmt -> EvalContext -> WorldSnapshot -> [(Text, Text)]
availableTransitions machines ctx snap =
  [ (machineNodeText, transitionIdent decl)
  | (machineNodeText, machine) <- Map.toList machines
  , decl <- machineTransitions machine
  , Just (True, _, _) <- [mayFire machine (transitionIdent decl) ctx snap]
  ]

-- Guard binding ------------------------------------------------------------

-- | Why a guard could not produce a binding. Both cases are refusals,
-- not falsehoods: the guard might well hold, but firing on it would
-- require the engine to make a choice that is not its to make.
data GuardError
  = -- | A @?binder@ matched more than one witness, with the candidates.
    --
    -- Deliberately a refusal rather than "pick the first." Picking would
    -- be an arbitrary choice whose consequences are fully observable
    -- (the effects use the witness), and this project already has the
    -- precedent: 'DMML.Fire.FireRetractAmbiguous' refuses for exactly
    -- this reason -- there is no principled way to pick one of several
    -- without something to match against.
    --
    -- The refusal is meant to be PRODUCTIVE. It names every candidate,
    -- and a @$param@ in the same guard narrows it to one, so an
    -- ambiguous guard is a well-formed question for whoever is making
    -- decisions ("which rock do you take?") with the options already
    -- enumerated by the engine. Refusing here is how the choice reaches
    -- the chooser instead of being silently made by a fold.
    GuardAmbiguousBinding Text [Text]
  | -- | A @?binder@ inside a negated guard. Nothing can be bound from
    -- the absence of a fact, so this is malformed rather than merely
    -- unsatisfied, and saying so beats binding nothing and carrying on.
    GuardBinderInNegatedGuard Text
  deriving (Eq, Show)

-- | Every @?binder@ named anywhere in a pattern, anchor included.
binderNames :: Pattern -> [Text]
binderNames p =
  [v | TermBind v <- patternAnchor p : map hopTerm (patternHops p)]

-- | One candidate walk: where it currently stands, and what it has bound
-- to get there.
type Env = (Text, Map Text Text)

-- | 'resolveTerm', but consulting bindings made earlier in THIS pattern
-- before those already in the context.
resolveWith :: Map Text Text -> PatternTerm -> EvalContext -> Maybe Text
resolveWith binds t@(TermBind v) ctx = case Map.lookup v binds of
  Just x -> Just x
  Nothing -> resolveTerm t ctx
resolveWith _ t ctx = resolveTerm t ctx

-- | Match one term against one reached object, extending the bindings.
-- 'Nothing' means this walk is dead.
matchTerm :: EvalContext -> PatternTerm -> Text -> Map Text Text -> Maybe (Map Text Text)
matchTerm ctx term objText binds = case resolveWith binds term ctx of
  -- Already fixed (literal, self, bound $param, or a ?binder bound
  -- earlier): this is a filter. A repeated ?name must agree with itself
  -- -- the intra-pattern unification a bare TermVar deliberately lacks.
  Just want -> if want == objText then Just binds else Nothing
  Nothing -> case term of
    TermBind v -> Just (Map.insert v objText binds)
    -- A TermVar or unbound $param stays what it has always been: open,
    -- matching anything, binding nothing.
    _ -> Just binds

stepHopEnv :: EvalContext -> WorldSnapshot -> [Env] -> PatternHop -> [Env]
stepHopEnv ctx snap envs hop =
  [ (objText, binds')
  | (subj, binds) <- envs
  , (factSubj, objText) <- factsForPredicate (hopPredicate hop) snap
  , factSubj == subj
  , Just binds' <- [matchTerm ctx (hopTerm hop) objText binds]
  ]

-- | 'evalExists', keeping the witnesses instead of discarding them.
--
-- This is the same walk 'evalExists' does. The only difference is that
-- 'evalExists' ends in @not . null@ and throws away the very thing it
-- just computed -- which is why binding needed no new search, only the
-- decision to stop dropping the answer.
evalExistsEnv :: Pattern -> EvalContext -> WorldSnapshot -> [Env]
evalExistsEnv pattern ctx snap =
  foldl' (stepHopEnv ctx snap) startEnvs (patternHops pattern)
  where
    anchor = patternAnchor pattern
    allSubjects = nub [subj | (subj, _pred) <- Map.keys (snapshotFacts snap)]
    startEnvs = case resolveTerm anchor ctx of
      Just n -> [(n, Map.empty)]
      Nothing -> case anchor of
        TermBind v -> [(subj, Map.singleton v subj) | subj <- allSubjects]
        _ -> [(subj, Map.empty) | subj <- allSubjects]

-- | Every guard, as one conjunctive query: a SET of candidate binding
-- environments is threaded through all of them, each guard filtering and
-- extending it, and ambiguity is judged only at the end.
--
-- Judging it per-guard was the first implementation and it was wrong, in
-- a way a real test caught: @guard ?rock \`in\` quarry\/north@ followed by
-- @guard ?rock \`grade\` ore\/rich@ would refuse on the first guard's
-- three candidates without ever consulting the second, which exists
-- precisely to narrow them. Guards are a conjunction; a binder is
-- ambiguous only if it is still ambiguous once every guard has had its
-- say.
--
-- Returns the accumulated bindings alongside the verdict. On @False@ the
-- bindings are empty -- a transition that cannot fire has no effects to
-- resolve.
evalGuardsBinding :: [GuardClause] -> EvalContext -> WorldSnapshot -> Either GuardError (Bool, Map Text Text)
evalGuardsBinding gs ctx snap = go gs [Map.empty]
  where
    allBinders = nub (concatMap (binderNames . existsPattern . guardExists) gs)

    go [] envs = finish envs
    go (g : rest) envs = do
      envs' <- stepGuard g envs
      if null envs' then Right (False, Map.empty) else go rest envs'

    stepGuard g envs
      | guardNegated g, (v : _) <- binderNames pat = Left (GuardBinderInNegatedGuard v)
      | guardNegated g = Right [e | e <- envs, null (evalExistsEnv pat (with e) snap)]
      | otherwise =
          Right (nub [Map.union b e | e <- envs, (_, b) <- evalExistsEnv pat (with e) snap])
      where
        pat = existsPattern (guardExists g)
        with e = ctx {ctxBindings = Map.union e (ctxBindings ctx)}

    finish envs
      | null envs = Right (False, Map.empty)
      | otherwise = case [(v, vals) | v <- allBinders, let vals = valuesFor v, length vals > 1] of
          ((v, vals) : _) -> Left (GuardAmbiguousBinding v vals)
          [] -> Right (True, Map.fromList [(v, x) | v <- allBinders, x : _ <- [valuesFor v]])
      where
        valuesFor v = nub [x | e <- envs, Just x <- [Map.lookup v e]]
