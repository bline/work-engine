# S5 review-amplification source map

## Identity and evidence cutoff

The selected case is **skills-migration-app-server-20260828, slice 6**, titled
“S5 — Hidden context-lifecycle integration with compiled roles.” It is not the
October Rust lifecycle S5 native-adapter proof or the terminal lifecycle handoff
fixture. Discovery on 2026-10-16 bound immutable Git checkpoints, three review
episodes and their linked histories, historical gate and provider output, a
retained scheduler item, the recorded metrics row and the continuity repair.

The exact object IDs, payload hashes, checkpoint metadata, linked review
history, provider attempt references, scheduler row and unresolved coverage are in
[`evidence/s5-source-map.json`](evidence/s5-source-map.json). This is a discovery
manifest, not a new authoritative workflow record or a complete golden corpus.

The connection to the [accepted design](../../../../app-server/docs/control-plane-causal-observability-ui.md)
is supported by the replacement episode's authority source, which explicitly
names `reported_episode_cannot_bind_later_candidate` and the earlier episode
revision. The metrics row also records one generic reviewer replacement and one
quota failure. These are stronger identity evidence than a shared “S5” label.

## Bound sources and their owners

| Meaning | Exact source / boundary | What is established; remaining qualification |
| --- | --- | --- |
| Candidate lineage and gates | Slice Checkpoint refs under `refs/work-engine/checkpoints/skills-migration-app-server-20260828/slice-6/`; candidate commits below | Six existing immutable commits and their metadata. Historical `run_gate.py` output for candidates 1–4 was recovered from the bound builder rollout; each canonical JSON digest equals its checkpoint gate digest, with 9/9, 5/5, 6/6 and 8/8 checks passed. The coordinator's candidate-5 constructed receipt also hashes to its checkpoint digest and lists five passed checks. The frozen metrics row reports `stage_total: 8` for that same digest. Its producer constructed the S5 semantic record by copying the prior slice’s metrics row and explicitly entered `stage_total: 8` alongside the new S5 digest; the assembler preserved that scalar. No separate eight-stage S5 source was found in the bounded producer path. The published count remains inconsistent with the five named receipt checks. Transcript output is evidence of producer output, not an owner-admitted gate read. |
| Initial generic review | Independent Review State head `f5773967f8f6a8470161ac4b1552738d49a23d8c` | Full linked history: four validated revisions, candidate 1 initial subject → candidate 2 reported result. Uncompiled-role test finding verified resolved; two other findings withdrawn. Episode remains active/reported. |
| Instruction review | Independent Review State head `e70f2b5caecacfafca3257586c5e4a09587f6ed9` | Full linked history: four validated revisions, candidate 2 initial subject → candidate 3 reported result. `AIR-CAND2-001` and `AIR-CAND2-002` verified resolved. |
| Replacement and provenance finding | Independent Review State head `24f3f38e5351b48bcd3e069978b22a597f590e82` | Full linked history: six validated revisions, candidate 3 → candidate 4 reported → candidate 5 reported. Authority explicitly cites the original reported revision. Identity cross-check finding verified resolved; dead-branch finding withdrawn. This is a recorded authority edge, not an inference from timestamps. |
| Historical reviewer grants | Three creation patches in the bound builder rollout; manifest hashes in retained review state | All three manifest JSON objects pass the current schema validator. Their canonical hashes, grant IDs, identities, writers and authority sources match retained bindings. The original `/tmp` files are gone; matching bytes and bindings do not establish historical launcher isolation or present-day UI read permission. |
| Provider attempts and quota | Eight distinct Claude CLI result UUIDs in bound builder/coordinator rollouts; frozen `docs/ai-workflow-metrics.jsonl` row 21 | Seven results succeeded, one has `is_error=true` and `api_error_status=429`. Per-attempt cost, input, cache creation/read and output sums reproduce the reported $15.7545495 and corresponding token totals, including the failed attempt's $1.107375. Raw thinking tokens sum to 101224; metrics reports 101197, excluding the failed envelope's 27. The row also records one generic replacement. These are historical CLI output records, not an owner-normalized provider event feed or a comparative savings proof. |
| Retry schedule | Role Scheduler SQLite item `skills-migration-s5-candidate4-generic-review-quota-retry-20260829` | Read-only row at store revision 15 has due time 2026-08-29 07:40 UTC, `acknowledged` status and result ref to replacement episode revision `034b4df528510e5f65029aa029f3fd2f78ef39d1`, which is in the verified chain at candidate 4. Its `presented_at` is null; the row cannot establish a presentation event or the full schedule mutation history. |
| Review continuity repair | Commit `cc4b644ff102c3eb8fa46de01ff6a73fa171d575`, `skills/independent-review-state/scripts/independent_review_state.py` | Exact before/after blobs bound. `record_remediation_subject` changes from requiring `remediation` to accepting active `remediation` or `reported`; documentation separates publication from retirement. A code change alone does not prove realized savings. |

