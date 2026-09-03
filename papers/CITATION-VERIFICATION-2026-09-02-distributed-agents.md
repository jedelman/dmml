# Citation Verification Pass — Distributed/Multi-Agent World Models (2026-09-02)

For `papers/text-world-model/DRAFT.md`'s revised thesis (Jason, 2026-09-02:
"we're now not talking about world models per se, but distributed world
models composed of heterogeneous and ephemeral agents. if anything we've
built a meta-agent"). Grades use the project's standard scale: ✅ Verified
· ⚠️ Plausible, unverified · ❌ Wrong · 🚫 Unverifiable · ☠️ Fabricated.
All four below fetched directly (arXiv abstract pages), not recalled.

## 1. "Distributed/decentralized world models" already has a real, different ML meaning — ✅ Verified

Toledo & Prorok, "CoDreamer: Communication-Based Decentralised World
Models," arXiv:2406.13600 (June 2024). Abstract, fetched directly:
extends the single-agent Dreamer algorithm to multi-agent RL; each agent
keeps its **own learned world model**, with a two-level Graph Neural
Network communication system reconciling them — one channel inside each
agent's world model, a separate one inside each agent's policy.

**Load-bearing for the paper's contrast, not just background**: in this
literature, "distributed world model" means N agents, each with a private
*learned latent* model, synchronized by a *learned communication channel*
that is itself part of the trained system. DMML's own distributed world
model is the structural opposite on both axes — one shared, symbolic,
content-addressed substrate (not N private latent models), and consistency
maintained by *declarative, inspectable mechanism* (governance arbitration,
retroconsistency, gating — all real, checkable code, not a learned channel)
rather than anything trained. Same functional problem (how do independently-
perspectived agents converge on one coherent world), structurally opposite
answer — the same "shares a name, different job" move Section 1 already
makes for "world model" itself, one level up.

## 2. "Meta-agent"/emergent collective structure — ✅ Verified, real and current

Riedl, "Emergent Coordination in Multi-Agent Language Models," ICLR 2026,
arXiv:2510.05174. Fetched directly. Central question, quoted: "When are
multi-agent LLM systems merely a collection of individual agents versus
an integrated collective with higher-order structure?" Finding: "multi-
agent LLM systems can be steered with prompt design from mere aggregates
to higher-order collectives" — an information-theoretic measure of
whether coordination is genuinely present, not assumed from the setup.

**Real, important disanalogy to state plainly, not paper over**: Riedl's
"higher-order collective" is *emergent* — a statistical property measured
after the fact across many agent interactions, steerable but not
designed-in. Whatever "meta-agent" behavior DMML's substrate exhibits is
the opposite in kind: mechanistically designed and checkable (a governed
transition either fires or it doesn't; `gateConsistentTree` either finds
a broken guard or it doesn't), not a statistical signature recovered from
interaction logs. Citing Riedl to say "we also have emergent higher-order
coordination" would overclaim — the honest claim is narrower and, if
anything, stronger: DMML's coordinating structure doesn't need to be
measured for because it's inspectable by construction. State this
contrast explicitly if Riedl is cited, not as a rebuttal but as the real
distinction between two different ways a system can be "more than an
aggregate of its agents."

## 3. General multi-agent LLM taxonomy anchor — ✅ Verified

Tran, Dao, Nguyen, Pham, O'Sullivan, Nguyen, "Multi-Agent Collaboration
Mechanisms: A Survey of LLMs," arXiv:2501.06322 (Jan 2025). Fetched
directly. Taxonomizes collaboration along five axes: actors, types
(cooperation/competition/coopetition), structures (peer-to-peer/
centralized/distributed), strategies (role-based/model-based), and
coordination protocols. Useful as a real, citable frame to place DMML's
own coordination structure against precisely, not just gesture at it:
structurally peer-to-peer (no central authority; each player's own repo
is sovereign), cooperative by default with disputes surfaced rather than
suppressed (collision-free mints), and coordinated by a *shared substrate
protocol* (git + DMML's own guard/governance/retroconsistency grammar)
rather than a communication protocol between agents' own private states.

## 4. Honest limitations of distributed multi-agent systems generally — ✅ Verified, real and useful for the "what this doesn't solve" section

Zhang, Li, Zhao, Zhu, Wang, Vasconcelos, "Achilles Heel of Distributed
Multi-Agent Systems," arXiv:2504.07461 (April 2025). Fetched directly.
Identifies four trustworthiness vulnerabilities in distributed multi-
agent systems where heterogeneous third-party agents act as remote
service providers: free riding, susceptibility to malicious attacks,
communication inefficiency, system instability. Real attack results
quoted: "up to 80%" performance degradation, "100% success rate" for
free-riding and malicious-attack strategies tested across seven
frameworks and four datasets.

**Real, checkable claim about which of these DMML's own design actually
touches, which it doesn't** — worth stating honestly rather than
assuming the paper's own architecture is immune by default:
- *Free riding*: not directly addressed. Nothing in DMML's grammar
  requires an agent to contribute value proportional to what it consumes;
  a free-riding agent can read and cite the whole graph while never
  producing anything real.
- *Malicious/fabricated content*: partially addressed, narrowly. A
  fabricated `consumes` reference is checkable (Section 3's own claim) —
  a reader can always ask what a commit claims to depend on and get a
  literal answer — but checkability is not prevention; nothing stops the
  fabrication from being *written* in the first place, only from being
  *silently believed*.
- *Communication inefficiency*: not applicable in the same shape — there
  is no learned communication channel to be inefficient (see item 1
  above), though a real, different cost exists (checkpoint-per-commit's
  own full-fact-store re-evaluation on every merge, `dmml`'s own recent
  work).
- *System instability*: this is closest to what retroconsistency's
  whole-tree gate (`gateConsistentTree`) actually targets — not stability
  under adversarial load, but a real, narrower, verified property:
  adding a fact can't silently break an unrelated, already-satisfied
  guard elsewhere in the tree.

None of the four is "solved" by DMML's existing design; at most one
(instability, narrowly) has real, tested mitigation. Cite this paper to
be honest about the gap, not to claim it's closed.
