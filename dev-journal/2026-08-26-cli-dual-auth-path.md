# CLI needs two auth paths, not one — human vs. agent caller (2026-08-26)

Last item in today's identity-binding thread. First pass at "same
crate, same path for CLI" read as CLI adopting OAuth outright, same as
Android. Corrected immediately: that's the right answer for a human
running CLI locally, but CLI has a second, genuinely different caller
this project already has direct experience with — an agent (a Claude
Code session, this one included) driving CLI from inside a sandbox or
container, with no interactive browser available to pop an OAuth
redirect through at all. Not a degraded version of the OAuth flow for
that caller; an actually-unavailable one, regardless of which crate
implements it.

So CLI's auth is two paths, selected at runtime by whether a browser is
actually available, not one decision:

- **Human-interactive CLI → OAuth**, same `atproto-identity`/
  `atproto-oauth` crate and same public-client shape as Android,
  substituting a loopback-HTTP-server redirect (the `gh auth login`
  pattern) for Android's App Link.
- **Sandboxed/agent-driven CLI → app-password**
  (`com.atproto.server.createSession`) — a different, weaker trust and
  revocation model, accepted specifically because it's the only one
  that actually works with no browser, not chosen for convenience over
  OAuth.

Both converge on the same thing CLI actually needs regardless of path:
an authenticated session to write the DID↔iroh binding record and its
own checkpoint commits through the existing `swapCommit`-gated write
path.

Updated `ARCHITECTURE.md`'s CLI paragraph and the corresponding "Open
design work" bullet to reflect the dual-path decision in place of the
single still-open question from the previous pass.
