# Native-review production-path claim evidence — implementation plan candidate

## Status and authority

- Status: planning candidate; not implementation, campaign-launch, provider-use,
  bootstrap-protocol, roadmap-priority, S14-ownership, commit, or push authority.
- Accepted proposal:
  `work-engine.production-path-claim-evidence.native-review-admission-terminalization`.
- Bounded objective: repair PPCE-03 and PPCE-04 as one native-review
  selection-to-terminalization vertical before S14 continues.
- The pre-S14 intake staging-path defect, PPCE-01, and PPCE-02 remain separate.

If the operator later authorizes this exact campaign, that activation supplies
implementation authority only through an accepted slice plan and boundary. It
does not by itself settle S14 ownership or authorize a provider/bootstrap route
that the activation does not name.

## Revision binding

- Repository commit: `9a1016987be7676d79a98ca44e8582dc83036548`.
- Accepted proposal packet SHA-256:
  `2f5d6222d48a938227521baacd28ff92e7f126d3c7dc243e2b73a16306a98358`.
- Proposal narrative SHA-256:
  `4063126d421ad0baf255f13e60242b17018b399eaf017e249af16dd9e8e0cb7e`.
- Decision SHA-256:
  `e620c99da7dfa33433399a7fc6b176ba42e9401f3942c978255f2e0e38f889dd`.
- Audit SHA-256:
  `4f669f2f2ff54bba785a43352cbefafc438dae899c0d8b0e10687286fbf8188a`.
- `DESIGN.md` SHA-256:
  `2213ab9a32925e7792353489599201bf71f3268eb8981cc192f87c57e2952095`.
- `PHILOSOPHY.md` SHA-256:
  `299b0ad8192f3cc442f6491d2908f421f22333024ffb1423cae6856b69c3a307`.
- Research architecture SHA-256:
  `651f8ca073b3c09dca2f8b5f63251ba3a13f98a5ca16b46841865609b7045599`.
- App Server claim-evidence architecture SHA-256:
  `66b6ec9605e08836e7450d2bd9033d094ceeacc7fdff7498010908a1413f7349`.
- Bidirectional repository-integration proposal SHA-256:
  `552bf0af24ba12e911805b6526e955183ee9388c9413aa399f3b06c09866a757`.
- Decision-gated implementation-compilation architecture SHA-256:
  `f6aa0b75cfc4d3dd8f1b1ca706e20f539bc33256903d7de19f89f275da628d50`.
- Candidate campaign SHA-256:
  `53c26cbe8bc9d1f2f252b0147a705b11549c8ce570e6662d26bfc179c67dbf5a`;
  Slice Supervisor version-2 preflight passed without launching it.

Any change to these subjects requires plan reconciliation before launch.

## Placement decisions proposed for plan acceptance

1. Extend the existing App Server `claim-evidence` service as the single owner
   of production-path claim identity, versioned profile mechanics, host
   observations, immutable establishment records, exact-revision reliance,
   persistence, and read projections. Add a `production-path-v1` domain
   profile; do not create another claim registry or identity scheme.
2. Slice Campaign continues to own required-claim declaration and consumption.
   The reviewer host continues to own execution observation. Claim-evidence
   admission evaluates the configured profile but cannot supply domain truth,
   reliance, or waiver authority. Review Episode owns the immutable
   admitted-result binding. Slice Campaign terminalization owns the acceptance
   consequence.
3. Record the compact normalized host receipt through the existing canonical
   observation store and bind the admission to its exact observation identity
   and digest. Keep large/native artifacts behind host-owned stable references
   with retention through every covered boundary, deterministic resolution,
   and explicit `unavailable` behavior. Do not put absolute host paths or
   Claude transcript formats into canonical contracts.
4. Version the persisted review selection rather than reinterpret version 1.
   A version-2 selection declares exact required claim revisions; terminal
   projection carries their establishment and reliance references plus any
   separately authorized owner dispositions.

