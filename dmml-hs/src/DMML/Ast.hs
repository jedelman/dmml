-- | AST for DMML (Desiring-Machine Markup Language), translated from the
-- real Rust @dmml@ crate's @src/ast.rs@ + the machine-body types in
-- @src/machine.rs@. There is no text grammar and no text parser here
-- either -- the Rust source retired its hand-written recursive-descent
-- parser once it became clear nothing hand-writes DMML source text
-- anymore, only agents author commits, and JSON is what a tool-calling
-- agent actually produces reliably. This module is the validated,
-- structured target that "DMML.FromJson" builds by checking "DMML.Json"'s
-- raw wire shapes.
module DMML.Ast
  ( Span (..)
  , Document (..)
  , TopLevelItem (..)
  , CommitStmt (..)
  , CommitItem (..)
  , DeclareStmt (..)
  , DeclKind (..)
  , FactStmt (..)
  , PredicateRef (..)
  , Value (..)
  , Literal (..)
  , NodeRef (..)
  , ConsumesBlock (..)
  , ConsumeEntry (..)
  , FactConsume (..)
  , StrongRef (..)
  , ReferenceStmt (..)
    -- * Machine body (DMML.MACHINE_SPEC.md's grammar)
  , MachineStmt (..)
  , StateDecl (..)
  , TransitionDecl (..)
  , GuardClause (..)
  , ExistsExpr (..)
  , Pattern (..)
  , PatternHop (..)
  , PatternTerm (..)
  , Effect (..)
    -- * Batching (from_json.rs's Update/Batch)
  , Batch (..)
  , Update (..)
  ) where

import Data.Map.Strict (Map)
import Data.Text (Text)

-- | A JSON Pointer (RFC 6901) into the authoring request payload this AST
-- node was built from, e.g. @\/facts\/2\/predicate@. Stands in for the
-- byte-range-into-source-text @Span@ the retired text parser produced:
-- same job (let an error point at where in the original request a
-- problem came from), different coordinate space, since there is no
-- source text anymore to take a byte range into.
newtype Span = Span {spanPointer :: Text}
  deriving (Eq, Show)

newtype Document = Document {documentItems :: [TopLevelItem]}
  deriving (Eq, Show)

data TopLevelItem
  = TopCommit CommitStmt
  | TopReference ReferenceStmt
  | TopMachine MachineStmt
  deriving (Eq, Show)

data CommitStmt = CommitStmt
  { -- | Open vocabulary: @mints@, @becomes@, @divides@, @grants@, ... --
    -- never validated against a closed enum.
    commitVerb :: Text
  , commitItems :: [CommitItem]
  , -- | Role-tagged commit-level references (e.g. @via@\/@respondsTo@\/
    -- @requires@) to a list of 'StrongRef's under that role. Every role
    -- is a list even when conventionally single-valued -- whether a role
    -- means "at most one" or "any number" is a validation-time rule for
    -- whoever checks that role, not a difference in JSON\/AST shape. This
    -- is what makes adding a new role free at this layer: no new field,
    -- no new constructor, just a new key.
    commitRefs :: Map Text [StrongRef]
  , commitSpan :: Span
  }
  deriving (Eq, Show)

-- | A bare 'DeclareStmt' or 'FactStmt' appearing directly in a commit
-- body (outside any explicit produces block) is sugar for "implicit
-- produces block" -- lowered identically to one inside an explicit
-- block. JSON authoring never distinguishes the two forms; kept as
-- separate constructors purely to mirror the Rust AST's own shape.
data CommitItem
  = ItemDeclare DeclareStmt
  | ItemFact FactStmt
  | ItemConsumes ConsumesBlock
  deriving (Eq, Show)

data DeclareStmt = DeclareStmt
  { declareKind :: DeclKind
  , declareIdent :: Text
  , declareSpan :: Span
  }
  deriving (Eq, Show)

data DeclKind = DeclRelation | DeclAttribute
  deriving (Eq, Show)

data FactStmt = FactStmt
  { factSubject :: NodeRef
  , factPredicate :: PredicateRef
  , factValue :: Value
  , factSpan :: Span
  }
  deriving (Eq, Show)

-- | @"a"@ is Turtle-style sugar for @rdf:type@; anything else is a bare
-- identifier.
data PredicateRef = RdfType | PredIdent Text
  deriving (Eq, Show)

data Value = ValueNode NodeRef | ValueLiteral Literal
  deriving (Eq, Show)

data Literal
  = LitNumber Text
  | LitBoolean Bool
  | LitString Text
  deriving (Eq, Show)

-- | @segment ( "\/" segment )*@, e.g. @room\/42@, @key\/7@,
-- @room\/42.reach@. Stored as its literal segments, not pre-joined.
newtype NodeRef = NodeRef {nodeRefSegments :: [Text]}
  deriving (Eq, Show)

data ConsumesBlock = ConsumesBlock
  { consumesEntries :: [ConsumeEntry]
  , consumesSpan :: Span
  }
  deriving (Eq, Show)

data ConsumeEntry
  = ConsumeStrong StrongRef
  | ConsumeFact FactConsume
  deriving (Eq, Show)

data FactConsume = FactConsume
  { factConsumeCommit :: StrongRef
  , factConsumeSubject :: NodeRef
  , factConsumePredicate :: Text
  , -- | 'Nothing' preserves the wildcard semantics: every triple that
    -- commit asserted for @(subject, predicate)@.
    factConsumeObject :: Maybe Value
  , factConsumeSpan :: Span
  }
  deriving (Eq, Show)

-- | @uri@ is an opaque, substrate-chosen identifier for another commit --
-- an atproto AT-URI, a git commit ref, an S3 object key, whatever the
-- substrate that recorded it uses. Never parsed or validated beyond
-- non-emptiness; not treated as an identifier this grammar names.
data StrongRef = StrongRef
  { strongRefUri :: Text
  , strongRefCid :: Text
  , strongRefSpan :: Span
  }
  deriving (Eq, Show)

data ReferenceStmt = ReferenceStmt
  { referenceTarget :: StrongRef
  , referenceAsName :: Maybe NodeRef
  , referenceSpan :: Span
  }
  deriving (Eq, Show)

-- Machine body -------------------------------------------------------------

data MachineStmt = MachineStmt
  { machineNode :: NodeRef
  , machineStates :: [StateDecl]
  , machineTransitions :: [TransitionDecl]
  , machineSpan :: Span
  }
  deriving (Eq, Show)

data StateDecl = StateDecl {stateIdent :: Text, stateSpan :: Span}
  deriving (Eq, Show)

data TransitionDecl = TransitionDecl
  { transitionIdent :: Text
  , transitionParams :: [Text]
  , transitionFrom :: Maybe Text
  , transitionTo :: Maybe Text
  , transitionGuards :: [GuardClause]
  , transitionEffects :: [Effect]
  , transitionSpan :: Span
  }
  deriving (Eq, Show)

data GuardClause = GuardClause
  { guardNegated :: Bool
  , guardExists :: ExistsExpr
  , guardSpan :: Span
  }
  deriving (Eq, Show)

data ExistsExpr = ExistsExpr
  { existsPattern :: Pattern
  , existsSpan :: Span
  }
  deriving (Eq, Show)

data Pattern = Pattern
  { patternAnchor :: PatternTerm
  , patternHops :: [PatternHop]
  }
  deriving (Eq, Show)

data PatternHop = PatternHop
  { hopPredicate :: Text
  , hopTerm :: PatternTerm
  }
  deriving (Eq, Show)

-- | @self@ carries no payload -- it always means the machine's own node.
data PatternTerm
  = TermSelf
  | TermParam Text
  | TermVar Text
  | TermNode Text
  deriving (Eq, Show)

-- | Always implicitly @(self, "state", \<ident\>)@.
data Effect = EffectAssert Text | EffectRetract Text
  deriving (Eq, Show)

-- Batching (from_json.rs's Update/Batch) ------------------------------------

-- | One successfully-built batch: every commit and machine, already
-- deserialized, validated for shape, and converted to AST.
data Batch = Batch
  { batchCommits :: [CommitStmt]
  , batchMachines :: [MachineStmt]
  }
  deriving (Eq, Show)

-- | A successfully-built update: every batch, in the order submitted.
-- Applied by the caller one commit at a time, batch by batch, in that
-- same order -- batching is an authoring-time convenience over N JSON
-- items, not a new runtime concept.
newtype Update = Update {updateBatches :: [Batch]}
  deriving (Eq, Show)
