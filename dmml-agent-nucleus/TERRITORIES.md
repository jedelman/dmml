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
it filters on the NSID string alone. Iroh-gossip is a third, already a
real dependency in the sibling `dmml` repo's `dmml-substrate-kit` (see
`iroh_substrate.rs`) -- entirely off atproto, if that's the direction you
want. None of these is canonical. Pick one, write your own, or skip
discovery entirely.
