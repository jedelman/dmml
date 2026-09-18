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
  , EffectValue (..)
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
--
-- 'TermBind' (added 2026-09-18, Jason: "build the guard binding. No
-- wasted data!") is a real BINDING variable, written @?name@ in Surface,
-- and it is deliberately a NEW constructor rather than a change to
-- 'TermVar'. The two look similar and are not:
--
-- * 'TermVar' is a slash-free bareword. It matches anything, never
--   binds, and a second occurrence of the same name is independent of
--   the first -- documented as deliberate in 'DMML.Guard.resolveTerm'
--   and @SURFACE.md@. Critically it is also what this surface produces
--   BY ACCIDENT: every slash-free token in a guard pattern parses as
--   one, which is the single most-repeated real authoring mistake in
--   this project and the whole reason "DMML.GuardLiterals" exists.
-- * 'TermBind' can only be written on purpose: @?@ appears nowhere else
--   in the grammar, and @pIdentRaw@ requires a letter, so @?rock@ was a
--   parse error until now and no existing content can contain one.
--
-- Making 'TermVar' bind instead would have silently changed the meaning
-- of every accidental bareword already committed -- a guard that used to
-- match anything would start demanding consistency, and one matching
-- several facts would start refusing. New constructor, zero blast
-- radius on existing content, and you cannot get a binder by accident.
--
-- What a binder does: the witness a guard finds is carried out of the
-- guard and into later guards and the transition's effects
-- ('DMML.Guard.evalGuardsBinding'). @guard quarry\/north \\`holds\\` ?rock@
-- followed by @retract quarry\/north \\`holds\\` ?rock@ takes the
-- particular rock the guard found. Within one pattern a repeated @?name@
-- must match consistently -- the unification 'TermVar' deliberately
-- lacks, now available where it is asked for by name.
data PatternTerm
  = TermSelf
  | TermParam Text
  | TermVar Text
  | TermBind Text
  | TermNode Text
  deriving (Eq, Show)

-- | Generalized 2026-09-03 (dev-journal/2026-09-03-authoring-tools-and-
-- narration-compulsion-result.md, Phase 2/3, done as one change per
-- Jason's own call: "machines should govern all transitions... build
-- phase 2/3 -- they're the same"). Was @EffectAssert Text | EffectRetract
-- Text@, always implicitly @(self, "state", \<ident\>)@ -- real state-
-- machine transitions only, nothing else a firing could do. Now a
-- transition's effect can assert or retract an arbitrary fact, on an
-- arbitrary subject term (not just @self@) -- which is also, for free,
-- how firing mints a new node: 'PatternTerm's already include
-- 'TermParam', and DMML is open-world (a node exists the moment any
-- fact mentions it, no separate registry to update), so an effect whose
-- subject is @$name@ and whose predicate/value assert real content
-- brings a brand new node into existence the instant that transition
-- fires with a concrete binding for @$name@ -- no separate "mint"
-- effect constructor needed, matching the project's own smallest-
-- generic-extension razor. The old bare @assert \<ident\>@\/@retract
-- \<ident\>@ syntax still parses (see 'DMML.Surface.pEffectLine') as
-- sugar for exactly its old meaning, since real, already-committed
-- machine examples (the endurance seed machines) use it and there was
-- no reason to force a mechanical rewrite of real evidence just to add
-- a new capability alongside it.
-- | 'EffectRetract'\'s trailing 'Maybe EffectValue' was added 2026-09-04
-- after a real eval (dev-journal/2026-09-04-complex-machine-eval.md)
-- found three independent free models, unprompted, all writing a
-- general retract WITH a trailing value symmetric to assert's --
-- @retract $target \`wardedBy\` self@ -- even though the grammar at the
-- time had no value slot there at all. Jason's call: accept it, even
-- though nothing discriminated by it AT THE TIME -- "even if the value
-- does nothing it may be useful in the future." It maps directly onto
-- 'DMML.Ast.FactConsume'\'s own pre-existing optional @object@
-- ('factConsumeObject' -- the exact same wildcard-vs-explicit shape a
-- @consumes@\/@fact@ block already has), so a fired retract with a
-- value renders it into the real @consumes@ citation's own object
-- position.
--
-- MADE LOAD-BEARING the same day: 'DMML.Materialize.applyConsume' now
-- honors 'factConsumeObject' for real -- @Nothing@ still wipes every
-- live alternative for the key (the original wildcard semantics,
-- unchanged), but @Just v@ now removes ONLY the alternative whose value
-- equals @v@, leaving every other live alternative at that key intact.
-- 'DMML.Fire.resolveSingleRetract' matches accordingly: a value-
-- qualified retract can now target one specific alternative out of
-- several live ones, rather than the earlier all-or-refuse choice
-- ('DMML.Fire.FireRetractAmbiguous') a value-less retract still has
-- (there is no principled way to pick just one of several without a
-- value to match against). The "may be useful in the future" hook
-- arrived the same day it was added.
-- | 'EffectRetract'\'s @[PatternHop]@ (added 2026-09-04, jedelman/dmml#5)
-- covers a CHAINED retract -- @retract self \`witnessedBy\` self \`at\`
-- $eruption@, real output from a real eval
-- (dev-journal/2026-09-04-complex-machine-eval.md's @trial-02.dmml@,
-- `minimax-m3` unprompted). The guard grammar already supports exactly
-- this shape (a multi-hop 'DMML.Ast.Pattern' walk); a chained retract
-- undoes the SAME walk once resolved -- every hop, not just the last,
-- since "undo the whole pattern I just checked" is the coherent reading
-- of what a model reaching for this was actually asking for. An
-- intermediate hop's term is always a 'PatternTerm' (it must resolve to
-- a concrete node to keep walking -- a literal has no further edges to
-- follow), matching 'DMML.Ast.PatternHop' exactly; only the FINAL step
-- (predicate + 'Maybe EffectValue') can be a literal, same as before
-- this change. Empty hop list is the pre-existing single-hop shape,
-- unchanged. See 'DMML.Fire.resolveOneEffect' for how a chain resolves
-- to real, individually-cited retractions (one per hop, each its own
-- @consumes@\/@fact@ entry) and jedelman/dmml#5 for the two real design
-- problems a chain raises that a single hop never did: per-hop
-- ambiguity (an intermediate hop's own live alternatives could fan out,
-- same refusal shape as the existing single-hop case) and whole-tree
-- consistency (removing several facts at once could newly break some
-- OTHER transition's positive guard elsewhere -- see
-- 'DMML.Retroconsistency.gateConsistentTree's 2026-09-04 generalization,
-- now wired into every 'DMML.Fire.fireTransition' call).
--
-- | 'EffectSpawn' (added for the "machines produce machines" case
-- Jason called "always in the plan" once assert/retract already let a
-- firing mint a brand-new NODE for free) -- a transition can now also
-- mint a brand-new MACHINE. Deliberately the smallest useful shape:
-- @EffectSpawn newNode templateRef@ names an existing, already-declared
-- machine (@templateRef@, looked up in the same known-machine-set every
-- 'DMML.Fire.fireTransition' call already takes) whose 'machineStates'
-- and 'machineTransitions' are copied verbatim onto a FRESH
-- 'machineNode' ('newNode', resolved the same 'self'\/@$param@\/literal
-- way any other effect's subject term is). No deep substitution is
-- needed inside the copied body: every transition already refers to its
-- owning node only via 'TermSelf', never by repeating the machine's own
-- literal node, so "same body, different node identity" is the whole
-- transformation -- matching a template MachineStmt's own commitment
-- that nothing internal to it names itself by literal node text.
-- Real, disclosed scope limit: the template itself is a NodeRef, not a
-- resolvable term -- always a fixed, already-known machine kind chosen
-- at authoring time, never itself picked from @$param@\/self at fire
-- time (there is no principled way to guard "spawn from whichever
-- machine this param happens to name" the same way a fact's subject
-- can be open). See 'DMML.SpawnCycles' for the static check this
-- capability makes newly necessary: a machine whose spawn effects loop
-- back to its own template, directly or through others', can keep
-- producing instances of itself forever.
-- | 'EffectGraft' (added 2026-09-18, Jason: "we were relying on the
-- LLMs to perform copying and references -- let's move that
-- functionality into the machines"). Where 'EffectSpawn' copies a
-- FIXED, authored template (a 'NodeRef' resolved against the known-
-- machine set at authoring time), a graft copies whatever machine its
-- SOURCE term names AT FIRE TIME: @EffectGraft target source@ resolves
-- both terms (each @self@\/@$param@\/a literal node, the ordinary
-- 'DMML.Guard.resolveTerm' way), decodes the source machine's whole
-- structure from the live snapshot's own facts
-- ('DMML.MachineFacts.decodeMachineFromSnapshot' -- so the source must
-- be fact-native, exactly what a prior spawn or graft produced), rebinds
-- it onto @target@, and re-encodes ('DMML.MachineFacts.encodeMachine'
-- derives every sub-node from the machine node, so all internal
-- references rewrite to the target's namespace for free). It resolves
-- to the same 'DMML.Fire.ResolvedSpawn' a spawn does, so nothing
-- downstream changes. This is the runtime copy 'EffectSpawn'
-- deliberately refused -- the price is that a graft's source is dynamic,
-- so 'DMML.SpawnCycles' cannot statically follow it (a real, disclosed
-- gap: graft cycles are a run-time budget concern, not a static one).
-- Recombination is then expressible without any further primitive:
-- graft parent A onto a fresh node, then graft parent B onto the same
-- node, and its @hasState@\/@hasTransition@ facts become the union of
-- both parents (clean when the parents' transition idents are disjoint;
-- overlapping idents collide on the shared sub-node, which fine-grained
-- crossover at authoring time -- the recombinant cannon -- handles by
-- renaming, not this primitive).
data Effect
  = EffectAssert PatternTerm PredicateRef EffectValue
  | EffectRetract PatternTerm [PatternHop] PredicateRef (Maybe EffectValue)
  | EffectSpawn PatternTerm NodeRef
  | EffectGraft PatternTerm PatternTerm
  deriving (Eq, Show)

-- | An effect's asserted value: either a node reference (resolved from
-- @self@\/@$param@\/a literal node at fire time, via the same
-- 'DMML.Guard.resolveTerm' guards already use) or a literal (string\/
-- number\/bool), matching the two shapes an ordinary fact's 'Value'
-- already supports.
data EffectValue
  = EffectValueTerm PatternTerm
  | EffectValueLiteral Literal
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
