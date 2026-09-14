# Reconciliation: Control Plane and Client Protocol

## Status

Reconciliation of `ideas/control-plane-and-client-protocol.md` against current
app-server implementation and current prospective architecture. Wave 1, item 1
of the sequel reconciliation queue.

Per instruction: the idea was not improved or modernized. Every clause was
checked against implemented code or an existing prospective-architecture
document. What is already implemented or already owned elsewhere is retired
below. One clause has no current owner and is placed, not rewritten.

## Idea summary (unchanged)

`ideas/control-plane-and-client-protocol.md` proposes a bounded coordination
layer owning: stable logical role identity, activation/delivery, scheduling,
runtime routing, acknowledgements, subscription/reconciliation, and
human-facing client interaction — explicitly not domain truth. It names
`role-scheduler` as its one active-construction foundation, lists candidate
control-plane responsibilities, calls out a "Future resource-claim
coordination" subsection as exploratory, defines a client-protocol contract
(control packets bind action/subject/revision/authority/expected
consequence), and disclaims ownership of proposal meaning, workflow state,
review findings, Git truth, model judgment, and human authority.

## What is already implemented

### `role-scheduler` (root substrate only, not yet ported)

`skills/role-scheduler` exists at the root/Condex substrate only. It has zero
references anywhere in `app-server/src`. `app-server/docs/skills-migration-plan.md`
already names it as a migration target — "Durable scheduled-item mechanics and
role-facing judgment" → "Server scheduler service plus role-scoped projection
where active" — in the amended-selection-priority candidate set for later
work. The idea's own "Current evidence" section already states this
prototype's capabilities and gaps accurately (durable schedules, logical-role
addressing, revisioning, delivery, ack/cancellation, daemon lifecycle,
blocking wait/streaming subscribe, restart persistence; no generalized
activation leases, authority validation, robust claim recovery, or
host-mediated wake-up). Nothing here needs correction — the idea's own
evidence section is accurate and the migration is already tracked. This is
not a semantic gap; it is a known, already-scheduled porting task.

### `app-server/src/operator-switchboard.mjs` (partial client-protocol realization, already in app-server)

`OperatorSwitchboard` already implements a real slice of the "Client
protocol" section: `OPERATOR_COMMANDS` (`help`/`agents`/`attach`/`detach`/
`status`/`threads`), a `bindingView()` projection over a role-binding
registry (`get`/`listBindings`), attach/detach flows, turn delivery via
injected `runtime.deliverTurn`/`startTurn`, and input-custody-aware queuing.
This covers "discover available controls," "runtime-binding lookup," and
"bounded projection routing" from the candidate-responsibilities list, and
part of "human/client control packets." It has no active-binding fencing or
runtime-selection-policy-overlay machinery, consistent with the idea's own
disclaimers.

### `workspace-coordination` (retires "Future resource-claim coordination")

The idea's "Future resource-claim coordination" subsection describes,
verbatim: atomic resource claims/leases over domain-defined resources, with
"claim identity, fencing, expiry, renewal, release, and crash reconciliation"
owned by a control-plane realization "without becoming the semantic owner of
the protected resource."

`app-server/src/services/workspace-coordination` (core: `contract.mjs` +
`service.mjs`) already is this, and predates the idea's framing as exploratory:

- operations: `acquire` / `inspect` / `release` / `admitMutation` /
  `savePublication`;
- typed `RESOURCE_TYPES` (`contract.mjs:4-6`): `directory`, `git-ref`,
  `git-index`, `port`, `index`, `review-budget`, `database` — already
  non-Git-specific, already domain-neutral;
- lease record fields (`contract.mjs:48-61`): `leaseId`, `resource`,
  `holder`, `intentId`, `fencingToken`, `issuedAt`, `expiresAt` — claim
  identity, fencing, and expiry are all present;
- authority model: fencing-token possession — `admitMutation` verifies the
  lease's generation before running the caller-supplied mutation
  (`service.mjs:11-29`);
- recovery: monotonic fencing-token generations plus CAS-checked mutation
  admission.

