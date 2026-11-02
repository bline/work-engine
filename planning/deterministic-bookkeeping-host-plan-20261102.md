# Deterministic bookkeeping host: plan and proposed budget

Prepared 2026-11-02 for the user's request to prepare a plan and budget.
Status: proposed; implementation, spending, merge, and release are not authorized
by preparation of this document. The phase order and allocations below are a
recommended execution route, not new repository doctrine.

**Recommendation:** fund a Codex-only dispatch/collection pilot with a target of
$80–100 API-equivalent spend and a ceiling of **$120 / 10 elapsed hours**.
Reserve **$20 and the final hour** for collection, review disposition, and closure.
Treat structured Dot grants as a separate **$40 / 4-hour** option after the pilot.
The combined proposed ceiling is $160 / 14 hours; neither amount is a current grant.

## Outcome and scope

The root supplies an authorized assignment and makes semantic decisions. A
deterministic host performs the already-determined dispatch, observation,
collection, and accounting operations and returns compact receipts backed by
durable evidence. A normal supported child lifecycle should require no new
root-authored Python, manual hashing, rollout parsing, or repeated budget and
context polling.

The first delivery covers the outer manual development workflow, one Codex
launch backend, and one bounded objective at a time. It includes root budget
observation and children, including separately launched reviewers where their
existing isolation contract can be preserved. It does not replace the Rust
engine's campaign, lifecycle, review, or coordination owners.

Decomposition, packet meaning, acceptance, scope and route changes, integration,
context-replacement decisions, and user communication remain with the root.
Completion, evidence collection, acceptance, and integration remain distinct.
The host executes only transitions determined by authorized inputs and observed
facts. It returns uncertainty to the owning decision-maker.

Excluded from the first delivery: Claude support; automatic successor roots or
context replacement; a universal workflow engine; new budget allocation policy;
automatic grant expansion; public services; a UI; deletion of historical records;
and migration of the Rust engine onto this host.

## Starting evidence and uncertainty

