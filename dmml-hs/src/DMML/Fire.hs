{-# LANGUAGE OverloadedStrings #-}

-- | Phase 3 of the 2026-09-03 authoring-tools plan (Phase 2 generalized
-- 'Effect'; this module is the "machines govern all transitions" half
-- Jason called for in the same breath: "build phase 2/3 -- they're the
-- same"). Real execution semantics for a machine transition, finally --
-- until this module, NOTHING in @dmml-hs@ ever applied an 'Effect'
-- (confirmed by reading 'DMML.Guard'\'s own doc comment before writing
-- any of this: @mayFire@ answers "is this legal," it never acts).
--
-- Deliberately does NOT mutate a 'DMML.Materialize.WorldSnapshot' in
-- place. Per the "DMML is the evidence, not any tool's or agent's
-- say-so" principle (Paper 2 §10; also why 'DMML.Retroconsistency'
-- renders a real, re-parseable commit instead of poking the snapshot
-- directly), firing a transition resolves its effects to concrete facts
-- and renders them as ordinary DMML Surface commit text -- the exact
-- same real @validate-commit@\/@check-declared@\/@retro-gate@ pipeline
-- any other commit goes through is what actually applies it. This module
-- only gets you from "a transition legally fired" to "here is the commit
-- that firing produced," nothing more.
--
-- UPDATED 2026-09-04 (jedelman/dmml#4): 'EffectRetract' firing used to
-- refuse outright -- DMML's real commit grammar only retracts a fact via
-- a @consumes@ block naming the specific prior commit (@uri#cid@) that
-- asserted it, and a 'DMML.Materialize.WorldSnapshot' built the ordinary
-- way ('DMML.Materialize.applyCommit') never carries one. The fix isn't
-- in this module -- it's that a caller can now build a snapshot WITH
-- real provenance ('DMML.Materialize.applyIdentifiedCommit'), and once
-- it does, this module builds a real @consumes@ citation from whatever
-- 'DMML.Materialize.currentValueWithProvenance' reports. A retract still
-- refuses, honestly, when the snapshot in hand has no real provenance to
-- cite -- see 'FireRetractNoProvenance'.
--
-- UPDATED AGAIN 2026-09-04 (jedelman/dmml#5): a retract can now be
-- CHAINED, mirroring a guard's own multi-hop 'DMML.Ast.Pattern' -- real
-- output a free model wrote unprompted (dev-journal/2026-09-04-complex-
-- machine-eval.md's @trial-02.dmml@). Every hop the chain walks gets
-- independently resolved and independently cited (see
-- 'resolveRetractHops') -- each is its own real fact, each needs its
-- own real provenance and its own ambiguity check, same discipline as
-- the single-hop case just applied per-hop. And because firing can now
-- remove several facts at once (not just add them), 'fireTransition'
-- gates every firing against 'DMML.Retroconsistency.gateConsistentTree'
-- (generalized the same day to check both guard polarities) -- Jason's
-- own framing, walked through directly: "what if a chained retract
-- leaves it in an invalid state? I guess the transition is invalid."
-- That's exactly the refusal 'FireWouldBreakConsistency' is for.
module DMML.Fire
  ( ResolvedFact (..)
  , ResolvedEffect (..)
  , FireError (..)
  , fireTransition
  , renderFiredCommit
  ) where

import Data.List (nub)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast
import DMML.Guard (EvalContext (..), mayFire, resolveTerm)
import DMML.Materialize (WorldSnapshot, applyCommit, currentValueWithProvenance)
import DMML.Retroconsistency (BrokenGuard (..), GateResult (..), gateConsistentTree)
import DMML.Surface (parseCommitSurface)

-- | One concrete fact a fired transition's assert effect resolved to.
data ResolvedFact = ResolvedFact
  { rfSubject :: Text
  , rfPredicate :: PredicateRef
  , rfValue :: Value
  }
  deriving (Eq, Show)

-- | One fired transition's effect, resolved: either a concrete fact to
-- assert, or a real, cited retraction. A single chained 'EffectRetract'
-- resolves to MULTIPLE 'ResolvedRetract's -- one per hop it walked, each
-- independently cited -- so this stays a flat list at the
-- 'fireTransition' level rather than mirroring 'Effect'\'s own 1:1
-- shape; 'renderFiredCommit' still renders every entry from one source
-- effect adjacently, in the order they were resolved (which is the
-- order the transition declared its hops in).
data ResolvedEffect
  = ResolvedAssert ResolvedFact
  | -- | @(subject, predicate, an optional resolved value, the real
    -- StrongRef being cited)@ -- the one live alternative this retract's
    -- citation actually consumes. The value, when the author's effect
    -- had one, renders into the produced @consumes@\/@fact@ entry's own
    -- pre-existing optional object position ('DMML.Ast.factConsumeObject')
    -- and is genuinely load-bearing as of 2026-09-04: 'DMML.Materialize'\'s
    -- @applyConsume@ now removes only the alternative matching this
    -- value, not the whole key -- see 'DMML.Ast.Effect'\'s own doc
    -- comment.
    ResolvedRetract Text PredicateRef (Maybe Value) StrongRef
  deriving (Eq, Show)

data FireError
  = -- | No such transition declared on this machine.
    FireNotDeclared
  | -- | The transition is declared, but its guards don't currently hold.
    FireBlocked
  | -- | An effect's subject term (or, for a chained retract, an
    -- intermediate hop's own term) didn't resolve to a concrete node --
    -- a @?var@ (never binds, per 'resolveTerm'\'s own doc comment) or an
    -- unbound @$param@ 'EvalContext' has no binding for. Nothing an
    -- effect targets is ever existentially open in a real firing:
    -- whatever fires must know exactly what it's acting on, at every
    -- step of a chain, not just the first.
    FireUnresolvedSubject Effect
  | -- | Same as above, for an 'EffectValueTerm' asserted (or retract's
    -- terminal) value.
    FireUnresolvedValue Effect
  | -- | A retract effect's (subject, predicate) has no live fact at all
    -- in the snapshot -- nothing to retract. For a chained retract, this
    -- can fire on any hop along the walk, not just the terminal one --
    -- the whole chain fails closed together (see 'resolveRetractHops'),
    -- never partially.
    FireRetractNoSuchFact Effect
  | -- | A retract effect's (subject, predicate) has a live fact, but it
    -- was materialized the ordinary, provenance-free way
    -- ('DMML.Materialize.applyCommit') rather than with a real
    -- 'DMML.Ast.StrongRef' ('DMML.Materialize.applyIdentifiedCommit') --
    -- there is genuinely no real @uri#cid@ to cite, so this refuses
    -- rather than fabricate one. Real, disclosed fix: materialize the
    -- world snapshot passed to 'fireTransition' with real identified
    -- commits (see @app/FireTransition.hs@'s use of
    -- 'DMML.LocalIdentity.localFileRef').
    FireRetractNoProvenance Effect
  | -- | A retract effect's (subject, predicate) currently has MORE THAN
    -- ONE live alternative -- at ANY step of a chain, not just the
    -- terminal one. 'DMML.Materialize'\'s own @consumes@ application
    -- (@applyConsume@) deletes every live alternative for a (subject,
    -- predicate) key unconditionally, regardless of which @uri#cid@ the
    -- @consumes@ entry actually names -- so citing just ONE of several
    -- live alternatives' provenance while the applied commit would
    -- silently delete ALL of them (including ones this retract never
    -- cited) would misrepresent what's actually being consumed. Refuses
    -- rather than pick one alternative's citation to stand in for a
    -- broader deletion nobody actually authorized.
    FireRetractAmbiguous Effect
  | -- | Firing legally, and resolving every effect soundly, would still
    -- leave the world in a state where some OTHER guard -- on this
    -- machine or any other in the known set -- that held before this
    -- firing is newly blocked after it. Jason's own framing, verbatim:
    -- "what if a chained retract leaves it in the invalid state? I
    -- guess the transition is invalid." This is that refusal. Carries
    -- every broken guard found (see
    -- 'DMML.Retroconsistency.gateConsistentTree'), not just the first,
    -- so a caller can report the whole real picture. A real, disclosed
    -- limitation shared with 'DMML.Retroconsistency'\'s own gate: a
    -- guard that itself depends on a @$param@ can't be generically
    -- re-checked here (its real meaning depends on a specific OTHER
    -- firing's own argument bindings, which this snapshot-level check
    -- doesn't have) and is excluded from the scan, not silently passed.
    FireWouldBreakConsistency [BrokenGuard]
  deriving (Eq, Show)

-- | Fires one named transition: checks it's declared and legal (via
-- 'mayFire', unchanged), resolves every effect under @ctx@'s bindings
-- against @snap@ (fails closed on the first effect, or hop, that can't
-- be soundly resolved), then gates the WHOLE resolved set against every
-- guard in @machines@ (the full known machine set, including the firing
-- one) before returning success -- never emits a partial or
-- consistency-breaking result. @machines@ is the caller's
-- responsibility to assemble as the real, currently-relevant machine
-- set (same division of responsibility 'DMML.Retroconsistency.
-- gateConsistentTree' already has for its own callers) -- passing just
-- the firing machine alone is legal (it still checks that machine's OWN
-- other transitions), just narrower coverage than including the rest of
-- a real world's machines.
fireTransition :: Map.Map Text MachineStmt -> MachineStmt -> Text -> EvalContext -> WorldSnapshot -> Either FireError [ResolvedEffect]
fireTransition machines machine ident ctx snap =
  case mayFire machine ident ctx snap of
    Nothing -> Left FireNotDeclared
    Just (False, _, _) -> Left FireBlocked
    Just (True, rawEffects, _to) -> do
      effects <- concat <$> traverse (resolveOneEffect ctx snap) rawEffects
      gateCheck machines snap effects
      pure effects

-- | Renders the resolved effects as a real commit, re-parses it, and
-- applies it to @before@ to get @after@ -- gating against the ACTUAL
-- commit firing would produce (dogfooding the real render+parse path,
-- not a shadow representation) rather than a hand-rolled diff. An empty
-- effects list (a transition with only a guard\/from-to pair, no real
-- effects at all) can't render to a parseable commit (DMML rejects an
-- empty commit) -- correctly treated as nothing to gate, since no
-- change at all can't break anything either.
gateCheck :: Map.Map Text MachineStmt -> WorldSnapshot -> [ResolvedEffect] -> Either FireError ()
gateCheck machines before effects = case parseCommitSurface (renderFiredCommit "gate_check" effects) of
  Left _ -> Right ()
  Right stmt ->
    let after = applyCommit "gate_check" before stmt
     in case gateConsistentTree machines before after of
          GateOk -> Right ()
          GateBroken broken -> Left (FireWouldBreakConsistency broken)

resolveTermOrFail :: Effect -> EvalContext -> PatternTerm -> Either FireError Text
resolveTermOrFail eff ctx t = maybe (Left (FireUnresolvedSubject eff)) Right (resolveTerm t ctx)

resolveEffectValueToNode :: Effect -> EvalContext -> EffectValue -> Either FireError Value
resolveEffectValueToNode eff ctx val = case val of
  EffectValueTerm t ->
    maybe (Left (FireUnresolvedValue eff)) (Right . ValueNode . NodeRef . T.splitOn "/") (resolveTerm t ctx)
  EffectValueLiteral lit -> Right (ValueLiteral lit)

-- | The one real lookup every retraction (a chain's own intermediate
-- hop or its terminal predicate alike) goes through: with a value to
-- match, exactly one LIVE ALTERNATIVE WHOSE VALUE EQUALS IT, with real
-- provenance, or refuse -- other live alternatives for the same key are
-- no longer even a consideration, since 'DMML.Materialize.applyConsume'
-- (updated the same day) now actually deletes only the one being cited,
-- not the whole key. Without a value (the old bare @retract <ident>@
-- sugar, or a general retract that never named one), falls back to the
-- pre-existing wildcard discipline: exactly one live alternative
-- overall, or refuse as ambiguous -- a value-less retract with several
-- live alternatives has no principled way to pick just one, and
-- wildcard-deleting all of them without the author ever citing more
-- than one would misrepresent what was actually authorized. Shared so a
-- chain's per-hop checks and the pre-existing single-hop case are
-- provably the same discipline, not two similar-looking copies.
resolveSingleRetract :: Effect -> WorldSnapshot -> Text -> PredicateRef -> Maybe Value -> Either FireError ResolvedEffect
resolveSingleRetract eff snap subjText predRef mVal =
  case currentValueWithProvenance (subjText, predText predRef) snap of
    [] -> Left (FireRetractNoSuchFact eff)
    alts -> case mVal of
      Nothing -> case alts of
        [(_label, Just ref, _v)] -> Right (ResolvedRetract subjText predRef mVal ref)
        [(_label, Nothing, _v)] -> Left (FireRetractNoProvenance eff)
        _ -> Left (FireRetractAmbiguous eff)
      Just v -> case [(label, ref) | (label, ref, v') <- alts, v' == v] of
        [] -> Left (FireRetractNoSuchFact eff)
        [(_label, Just ref)] -> Right (ResolvedRetract subjText predRef mVal ref)
        [(_label, Nothing)] -> Left (FireRetractNoProvenance eff)
        _ -> Left (FireRetractAmbiguous eff)

-- | Walks a chained retract's intermediate hops from @anchor@,
-- resolving each hop's own term to a concrete node (never fans out --
-- an intermediate hop's term must resolve via 'resolveTerm' alone, the
-- same way a guard's bound hop would, since there is no principled
-- "any of several" answer for what to actually delete), retracting the
-- walked edge at each step -- CITING the resolved target as the value
-- to match (not a wildcard), so a hop whose (subject, predicate) has
-- OTHER live alternatives besides the one actually walked only removes
-- the walked one, leaving the rest intact. Returns the FINAL anchor the
-- transition's terminal predicate\/value applies to. An empty hop list
-- (the ordinary, pre-existing single-hop case) is a no-op: @(anchor,
-- [])@.
resolveRetractHops :: Effect -> EvalContext -> WorldSnapshot -> Text -> [PatternHop] -> Either FireError (Text, [ResolvedEffect])
resolveRetractHops _ _ _ anchor [] = Right (anchor, [])
resolveRetractHops eff ctx snap anchor (hop : rest) = do
  targetText <- resolveTermOrFail eff ctx (hopTerm hop)
  let targetValue = ValueNode (NodeRef (T.splitOn "/" targetText))
  retracted <- resolveSingleRetract eff snap anchor (PredIdent (hopPredicate hop)) (Just targetValue)
  (finalAnchor, more) <- resolveRetractHops eff ctx snap targetText rest
  pure (finalAnchor, retracted : more)

resolveOneEffect :: EvalContext -> WorldSnapshot -> Effect -> Either FireError [ResolvedEffect]
resolveOneEffect ctx _snap eff@(EffectAssert subjTerm predRef val) = do
  subjText <- resolveTermOrFail eff ctx subjTerm
  value <- resolveEffectValueToNode eff ctx val
  pure [ResolvedAssert ResolvedFact {rfSubject = subjText, rfPredicate = predRef, rfValue = value}]
resolveOneEffect ctx snap eff@(EffectRetract subjTerm hops finalPred finalMVal) = do
  subjText <- resolveTermOrFail eff ctx subjTerm
  (finalAnchor, hopRetracts) <- resolveRetractHops eff ctx snap subjText hops
  finalValResolved <- traverse (resolveEffectValueToNode eff ctx) finalMVal
  finalRetract <- resolveSingleRetract eff snap finalAnchor finalPred finalValResolved
  pure (hopRetracts ++ [finalRetract])

predText :: PredicateRef -> Text
predText RdfType = "a"
predText (PredIdent t) = t

renderValue :: Value -> Text
renderValue (ValueNode n) = T.intercalate "/" (nodeRefSegments n)
renderValue (ValueLiteral (LitString s)) = "\"" <> T.concatMap esc s <> "\""
  where
    esc '"' = "\\\""
    esc '\\' = "\\\\"
    esc c = T.singleton c
renderValue (ValueLiteral (LitNumber n)) = n
renderValue (ValueLiteral (LitBoolean b)) = if b then "true" else "false"

-- | Renders resolved effects as a real, parseable DMML Surface commit --
-- asserts follow the same idiom as
-- 'DMML.Retroconsistency.renderImpliedCommit' (declares every plain-ident
-- predicate it uses, harmless if already declared elsewhere; an
-- 'RdfType' predicate uses the @::@ sugar and needs no @declare@ line);
-- every retract (each hop of a chain included) becomes its own @fact@
-- entry inside a single trailing @consumes@ block, citing the real
-- 'StrongRef' 'fireTransition' already resolved it against -- never a
-- fabricated one. Facts and retracts render in the order they were
-- resolved, which for a chain is hop order.
renderFiredCommit :: Text -> [ResolvedEffect] -> Text
renderFiredCommit verb effects =
  T.unlines $
    ["commit " <> verb]
      ++ ["  declare relation " <> p | p <- nub [predText (rfPredicate f) | ResolvedAssert f <- effects, rfPredicate f /= RdfType]]
      ++ [factLine f | ResolvedAssert f <- effects]
      ++ consumesBlock
  where
    factLine f = case rfPredicate f of
      RdfType -> "  " <> rfSubject f <> " :: a " <> renderValue (rfValue f)
      PredIdent p -> "  " <> rfSubject f <> " `" <> p <> "` " <> renderValue (rfValue f)

    retracts = [(subj, predRef, mVal, ref) | ResolvedRetract subj predRef mVal ref <- effects]
    consumesBlock
      | null retracts = []
      | otherwise =
          ["  consumes"]
            ++ concat
              [ [ "    fact " <> strongRefUri ref <> "#" <> strongRefCid ref
                , "      " <> subj <> " . " <> predText predRef <> maybe "" ((" = " <>) . renderValue) mVal
                ]
              | (subj, predRef, mVal, ref) <- retracts
              ]
