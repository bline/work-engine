# Proposed public contracts, revision 1

Status: phase 0 design proposal. These names and fields fix the consumer boundary
for the next implementation bid; no endpoint accepts them today. Accepting the
implementation bid includes these proposed semantics. Existing owner-answer/v1
and bridge RPC versions retain their current bytes and meaning. Implementation
may revise a proposed contract explicitly before qualification; it may not claim
compatibility with this revision after silently changing its material meaning.

## Shared representation

New schema identifiers below end in `/v1-proposal`. IDs are opaque nonempty
strings; revisions identify immutable owner-issued versions. `Ref` means
`{id, revision, sha256}` of a protected immutable object, with a lowercase
64-character SHA-256 digest. A handle is a locator, never authority by itself.
Times are UTC RFC3339; money is a decimal string in the named accounting unit.
`null` means unavailable/not yet observed where allowed, never zero or success.
Public projections include `observed_at` and `source_revision` so stale snapshots
cannot masquerade as current runtime state. Private credentials and keys do not
appear in these objects.

Unknown schema versions, unknown effect/authority kinds, missing material terms
or unsupported profiles produce a visible hold before effects. Omitted fields
do not confer discretion beyond the approved envelope. Ordinary implementation
choices remain with the acting model inside that envelope.

## Task intent: `dot.task-intent/v1-proposal`

The exact board request row contains `data.intent`. Every material field below
is required and must be rendered in the supported task review. References need
an immutable, intelligible review projection; showing only a hash is inadequate.

| Field | Type and meaning |
|---|---|
| `schema` | Exact task schema identifier above |
| `task_id`, `lane_id`, `revision` | Stable task/lane identities and immutable intent revision |
| `proposal` | `Ref` for the exact immutable proposal revision and digest; current ID lookup cannot substitute a newer proposal |
| `objective`, `input` | Human-readable objective and `Ref` for the exact worker input/artifacts |
| `operation` | `start`; first slice has no general steering admission |
| `provider`, `execution_profile` | `codex` and an explicitly qualified, server-resolved disposable workspace-write profile |
| `workspace` | `{profile_id, checkpoint, configuration: Ref}`; protected profile resolves the repository and permitted execution configuration |
| `model` | `{id, effort, tier}`; effective values are checked against the qualified profile |
| `effects` | `{write_paths: string[], network: boolean, max_concurrent_tasks: integer, max_child_tasks: integer}`; paths/profile confinement must have an admissible realization |
| `prerequisites` | `Ref[]` for required evidence, including the qualified profile/release |
| `clock` | `{first_funded_rule, admit_until, close_by}`; setup/readiness funding starts the same task clock, including after restart |
| `budget` | `{objective_id, units, maximum, productive_maximum, closure_reserve, coverage}`; coverage includes coordination, workers, review and closure |
| `submitter` | Trusted-context actor identifier permitted to request admission |
| `authority` | `{mode: Ref, mandate: Ref or null, allowed_operations: string[], reserved_decisions: string[]}`; first slice is present mode, no mandate, start only |

First slice qualifications are one Codex task, one disposable repository, no
task network effect, no production workspace, no child delegation and no additional
funded task. If the actual native profile cannot realize a required effect bound,
the profile remains disabled and that claim is unestablished. The review does
not promise confinement based only on a requested setting.
The worker's qualified grant must protect the authority store, key policy,
canonical budget state and runtime configuration, including path escape cases;
same-user file permissions alone do not establish the required boundary.

## Decision evidence: `dot.decision-evidence/v1-proposal`

Fields are `schema`, `request_json`, `answer_json`, `proof_ref`, `key_policy_ref`
and `board_reply_ref`. The first two JSON fields retain the exact original UTF-8
strings; references locate protected proof/key-policy/board data. The public
view may redact credential details; the acting consumer retrieves and verifies
the original evidence through its trusted context.

The answer remains **work-engine.owner-answer/v1**. Its exact signed bytes bind
the authoritative request row using the existing Python request codec. No new
canonicalization of signed answer bytes is introduced. Verification includes
pinned key trust, owner, RP/origin, user verification, digest, expiry and current
task/authority state. An OAuth identity or `verified: true` field cannot replace
that verification.

Proposed first writable consumer rule: a request closed by this exact verified
approving answer may be considered, provided the task and proposal are still
live and its authoritative signed board reply is available. A mailbox answer
alone is insufficient. The signed intent's exact proposal reference must still
be current; a stale proposal revision holds without effects. Closure by another answer, withdrawal or supersession
does not qualify. This explicitly refines the current documentation's still-open
consumption rule and requires owner acceptance before implementation effects;
phase 0 does not change the live rule. Exact existing admission is returned
idempotently. Rejection, defer and ask never start work. The first profile also
holds **every nonempty signed comment**, preserving it for reconciliation.