- [DESIGN §9](../DESIGN.md#9-determinism-and-model-judgment) supplies the governing
  split; §§11–12 preserve authority and provenance.
- The user's study reports 27.8 bookkeeping hours out of 52.8 root hours, including
  an estimated 22.4 mechanical hours. These numbers and the original measurement
  scripts have not been independently verified here. The latter ratio is 42.4%,
  a candidate removable-work share, not a savings forecast.
- [Budget observation revision 3](work-engine-budget-observation-implementation-plan.md)
  records phases 0–3 as merged and released at `99450c33`. Its scope is the outer
  Python tooling; it explicitly avoids a competing budget registry and new
  per-assignment allocation bureaucracy.
- Local `budget-close-integration` points to
  `aa95c285b70b72752f801fda0b424bb4b6617cf7`. Its recent history includes record
  serialization, startup verification, and failed-listener process-group cleanup.
  This planning pass did not rerun its tests or establish release readiness.
- [role_host.Codex.launch](../skills/manual-orchestration/scripts/role_host.py)
  starts a turn before appending its launch ledger row. A wrapper needs to account
  for the interrupted-launch window; composing commands alone does not close it.
- [Context control](../skills/manual-orchestration/references/context-control.md)
  already defines explicit enrollment, observed delivery, cursors, and unavailable
  state. It also says the CLI cannot wake an ended session. Actual root delivery
  is a phase 0 feasibility question, not an assumed capability.
- [assemble_receipt](../skills/slice-supervisor/scripts/assemble_receipt.py)
  is a useful composition precedent: semantic input and mechanically harvested
  telemetry retain distinct origins. Its campaign schema is not automatically
  the manual host's schema.
- [Rust direction](../docs/rust-development.md) favors Rust for a new executable.
  The preferred candidate is a small Rust outer-workflow host invoking existing
  released tools through explicit interfaces. Existing Python tools retain their
  owners; narrowly necessary adapter repairs are in scope. Final placement and
  dependency reuse are settled in phase 0. Existing Rust task/campaign hosts are
  candidates to inspect, not permission to absorb their semantics or rewrite them.

Source observations were made on a dirty shared checkout at HEAD
`b24135b74d53edb521b07cdaabf69c0f1e1987e5`; they are not an immutable build baseline.
The implementation begins from a newly bound, isolated baseline and preserves
other sessions' edits. Graph verification used project
`home-bline-code-work-engine`, store generation `uab1f840cdc5ef601g2`, indexed
2026-11-02T16:31:16Z. Checked cited paths had no recorded coverage issue and were
content-verified; this was targeted verification, not an exhaustive system audit.

## Proposed implementation shape

Expose a small command surface: dispatch an assignment, inspect its state,
collect/reconcile its evidence, and close its observation scope. Names and wire
shapes can change during design; accepted authority and recovery properties cannot.

The host owns execution attempts, effect reconciliation, observer ownership,
collection status, and delivery progress. It references the existing packet,
budget, custody, review, and acceptance records. A consumer projection may combine
them without becoming a second owner of their meaning.

Each operation has a stable identity and input revision. Repeated submission of
the same operation returns or resumes its existing outcome; conflicting inputs
under the same identity are rejected. Before an external action, enough intent is
durable to reconcile its result. When a launch result is unknown and the backend
cannot establish whether it ran, the host reports that ambiguity and does not
blindly launch again. An event stream is a trigger, with restart reconciliation
against authoritative state; a deterministic process may poll if that is the
supported transport. The root should not have to perform the polling.

Dispatch composes the existing claim, task-profile, launch, observed-settings,
identity-binding, budget-binding, and context-enrollment operations. Readiness
reports each establishment result truthfully. Required pre-execution constraints
must hold before protected work starts; post-launch inspection alone cannot
establish that an earlier invalid execution was prevented.

Collection binds result and artifact identities, uses the applicable evidence and
custody checks, captures objective usage, and releases only resources whose owned
work is actually quiescent. Child completion does not release an integration or
review claim that the root still needs. Artifact mutation during collection is
an explicit conflict, not a silently refreshed digest.

Receipts have a compact normal view and inspectable details: operation/task identity,
observed outcome, evidence references, incomplete checks, accounting coverage,
and the decision still owned by the root. Delivery is tracked separately from
receipt creation; producing stdout does not establish that the root received it.
Repeated delivery may occur with the same receipt identity and no repeated effect.

Budget/context observers surface meaningful changes and loss of observation.
Unavailable cost is not zero. Context pressure does not cause automatic replacement.
Budget monitoring preserves the existing distinction between observation and an
authorized enforcement action; this project introduces no universal kill policy.

## Phases, outputs, and planning allocations

These dollar allocations include supervisor, builder, reviewer, and test-agent
inference. They are one objective envelope, not separate grants or mandatory
approval checkpoints. The root may rebalance phases inside accepted scope and
the productive envelope while preserving the reserve.

| Phase | Reviewable output and completion condition | Allowance | Time |
| --- | --- | ---: | ---: |
| 0. Reduce and bind | A bounded field/consumer audit, host owner/interface decision, verified launch/completion/delivery route, and pinned implementation subjects. No unresolved feasibility premise hidden in the build packet. | $10 | 1 h |
| 1. Qualify budget close/listen | Exact branch delta and retained review evidence reconciled with the current target; relevant tests pass; closure, listener cleanup, and accounting tails demonstrated; release candidate and rollback identified. | $15 | 1.5 h |
| 2. Build one complete path | Dispatch through automatic observation and collection for the selected Codex backend, with durable operation identity and restart reconciliation. A normal run returns a receipt without root-authored bookkeeping code. | $40 | 3.5 h |
| 3. Challenge the composition | Fault tests and a separate adversarial review of the exact composed subject; findings repaired or explicitly dispositioned; local release/install/rollback checks pass. | $25 | 2 h |
| 4. Pilot and measure | Small bounded live tasks, delivered receipts, accounting reconciliation, and a report comparing equivalent manual and hosted operations. Release-ready result, or an explicitly authorized release and its verification. | $10 | 1 h |
| Protected reserve | Collect active work, resolve review disposition, reconcile final usage, preserve an unfinished continuation if needed, and report. No new features. | $20 | 1 h |
| **Core ceiling** | **One Codex host pilot** | **$120** | **10 h** |

Phase 0 audits one representative S10 assignment/closure family and a small sample
of dispatch/collection records, rather than every historical record. Classify
fields as owned input, derived projection, required recovery/provenance evidence,
or removable. Identify a consumer or protected distinction for retained fields;
unobserved readers alone do not justify deletion. Retire unnecessary future writes
and preserve historical evidence. Stop the audit at its time allowance.

Also locate the user's original analysis scripts in phase 0. If unavailable,
record that limitation and establish a prospective baseline; reconstructing the
entire historical study is outside this envelope. Before phase 2, re-estimate if
actual event delivery, launch reconciliation, or semantic ownership requires a
larger subsystem. Report the bounded result instead of silently expanding scope.

Phase 1 prepares and, when already explicitly authorized, performs the merge and
release of the qualified budget work. Shared skill activation follows the
[skill-release contract](../skills/skill-release/SKILL.md): it says to “Deploy only
on the user's decision.” An implementation-only approval permits preparation of
the exact release plan; it does not implicitly switch active agents. A private
pinned candidate can support the bounded pilot without switching all sessions.
No waiting for a release decision extends the spending or elapsed-time ceiling.
Skill release exports the skill tree; it does not by itself install a new Rust
executable. The candidate includes an explicit binary provisioning/version contract,
immutable binary identity, and compatibility with its released wrappers. Verify
the pair during installation and rollback. A permanently installed system service
is not required for the first owned, bounded host process.

## Evidence required for the core result

| Scenario | Required observable result |
| --- | --- |
| Normal authorized child | Requested and observed identity/settings are distinguished; budget and context bindings are established; completion reaches collection and a delivered receipt. |
| Duplicate dispatch / two host instances | Same operation cannot cause two admitted children; conflicting inputs fail explicitly. |
| Crash around launch | Recovery finds the original child or reports an unresolved launch; no blind retry and no false “never started” claim. |
| Duplicate, late, or missed completion | Collection converges on one logical result; replay preserves lineage; restart discovers outstanding work. A turn ending does not by itself mean assignment acceptance. |
| Interrupted collection / changed artifact | Resume preserves established evidence; changed input is identified; no stale digest presented as current. |
| Observer failure / delivery interruption | State becomes unavailable or delivery-pending; restart resumes the applicable cursor and binding; no false healthy status or promise to recover all transient observations. |
| Budget close / concurrent objectives | Unrelated sessions remain excluded; included calls are counted once; root, child, reviewer, and attributable peer costs are included; post-close tails and unknowns remain visible. |
| Custody / claim release | Relevant owned processes are collected; root-owned claims remain until their owning condition is satisfied; no orphan observer is concealed by a success receipt. |
| Acceptance separation | A failed mechanical check blocks the dependent claim; passing checks never manufacture semantic acceptance, new authority, integration, or context replacement. |
| Release / rollback | Exact binary and script versions are pinned; supported install path works; restoring the prior version preserves unresolved task records or refuses incompatible state truthfully. |

Use deterministic fixtures for faults and races, plus a small number of actual
backend launches for the integration claims. The core allowance includes the
model work of those bounded test tasks. Do not hide a larger product campaign's
spend inside “measurement”; using such a campaign requires its own already-granted
scope and transparent attribution of shared cost.

## Staffing and delivery

The proposed implementation organization has one root integration/decision owner,
one retained builder, and a separately launched read-only reviewer. It adds no
supervisor layer. These roles are a future execution proposal; none was launched
to prepare this plan.

Follow [model-selection guidance](../docs/model-selection.md): `gpt-6.1-sol`/`xhigh`
for root decisions, `gpt-6.1-terra`/`medium` for a settled implementation packet,
or `gpt-6.1-sol`/`medium` if concrete recovery/concurrency decisions remain with
the builder. Use `gpt-6-astra` at standard service tier for separate review (high for implementation,
xhigh for unresolved ownership/design). Record requested and observed settings.
This is different-model review, not a cross-provider independence claim.
Retain useful builder/reviewer context through ordinary remediation.

Code belongs in an isolated worktree, with an agreed Rust owner and only the
necessary existing-tool adapters. Keep operating guidance with manual
orchestration, results in `records/`, and measurements in `metrics/`. Prefer
extending owned records over adding parallel assignment/closure schemas. One
root integrates the composed result and reconciles concurrent shared-workspace
changes. Final delivery includes the source subject, validation, unresolved
limits, release/rollback plan, and observed spend.

## Budget basis and stop behavior

The dollar figures are proposed **API-equivalent accounting limits**, using the
repository usage ledger's versioned rate/service-tier treatment. They are not
provider price quotes, invoices, or subscription-quota guarantees. The $80–100
target and phase allocations are engineering estimates with low-to-moderate
confidence, not a fitted estimate from the four-root study. The allowance covers
one backend, reuse of existing scripts, focused fault testing, and ordinary review
repair. A new native transport, general scheduler, or state-owner migration needs
a new estimate.

As a rough scale check only, the budget-observation plan records $133.73 for an
S10 root tree over approximately 17.6 hours, about $7.60 per elapsed hour. A $120
ceiling over 10 hours allows $12 per hour across the whole objective, with a
protected closure reserve. Task mix, service tier, context size, and review effort
can change that rate substantially; this single historical window is not a
reliable cost model.

The implementation window begins only on authorization, with an exact start,
deadline, objective binding, and meter basis recorded. This planning turn is not
retroactively charged against that future grant. Include all related root,
builder, reviewer, helper, replacement, and pilot work once; name missing coverage.
Do not add another objective's unused allocation or sum nested inclusive totals.

At **$100 observed spend or 9 elapsed hours, whichever comes first**, stop new
feature work and use the remaining $20 / 1 hour for safe collection and closure.
Admission considers in-flight work and telemetry delay before that threshold;
it is not a promise that delayed measurements impose a hard provider billing cap.
If observer failure makes headroom unestablished, stop admitting new paid work
until accounting is restored. Do not extend a grant because a child is unfinished.
Return a truthful partial result and continuation if the ceiling prevents completion.

One core grant can cover the internal phases without repeated small approval
requests. Scope, authority, or ceiling changes still return to the user; ordinary
repairs and phase rebalancing within the grant do not.

## Pilot evaluation and optional expansion

Compare equivalent operations with unchanged grant size and acceptance conditions.
Record root active time, bookkeeping-only turns, new ad hoc bookkeeping programs,
root and total API-equivalent spend, event-to-receipt latency, repair/manual-recovery
effort, review findings, and evidence omissions. Distinguish active model time from
wall-clock waiting; report mismatched tasks and small sample size.

A useful pilot target is **at least 50% fewer root bookkeeping turns on supported
operations**, with no newly observed authority/evidence defect and no increase in
total operational cost for comparable tasks. This is a proposed adoption target,
not an existing doctrine or proof that rare failures are absent. A small sample
can establish functionality and expose regressions; sustained savings require
subsequent ordinary runs. The 42.4% historical ratio remains unverified.

**Optional structured grants: $40 / 4 hours**, comprising $30 / 3 hours of productive
work and $10 / 1 hour of closure reserve. Target spend $25–30. Reuse the established
host and budget revision owner. Bind grant identity, issuing authority, target
objective, scope, expected prior revision, changed limits/deadline, validity, and
source evidence. Applying the same grant twice has one effect; stale, conflicting,
wrong-objective, or unauthorized grants fail explicitly. The root retains semantic
interpretation when a grant cannot be mechanically applied.

This option assumes an existing trustworthy Dot event ingress and defined
delegation. Establish those before construction. Building authentication,
transport, or a new delegation system is outside the $40 option. Grant size itself
remains the user's choice and is not changed to improve the pilot's measurements.

Broader provider support, many concurrent objectives, and migration into the Rust
engine are unbudgeted follow-ons. Estimate them from pilot evidence rather than
including a speculative platform build in this first grant.
