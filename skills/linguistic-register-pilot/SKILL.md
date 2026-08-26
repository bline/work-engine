---
name: linguistic-register-pilot
description: Build and validate profile-separability evidence for experimental linguistic-register transfer without changing canonical roles, production compilation, or execution infrastructure.
---

# Linguistic Register Pilot

Determine whether a recognizable expert-practice register survives semantic
filtering before creating rendered roles or running behavioral trials.

Keep canonical role meaning external and immutable. Bind every profile to the
exact role and mini-corpus artifacts it was judged against. Treat feature
extraction, semantic classification, recognizability, and final candidate
selection as model- or human-judged evidence; the deterministic validator owns
only artifact shape, compatibility, provenance, arithmetic, and preregistered
gate application.

Use `scripts/pilot_artifacts.py` to validate role, corpus, and profile artifacts
or to emit a separability report. Human-authored inputs are YAML; generated
reports are JSON. The evaluation command refuses to overwrite an existing
report.

```bash
python3 skills/linguistic-register-pilot/scripts/pilot_artifacts.py validate-role <role.yaml>
python3 skills/linguistic-register-pilot/scripts/pilot_artifacts.py validate-corpus <corpus.yaml>
python3 skills/linguistic-register-pilot/scripts/pilot_artifacts.py validate-profile \
  --role <role.yaml> --corpus <corpus.yaml> <profile.yaml>
python3 skills/linguistic-register-pilot/scripts/pilot_artifacts.py evaluate \
  --role <role.yaml> --corpus <corpus.yaml> --profile <profile.yaml> --output <report.json>
```

After human acceptance of viable profiles, use
`scripts/recognizability_artifacts.py` to freeze and prepare one blinded
retained-only recognizability pre-check, then score a packet-bound classifier
result. Keep the blinding key outside the classifier context. This pre-check
does not replace the later manipulation check on rendered C0/C1/C2 artifacts.

```bash
python3 skills/linguistic-register-pilot/scripts/recognizability_artifacts.py validate-plan <plan.yaml>
python3 skills/linguistic-register-pilot/scripts/recognizability_artifacts.py prepare <plan.yaml> \
  --packet-output <packet.json> --key-output <key.json>
python3 skills/linguistic-register-pilot/scripts/recognizability_artifacts.py score \
  --plan <plan.yaml> --packet <packet.json> --key <key.json> \
  --result <classifier-result.json> --output <report.json>
```

Read [references/artifact-contracts.md](references/artifact-contracts.md) when
authoring or interpreting artifacts. A mechanically valid report does not
establish semantic equivalence, recognizable register, or authority to proceed
to rendering. Only a human-accepted frozen role classification may enter an
evaluation.

Do not modify `app-server`, production role sources, the production compiler,
or `review-bench` through this skill. Rendering and the 48-trial behavioral
pilot and its rendered-artifact manipulation check belong to later slices. A
profile recognizability report is pre-check evidence only and cannot select the
corpus without human acceptance.

When a profile pre-check saturates its score scale, use
`scripts/calibration_artifacts.py` before interpreting candidate differences.
The calibration freezes three authentic cards, three count/layer/weight-matched
cross-profile composites, one neutral card, and five independently randomized
fresh-classifier passes. Its aggregate gate tests execution stability and
authentic-versus-composite coherence; it does not create independent-judge
evidence or candidate-ranking authority.

For the human-authorized micro-render diagnostic in this pilot, use
`scripts/micro_render_artifacts.py` to test short fixed semantic briefs before
constructing full role artifacts. A future calibration failure does not by
itself authorize this diagnostic or make it the only valid next experiment.
Renderers receive one anonymous retained profile and one brief. Semantic
reviewers receive one rendering and its brief without style identity. Only
renderings accepted as proposition-complete, speech-act equivalent, and free of
added meaning enter repeated blinded matching against anonymous profile cards.
This remains candidate-selection evidence upstream of C0/C1/C2 validation.

