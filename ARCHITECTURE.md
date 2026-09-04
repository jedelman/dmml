# Architecture — `dmml-hs`

The canonical implementation is one Haskell package
(`dmml-hs/dmml-hs.cabal`): a library, `src/DMML/*.hs`, plus one real
executable per real capability under `app/`. No crate split, no
substrate trait — `dmml-hs` talks to exactly one real substrate
(atproto) directly, rather than abstracting over a substrate it doesn't
yet have a second implementation of. If a second substrate becomes real,
that's the point to introduce an abstraction — not before.

## The module dependency shape

```
DMML.Ast            -- the grammar: CommitStmt, MachineStmt, Effect,
                        ConsumeEntry, StrongRef, NodeRef, Value, ...
   |
DMML.Surface         -- the real, current text grammar (megaparsec)
DMML.Json/FromJson    -- the legacy JSON front-end (superseded by Surface
                         for new writes, kept readable)
   |
DMML.Materialize     -- applies commits to a WorldSnapshot. Facts are
                         genuinely multi-valued (Alternatives, deduped on
                         VALUE not identity) -- a second independent
                         assert for the same (subject, predicate) never
                         overwrites or gets flagged, it just adds a live
                         alternative. Collapsing many alternatives to one
                         is never this module's job.
   |
   +-- DMML.Guard          -- a faithful, Datalog-shaped EXISTS evaluator
   |                          over a WorldSnapshot (structural recursion
   |                          over a guard pattern's hop list)
   +-- DMML.Governance     -- finds which machine (if any) governs a
   |                          disputed pair, and whether a live
   |                          alternative is currently a legal transition
   |                          outcome on it -- real arbitration, never
   |                          picking arbitrarily
   +-- DMML.Retroconsistency -- the whole-machine-set consistency gate:
   |                          would applying a set of effects break any
   |                          OTHER machine's currently-held guard,
   |                          anywhere in the known machine set
   +-- DMML.SelfDeclaration -- every predicate used in a live fact must
   |                          be declared somewhere in the same batch
   +-- DMML.CitationIntegrity -- a consumes citation's cid, checked
                              against either a real, independently-known
                              identity (a file actually in the batch) or
                              an earlier citation of the same uri
   |
DMML.Fire            -- real execution semantics: resolves a fired
                         transition's Effects to concrete facts, using
                         DMML.Guard to check firing is legal and
                         DMML.Retroconsistency to gate the result against
                         the whole known machine set, and renders a real,
                         re-parseable Surface commit
   |
DMML.LocalIdentity   -- FNV-1a-64 content fingerprint for a local file's
                         exact bytes (local:<path>#fnv1a64:<hash>) -- NOT
                         a real atproto CID (no SHA-2/CBOR-canonicalization
                         library available), labeled honestly as what it
                         is. What DMML.Fire cites when it needs a real,
                         re-checkable consumes provenance.
   |
DMML.Atproto         -- a minimal XRPC client: resolveHandle,
                         resolveDidToPdsEndpoint (did:plc only),
                         createSession, createRecord, deleteRecord,
                         listRecords. Shells out to curl via
                         System.Process rather than an HTTP client
                         library -- deliberate, see its own doc comment.

DMML.Checkpoint      -- content-addressed, incremental WorldSnapshot
                         checkpointing (folds only a merge's new files
                         into the parent checkpoint)
DMML.Entropy         -- entropy/compliance sidecar tooling
DMML.StringCap       -- a real, disclosed string-length cap check
```

## Real design principles this codebase actually holds to

**Collision-free mints.** Two independently-asserted values for the same
`(subject, predicate)` are never a conflict to silently resolve — they're
both true until something governs and arbitrates between them
(`DMML.Governance`), or they just stay multi-valued indefinitely if
nothing does. This is the mechanism `sync-spike/`'s cross-player
divergence detection is built entirely out of: materialize both sides,
union the snapshots, report what's still multi-valued after governance.

**A citation is only as strong as what it's checked against.** `consumes`
citing `uri#cid` is checked two different ways depending on what's
actually known: a real, independently-observed identity (a file
genuinely present in the batch being checked, via `DMML.LocalIdentity`)
is a strong check; a uri with no local file falls back to first-citation-
wins, the same real, disclosed-as-weak check the retired production
scheme had. Never conflate the two, and never treat the weak case as if
it were the strong one.

**Firing gates against the whole known machine set, not just the one
transition being fired.** `DMML.Fire.fireTransition` takes every machine
the caller knows about and checks the proposed effects wouldn't strand
some OTHER machine's currently-held guard — a transition that looks
locally sound can still break global consistency, and the gate exists
specifically to catch that before it's committed, not after.

**Every binary is a real, runnable, dogfooded artifact, not a design
sketch.** `dmml-hs/examples/` holds real `.dmml` fixtures for every
non-trivial feature (chained retract, value-qualified retract, citation
integrity's both real cases), each verified by actually running the real
binary against it and checking the real output — not by argument.

## What's deliberately not here

- No substrate abstraction. One real substrate (atproto), talked to
  directly.
- No real cryptographic content-addressing (`DMML.LocalIdentity` is
  explicit about this).
- `DMML.Atproto.resolveDidToPdsEndpoint` handles `did:plc` only —
  `did:web` is a deliberate scope boundary (see `written-world/README.md`),
  not an oversight.
