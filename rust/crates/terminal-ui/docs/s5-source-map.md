# S5 review-amplification source map

## Identity and evidence cutoff

The selected case is **skills-migration-app-server-20260828, slice 6**, titled
“S5 — Hidden context-lifecycle integration with compiled roles.” It is not the
October Rust lifecycle S5 native-adapter proof or the terminal lifecycle handoff
fixture. Discovery on 2026-10-16 bound immutable Git checkpoints, three review
state revisions, the recorded metrics row and the continuity repair.

The exact object IDs, payload hashes, checkpoint metadata, source revisions,
finding dispositions and unresolved coverage are in
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
| Candidate lineage | Slice Checkpoint refs under `refs/work-engine/checkpoints/skills-migration-app-server-20260828/slice-6/`; candidate commits below | Six existing immutable commits and their tree/checkpoint metadata read directly. Gate digests are references; actual gate receipts are not yet resolved. |
| Initial generic review | Independent Review State blob `f5773967f8f6a8470161ac4b1552738d49a23d8c` | Payload hash verified. Original episode is active/reported; uncompiled-role test finding verified resolved, two other findings withdrawn. A reported result must not be presented as retirement. |
| Instruction review | Independent Review State blob `e70f2b5caecacfafca3257586c5e4a09587f6ed9` | Payload hash verified; `AIR-CAND2-001` and `AIR-CAND2-002` verified resolved. Exact source references remain owned by the episode. |
| Replacement and provenance finding | Independent Review State blob `24f3f38e5351b48bcd3e069978b22a597f590e82` | Payload hash verified. Authority explicitly cites the original reported episode. Identity cross-check finding verified resolved; dead-branch finding withdrawn. This is recorded replacement authority, not an inferred causal edge from timestamps. |
| Recorded cost and quota aggregate | `docs/ai-workflow-metrics.jsonl`, frozen commit/blob/row digest in the manifest | Campaign/slice and checkpoint join verified. Recorded provider cost is $15.7545495, one quota failure, one generic replacement. Per-attempt raw telemetry, cost non-overlap and comparative baseline remain unqualified; do not label all extra cost waste. |
| Review continuity repair | Commit `cc4b644ff102c3eb8fa46de01ff6a73fa171d575`, `skills/independent-review-state/scripts/independent_review_state.py` | Exact before/after blobs bound. `record_remediation_subject` changes from requiring `remediation` to accepting active `remediation` or `reported`; documentation separates publication from retirement. A code change alone does not prove realized savings. |
| Retry schedule | Role Scheduler, exact item not yet bound | The accepted design describes a scheduled retry. A metrics quota count does not identify its schedule, timing, acknowledgment or authority. |

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

The first useful projection is a read-only historical investigation by exact
campaign, slice and evidence cutoff. A host-side adapter resolves checkpoint
objects, owner-admitted review reads/history, gate receipts, and normalized
provider/scheduler records. It returns bounded display facts and explicit
reference edges with source owner, revision, evidence class and completeness.
The terminal client renders those projections and follows references; it does
not open writable stores, reconstruct private raw sessions, or decide findings
or acceptance. The existing task socket is not a campaign-discovery contract.

For this historical case the adapter must preserve original owner/version
identity. Current Rust campaign/review readers are possible delivery boundaries,
not proof that these old Git-backed records have been migrated or admitted.
Direct `git cat-file` discovery here does not replace the eventual read-access
contract. Payload hash checks establish bytes, not authorization or semantic
acceptance. Authoritative reader grants and confidentiality projection remain
required before exposing reviewer material to UI users.

A first vertical test should load the bound candidate/review records through
that admitted read boundary and traverse candidate 3 → replacement authority →
original reported episode. It should show the resolved provenance finding and
withdrawn finding separately, and follow the exact repair revision. Negative
cases should refuse corrupt payloads, mismatched campaign/slice, unsupported
schema, missing referenced objects and unauthorized/private material. Missing
gates, quota events or schedule records must remain visible coverage gaps rather
than being filled from the design narrative. Full UI-0 acceptance still needs
all required sources and a reconstructed golden trace.

## Remaining source work

Resolve raw gate receipts behind the checkpoint digests, exact provider attempt
and quota records, and the actual retry schedule. Traverse complete relevant
review predecessor history and validate the authority manifests through their
owner. Bind a compatible frozen comparison cohort before deriving amplification
cost. Decide the initial host projection's access policy and placement from those
concrete reads. No causal screen implementation or UI-0 completion is claimed.

Discovery used exact document/JSON searches and immutable historical Git objects;
the current code graph cannot establish historical bytes. Three durable payload
hashes were recomputed, but full owner validation and historical pagination were
not performed. Search was bounded to the design, metrics and references reached
from them; missing-source statements above mean unresolved in this map, not
repository-wide absence.
