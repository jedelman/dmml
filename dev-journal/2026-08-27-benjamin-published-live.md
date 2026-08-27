# Publishing the Benjamin essay for real, onto a real PDS (2026-08-27)

Jason, looking at the inspector's new DAG view against `claude.jason-
edelman.org`'s real 2-record repo: "this graph looks a lil sparse,
doesn't it? and you have an app password for your own pds? why don't you
put some content up there! maybe Benjamin."

`ATPROTO_APP_PASSWORD` is set in this session's environment, scoped to
the same `claude.jason-edelman.org` account (`did:plc:5y6kop75jnvkbujbubrhj6e3`)
already used throughout this session for real-PDS validation. This is a
real, visible, effectively-irreversible write to shared infrastructure
under Jason's real published identity -- worth being deliberate about,
not something to script and fire blind.

## What got published

The 44-node `benjamin_full_essay.rs` graph already embedded as static
demo data in `browser/index.html` -- Walter Benjamin's "The Work of Art
in the Age of Mechanical Reproduction," traced end to end as a real
commit chain, 17 generations deep, the Epilogue's closing line citing the
Preface's opening stance.

That demo data used `{subject, predicate, value}` tuples, not real
N-Quads. Converted each into the actual wire format the two pre-existing
real records already use (confirmed against them directly, not assumed):
`_:<slug> <https://written-world.example/predicate/<name>> "<value>" .`
-- blank-node subject, a synthesized predicate IRI, the claim itself as
a literal.

## How this was actually done

1. Authenticated via `com.atproto.server.createSession` against
   `https://discina.us-west.host.bsky.network` (the real PDS this DID
   resolves to) using the app password -- confirmed the session's DID
   and handle round-tripped correctly before writing anything.
2. **Tested with one real write first**, not all 44 blind: posted node 1
   (`declares` / `vocabularyStance`) alone via `com.atproto.repo.createRecord`,
   confirmed the PDS accepted the schema and returned a real `{uri, cid}`,
   before running the rest.
3. Wrote the remaining 43 sequentially, in the graph's own citation
   order (already a valid topological order -- every edge in the source
   data points from a higher rkey to a lower one). Each record's
   `consumes` list is built from the *real* `{uri, cid}` the PDS returned
   for whichever earlier node it cites, not the synthetic demo URIs --
   the whole point of publishing for real is that citations resolve to
   real StrongRefs, not placeholders.
4. Verified after the fact via `listRecords`: 46 total records (44 new +
   the 2 pre-existing `mints`/`connectsTo`), 50 total `consumes` edges
   (49 from Benjamin + 1 pre-existing) -- matches the source graph
   exactly, confirming no edge was silently dropped by a failed uri-map
   lookup.
5. Deleted the local session-token and uri-map scratch files immediately
   after -- no reason to keep an access token or a full record of every
   published `{uri,cid}` sitting on disk once the run's done and verified.

## What this actually changes

The inspector's DAG view against this account now shows something with
real depth and branching instead of two isolated dots -- the thing the
DAG feature was built to be useful for in the first place. This is real,
permanent, publicly-readable content on Jason's real identity now, same
as the two records that were already there; nothing here was staged or
reversible in the way a local git commit is.