These are candidate ownership decisions. Human plan acceptance may revise
them; the builder and supervisor may not.

## Contract shape

The existing claim-evidence core is extended with one domain profile and one
new immutable relation rather than duplicated. Its records divide as follows:

- the production-path claim revision binds the required proposition, complete
  claim identity, evidence profile, and authority route;
- the existing observation record binds attributed host facts and native
  artifact references;
- an establishment record binds one exact claim revision, the admitted
  observation or recording revisions, evaluator/build identity, profile
  result, status, reasons, and any correction/supersession predecessor; and
- existing exact-revision reliance binds the builder or campaign consumer to
  the claim revision, while the Review Episode or Slice Campaign consumption
  binding separately identifies the exact establishment record it consumed.

Each production-path claim revision binds:

- stable claim ID and revision;
- proposition/kind;
- exact candidate and review-episode subject;
- covered state;
- one consumption boundary and consumer;
- acceptance source and owner route;
- claim-relative evidence profile, including allowed mechanisms and admissible
  recorder/observer identities;
- evidence references and integrity requirements.

The establishment record, not visibility or the claim revision alone, carries
`established`, `false`, or `unestablished` with attributable reasons. A
constraint contributes no establishment evidence by itself.

Different consumers or boundaries are different claims. In particular, first
projection of findings into builder state and campaign terminal acceptance are
not silently collapsed. They may share the same host observation, but each
requires its own declared claim identity and evaluation.

The initial native-review independence profile combines constraints with
independent observation. Constraints include immutable subject, the admitted
read-only capability envelope, no mutation authority, and the selected
fresh/retained continuity rule. The App Server reviewer host observation is the
evidence mechanism. A specialist or provider cannot attest its own
independence. Other profiles may permit actor recording when their consumer
requires only attributable conduct; independent observation is not universal.

The normalized host observation binds selection, obligation, execution
attempt, candidate, review episode, requested and observed realization,
capability/mutation envelope, session/continuity facts, transport digest,
observer identity, observation time, adapter version, and native artifact
references. After Implementation Review admits the semantic result under its
own contract, the claim-evidence profile evaluator verifies the production-path
bindings. Review Episode may expose only their combined immutable binding to a
consumer.

`reported` remains workload lifecycle state. It never implies evidence
establishment. Terminalization evaluates only claims actually consumed by the
requested outcome and requires each to be established or to carry a separate,
matching disposition from its acceptance owner. An unestablished claim routes
to that owner; it does not independently halt unrelated work. A false claim is
not waivable by an adapter, reviewer, builder, or terminalizer.

## Bidirectional documentation and code compatibility

Production-path profile revisions use the existing evidence and sensitivity
reference mechanics to identify both governing documentation anchors and the
code symbols, schemas, tests, and projections that realize or consume them.
The canonical relationship is owned once; forward and reverse traversal are
derived projections rather than separately maintained backlinks.

This makes later impact queries possible in both directions:

- a change to `DESIGN.md` section 3.9 can nominate affected selection,
  admission, receipt, terminalization, and test surfaces; and
- a change to a validator or terminal gate can nominate the governing claims
  and documentation whose realization may have changed.

PPCE preserves stable, typed, integrity-bound endpoints suitable for the
planned Codebase Memory current-world projection. It does not implement
`sync_projection`, automatic `may_affect` publication, semantic refresh, or
the full claim-maintenance and reliance-propagation proposal. Codebase Memory
remains a rebuildable traversal projection, not the canonical claim owner or
the synchronous terminal acceptance authority.

## Compatibility with implementation compilation

The pending decision-gated implementation-compilation document remains an
overall architectural plan, not a PPCE runtime dependency or activated
workflow. PPCE nevertheless preserves its separations: claims describe
evidence-backed state, owner decisions select materially different routes, and
implementation plans derive repository transformations. Exact revisions and
repository bindings remain available for a future implementation basis.

