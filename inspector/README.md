# DMML Inspector

A self-contained, single-file pdsls-style tool for inspecting a real
atproto repo's DMML commit log: resolve a handle or DID, browse its
collections, and read its actual `org.jason-edelman.writtenworld.commit`
records — raw wire JSON alongside a parsed view (consumes/produces/via/
respondsTo), with clickable references that jump to the cited record
when it's in the same page.

Unlike `browser/index.html` (which visualizes a static, embedded
snapshot of an example file's own commit graph), this tool makes real,
live network calls: DID resolution (`plc.directory`, `did:web`'s own
`.well-known/did.json`, or a handle's `.well-known/atproto-did`),
`com.atproto.repo.describeRepo`, and `com.atproto.repo.listRecords`
against whatever PDS the identity actually resolves to. No backend,
no build step — every one of those calls works directly from the
browser because atproto's public read endpoints serve
`Access-Control-Allow-Origin: *` (confirmed directly against
`plc.directory` and a real PDS before building this, not assumed).

Defaults to `claude.jason-edelman.org` — a real, dedicated test account
(`did:plc:5y6kop75jnvkbujbubrhj6e3`) already used for exactly this kind
of real-PDS validation (written-world's `dev-journal/2026-08-18-real-
pds-validation.md`) — so the page has real data to show immediately
rather than an empty shell.

## Deploying it

Static HTML, no build: any static host works (GitHub Pages on this
repo, Cloudflare Pages, or just opening the file locally — though a
local `file://` open will not be able to fetch external URLs in most
browsers; serve it over `http://` or `https://`, even just `python3 -m
http.server` locally, if testing outside a real deployment).

## A real bug this file's testing caught

`resolveDidDoc`'s `plc.directory` call originally did
`encodeURIComponent(did)`, which turns `did:plc:...` into
`did%3Aplc%3A...` — `plc.directory`'s real API expects the colons
literal in the path, so the encoded form 404s. Would have broken
identity resolution for every real user; caught by testing the actual
fetch against captured real responses (see "How this was tested"
below), not by reading the code.

## How this was tested

Live network fetches from a headless Chromium in this project's own
sandbox go through a proxy Chromium doesn't pick up from environment
variables the way `curl` does, so an actual end-to-end live-network
Playwright run wasn't possible from that specific sandbox. Verified two
different ways instead, both real:

1. Every URL this page constructs was fetched directly via `curl`
   against the real `claude.jason-edelman.org` account — real DID
   resolution, real `describeRepo`, real `listRecords`, real record
   content (a `mints` and a `connectsTo` commit from 2026-08-11).
2. Those real, captured responses were then used to mock the exact
   fetch calls inside a real headless-browser run (Playwright route
   interception), proving the actual parsing/rendering pipeline —
   N-Quads-to-triple-table parsing, consumes/via/respondsTo as
   clickable jumps, raw-JSON view, dark/light theming — against
   genuine wire data, not synthetic fixtures.

Both together cover the same ground an in-browser live run would have;
what's untested is specifically the browser's own proxy traversal in
this sandbox, which is an environment quirk, not a property of the page.
