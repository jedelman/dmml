{-# LANGUAGE OverloadedStrings #-}

-- | Wire-format types, translated field-for-field from the real Rust
-- @dmml@ crate's @src/from_json.rs@ -- the @*Input@ structs an agent's
-- JSON tool-call output is deserialized into, before any DMML-level
-- validation runs. This module only documents shape (what 'Data.Aeson'
-- will accept as syntactically well-formed); "DMML.FromJson" carries the
-- lexical\/semantic checks (identifier syntax, node-ref syntax, "commit
-- has no facts\/consumes\/refs", duplicate facts, ...) the Rust source
-- keeps as a separate pass over these same shapes.
module DMML.Json
  ( StrongRefInput (..)
  , ObjectInput (..)
  , DeclareKind (..)
  , DeclareInput (..)
  , FactInput (..)
  , FactConsumeInput (..)
  , ConsumeEntryInput (..)
  , CommitInput (..)
  , StateInput (..)
  , PatternTermInput (..)
  , PatternHopInput (..)
  , ExistsInput (..)
  , GuardInput (..)
  , EffectInput (..)
  , EffectValueInput (..)
  , TransitionInput (..)
  , MachineInput (..)
  , ReferenceInput (..)
  , BatchInput (..)
  , UpdateInput (..)
  ) where

import Data.Aeson
import Data.Aeson.Types (Parser)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import Data.Text (Text)

-- | A whole-commit reference target: @{"uri": "...", "cid": "..."}@.
data StrongRefInput = StrongRefInput
  { sriUri :: Text
  , sriCid :: Text
  }
  deriving (Eq, Show)

instance FromJSON StrongRefInput where
  parseJSON = withObject "StrongRefInput" $ \o ->
    StrongRefInput <$> o .: "uri" <*> o .: "cid"

-- | Mirrors @dmml::lower::TripleValue@'s shape: an agent picks which kind
-- of object a fact (or a 'FactConsumeInput'\'s object) has. @kind@ is the
-- one discriminant, always.
data ObjectInput
  = ObjNode {objValue :: Text}
  | ObjStr {objValue :: Text}
  | ObjNumber {objValue :: Text}
  | ObjBoolean {objBool :: Bool}
  deriving (Eq, Show)

instance FromJSON ObjectInput where
  parseJSON = withObject "ObjectInput" $ \o -> do
    kind <- o .: "kind" :: Parser Text
    case kind of
      "node" -> ObjNode <$> o .: "value"
      "str" -> ObjStr <$> o .: "value"
      "number" -> ObjNumber <$> o .: "value"
      "boolean" -> ObjBoolean <$> o .: "value"
      other -> fail ("unknown ObjectInput kind " <> show other)

data DeclareKind = DeclareRelation | DeclareAttribute
  deriving (Eq, Show)

instance FromJSON DeclareKind where
  parseJSON = withText "DeclareKind" $ \t -> case t of
    "relation" -> pure DeclareRelation
    "attribute" -> pure DeclareAttribute
    other -> fail ("unknown declare kind " <> show other)

data DeclareInput = DeclareInput
  { diKind :: DeclareKind
  , diName :: Text
  }
  deriving (Eq, Show)

instance FromJSON DeclareInput where
  parseJSON = withObject "DeclareInput" $ \o ->
    DeclareInput <$> o .: "kind" <*> o .: "name"

data FactInput = FactInput
  { fiSubject :: Text
  , fiPredicate :: Text
  , fiObject :: ObjectInput
  }
  deriving (Eq, Show)

instance FromJSON FactInput where
  parseJSON = withObject "FactInput" $ \o ->
    FactInput <$> o .: "subject" <*> o .: "predicate" <*> o .: "object"

-- | Omitting @object@ entirely means 'Nothing' -- 'FactRef'\'s existing
-- wildcard semantics (every triple asserted for @(subject, predicate)@).
-- @null@ is never sent or expected.
data FactConsumeInput = FactConsumeInput
  { fciCommit :: StrongRefInput
  , fciSubject :: Text
  , fciPredicate :: Text
  , fciObject :: Maybe ObjectInput
  }
  deriving (Eq, Show)

instance FromJSON FactConsumeInput where
  parseJSON = withObject "FactConsumeInput" $ \o ->
    FactConsumeInput
      <$> o .: "commit"
      <*> o .: "subject"
      <*> o .: "predicate"
      <*> o .:? "object"

-- | @kind: "strong"@ for a whole-commit reference, @kind: "fact"@ for a
-- fact-level reference -- the one discriminant, same convention as
-- 'ObjectInput'.
data ConsumeEntryInput
  = ConsumeStrongInput StrongRefInput
  | ConsumeFactInput FactConsumeInput
  deriving (Eq, Show)

instance FromJSON ConsumeEntryInput where
  parseJSON v@(Object o) = do
    kind <- o .: "kind" :: Parser Text
    case kind of
      "strong" -> ConsumeStrongInput <$> parseJSON v
      "fact" -> ConsumeFactInput <$> parseJSON v
      other -> fail ("unknown ConsumeEntryInput kind " <> show other)
  parseJSON _ = fail "ConsumeEntryInput: expected an object"

data CommitInput = CommitInput
  { ciVerb :: Text
  , ciDeclares :: [DeclareInput]
  , ciFacts :: [FactInput]
  , ciConsumes :: [ConsumeEntryInput]
  -- | Role-tagged commit-level references, e.g. @{"via": [...],
  -- "respondsTo": [...], "requires": [...]}@ -- an open role name, so a
  -- new role needs no schema change here.
  , ciRefs :: Map Text [StrongRefInput]
  }
  deriving (Eq, Show)

instance FromJSON CommitInput where
  parseJSON = withObject "CommitInput" $ \o ->
    CommitInput
      <$> o .: "verb"
      <*> o .:? "declares" .!= []
      <*> o .:? "facts" .!= []
      <*> o .:? "consumes" .!= []
      <*> o .:? "refs" .!= Map.empty

newtype StateInput = StateInput {siIdent :: Text}
  deriving (Eq, Show)

instance FromJSON StateInput where
  parseJSON = withObject "StateInput" $ \o -> StateInput <$> o .: "ident"

-- | @kind@ is the one discriminant; @value@ is present for every variant
-- except @self@ (which needs no payload -- it always means the
-- machine's own node).
data PatternTermInput
  = TermSelfInput
  | TermParamInput Text
  | TermVarInput Text
  | TermNodeInput Text
  deriving (Eq, Show)

instance FromJSON PatternTermInput where
  parseJSON = withObject "PatternTermInput" $ \o -> do
    kind <- o .: "kind" :: Parser Text
    case kind of
      "self" -> pure TermSelfInput
      "param" -> TermParamInput <$> o .: "value"
      "var" -> TermVarInput <$> o .: "value"
      "node" -> TermNodeInput <$> o .: "value"
      other -> fail ("unknown PatternTermInput kind " <> show other)

data PatternHopInput = PatternHopInput
  { phiPredicate :: Text
  , phiTerm :: PatternTermInput
  }
  deriving (Eq, Show)

instance FromJSON PatternHopInput where
  parseJSON = withObject "PatternHopInput" $ \o ->
    PatternHopInput <$> o .: "predicate" <*> o .: "term"

data ExistsInput = ExistsInput
  { eiAnchor :: PatternTermInput
  , eiHops :: [PatternHopInput]
  }
  deriving (Eq, Show)

instance FromJSON ExistsInput where
  parseJSON = withObject "ExistsInput" $ \o ->
    ExistsInput <$> o .: "anchor" <*> o .: "hops"

data GuardInput = GuardInput
  { giNegated :: Bool
  , giExists :: ExistsInput
  }
  deriving (Eq, Show)

instance FromJSON GuardInput where
  parseJSON = withObject "GuardInput" $ \o ->
    GuardInput <$> o .:? "negated" .!= False <*> o .: "exists"

-- | An effect's asserted value on the wire: either a 'PatternTermInput'
-- (a node reference, resolved at fire time -- @self@\/@$param@\/@?var@\/a
-- literal node) or a literal (string\/number\/boolean), using the same
-- @kind@-tagged discriminant space as 'PatternTermInput' and
-- 'ObjectInput' respectively -- they never collide, since @self@\/
-- @param@\/@var@\/@node@ and @str@\/@number@\/@boolean@ are disjoint tag
-- sets.
data EffectValueInput
  = EffectValueTermInput PatternTermInput
  | EffectValueStrInput Text
  | EffectValueNumberInput Text
  | EffectValueBooleanInput Bool
  deriving (Eq, Show)

instance FromJSON EffectValueInput where
  parseJSON = withObject "EffectValueInput" $ \o -> do
    kind <- o .: "kind" :: Parser Text
    case kind of
      "self" -> pure (EffectValueTermInput TermSelfInput)
      "param" -> EffectValueTermInput . TermParamInput <$> o .: "value"
      "var" -> EffectValueTermInput . TermVarInput <$> o .: "value"
      "node" -> EffectValueTermInput . TermNodeInput <$> o .: "value"
      "str" -> EffectValueStrInput <$> o .: "value"
      "number" -> EffectValueNumberInput <$> o .: "value"
      "boolean" -> EffectValueBooleanInput <$> o .: "value"
      other -> fail ("unknown EffectValueInput kind " <> show other)

-- | @kind: "assert" | "retract"@. Two shapes per kind, distinguished by
-- which fields are present (never by a second discriminant): the OLD
-- sugar form carries a bare @ident@ (always implicitly @self . state@,
-- preserved for real, already-committed machine examples that use it --
-- see 'DMML.Ast.Effect'\'s own doc comment) and the general form carries
-- @subject@\/@predicate@\/(for assert) @value@. An @ident@ field wins if
-- present, matching the parser's own precedence in "DMML.Surface" of
-- trying the general form first and falling back to sugar.
--
-- The general retract's @value@ is OPTIONAL (added 2026-09-04, see
-- 'DMML.Ast.Effect'\'s own doc comment for why) -- omit the field
-- entirely for a value-agnostic retract, same as the old sugar's own
-- always-value-agnostic meaning.
data EffectInput
  = EffectAssertInput Text
  | EffectRetractInput Text
  | EffectAssertGeneralInput PatternTermInput Text EffectValueInput
  | EffectRetractGeneralInput PatternTermInput Text (Maybe EffectValueInput)
  deriving (Eq, Show)

instance FromJSON EffectInput where
  parseJSON = withObject "EffectInput" $ \o -> do
    kind <- o .: "kind" :: Parser Text
    mIdent <- o .:? "ident"
    case (kind, mIdent) of
      ("assert", Just ident) -> pure (EffectAssertInput ident)
      ("assert", Nothing) -> EffectAssertGeneralInput <$> o .: "subject" <*> o .: "predicate" <*> o .: "value"
      ("retract", Just ident) -> pure (EffectRetractInput ident)
      ("retract", Nothing) -> EffectRetractGeneralInput <$> o .: "subject" <*> o .: "predicate" <*> o .:? "value"
      (other, _) -> fail ("unknown EffectInput kind " <> show other)

data TransitionInput = TransitionInput
  { tiIdent :: Text
  , tiParams :: [Text]
  , tiFrom :: Maybe Text
  , tiTo :: Maybe Text
  , tiGuards :: [GuardInput]
  , tiEffects :: [EffectInput]
  }
  deriving (Eq, Show)

instance FromJSON TransitionInput where
  parseJSON = withObject "TransitionInput" $ \o ->
    TransitionInput
      <$> o .: "ident"
      <*> o .:? "params" .!= []
      <*> o .:? "from"
      <*> o .:? "to"
      <*> o .:? "guards" .!= []
      <*> o .:? "effects" .!= []

data MachineInput = MachineInput
  { miNode :: Text
  , miStates :: [StateInput]
  , miTransitions :: [TransitionInput]
  }
  deriving (Eq, Show)

instance FromJSON MachineInput where
  parseJSON = withObject "MachineInput" $ \o ->
    MachineInput
      <$> o .: "node"
      <*> o .:? "states" .!= []
      <*> o .:? "transitions" .!= []

data ReferenceInput = ReferenceInput
  { riTarget :: StrongRefInput
  , riAsName :: Maybe Text
  }
  deriving (Eq, Show)

instance FromJSON ReferenceInput where
  parseJSON = withObject "ReferenceInput" $ \o ->
    ReferenceInput <$> o .: "target" <*> o .:? "asName"

-- | One group of commits (and machines) meant to land as ONE simultaneous
-- snapshot. See 'DMML.Ast.Batch' and "DMML.FromJson"'s @updateFromJson@
-- for the ordering semantics this shape carries.
data BatchInput = BatchInput
  { biCommits :: [CommitInput]
  , biMachines :: [MachineInput]
  }
  deriving (Eq, Show)

instance FromJSON BatchInput where
  parseJSON = withObject "BatchInput" $ \o ->
    BatchInput <$> o .:? "commits" .!= [] <*> o .:? "machines" .!= []

-- | The top-level batching shape: an ordered sequence of 'BatchInput'
-- groups. A lone commit is just a batch of one, in a sequence of one:
-- @{"update": [{"commits": [c]}]}@.
newtype UpdateInput = UpdateInput {uiUpdate :: [BatchInput]}
  deriving (Eq, Show)

instance FromJSON UpdateInput where
  parseJSON = withObject "UpdateInput" $ \o ->
    UpdateInput <$> o .: "update"