The present campaign does not implement decision-set, compiler,
plan-conformance, or model-routing infrastructure. During planning or coding,
a newly discovered route-variant ownership, schema, migration, recovery,
security, or acceptance choice returns to the relevant decision owner instead
of being hidden as builder discretion.

## Compatibility and recovery

- Version-1 selection and already-terminal historical records retain their
  original meaning; no retroactive establishment is inferred.
- A version-1 in-flight result may be normalized only from exact retained host
  evidence. If that evidence cannot be resolved, the affected claim is
  `unestablished` and routes to its owner. Never replay provider work merely to
  rebuild discarded evidence.
- Corrections and remediation append `corrects`/`supersedes` admissions. They
  do not overwrite prior evidence or status. A changed candidate, selection,
  covered state, boundary, or consumer creates a new claim identity.
- Recovery may resolve an existing host reference and repeat deterministic
  admission. It must preserve idempotency and distinguish unavailable evidence
  from contradictory evidence.
- Compatibility review with no independently required route claim remains
  outside this new obligation.

## Planned implementation slices

### Slice 1 — versioned contract and one complete vertical

Implement the backend-neutral contract and the native-Claude success path from
version-2 selection through host observation, admission, Review Episode,
builder projection, terminal receipt, and terminal gate. Prove both a valid
established result and a valid semantic result whose missing/mismatched
evidence remains unestablished and cannot be consumed.

Likely owned surfaces: the existing
`app-server/src/services/claim-evidence/` contract, validation, observation,
SQLite, and a production-path domain adapter; Slice Campaign contract/service
and native closure/host; Review Episode service and store compatibility;
native Claude receipt adapter; and focused service and vertical tests.

### Slice 2 — lifecycle, recovery, and mixed revisions

Add false/unestablished owner routing, disposition validation, correction and
supersession, remediation, multi-specialist behavior, restart/reference
resolution, version-1 in-flight classification, and retention/unavailability
tests. Prove no provider replay and no inference from a role label, session ID,
or `reported` state.

### Slice 3 — projections, instructions, and transition audit

Update Slice Supervisor review-selection and terminal-receipt contracts so the
execution role receives the evidence obligation and the terminal consumer can
evaluate it. Update capability projections and assemblers only where the new
contract crosses them. Add canonical documentation/code sensitivity references
and prove their deterministic forward/reverse projection shape without
implementing Codebase Memory synchronization. Run focused and full applicable
suites, generated/drift checks, an agent-instruction review for changed
normative role text, and a shadow inventory of current
selections/receipts/manifests. Produce a separate transition audit; do not
self-accept the migration.

Each slice must first prove its smallest vertical consequence. If ownership or
consumer identity changes, return to plan acceptance rather than broadening the
slice.

## Acceptance evidence for the implementation campaign

The eventual campaign must show:

1. a selection cannot declare an incomplete or self-authorized claim;
2. a successful host execution produces a normalized, integrity-bound
   observation without canonicalizing Claude artifacts;
3. Review Episode cannot expose a result before claim-relative admission;
4. builder reliance and accepted terminalization reject missing, mismatched,
   self-attested, expired, or unavailable required evidence;
5. matching owner disposition is required when a consumed claim is not
   established;
6. recovery and correction preserve history and never replay provider work to
   reconstruct evidence;
7. version-1 records remain truthful and are not silently upgraded; and
8. the ordinary compatibility path and claims requiring only recording remain
   no more constrained than their consumers require; and
9. documentation/code relationships have one canonical owner, stable typed
   endpoints, and equivalent forward/reverse projections without granting the
   projection impact or acceptance authority.

## Launch blockers still owned by the operator

- Accept or amend this detailed plan and its placement decisions.
- Authorize campaign launch and implementation.
- Authorize the configured Claude review provider or replace it with another
  explicitly classified review route.
- If current native review/session artifacts are to validate the repair,
  accept a bounded bootstrap protocol naming exact artifacts, covered claims,
  retention, expiry, and authority. The broken path cannot certify itself.
- Separately authorize any repair of the pre-S14 intake staging-path defect.