`app-server/docs/service-plane-inventory.md:352-381` already scored this as
"the strongest existing kernel-shaped primitive found in this inventory" and
"domain-neutral coordination infrastructure today, not a code-domain adapter
in disguise." `hierarchical-planning-and-multi-supervisor-orchestration.md:685`
independently cites the same mechanism as authoritative and already relied
upon: "Existing worktree isolation, resource coordination, fencing, provider
admission, and publication boundaries remain authoritative. The orchestrator
coordinates access to those mechanisms rather than replacing them."

One named verb in the idea, "renewal," has no distinct operation in
`workspace-coordination`'s current surface (`acquire`/`inspect`/`release`/
`admitMutation`/`savePublication` — no separate renew). This is a
completeness question inside an already-owned mechanism, not an unowned
semantic consequence, so it is not placed as a gap here.

Disposition: retire this subsection. The resource-claim/lease/fencing
mechanism it anticipated is already built, already inventoried, and already
in cross-service use elsewhere in the codebase.

## What is already owned by prospective architecture

### `OperatorProjection` (`provider-turn-harness-runtime-and-operator-projection.md` §4)

`OperatorProjection` already claims most of the idea's "Client protocol" and
"Control-plane ownership" sections almost verbatim:

- "logical roles and active realization identities" — stable logical routing
  identity, runtime-binding lookup;
- "admitted, running, paused, stale, or queued operations" — delivery/claim
  state;
- "leases, fences, approvals, and reconciliation state" — subscription and
  reconciliation cursors, runtime/delivery health (partial — see below);
- "runtime-selection and cost-policy settings" and "administrative commands"
  — human/client control packets;
- "attributable history of operator-directed changes."

It also states the same non-authority constraint the idea states independently:
"The projection must not make a UI session the canonical role, workflow,
context, or policy identity. UI controls submit commands or proposed overlay
revisions to Work Engine; they do not directly mutate active adapters" —
matching the idea's "UI buttons do not create authority."

Disposition: retire "Client protocol" and most of "Control-plane ownership"
as independently, compatibly owned by `OperatorProjection`. This is a
`CORRESPONDS` relationship, not a conflict — both documents converge on the
same shape from different starting points.

### `pre-indexed-capability-resolution-and-frozen-runtime-realization.md` (lease/fence state as consumed input, not minted here either)

This doc explicitly lists "leases and fences," "retained-session state," and
"reconciliation obligations" as inputs its resolution step must account for
(`§1`, "Current role and workflow state") and continuously observes "leases
and fence state" (`§11`) to invalidate realization dependencies. It does not
mint active-binding fences; it consumes fence state minted elsewhere. This
confirms the gap identified below rather than closing it — no document
inspected in this reconciliation claims ownership of *minting* the fence
that determines which admitted realization currently holds authority to run
as a given logical role instance.

This also yields a useful distinction the reconciliation surfaces but does
not resolve:

```text
realization admission
    determines what may run

active-binding fence
    determines which admitted realization
    currently has authority to run as this role
```

`pre-indexed-capability-resolution-and-frozen-runtime-realization.md` owns
the former. Nothing inspected here owns the latter.

### `hierarchical-planning-and-multi-supervisor-orchestration.md` §4 (Operator interface — adjacent, not duplicative)

This doc's "Operator interface" (operator ↔ orchestrator natural-language
interaction, becoming a durable typed decision) is a workflow-layer concern
riding on top of whatever transport/delivery/session mechanics exist —
exactly the kind of consumer the control-plane idea describes serving. It is
not a duplicate of the client-protocol idea; it is a prospective consumer of
it. No correction needed; noted as a `CORRESPONDS`/`SUPPLIES` seam for future
reference, not a disposition-changing finding.

### `control-plane-causal-observability-ui.md` (`app-server/docs/`, already accepted — adjacent, not duplicative)

An already-accepted, read-only "control-plane" UI design exists, but it is
explicitly scoped to causal observability (live work, workflow causality,
cost, evidence-backed improvement) and explicitly disclaims authority: "do
not make the UI a new owner of workflow truth or authority." It does not
reference `OperatorProjection`, `role-scheduler`, or this idea, and does not
address control-packet submission (the write path). It is a distinct,
non-conflicting neighbor — a plausible future read-side consumer of the same
protocol, not a competing owner. No correction needed.

