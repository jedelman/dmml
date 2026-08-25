# Nothing is fixed or final: self-dispute as a first-class move (2026-08-25)

Jason's observation, made in conversation while reviewing the multi-model
prose-editing pipeline (ox-alpha reviews, Gemini prose passes, Deepseek
prompt critique) this session actually ran by hand: an open ontology
doesn't just let a *different* author dispute a prior resolution — it
lets the *same* author go back in and dispute or alter their own earlier
resolution. Nothing is fixed or final, including your own prior verdicts,
which is a real, checkable property of DMML's grammar, not just a
philosophical flourish: there is no primitive that marks a commit as
closed to further revision by its own author or anyone else. A
resolution commit is exactly as revisable as any other fact.

This connects directly to the same day's metacognitive discussion of
applying DMML's own ontology to the editorial workflow that produced it:
each dispatch (ox-alpha review, Gemini suggestion, Deepseek critique) is
structurally a petition/resolution cycle already; formalizing it as real
DMML commits (rather than prose dev-journal entries) would make each
model's actual output, and Dev Lead's actual accept/reject/revise
decisions, permanent, independently-citable facts — and, per this note,
would let Dev Lead's own prior "accepted" or "rejected" verdicts be
reopened and revised later without erasing the original, the same way
`pantheon.rs` already demonstrates for Helios/Selene/Eos's rival origin
claims.

`dmml/examples/editorial_loop.rs` builds this concretely: a suggestion
from one identity, a first resolution from Dev Lead, and a SECOND
resolution — same Dev Lead identity, later commit — that disputes and
revises the first, consuming it by `FactRef` rather than ignoring it.
Checked: the original resolution remains fully present and independently
re-materializable after being superseded in the current view, exactly
the "captured by neither, only built upon" property `pantheon.rs`
established for rival first-order claims, now shown to hold for an
author's dispute with their own past self.
