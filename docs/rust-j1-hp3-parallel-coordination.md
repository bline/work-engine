# J1 and HP3-E1 parallel implementation recommendation

Prepared against published ER1/P1 and replay follow-up commit
`035a970443c4b4ca8e663dd0e66f4b8b28746268`. The user authorized parallel planning.
This document reconciles the two plans; it does not authorize implementation,
new storage semantics, providers, migration, activation or another commit.
The two planners were requested as `gpt-6-astra/high`. Accepted implementation
and ordinary remediation use the user's chosen `gpt-6-sol`.

## Recommended sequence

1. Accept and implement **E0**, the small permanent execution-evidence contract
   checkpoint. One integration owner registers its crate and Cargo/lock edges,
   exact exports, encodings/limits and controlled fixture route. E0 has no process
   launcher, store, full record validator, host service or runtime qualification.
2. Run **J1** and **HP3-E1** concurrently against those same exports.
3. Reconcile the two completed subjects and run their real-owner composition gate.
   J2 follows the campaign join; it is not a concurrent editor of J1's state/store.

| Lane | Owned result | Product edit boundary |
| --- | --- | --- |
| [J1](rust-j1-owner-join-implementation-plan.md) | Real CE3 admission, durable exact completion stages, checked historical ER1 readback and partial-commit recovery | Campaign application, contracts, completion, store and joined tests |
| [HP3-E1](rust-hp3-execution-evidence-implementation-plan.md) | Permanent execution request/process/result custody, immutable bounded records and checked reader, proved with an actual controlled child | New `review-execution-evidence` crate and its producer/store/reader tests |
| Integration owner | E0 exports, shared Cargo/lock edits, interface changes and final composition | Shared contract checkpoint and explicit cross-lane handoffs |

The separate crate has concrete consumers: campaign and the later review host.
It depends on the existing claim codec and custody types; claims does not depend
back on it, and the crate does not depend on campaign, episode or lifecycle.
This avoids a campaign/host dependency cycle without introducing a generic
platform. E1's narrow process capture remains local because the inspected
lifecycle mechanisms do not expose its byte capture and precise spawn boundary.
It does not copy lifecycle admission or its operation ledger.

## Reconciled contract

The trusted startup composition installs `ExecutionEvidenceOwner`; individual
requests cannot install a reader or choose authority. The shared interface has
`read_result(exact_ref)` and `read_custody(exact_ref, full_binding, observation)`.
The result preserves exact native result bytes and distinct raw/claim digests,
execution provenance and checked observation presence or absence. CE custody
uses the existing `CustodyEvidence` type. Missing, corrupt or inaccessible data
is an error or unresolved custody, not observation absence.

The execution owner attests only stable attempt facts it can observe. J1 checks
current campaign admission and later stage facts, including child request hashes,
completion revisions and episode result identity. Original CE command/admission
bindings remain saved across recovery. A current revision cannot replace the
original revision simply to make a recovered command admissible.

The selected episode target may depend on CE receipts. J1 therefore registers
and validates the descriptor writer's exact proposed content as data, then
trusted composition opens ER1's immutable scope for the registered target.
Unregistered target, unavailable reader and confirmed absent exact transition
are separate pending states. J1 neither expands an existing ER1 scope nor gains
an episode writer capability.

The nested order is campaign gate, bounded local evidence read, then CE's own
transaction; the evidence reader releases internal locks before CE writes and
cannot call back into campaign. No campaign SQL transaction spans another owner.
Campaign receipt publication happens after the CE lease is dropped and after a
fresh CAS check. Downstream committed effects remain explicit if that CAS fails.

Controlled fixtures and actual controlled processes have distinct evidence
classes. Profile checks remain mandatory even in all-feature builds; a default-off
feature is not the authority boundary. J1's fixture gate proves domain composition,
E1's process gate proves controlled record production, and neither proves real
provider attestation or campaign dispatch admission by itself.

## Decisions recommended for implementation acceptance

- **Campaign schema 3 on new private roots only.** Freeze the documented J1/J2
  record vocabulary now; J1 refuses nonempty J2-only states until their semantic
  validator exists. Do not reinterpret or migrate existing schema-1/2 roots.
- **Evidence schema 1 on new private roots.** Use the bounded append-only record
  and SQLite BLOB custody contract in E1, with explicit record codec and limits.
- **Controlled actual-process first exit.** No credentials, live provider, general
  adapter framework or full host is required for E1. Local observed pre-entry
  facts never become campaign retry authority or unproved remote non-entry.
- **Serial shared-contract ownership.** E0 must be an accepted, compilable handoff
  before the two implementation lanes depend on it. Later interface changes
  return to that owner; the lanes do not independently invent lookalike types.

These are recommended choices awaiting implementation acceptance, not decisions
already granted by planning authorization. A demand for real-provider attestation
as E1's first exit changes the dependency graph and removes the proposed independent
completion: actual dispatch, selected provider adapter and startup ownership must
join first.

## Qualification and remaining boundaries

After both lanes pass, use E1's actual controlled records through the permanent
reader in J1's real campaign/CE/episode composition, including exact two-claim
readback and partial-commit recovery without another child invocation. A passing
fixture-only J1 test cannot substitute for that joined proof. Preserve P1's
integrity checks and measured read improvement; remeasure changed paths only
when their changes or observations justify it.

The earlier broad roadmap's HP3/HP4 terminology is clarified: HP3-E1 owns the
permanent custody prerequisite and later HP3 composes the controlled host;
HP4 supplies the actual provider/dispatcher/startup qualification. J2 findings,
consumption and evaluation follow J1. Retained remediation, correction/succession,
large-history qualification, state migration and operational cutover retain their
separate scopes. No temporary Node bridge or continuous-availability system is
introduced. Lifecycle S6/S7 and terminal UI remain with their current owners.

## Planning record

Parent reconciliation caught and resolved the immutable ER1 target timing gap,
future J2 state admission, and the distinction between a test feature and an
actual evidence-profile boundary. This is source-backed planning and reciprocal
interface coordination, not an independent adversarial implementation review.
No product edits, builds, tests, providers or commits ran in this planning task.

Evidence and exact plan hashes are retained under
`/home/bline/.local/state/work-engine/rust-j1-hp3-planning-20261008/`.
Workflow: two parallel planners and parent reconciliation. Token/cost totals were
not exposed; no values are inferred. There were no test or runtime qualification
results in this task.