## What the idea itself already excludes (no reconciliation needed)

The "Domain boundary" section's exclusions (proposal meaning, workflow
semantic state, review findings, Git/checkpoint truth, model judgment, human
authority) already match how every other document in this reconciliation
divides ownership. No conflict found.

## The smallest remaining semantic consequence still lacking an owner

**Fenced active-binding coordination for logical role instances** —
ensuring that at most one runtime realization generation holds the
authoritative active binding for a logical role instance, with fencing,
expiry, renewal/reacquisition, release, and crash reconciliation — is not
owned by any current implementation or prospective-architecture document
inspected in this reconciliation:

```text
logical role instance
        |
        | authoritative active binding
        | fence generation N
        v
realization A
```

and during replacement:

```text
logical role instance
        |
        | fence generation N+1
        v
realization B

realization A
    still may physically exist
    but no longer possesses active-role authority
```

Operator clients are orthogonal to this gap, not part of it — multiple
`OperatorProjection` clients observing or controlling the same role does not
by itself create split-brain role execution:

```text
client A ─┐
client B ─┼─> OperatorProjection
client C ─┘
              |
              v
        Work Engine state
```

Evidence for the gap:

- `role-scheduler` explicitly does not yet establish this (idea's own
  "Current evidence" section, unchanged and accurate);
- `operator-switchboard.mjs` has an attach/detach registry but no fencing or
  expiry semantics on the active binding;
- `workspace-coordination`'s `RESOURCE_TYPES` enum (`directory`, `git-ref`,
  `git-index`, `port`, `index`, `review-budget`, `database`) has no
  role-active-binding-slot kind — the mechanism that would naturally host
  this is present, but role active-binding is not one of its owned resource
  types;
- `OperatorProjection` projects lease/fence state for display and control but
  does not claim to mint it;
- `pre-indexed-capability-resolution-and-frozen-runtime-realization.md`
  determines what may run (realization admission) but explicitly consumes
  fence state as an observed input rather than minting the fence that
  determines which admitted realization currently holds authority to run.

This is the one clause placed rather than retired. It is not rewritten or
designed here, per the standing constraint. A future owner should decide
whether it is a new `workspace-coordination` resource type (a
role-active-binding slot) or a distinct mechanism — that decision is out of
scope for this reconciliation.

## Disposition summary

| Idea section | Disposition |
| --- | --- |
| Status / Current evidence (`role-scheduler`) | Accurate as written; already tracked as a migration target in `skills-migration-plan.md`. Retired — no action needed. |
| Control-plane ownership (routing identity, runtime-binding lookup, delivery/claim state, subscription/reconciliation, projection routing, control packets) | Retired — already implemented (`operator-switchboard.mjs`) or already claimed compatibly by `OperatorProjection`. |
| Future resource-claim coordination | Retired — already implemented by `workspace-coordination`, already inventoried as domain-neutral kernel infrastructure, already relied upon elsewhere. |
| Authoritative active-binding coordination for logical role instances | **Not retired.** Smallest remaining semantic consequence lacking an owner. Placed, not designed. |
| Domain boundary | Retired — consistent with every other document's ownership split. |
| Client protocol | Retired — already claimed by `OperatorProjection` (`CORRESPONDS`). |
| Relationship to scheduler / runtime adapters / Studio | Retired — consistent with `role-scheduler`'s migration status, `provider-turn-harness-runtime-and-operator-projection.md`'s port boundaries, and `control-plane-causal-observability-ui.md`'s Studio-adjacent, non-authoritative framing. |

## Recommended status change to the idea file

Update `ideas/control-plane-and-client-protocol.md`'s Status section to note
that this reconciliation exists and that all but one clause is retired,
pointing to this document. The remaining open clause (authoritative
active-binding coordination for logical role instances) should be the only
thing left active in that idea file going forward.

## Acceptance

**Accepted 2026-09-14** (explicit user decision, after a closer-look review
of this reconciliation as part of the sequel queue's acceptance pass). This
document's findings and disposition are confirmed accurate. Implementation
of the stated residue — fenced active-binding coordination for logical role
instances — is authorized to proceed.
