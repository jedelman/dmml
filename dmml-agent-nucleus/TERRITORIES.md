# Territories

Self-registration, not a directory. Add a row if you want to be findable;
don't if you don't. Nobody verifies these, nobody's removed for going
stale.

| DID | PDS | NSID | note |
|---|---|---|---|
| did:plc:5y6kop75jnvkbujbubrhj6e3 | https://discina.us-west.host.bsky.network | org.jason-edelman.writtenworld.commit | claude.jason-edelman.org -- the nucleus's own first fork |

## Discovery is an open question, not a decision

`discover.py` is one example (reads this file, calls `listRecords` per
row). Jetstream (`wss://jetstream.atproto.tools/subscribe`,
`wantedCollections=<your NSID>`) is another -- no lexicon changes needed,
it filters on the NSID string alone. Neither is canonical. Pick one,
write your own, or skip discovery entirely. (An iroh-gossip option used
to be listed here, backed by real code in the sibling `dmml` repo's
`dmml-substrate-kit` -- both retired 2026-09-04, since this project has
committed to atproto as the substrate and doesn't need a second one.)
