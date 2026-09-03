# Citation verification — commons/biopolitical-production extension, 2026-09-03

Real sources fetched directly this session (WebFetch on the primary essay;
a real, page-cited academic summary PDF for Commonwealth, read via the
Read tool's PDF extraction after WebFetch's own text extraction failed on
the binary stream) — not recalled from training. Matches this repo's
established per-date verification convention (see the 2026-09-02
distributed-agents file for the prior instance of this discipline).

Context: Jason's framing, unprompted, after the de-prose agent's
confabulation incident and the paper's new "DMML as evidence, not
self-narration" paragraph — "we're building truth as a commons in the
Wittgensteinian sense, extending Hardt and Negri's concept of
biopolitical production to machine minds," then, when I raised the real
tension between this and the paper's already-established refusal to
claim cognition/desire in any participant: "I'll own the extension - but
dmml being a protocol with a reference implementation instead of a
platform is the basis for the move."

## Hardt and Negri, *Commonwealth* (2009) — via Thomas Allmer's summary

**Grade: ✅ Verified, but one level removed from primary text.** Allmer's
summary (hosted at thomasallmer.net, an academic's own site, not a
predatory/content-farm source) quotes Hardt and Negri directly with page
numbers for every claim below, which is strong evidence the quotes are
real and accurately attributed — but it is a secondary summary, not the
book itself. Treat the quotes as verified; treat Allmer's own framing/
selection as one scholar's reading, not as equivalent to reading
*Commonwealth* cover to cover.

Real, page-cited quotes:

- **"The common," defined (p. viii)**: "the common wealth of the
  material world — the air, the water, the fruits of the soil, and all
  nature's bounty — which in classic European political texts is often
  claimed to be the inheritance of humanity as a whole, to be shared
  together. We consider the common also and more significantly those
  results of social production that are necessary for social
  interaction and further production, such as **knowledges, languages,
  codes, information, affects**, and so forth." [emphasis added] This is
  the load-bearing quote for the extension: Hardt and Negri's own
  paradigm case for "the common" is explicitly linguistic/informational,
  not just material — a protocol/grammar for representing and producing
  shared content is squarely inside their own stated category, not an
  analogist's stretch onto it.
- **Biopolitical production and its aim (p. 128)**: "The overall aim is
  a collective practice not of being common, but of **making** common."
  Active, ongoing production of commonality, not a static shared
  resource — this maps closely onto what a live, continuously-authored
  DMML world actually is (a growing graph, not a fixed corpus).
- **Capital vs. the common (p. 132)**: "current capitalist accumulation
  expropriates and destroys the common." Named directly as the real
  tension Jason's framing has to answer, not avoid — see below.
- **Exodus (p. 152)**: class struggle as "a process of subtraction from
  the relationship with capital by means of actualizing the potential
  autonomy of labor-power." Central to Hardt and Negri's own political
  argument; explicitly NOT part of what Jason is extending (see the
  disanalogy section below).
- **The multitude as a network (Allmer's footnote 1, quoting *Multitude*
  2004, p. xiii–xv)**: "a distributed network such as the Internet is a
  good initial image or model for the multitude... the various nodes
  remain different but are all connected... the external boundaries of
  the network are open such that new nodes and new relationships can
  always be added." Real and useful: Hardt and Negri themselves reach
  for a network/protocol-shaped image for the multitude, independent of
  anything Jason or this project introduced — a genuine bridge, not a
  connection invented from scratch to fit the argument.

## Mike Masnick, "Protocols, Not Platforms: A Technological Approach to
Free Speech" (Knight First Amendment Institute, 2019)

**Grade: ✅ Verified, primary source, fetched directly.**
`knightcolumbia.org/content/protocols-not-platforms-a-technological-approach-to-free-speech`

Real quotes: the central thesis is to "push the power and decision
making out to the ends of the network, rather than keeping it
centralized among a small group of very powerful companies"; platforms
work by "locking users in, rather than merely providing an interface";
the protocol alternative is illustrated through email (SMTP/IMAP) —
open standards nobody owns, so no single implementation's failure or
enclosure removes anyone's ability to participate. **One honest
correction to my own initial framing**: Masnick's essay does NOT use
"commons" or "enclosure" as its own vocabulary — that framing is a real,
independent strand of protocol-vs-platform commentary (found via the
same search, e.g. Guggenberger's "Platform-Property Paradox"), not
something to attribute to Masnick specifically. Cite Masnick for the
protocol/platform distinction and its decentralizing effect; don't put
"enclosure" language in his mouth.

## What's genuinely established vs. what's Jason's extension

**Established, directly from the sources above**: (1) Hardt and Negri's
own definition of "the common" is centered on linguistic/informational
production, not only material resources — a real, close fit for a
protocol/grammar object, not an analogist's reach. (2) The
protocol-vs-platform distinction is real, independently documented
commentary (Masnick, and the broader literature it sits in) about how
open, unowned protocols resist the specific kind of enclosure a single
controlled platform enacts, structurally distinct from whether any one
implementation of that protocol is itself privately held.

**Jason's extension, stated as his, not theirs**: using the
protocol/reference-implementation distinction to answer the real
objection I raised (that `dmml-hs` and this repo are, in the ordinary
sense, privately held and thus already "enclosed") — the object that is
common, on this reading, is DMML *the grammar/protocol* (checkable,
fixture-testable, per `written-world/SPEC.md`'s own "A/B razor": "if a
rule can be expressed as a pass/fail fixture, it's DMML; if it can't, it
isn't protocol" — real, pre-existing project language, not invented for
this argument), not any one implementation of it. Nobody's private
`dmml-hs` checkout owning the protocol is exactly as true or false as
nobody's private mail server owning SMTP.

**The real disanalogy this doesn't resolve, stated plainly rather than
smoothed over**: Hardt and Negri's entire political apparatus — exodus,
the multitude's capacity for resistance, biopolitical production's
value as a site of struggle against capital — presupposes subjects
capable of being exploited and of organizing against it. Paper 2 has
already, deliberately, refused exactly that register for DMML's
participants ("nothing here wants anything," Section 10). The
protocol/platform move rescues the *enclosure* objection (the common
object isn't owned even if one instance of it is); it does nothing to
rescue the *subject* objection, and doesn't need to, if the extension
being made is narrower than Jason's own words first suggested: not "the
models are an exploited multitude," but "the protocol-not-platform
structure is what lets machine-authored linguistic/informational
production accumulate as commons rather than as any one party's private
product, independent of whether the producers are subjects in Hardt and
Negri's sense at all." That's a real, defensible, narrower claim — worth
stating as narrower explicitly in the paper, not left ambiguous, so it
doesn't read as smuggling in the political-subject apparatus by
association with the citation.
