# Profile-Separability Artifact Contracts

Each artifact-family script is authoritative only for the artifacts it names:
`pilot_artifacts.py` for profile separability, `recognizability_artifacts.py`
for the retained-card pre-check, `calibration_artifacts.py` for matched-decoy
calibration, `micro_render_artifacts.py` for fixed-semantics v1 rendering, and
`micro_render_v2_artifacts.py` for semantic-licensed v2 rendering and transport
stop reports, and `micro_render_v2b_artifacts.py` for the authorized exact-once
v2b transport experiment, and `micro_render_v2c_artifacts.py` for the
apparatus-corrected v2c experiment.
Those scripts own their field names, closed vocabularies, referential integrity,
compatibility, and gate arithmetic; none owns semantic truth.

Post-review semantic adjudication is a separate diagnostic artifact. It may
qualify interpretation of a frozen run but cannot replace original judgments,
reopen a failed gate, or authorize matching. Its packet withholds style and
original verdicts while exposing all brief/text pairs to one calibrated rubric.

## Semantic-licensed micro-render v2

The v2 plan binds each candidate profile and maps five retained
`realization_only` features to narrower licensed instructions. Source feature
IDs establish lineage but are removed from renderer and matcher packets. Every
condition receives the same six brief meanings, including an explicit positive
evaluation and a bounded contrast, so style may realize those stances without
inventing them.

The preregistration must exist in an immutable checkpoint before model jobs.
Render preparation verifies the exact plan blob at that checkpoint and binds
the commit into every packet and the private key. The packet-only runner emits
a receipt binding the model configuration, isolated inputs, prompt, schema,
event stream, and raw output. Normalization rejects nested structured output
before semantic review.

One style-blind adjudicator applies the same rubric across all sixteen fixed
brief/text pairs. Matching artifacts are prohibited unless every pair is
equivalent. The v2 terminal vocabulary distinguishes
`not_run_transport_gate_failed`, `not_run_semantic_gate_failed`, and completed
matching. A transport-stop report may claim only absence of retained execution
evidence unless a separate attempt ledger proves non-launch. Execution receipts
must be immutably published before reporting changes if pre-report byte
immutability is material. No terminal report can rank or select a corpus by
itself.

## Exact-once micro-render v2b

V2b binds a fresh experiment ID to the frozen v2 semantic design while
excluding every v1/v2 output from reuse. Its preregistration fixes a launch
manifest of sixteen new request digests. Each runner writes an attempt marker
before model launch, then retains raw response, events, completion receipt, and
their digests. Blinded ingestion accepts unchanged prose or unwraps exactly one
JSON object having only a string `text` field. Extra keys, arrays, scalar JSON,
code fences, malformed JSON-looking strings, second wrappers, and prose-shape
violations are terminal transport rejections.

The immediate batch evidence owns expected, attempted, completed, accepted,
rejected, and missing job state by sample ID without condition identity. The
post-batch checkpoint must contain both the frozen launch artifacts and every
retained observation before the transport report may unblind normalization
rates. Those rates describe transport compliance only. Semantic adjudication
is prohibited unless all sixteen renderings pass transport; matching is also
prohibited until the separate all-sample semantic-equivalence gate passes.

The retained v2b experiment stopped at 15/16 transport-valid samples. Its
semantic and matching terminal states are `not_run_transport_gate_failed`; no
retry, substitution, recognizability inference, or candidate ranking follows
from that record. Interpret it through the retained review qualification: the
sample-level stop survives, while complete-manifest pre-launch immutability,
sealed-key packet binding, condition-level aggregation, and exhaustive
renderer-visibility claims are withdrawn or unestablished.

## Apparatus-corrected micro-render v2c

V2c binds a wholly fresh experiment to the frozen v2 semantic design. Sample
IDs derive only from an opaque identity seed; a separate assignment seed maps
them to brief-condition-replica cells. Render packet bytes are finalized before
their digests are written to a separate sealed unblinding artifact, and the
harness refuses to overwrite any emitted packet or report.

For each stage, the canonical launch set binds the complete ordered job set to
packet, schema, prompt, model, and reasoning-effort digests. Its digest is
carried by every canonical request, pre-launch attempt marker, and execution
receipt. The complete launch set, final packet bytes, and sealed key must be in
an immutable checkpoint before the first attempt; marker and receipt chronology
must not predate that checkpoint. A renderer receipt establishes the two staged
inputs and explicitly makes no exhaustive filesystem-visibility claim.

Render ingestion remains condition-blind until immediate batch evidence and
all observed execution artifacts are checkpointed together. Exact-once
normalization and the frozen 90--130-word acceptance boundary are unchanged;
the generation request targets 110--120 words. Only 16/16 transport completion
authorizes the style-blind all-sample semantic gate, and only 16/16 semantic
equivalence authorizes the three matching passes. Each downstream launch set
and raw evidence chain is checkpointed before scoring.

V2c's retained run passed all gates and reached a 48/48 same-family matching
ceiling. That result is evidence that one Sol-family renderer/adjudicator/
matcher pipeline can realize and recover these licensed cards under the frozen
probe. It is not independent reliability, cross-family recognizability,
candidate ranking, corpus selection, C0/C1/C2 validation, or production
authority. Transport wrapper incidence is compliance metadata only.