## Admission: `dot.admission-request/v1-proposal`

Fields are `schema`, `request_id`, `task: Ref`, `decision: Ref` and
`expected_task_revision`. The authenticated caller is resolved by the server;
an actor or grant supplied in JSON cannot select its own authority. Changing
the content under `request_id` is a conflict.

`dot.admission-view/v1-proposal` returns `schema`, `admission_id`, `task: Ref`,
`decision: Ref`, `answer_id`, `revision`, `status`, `hold_reasons`,
`reservation_ref`, `authority_revision`, `native_operation_id`, `observed_at`
and `source_revision`. Unavailable reservation/native IDs are explicitly null.
Statuses are `held`, `rejected`, `reserved`, `delivery_pending`,
`start_unknown`, `running`, `closure_pending`, `closed`.

The protected task/admission owner commits answer-to-admission uniqueness,
accepted state and outbox intent atomically. Canonical accounting owns budget
facts; any cross-store reservation uses a stable, idempotent reservation
identity and recovery, not a claim of a distributed atomic transaction. A
reservation may remain stranded pending reconciliation; no effect proceeds
without a valid current reservation. Recheck revocation, authority and deadline
at the effect boundary. Record effect intent before native delivery. Retry an
exact admission read, not an uncertain native create/start. A lost ACK stays
unknown until authoritative reconciliation can establish what happened.

## Control: `dot.control-request/v1-proposal`

Fields are `schema`, `control_id`, `task: Ref`, `admission_id`, `session_id`,
`turn_id`, `expected_task_revision`, `operation`, `instruction: Ref or null`,
`authority: Ref` and `deadline`. Operations contemplated for later qualification
are `steer`, `interrupt`, `close`; none is enabled by phase 0. Control identity
has the same exact-retry/conflict rule. A changed objective/effect/cost/time
envelope needs a new decision; task-bound steering does not extend authority.
Cancellation requested, acknowledged, interruption observed and closure settled
are separate observations. No unrestricted shell or arbitrary RPC is exposed.

## Result: `dot.result-view/v1-proposal`

Fields are `schema`, `task: Ref`, `admission_id`, `session_id`, `turn_id`,
`worker_contract_revision`, `outcome`, `operational_status`, `exceptions`,
`artifacts: Ref[]`, `evidence: Ref[]`, `clock`, `accounting`, `custody`,
`observed_at` and `source_revision`.

`outcome` is null until supported, or `completed`, `partial`, `blocked`,
`failed`, `interrupted`, `limit`, `cancelled`. `operational_status` separately
reports missing/invalid worker contract, unavailable observations and recovery.
`clock` includes first-funded, admitted, observed-start and close-by times;
`accounting` includes units, limit, reservation, observed usage, coverage and
unsettled amounts; `custody` lists native/child/process identities and known or
unknown dispositions. Child/process closure is not inferred from a turn ending.
The exact selected-turn worker writes the structured result; a permanent model
courier/summarizer is unnecessary. Worker-reported success alone does not prove
acceptance, cost settlement or native closure.

## Mode and mandate

`dot.mode-view/v1-proposal` has `schema`, `owner_id`, `revision`, `mode`,
`effective_at`, `pending_transition`, `mandates: Ref[]`, `in_flight: Ref[]`,
`observed_at`, `source_revision`. Mode is `present` or `away`; the first slice
supports only present. An authenticated revision orders a mode transition
against admissions and records its effect on queued and in-flight work.

`dot.mandate/v1-proposal` has `schema`, `mandate_id`, `revision`, `issuer`,
`actor`, `objective_lanes`, `providers_models`, `workspace_profiles`, `effects`,
`delegated_material_classes`, `reserved_decisions`, `budget`, `clock`,
`concurrency`, `revocation` and `reopening_rule`. It requires its own verified
decision and current revision. All fields are explicit bounds; this is a
reserved future contract, not supported Away operation. Presence, silence,
identity refresh or an omitted mandate never authorizes delegated work.

## Acceptance and fixture scope

The UI [byte vectors](ui-binding-vectors.json) intentionally use a smaller
illustrative intent, not a full instance of this task contract. They test the
existing digest/challenge boundary only. [UI cases](acceptance-cases.json) and
[backend cases](backend-acceptance-vectors.json) are proposed semantic test
obligations. They contain no valid proof or execution authority and establish
no runtime acceptance result. Implementation must turn the material cases into
tests and add full-contract display/completeness tests before qualification.
