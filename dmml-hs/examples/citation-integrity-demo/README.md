# citation-integrity-demo

Real worked example for `check-citations` (jedelman/dmml#6). Three
tiny commits, no machines needed:

- `a.dmml` — mints `key/foo`'s `state` to `seen`.
- `b.dmml` — retracts it, citing an EXTERNAL commit (`at://external.
  example/repo/commit1#bafyfirstcid`) as the source — a uri this batch
  has no real file for, so there's nothing to independently check it
  against.
- `c.dmml` — retracts the same fact again, citing the SAME uri but a
  DIFFERENT cid (`bafyDIFFERENTcid`) — a real, internally inconsistent
  citation.

```sh
check-citations a.dmml b.dmml     # OK -- first (and only) citation of
                                  # that uri, nothing to disagree with
check-citations a.dmml b.dmml c.dmml  # CITATION MISMATCH -- c.dmml's
                                  # citation disagrees with b.dmml's
                                  # earlier one for the same uri
```

This demonstrates the "no local file, first-citation-wins" fallback
path specifically — see `DMML.CitationIntegrity`'s own doc comment for
the OTHER, stronger case (a citation naming a file that IS actually
present in the batch, checked against that file's own real
`DMML.LocalIdentity.localFileRef`, not just trusted on say-so). That
case doesn't need its own fixture here: any existing `fire-transition`
output already demonstrates it, since `renderFiredCommit` writes
`local:<path>#fnv1a64:<hash>` citations straight from the real
`--world`/`--machine` files given on its own command line — e.g.:

```sh
cd ../chained-retract-demo
fire-transition keeper.dmml witnessEruption witnesses \
  --world world.dmml --param eruption=volcano/ashkar > /tmp/fired.dmml
check-citations world.dmml /tmp/fired.dmml   # OK -- real citation
sed 's/fnv1a64:[0-9a-f]*/fnv1a64:deadbeefdeadbeef/' /tmp/fired.dmml \
  > /tmp/tampered.dmml
check-citations world.dmml /tmp/tampered.dmml   # CITATION MISMATCH
```
