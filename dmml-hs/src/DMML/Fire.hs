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
module DMML.Fire
  ( ResolvedFact (..)
  , ResolvedEffect (..)
  , FireError (..)
  , fireTransition
  , renderFiredCommit
  ) where

import Data.List (nub)
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast
import DMML.Guard (EvalContext (..), mayFire, resolveTerm)
import DMML.Materialize (WorldSnapshot, currentValueWithProvenance)

-- | One concrete fact a fired transition's assert effect resolved to.
data ResolvedFact = ResolvedFact
  { rfSubject :: Text
  , rfPredicate :: PredicateRef
  , rfValue :: Value
  }
  deriving (Eq, Show)

-- | One fired transition's effect, resolved: either a concrete fact to
-- assert, or a real, cited retraction. Kept as one sum type (rather than
-- two separate lists) so 'renderFiredCommit' renders them in the exact
-- order the transition declared its effects -- an author who interleaves
-- asserts and retracts sees that order preserved in the produced commit.
data ResolvedEffect
  = ResolvedAssert ResolvedFact
  | -- | @(subject, predicate, the real StrongRef being cited)@ -- the
    -- one live alternative this retract's citation actually consumes.
    ResolvedRetract Text PredicateRef StrongRef
  deriving (Eq, Show)

data FireError
  = -- | No such transition declared on this machine.
    FireNotDeclared
  | -- | The transition is declared, but its guards don't currently hold.
    FireBlocked
  | -- | An effect's subject term didn't resolve to a concrete node --
    -- a @?var@ (never binds, per 'resolveTerm'\'s own doc comment) or an
    -- unbound @$param@ 'EvalContext' has no binding for. An effect's
    -- subject is never existentially open in a real firing: whatever
    -- fires must know exactly what it's asserting about.
    FireUnresolvedSubject Effect
  | -- | Same as above, for an 'EffectValueTerm' asserted value.
    FireUnresolvedValue Effect
  | -- | A retract effect's (subject, predicate) has no live fact at all
    -- in the snapshot -- nothing to retract.
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
    -- ONE live alternative. 'DMML.Materialize'\'s own @consumes@
    -- application ('DMML.Materialize.applyCommit'\'s @applyConsume@)
    -- deletes every live alternative for a (subject, predicate) key
    -- unconditionally, regardless of which @uri#cid@ the @consumes@
    -- entry actually names -- so citing just ONE of several live
    -- alternatives' provenance while the applied commit would silently
    -- delete ALL of them (including ones this retract never cited) would
    -- misrepresent what's actually being consumed. Refuses rather than
    -- pick one alternative's citation to stand in for a broader deletion
    -- nobody actually authorized.
    FireRetractAmbiguous Effect
  deriving (Eq, Show)

-- | Fires one named transition: checks it's declared and legal (via
-- 'mayFire', unchanged), then resolves every effect under @ctx@'s
-- bindings against @snap@. Fails closed on the first effect that can't
-- be soundly resolved -- never emits a partial result, since a caller
-- rendering only SOME of a transition's effects as a commit would
-- silently misrepresent what actually fired.
fireTransition :: MachineStmt -> Text -> EvalContext -> WorldSnapshot -> Either FireError [ResolvedEffect]
fireTransition machine ident ctx snap =
  case mayFire machine ident ctx snap of
    Nothing -> Left FireNotDeclared
    Just (False, _, _) -> Left FireBlocked
    Just (True, effects, _to) -> traverse (resolveOneEffect ctx snap) effects

resolveOneEffect :: EvalContext -> WorldSnapshot -> Effect -> Either FireError ResolvedEffect
resolveOneEffect ctx _snap eff@(EffectAssert subjTerm predRef val) = do
  subjText <- maybe (Left (FireUnresolvedSubject eff)) Right (resolveTerm subjTerm ctx)
  value <- case val of
    EffectValueTerm t ->
      maybe
        (Left (FireUnresolvedValue eff))
        (Right . ValueNode . NodeRef . T.splitOn "/")
        (resolveTerm t ctx)
    EffectValueLiteral lit -> Right (ValueLiteral lit)
  pure (ResolvedAssert ResolvedFact {rfSubject = subjText, rfPredicate = predRef, rfValue = value})
resolveOneEffect ctx snap eff@(EffectRetract subjTerm predRef) = do
  subjText <- maybe (Left (FireUnresolvedSubject eff)) Right (resolveTerm subjTerm ctx)
  case currentValueWithProvenance (subjText, predText predRef) snap of
    [] -> Left (FireRetractNoSuchFact eff)
    [(_label, Just ref, _v)] -> Right (ResolvedRetract subjText predRef ref)
    [(_label, Nothing, _v)] -> Left (FireRetractNoProvenance eff)
    _ -> Left (FireRetractAmbiguous eff)

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
-- every retract becomes one @fact@ entry inside a single trailing
-- @consumes@ block, citing the real 'StrongRef' 'fireTransition' already
-- resolved it against -- never a fabricated one. Facts and retracts
-- render in the transition's own declared effect order.
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

    retracts = [(subj, predRef, ref) | ResolvedRetract subj predRef ref <- effects]
    consumesBlock
      | null retracts = []
      | otherwise =
          ["  consumes"]
            ++ concat
              [ [ "    fact " <> strongRefUri ref <> "#" <> strongRefCid ref
                , "      " <> subj <> " . " <> predText predRef
                ]
              | (subj, predRef, ref) <- retracts
              ]