## Role manifest

The role manifest binds one existing canonical role to an immutable repository
revision and source digest. Its semantic units are stable identifiers for
coverage and classification. Behavior classifications distinguish `encoded`,
`unencoded`, and `ambiguous`; they are frozen human-owned experimental
judgments, not conclusions produced by the validator.

The CLI resolves each recorded source path against the local repository,
verifies its working bytes, and verifies the same bytes at the recorded Git
commit. An unavailable, changed, uncommitted, or path-escaping source fails
validation rather than silently weakening the binding.

Only `status: frozen` manifests may enter evaluation. Changing a classification
requires a new role artifact rather than editing evidence after outcomes exist.

## Mini-corpus manifest

The first separability slice uses exactly two publicly readable works per
candidate: one formal work and one responsive or informal work. Every source
records authorship basis, access and rights status, a stable URL, and a SHA-256
digest of the exact bytes used for extraction. Public readability does not
imply redistribution permission; source bytes need not be committed.

## Extracted profile

Each feature records its layer, abstract description, preregistered
distinctiveness weight, source evidence, semantic disposition, and judgment
rationale. Evidence from both mini-corpus genres is required before
`cross_genre` may be true.

The judgment provenance attests that distinctiveness weights were assigned from
corpus evidence before semantic classification. This preserves attenuation as
an outcome instead of allowing excluded features to be down-weighted later.

Only `realization_only` features are retained. `semantic_duplicate`,
`semantic_addition`, and `uncertain` remain in the ledger so attenuation is
observable rather than silently discarded. Content-screening fields record the
judgment that names, copied language, domain terminology, and named methods do
not survive in retained descriptions; boolean acceptance is evidence of a
completed screen, not mechanical proof of that semantic claim.

## Separability report

The report recomputes counts and distinctiveness weights by disposition. Its
disposition is preregistered mechanical gate state:

- `candidate_viable`: all configured retention and cross-genre thresholds pass;
- `candidate_attenuated`: some realization-only material remains but a threshold fails;
- `candidate_not_separable`: no realization-only material remains.

These states do not select a corpus or authorize rendering. The human operator
owns that decision and may reject a mechanically viable candidate for recorded
semantic, legal, or experimental reasons.

## Blinded profile-recognizability pre-check

The frozen preregistration binds exactly three retained profiles, one neutral
decoy, a deterministic randomization seed, a classifier contract, and gate
thresholds before classifier outcomes exist. Preparation copies only
`realization_only` feature layer, category, abstract description, and frozen
salience weight into anonymous cards. It omits candidate identifiers, source
evidence, dispositions, semantic rationales, and role content.

The packet and blinding key are separate artifacts. The classifier receives
only the packet in a fresh context and returns scores bound to its exact
SHA-256 digest. The result must score every card and every unordered pair once,
identify the most neutral card, and record observed execution provenance.
Raw structured classifier output is retained separately; normalization adds
launcher-observed provenance and packet bindings without changing any score or
rationale.

Mechanical scoring unblinds the result and checks, for every candidate,
internal coherence, non-neutral distinctiveness, and separation from the
neutral decoy. The overall gate also requires correct neutral identification.
Before scoring, the harness deterministically reconstructs the packet and
blinding key from the frozen plan and requires exact equality; the report binds
the exact plan, packet, key, and classifier-result digests.
Passing establishes only that this one classifier could recognize the frozen
feature cards under the preregistered pre-check; it is not the manipulation
check on rendered C0/C1/C2 artifacts, a reliability estimate, corpus selection,
or authority to render a role.

## Profile-card stability calibration

When the pre-check reaches a score ceiling, the calibration plan binds five
fresh classifier passes and one deterministic composite decoy per candidate.
Each composite takes unique retained descriptions from the other candidates
while preserving the matched authentic card's feature count, layer multiset,
and salience-weight multiset. Per-pass card labels and feature order vary under
a frozen seed; content and scoring rules do not.

The aggregate gate requires repeated neutral identification, stable authentic
distinctiveness, separation among authentic cards, and an authentic card to
outscore its matched composite on coherence in the preregistered number of
passes. Five executions from one model family estimate prompt-and-schedule
stability only. Passing does not establish independent recognizability, rank a
candidate, select a corpus, or replace rendered-artifact checks.

## Fixed-semantics micro-render probe

The micro-render plan freezes two non-task semantic briefs, four anonymous
styles, two render replicas per brief/style cell, and three blinded matching
passes. Render packets contain one brief and one style card. Semantic-review
packets contain the brief and rendered prose but omit style identity. A
rendering enters matching only when every proposition is preserved, the speech
act is equivalent, and no meaning is added.

Matching packets contain accepted prose and anonymous reference cards with
labels randomized independently per pass. The aggregate gate requires both
total assignment accuracy and within-pass recurrence for every condition. This
tests realized recognizability while remaining upstream of full role rendering:
it cannot select a corpus without human audit, establish cross-family
reliability, or substitute for C0/C1/C2 artifact validation.
