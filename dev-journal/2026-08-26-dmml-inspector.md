# A pdsls-style DMML inspector, and a real bug it caught (2026-08-26)

Jason: "let's set up a basic web deployment just to test dmml - think of
it like pdsls." Different from everything else this session — the
`browser/index.html` tool visualizes static, embedded example data;
this one needed to make real, live calls to real atproto infrastructure.

**Verified the CORS story before building anything**, since it decides
whether this can be a pure static page at all: `plc.directory` and a
real PDS (`bsky.social`) both serve `access-control-allow-origin: *` on
their public read endpoints, confirmed directly via `curl -I`. Same
property pdsls.dev itself relies on — no backend needed, identity
resolution and record listing both work from a browser calling the real
services directly.

**Found written-world's own real test fixture** rather than inventing
one: `claude.jason-edelman.org` / `did:plc:5y6kop75jnvkbujbubrhj6e3`, a
dedicated account already used for real-PDS validation
(`written-world/dev-journal/2026-08-18-real-pds-validation.md`).
Confirmed it's still live and still holds two real
`org.jason-edelman.writtenworld.commit` records from 2026-08-11 (a
`mints` and a `connectsTo` commit) — set it as the page's default target
so there's always something real to look at on load, not an empty shell.

Built `inspector/index.html`: identity resolution (DID doc, handle,
PDS endpoint), a collection picker (`describeRepo`), a paginated record
list (`listRecords`), and a detail pane with both raw wire JSON and a
parsed DMML view — consumes/via/respondsTo as clickable jumps to the
cited record when it's on the same page, `produces` parsed from N-Quads
into a plain triple table. Reused `browser/index.html`'s existing token
system (same sediment/strata palette, same Fraunces + IBM Plex pairing)
for brand consistency across the two sibling tools rather than inventing
a new one.

## The proxy problem, and the real bug underneath it

Tried to verify this live in a headless Chromium the way every prior
UI change this session was tested. Hit a wall: this sandbox routes all
outbound HTTPS through a proxy `curl` picks up from `HTTPS_PROXY`
automatically, but Chromium doesn't — it needs an explicit
`--proxy-server` launch argument. Configured that, and the page still
hung indefinitely on every external fetch with no error at all. Spent
real effort chasing this (bypass lists, `--ignore-certificate-errors`,
serving over local HTTP instead of `file://` to rule out a same-origin
restriction) before accepting it as a sandbox-specific Chromium+proxy
interaction, not a bug in the page — `curl` reaching the exact same
endpoints throughout confirms the target services and the page's own
request shapes are fine.

Switched strategy rather than giving up on real verification: captured
the actual, real HTTP responses (`curl`) for every call the page makes
against the real `claude.jason-edelman.org` account, then used
Playwright's route interception to fulfill the page's real `fetch()`
calls with that real, captured data — genuine wire content, not a
synthetic fixture, exercised through the real parsing/rendering code.

**This caught a real bug the "it probably works" version would have
shipped**: the router needed exact URL matching to work at all
(Playwright checks routes in reverse-registration order — a broad
catch-all registered after a specific route shadows it, learned by
watching requests silently fall through to a catch-all instead of the
intended mock). Once request tracing was in place and showed the
`plc.directory` call being sent as
`https://plc.directory/did%3Aplc%3A5y6kop75jnvkbujbubrhj6e3` instead of
the real, working `https://plc.directory/did:plc:...` (confirmed
correct via the original `curl`), the actual bug was obvious:
`encodeURIComponent(did)` on a value whose colons `plc.directory`'s real
API needs literal. Would have 404'd for every real user; caught only
because the mocked-but-real-data test surfaced the mismatched URL, not
by re-reading the code.

Fixed, then re-verified the full pipeline against the same real,
captured data: identity resolution, collection listing, both real
records, N-Quads-to-table parsing (blank nodes, IRIs, literals all
render correctly), consumes/via/respondsTo ref-link navigation between
the two real records, raw JSON view, both themes. All correct.

## What's honestly not verified

The browser's own live-network path in this specific sandbox — a
Chromium/proxy interaction, not a property of the page itself. Every
URL the page constructs has been confirmed correct against the real
service via `curl`, and the full parsing/rendering pipeline has been
confirmed correct against that same real, captured data in a real
browser. What's untested end-to-end is specifically "does Chromium's
own network stack, unassisted, complete these fetches" — genuinely
different from "is the page's logic correct," and worth being honest
about rather than claiming a false all-clear.
