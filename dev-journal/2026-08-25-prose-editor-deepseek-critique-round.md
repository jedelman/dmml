# Prose-editor prompt improved via Deepseek critique, third Gemini pass (2026-08-25)

Per Jason's direction, dispatched the prose-editor's actual dispatch
prompt (the one sent to Gemini for round two) to `deepseek/deepseek-v4-flash-0731`
for critique before running it through Gemini again, rather than just
re-running the same prompt.

Deepseek's diagnosis was sharp and correct: the rule treated "tics" as a
surface-syntax phenomenon (specific phrases, "rather than"/"not X, Y"
constructions) when the real distinction is functional, not syntactic.
The prompt gave exactly one example of "real content, don't cut"
("relative, not absolute deterritorialization" — a technical contrast),
but the three suggestions the reviewer had rejected in round two were a
different, unlabeled category entirely: epistemic-honesty flags and
disambiguations. Both look identical on the surface to a tic (a
qualifying clause after a claim); nothing in the prompt taught the model
the functional difference.

Deepseek proposed a concrete four-category taxonomy — (a) technical
contrasts, (b) epistemic-honesty flags, (c) disambiguations preventing a
known misreading, (d) genuine tics — plus a functional test ("if cutting
the clause leaves the sentence making the same claim with the same
force, it's a tic; if cutting it would overstate, understate, or misstate
scope/provenance/evidentiary status, it's real content"), a bias-toward-
under-cutting instruction, a warning that instances surviving a prior
pass are not automatically "missed tics" (some are deliberate keeps), and
a requirement that per-suggestion output include category + confidence
rather than just a plausible-sounding one-line reason.

Folded all of this into `.claude/agents/prose-editor.md`'s standing house
rule (durable, not just this dispatch's prompt) and re-ran Gemini 3.7
Flash with the improved prompt as a third pass on both papers.

**Result: one proposed edit, not seventeen.** The improved taxonomy
visibly worked — Gemini's own "borderline instances left alone" section
(7 entries) showed correct category reasoning for exactly the kinds of
passages that were wrongly flagged in round two (epistemic-honesty flags
on scholarly provenance, disambiguations correcting plausible
misreadings, technical contrasts). The one proposed edit (Paper 1,
Section 5's closing sentence, "...is the same discipline the rest of
this paper practices") was itself a mixed case: correctly identified as
mostly tic, but its first half ("Declining to answer it here, rather than
reaching for a plausible-sounding claim unsupported by data") was doing
real work naming the paper's actual methodological choice, matching a
pattern of explicit self-restraint statements kept deliberately
throughout both papers. Applied a modified cut — kept the substantive
half, cut only the self-referential comparison clause — rather than
either Gemini's full cut or a flat rejection.

Net effect of the improved prompt: precision went up sharply (1/1 useful
suggestion vs. 14/17 in round two, with the 3 rejects this time reduced
to zero outright errors) without needing more review effort — the
opposite of the usual precision/recall tradeoff, because the earlier
round's imprecision was a prompt defect, not a fundamental model
limitation.
