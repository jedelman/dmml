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
module DMML.Fire
  ( ResolvedFact (..)
  , FireError (..)
  , fireTransition
  , renderFiredCommit
  ) where

import Data.List (nub)
import Data.Text (Text)
import qualified Data.Text as T

import DMML.Ast
import DMML.Guard (EvalContext (..), mayFire, resolveTerm)
import DMML.Materialize (WorldSnapshot)

-- | One concrete fact a fired transition's effect resolved to.
data ResolvedFact = ResolvedFact
  { rfSubject :: Text
  , rfPredicate :: PredicateRef
  , rfValue :: Value
  }
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
  | -- | A real, disclosed scope limit, not a bug: DMML's actual commit
    -- grammar only retracts a fact via a @consumes@ block naming the
    -- specific prior commit (@uri#cid@ -- 'DMML.Ast.StrongRef') that
    -- asserted it (see @SURFACE.md@'s @consumes@\/@fact@ grammar and
    -- 'DMML.Ast.FactConsume'). A 'DMML.Materialize.WorldSnapshot's own
    -- 'DMML.Materialize.Alternatives' only carry a branch\/agent-name
    -- provenance label, never a real commit @uri#cid@ -- there is no
    -- sound way to synthesize a @consumes@ entry from a snapshot alone
    -- without fabricating provenance that doesn't exist. Firing a
    -- transition whose effects include a retract currently refuses
    -- rather than emit an unsound commit; a real fix needs the caller to
    -- supply real strong-ref provenance for whatever it wants retracted,
    -- which this module has no access to.
    FireRetractNeedsProvenance Effect
  deriving (Eq, Show)

-- | Fires one named transition: checks it's declared and legal (via
-- 'mayFire', unchanged), then resolves every effect to a concrete fact
-- under @ctx@'s bindings. Fails closed on the first effect that can't be
-- soundly resolved -- never emits a partial result, since a caller
-- rendering only SOME of a transition's effects as a commit would
-- silently misrepresent what actually fired.
fireTransition :: MachineStmt -> Text -> EvalContext -> WorldSnapshot -> Either FireError [ResolvedFact]
fireTransition machine ident ctx snap =
  case mayFire machine ident ctx snap of
    Nothing -> Left FireNotDeclared
    Just (False, _, _) -> Left FireBlocked
    Just (True, effects, _to) -> traverse (resolveOneEffect ctx) effects

resolveOneEffect :: EvalContext -> Effect -> Either FireError ResolvedFact
resolveOneEffect ctx eff@(EffectAssert subjTerm predRef val) = do
  subjText <- maybe (Left (FireUnresolvedSubject eff)) Right (resolveTerm subjTerm ctx)
  value <- case val of
    EffectValueTerm t ->
      maybe
        (Left (FireUnresolvedValue eff))
        (Right . ValueNode . NodeRef . T.splitOn "/")
        (resolveTerm t ctx)
    EffectValueLiteral lit -> Right (ValueLiteral lit)
  pure ResolvedFact {rfSubject = subjText, rfPredicate = predRef, rfValue = value}
resolveOneEffect _ eff@(EffectRetract _ _) = Left (FireRetractNeedsProvenance eff)

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

-- | Renders resolved facts as a real, parseable DMML Surface commit --
-- same idiom as 'DMML.Retroconsistency.renderImpliedCommit': declares
-- every plain-ident predicate it uses (harmless if already declared
-- elsewhere -- this module has no way to know what the target repo has
-- already declared), then one fact line per resolved effect. An
-- 'RdfType' predicate is rendered via the @::@ sugar rather than a
-- backtick application (the only form @DMML.Surface@'s parser accepts
-- for it) and needs no @declare@ line.
renderFiredCommit :: Text -> [ResolvedFact] -> Text
renderFiredCommit verb facts =
  T.unlines $
    ["commit " <> verb]
      ++ ["  declare relation " <> p | p <- nub [predText (rfPredicate f) | f <- facts, rfPredicate f /= RdfType]]
      ++ [factLine f | f <- facts]
  where
    factLine f = case rfPredicate f of
      RdfType -> "  " <> rfSubject f <> " :: a " <> renderValue (rfValue f)
      PredIdent p -> "  " <> rfSubject f <> " `" <> p <> "` " <> renderValue (rfValue f)