Micro-render v2 is a historical stopped experiment; do not launch additional
v2 jobs or treat its executable route as current guidance. Its licensed style
cards demonstrate how style moves can be bound to meanings supplied equally to
every condition, and its pre-outcome checkpoint and packet-only receipts remain
useful evidence patterns. The run stopped because its strict outer-`text`
transport contract rejected nested wrappers.

Micro-render v2b is also historical and stopped; do not retry S04, launch
semantic adjudication or matching, or substitute any v1/v2 artifact. Its fresh
all-sixteen batch proved the attempt/receipt chain and exact-once normalization
mechanism, but the preregistered transport-completeness gate failed at 15/16
because one unchanged extracted string contained 88 words rather than the
required 90--130. Wrapper incidence is transport-compliance metadata only, not
register evidence. The retained review qualification also withdraws
complete-manifest pre-launch immutability, sealed-key packet binding,
condition-level transport aggregation, and exhaustive renderer-visibility
claims. Read the v2b summary, qualified transport report, and review
qualification before designing any successor; a successor requires new human
authorization and a fresh experiment contract.

Micro-render v2c is the current completed apparatus-corrected experiment. Its
opaque sample identities are independent of condition labels; final render
packet bytes are written once; the condition map is a separate sealed
unblinding artifact; and every attempt marker and receipt binds the canonical
complete launch-set digest. Receipts claim only the two staged inputs and make
no exhaustive filesystem-visibility claim. The fresh run passed transport at
16/16 and style-blind semantic equivalence at 16/16, then the same Sol family
matched all 48 assignments with confidence medians at the scale ceiling.
Interpret that ceiling as strong same-family upstream separability evidence,
not independent recognizability, candidate ranking, corpus selection, or
production authority. Do not reuse or retry v2c outputs; any confirmatory
different-family or human check requires a separately authorized fresh
experiment contract.

The expanded Leveson treatment is a separate pre-outcome construction family,
not a revision of the two-document candidate package. Use
`scripts/expanded_leveson_artifacts.py` to bind exact externally retained
source bytes, prepare balanced source-only extraction input, assemble the
separately classified profile, audit quotation survival, prepare the
equal-feature-count profile-card check, and enumerate shallow single-scalar cue
rules. Use `scripts/expanded_leveson_model_job.py` only for its isolated
packet-only judgments. Read the construction report before downstream use: a
recognizability pass cannot cure a failed retention gate or a sufficient
incidental cue. Do not expose extraction to behavioral tasks or keys, and do
not generate role renderings or behavioral outcomes through this artifact
family.

For the fresh `leveson-expanded-v1b` recovery, read its preregistration before
using any result. The recovery keeps the original eight-source corpus and
three-document recurrence threshold, binds newly found support without
rewriting v1, and length-matches the profile cards before a fresh same-family
recognizability check. Treat card salience weights as card-local normalized
ranks; the profile's source weights remain unchanged. Downstream construction
is available only when the consolidated v1b gate report passes every retained
authorship, genre, recurrence, retention, quotation, target-recognizability,
and shallow-cue gate.

After a human authorizes behavioral-contract construction, read
[references/behavioral-pilot-contracts.md](references/behavioral-pilot-contracts.md).
The behavioral contract freezes construct classification before tasks, keeps
task authors blind to profile wording, and binds the complete balanced schedule
and scoring rules. Its launch check must refuse behavioral outcomes until a
separate group-3 artifact-rendering and prelaunch gate passes; a sealed contract
alone is not execution authority.

The retained group-3 preoutcome attempt is terminal and failed its frozen gate.
Read its group report before proposing behavioral execution or a rendering
successor. The six artifacts preserve canonical coverage and advisory speech
act, but semantic/salience, incremental C2 manipulation, shallow-cue, and
profile-mediated quotation gates failed. Do not launch T001, repair these
artifacts in place, or reuse their outputs. A successor requires fresh human
authorization and a new preregistration.
