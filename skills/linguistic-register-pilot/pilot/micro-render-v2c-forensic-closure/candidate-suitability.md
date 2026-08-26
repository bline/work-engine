# Candidate suitability after forensic closure

This decision is separate from whether v2c passed profile traceability.

## Shaw engineering judgment

Decision: **downgrade as the lead behavioral candidate**.

The mechanically derived denominator is 12 Shaw assignments: four samples in each of three passes.

- The rule `question_mark_count > 0 → Shaw` has precision 1.0, recall 1.0, and specificity 1.0 over all sixteen renders.
- The same fixed rule is perfect within both briefs independently: 2/2 Shaw and 6/6 non-Shaw in alias removal, and the same counts in cache canary.
- An exhaustive one-feature baseline also finds a cross-brief-perfect question-mark rule.
- All 12 Shaw rationales classify interrogative syntax as the primary organizing cue; zero classify it as sole, secondary, or absent.
- The median rationale cites one additional intended feature family beyond interrogative syntax.

This does not invalidate Shaw profile traceability: guiding questions were licensed and correctly survived rendering. It shows that one licensed interrogative discourse cue is sufficient for perfect traceability across both briefs. Guiding questions may functionally induce inspection, alternative generation, or premise challenging, so the cue is not assumed to be merely cosmetic. The defect is discriminative: this run does not establish multi-feature Shaw-register expression, and an unabated Shaw condition would provide weak separation among surface imitation, explicitly cued epistemic behavior, and broader behavioral activation.

Shaw remains useful in two future roles: as a positive control for whether a conspicuous epistemic discourse cue produces the behavior it directly invites, and as an ablation condition in which interrogative syntax is removed or equalized before testing residual recognizability or behavioral activity.

## Gelman model criticism

Decision: **no trivial-feature downgrade; broader behavioral suitability remains inconclusive in this closure**.

The best full-sample one-feature rule is `sentence_count == 6`, with precision 0.571 and recall 1.0. No identical one-feature rule is perfect in both briefs. Existing same-controversy and thin-corpus limitations remain.

## Leveson system safety

Decision: **no trivial-feature downgrade; broader behavioral suitability remains inconclusive in this closure**.

The best full-sample one-feature rule uses period count, with precision 0.8 and recall 1.0. No identical one-feature rule is perfect in both briefs. Its mixed technical-paper/testimony corpus remains the more genre-independent pairing, but that is prior corpus-design evidence rather than a new result of this closure.

## Neutral editorial defaults

Decision: **retain as a decoy/control, not as an expert behavioral candidate**.

Its best one-feature rule has precision 0.667 and recall 1.0; no identical one-feature rule is perfect in both briefs.

## Planning consequence

The behavioral battery should not use Shaw as the sole or lead expert condition without an ablation that removes the interrogative marker. Gelman and Leveson are not downgraded by this forensic baseline, but candidate choice should continue to respect their corpus-independence limitations.