| Checkpoint | Commit |
| --- | --- |
| Candidate 1 | `2cfc945627f4f6d141385c46bd10f242601fef0b` |
| Candidate 2 | `fd66ba9dbb9892596755c496dee6a54e491fd193` |
| Candidate 3 | `649349f57435b34d952b8ebd9e8f67a622d10dda` |
| Candidate 4 | `0195378488a65f3d1926ecb15f3613ab095971a2` |
| Candidate 5 | `c1a90a55ce326898996b5ab0b5925b91480b9f1c` |
| Accepted | `8d9caea5322edfeebb50b7e612f9c850224fd080` |

The accepted design's four-candidate sketch is not the terminal inventory:
retained checkpoints include candidate 5 and acceptance. Preserve both the
historical sketch and the fuller inventory; do not relabel candidate 5 as 4.
The metrics record points to candidate 5 and the accepted checkpoint's same tree.

## Producer-to-consumer proposal

The smallest useful projection is one read-only historical investigation of
`skills-migration-app-server-20260828` slice 6 at an explicit cutoff. A host-side
adapter asks each owner for its admitted historical read, validates source
revision and digest, and emits bounded checkpoint, gate, review, provider and
schedule facts plus typed reference edges. Each projected fact carries its
owner, exact revision or result UUID, evidence class, confidentiality decision
and completeness state. The terminal client renders the projection and follows
references. The existing task socket has no campaign-discovery contract.

The review owner has an exact-revision and paged-history read interface gated by
an episode authority manifest. The original three `/tmp` manifests are absent,
and their creation patches only establish matching historical bytes. The
scheduler's current agenda excludes acknowledged items; this map read its
retained SQLite row directly. Five gate results and eight provider result
envelopes came from private historical rollouts. These direct observations
establish candidate sources for an adapter; none is a present-day owner grant or
a public confidentiality projection. Current Rust campaign/review readers are
possible delivery boundaries, not evidence that these old Git-backed records
have been migrated or admitted. The adapter must preserve their original owner
and version identities.

A first vertical test should load the exact candidate/review records through
that admitted read boundary and traverse candidate 3 → replacement authority →
original reported episode, then candidate 4 → 429 attempt → scheduler item →
acknowledged result revision `034b4df528510e5f65029aa029f3fd2f78ef39d1`.
It should show the resolved provenance finding and withdrawn finding separately,
follow the repair revision, and reconcile the eight distinct attempt UUIDs to
the frozen aggregate without counting one result twice. Negative cases should
reject corrupt payloads, mismatched campaign/slice, unsupported schema,
unreachable predecessors, missing owner records and unauthorized/private
material. Candidate 5's published receipt/metrics count inconsistency and any unadmitted owner
read remain explicit coverage gaps. Full UI-0 acceptance still needs the required source contracts
and a qualified golden trace.

## Deterministic gate count retained — 2026-10-16

The [evidence map](evidence/s5-source-map.json)'s `gate_count_investigation`
now includes the complete five-check receipt, its digest, an explicit counting
rule and the immutable metrics-row identity. Counting top-level `checks` entries
gives **5**, all passed. Nested test totals and counts from other candidates are
different measurements. This fact can be recomputed from repository data without
opening a session transcript or inferring what the producer intended.

The historical `stage_total: 8` remains a separately labeled observation, not a
verified check count. The precise presentation is: **5 receipt checks passed;
historical metrics reports 8 stages (meaning unestablished).** The retained
receipt is coordinator-constructed historical evidence; digest verification
establishes its binding, not execution attestation or a current reader grant.

## Remaining source work

The candidate-5 constructed receipt is recovered and hashes to the checkpoint.
It lists five passed checks, while the frozen metrics row says eight stages for
the same digest. The historical metrics producer copied the previous slice
record, then manually set the new S5 digest with `stage_total: 8` in the S5
semantic receipt (Aug 29 rollout `01a04e71`, line 313). The assembler deep-copied
that input without deriving a count from the gate receipt. The prior slice row
also reports eight stages, so the retained producer path supports a copied
scalar rather than an independently evidenced eight-stage S5 gate. The metrics
owner should preserve the frozen row, establish the intended `stage_total`
semantics, and publish a revision-bound correction or explanatory erratum
linked to the five-check receipt before a consistent terminal trace is claimed. The provider
metrics owner must explain whether excluding the failed call's 27 thinking
tokens was intentional. The review owner must
admit historical reads under a current, appropriate reader grant; the gate,
provider and scheduler owners need an authorized read/projection path for their
legacy records. The scheduler row cannot show a presentation event or its own
mutation history because `presented_at` is null and this read covered only the
current retained row. Bind a compatible frozen comparison cohort and pricing
basis before claiming amplification cost or savings. Choose the public access
policy and projection placement with those owners. No causal screen
implementation or UI-0 completion is claimed.

Discovery used exact documents and immutable historical Git objects, all linked
predecessors of the three review keys, the bound child/coordinator rollouts on
August 28–29, and the current eight-row scheduler store. All 14 linked review
revisions passed envelope integrity and state-schema checks with no further
predecessor, and three reconstructed authority manifests matched state bindings.
Five gate result hashes and the provider cost/input/cache/output sums were
recomputed; the thinking-token exception remains explicit.
The manifest records exact result UUIDs, source lines, revisions, digests and
search limits. The current code graph supports reader structure but cannot prove
historical bytes; direct reads supplied those. Missing-source statements are
bounded to these sources, not repository-wide absence.
