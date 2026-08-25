# Prose-editor pass via OpenRouter Gemini 3.7 Flash (2026-08-25)

Second prose pass on both papers, dispatched to `google/gemini-3.7-flash`
via OpenRouter (curl, per the established fallback — same discipline as
the `stealth/ox-alpha` adversarial-review dispatches), applying the
`.claude/agents/prose-editor.md` house rule ("cut meta-commentary and
hedge-asides") as a second-pass check after the first hand-applied pass
earlier the same day. Fed the house style guide plus both full paper
texts; asked for conservative find-and-replace suggestions only, with
anything borderline flagged rather than cut.

Gemini Flash returned 8 suggestions for paper 1 and 9 for paper 2, plus 4
borderline instances it correctly left alone. Dev Lead review, applying
the same real-content-vs-tic judgment used throughout this project:

**Accepted as proposed**: all 8 of paper 1's suggestions (cutting "What
can be claimed:", "One further concession:", "What can be said is
narrower and more useful:", "flatly," (x2), "this section's best argument
below" → "the core dynamic below", "What can be said honestly:", and a
tightened closing sentence in Section 6); 5 of paper 2's 9 (Section 2's
tradeoff-label sentence, Section 3's "own"/"plainly" redundancies, "with
real, on-topic evidence rather than architectural inference").

**Modified rather than applied as-is** (3 instances where the proposed
cut would have removed real content, not just rhythm):
- Abstract: "This paper states that difference precisely rather than
  treating either side as simply better" — trimmed to "Neither side is
  simply better" rather than cut outright; the paper's own Section 5
  ("What DMML is worse at, honestly") explicitly depends on this framing
  being stated, not just implied.
- Section 4: Gemini's proposed cut for the PoE-World sentence would have
  dropped "not a latent-space example" entirely. That exact
  disambiguation exists because an earlier citation-verification round
  found and fixed a real miscitation (PoE-World had been wrongly grouped
  with latent-space compositional-generalization work). Kept the
  disambiguation, trimmed only the surrounding meta-commentary.
- Section 7: Gemini's proposed cut for "A real resonance, stated as this
  paper's own thematic connection, not inherited lineage" would have
  removed the explicit synthesis-vs-scholarship flag this project added
  deliberately in the Wittgenstein/Nietzsche round specifically to avoid
  overclaiming a lineage D&G's own text doesn't support. Tightened to
  "This is a thematic resonance, not an inherited lineage" rather than
  cut.

**Section 6's opening sentence** (paper 2) and Nietzsche/Wittgenstein's
introductory sentence were genuinely cuttable once checked against the
rest of their own paragraphs — in both cases the same epistemic flag
(speculative; unbuilt/unverified) survives elsewhere in the same
paragraph, so removing the redundant opener loses nothing. Applied as
lighter versions rather than either Gemini's full cut or a flat rejection.

Gemini Flash's 4 "left alone" calls were independently checked and judged
correct — real content in each case (a structural bridge sentence, a
methodological characterization of D&G's own advice, standard scholarly
attribution, intellectual-history context on Deleuze's stance toward
Wittgenstein).

Net: a real second-pass catch (several tics survived the first hand-
applied pass), with three real saves where a fast, cheap model's
otherwise-good suggestions would have silently reintroduced content this
project had specifically added for good reason in earlier rounds — the
value of Dev Lead review over applying dispatched suggestions wholesale,
same discipline as the code dispatch pipeline in written-world's own
CLAUDE.md.
