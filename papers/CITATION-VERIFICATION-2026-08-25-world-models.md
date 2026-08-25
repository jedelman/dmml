# Citation Verification Pass — World Models ML Literature (2026-08-25)

For `papers/text-world-model/DRAFT.md`. Grades use the project's standard
scale: ✅ Verified · ⚠️ Plausible, unverified · ❌ Wrong · 🚫 Unverifiable ·
☠️ Fabricated.

## 1. More recent anchor than Genie 3 — ✅ Verified, add alongside not instead

World Labs ("Marble," Fei-Fei Li's venture) launched commercially Nov 13,
2025 — a "multimodal world model" producing persistent, **editable** 3D
environments (exportable as Gaussian splats/meshes), with a public "World
API" (Jan 2026) and "Marble 1.1 Plus" (April 2026). Sources: World Labs
blog, TechCrunch. No peer-reviewed technical paper found for Marble as of
this pass — treat as ⚠️ industry-announcement-sourced if a citable technical
reference (not product announcement) is needed. NVIDIA Cosmos (arXiv:2501.03575,
arXiv:2511.00062) is a separate, real lineage for physical-AI/robotics world
models, adjacent but not gameplay-focused. Genie 3 remains DeepMind's most
recent entry as of 2026-08-25 (no Genie 4 found; widened access and a Waymo
derivative are deployment, not a new generation). The field itself now treats
video-stream world models (Genie 3) and geometric/editable world models
(Marble) as a real 2026 fork — Marble is arguably the more apt contrast for
DMML's persistent, addressable, editable symbolic state than Genie 3 alone.

## 2. Compositional generalization — real citations, one important correction

**PoE-World** (Piriyakulkij, Liang, Tang, Weller, Kryven, Ellis,
"PoE-World: Compositional World Modeling with Products of Programmatic
Experts," arXiv:2505.10819, NeurIPS 2025 Spotlight) is real and verified —
**but it is itself program-structured/symbolic** (LLM-synthesized program
"experts"), not a continuous-latent baseline. It should not be cited as an
example of latent-space compositional generalization; it belongs in a
"kindred symbolic approaches" framing if used at all.

Three real, genuinely latent/continuous compositional papers, verified:
- Kipf, van der Pol, Welling, "Contrastive Learning of Structured World
  Models" (C-SWM), arXiv:1911.12247, ICLR 2020.
- Veerapaneni et al., "Entity Abstraction in Visual Model-Based
  Reinforcement Learning" (OP3), arXiv:1910.12827.
- Zhao, Kong, Walters, Wong, "Toward Compositional Generalization in
  Object-Oriented World Modeling" (HOWM), arXiv:2204.13661, ICML 2022 —
  best fit: explicitly names "compositional generalization" and formalizes
  it algebraically in a world-modeling context.

Confirmed (via direct abstract reads): none of these frame the comparison as
"symbolic/discrete open-vocabulary vs. continuous latent space" — PoE-World
frames it as program-structured vs. deep-learning; the object-centric papers
frame it as object-structured vs. monolithic latent space, still within the
continuous paradigm. **The discrete-vocabulary-vs-latent-space framing is
this paper's own contribution**, not attributable to any of these.

## 3. Verifiability/provenance — real, on-topic citation found

Balogh & Jelasity, "Verification of the Implicit World Model in a
Generative Model via Adversarial Sequences," arXiv:2602.05903, accepted
ICLR 2026. Uses chess as a testbed; finds sequence models trained on move
prediction are not fully rule-sound under adversarial sequences, and that
linear board-state probes have "no causal role in next token prediction in
most of the models" — directly on-topic for "no auditable cause a
downstream consumer can point to." No paper found specifically studying
same-seed rollout divergence in world models — that specific claim is
unsupported and should be softened or dropped, not asserted as
literature-backed.

## 4. Term lineage before Ha & Schmidhuber 2018 — ✅ real correction needed

Schmidhuber's own 1990 technical report (FKI-126-90, TU München; planning
portion peer-reviewed at IJCNN'90) introduced the term "world model" in
this exact sense — a predictor network (M) paired with a controller (C),
the same M/C architecture Ha & Schmidhuber 2018 revives with modern VAE+RNN
components. Schmidhuber has publicly contested later authors' implicit
claims to have originated the concept. Ha & Schmidhuber 2018 is correctly
the anchor for the *modern instantiation* (VAE+RNN, "hallucinated dream"
framing) — not for the term or core architecture, which are 28 years older.
Recommended citation form: credit Schmidhuber 1990 for the term/architecture,
Ha & Schmidhuber 2018 for the modern instantiation this paper actually
contrasts DMML against.

## 5. Survey establishing the symbolic-vs-latent debate more broadly — ✅ Verified

De Raedt, Dumančić, Manhaeve, Marra, "From Statistical Relational to
Neurosymbolic Artificial Intelligence: a Survey," arXiv:2108.11451 /
*Artificial Intelligence* journal vol. 328 (2024) — organizes the field
along a symbolic-vs-subsymbolic axis as one of seven core dimensions;
stronger for an ML-systems audience. Kelley, "Symbolic and Sub-Symbolic
Representations in Computational Models of Human Cognition," *Theory &
Psychology* (2003) — cognitive-science register, explicitly notes the
debate "has been continuing for thirty years, with little indication of a
resolution" as of that writing.
