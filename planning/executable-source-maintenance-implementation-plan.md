# Executable-source maintenance — implementation plan 1

## Decision requested

Accept one bounded **out-of-band implementation slice** against
`ad99d46cd73996b580f1263496ccba75acc2c431`: separate executable-source location
from the preserved role realization, and make candidate staging inert until
durable admission. No live activation, PPCE continuation, incident-488 replay,
publication, or push is included in implementation acceptance.

This exact plan consolidates [design v2](executable-generation-location-reconciliation-plan-v2.md)
(`9a4b65e35ee9f78cdc0b3952fe50218ba919d412945d4b02123928f1353b4801`)
and its [review disposition](executable-generation-location-plan-v2-review-disposition.md)
(`9fd6a400d76a5c13e0a4ef4eff5385ee4685ae02a0029d818581dcbb59375c87`).
Those artifacts remain unchanged. The v2 review was advisory and mixed, not a
clean independent pass. Its concrete implementation seams are resolved in the
scope below; feasibility still requires executable proof and independent review.

## Protected result

An admitted source successor can operate on the predecessor's exact operational
repository and artifact destinations while retaining canonical path identities,
per-role binding fingerprints, binding revisions, delivery history, attachments,
and queued-input custody. Candidate code and instruction bytes remain attributable
to the candidate, not to dirty files in the operational repository.

The original generation and per-role fingerprint algorithms remain authoritative.
Neither equality check is relaxed. No historical fingerprint, binding, delivery,
or queue record is rewritten to make the candidate fit. A genuine semantic
environment change remains a rejected migration requiring another plan.

## Confirmed consumer consequences

- Manifest projection already accepts an identity base distinct from its
  activated-skill base. Bootstrap must preserve that distinction when capturing
  the generated configuration and when hydrating requirements.
- Product-development environment identity hashes the actual repository and
  artifact roots. Preserve those targets rather than normalizing away their paths.
- Live-host construction opens writable semantic state and starts recovery;
  validation needs a distinct inert path, not relocation of the same constructor
  into a validate callback.
- Both bootstrap startup and in-process generation reload currently call worker
  activation before durable selection. Both paths must respect the effect fence.
- Legacy supervisor and chatboard adapters derive helper-script paths from the
  operational repository. Separating roots only at worker launch would still
  execute dirty predecessor helpers. Their source/target split is required.

These observations are direct-source checks plus bounded graph discovery.
Coverage metadata generation `2026-09-12T16:53:10Z` reported no recorded gaps
for these inspected consumers. That is not proof of exhaustive coverage; tests
must verify the actual runtime routes and source closure.

## Implementation contracts

### 1. Source/realization configuration and compatibility

Introduce one normalized host input representing executable source, manifest
identity base, requirements identity base, operational repository, artifact
root, and explicit provider working-directory semantics. Non-default mappings
are bound to the exact predecessor and maintenance authorization. Same-root
startup keeps compatible defaults. Do not expose an unrestricted fingerprint
override or caller assertion of equivalence.

Bootstrap captures manifest, skills, requirements and dependency bytes from
the candidate and propagates preserved canonical bases through
`roleEnvironmentSource` to `projectRuntimeManifest` and the generated role config.
Preserve actual source provenance separately. Runtime requirements checks must
compare the correct canonical contract identity against the candidate's verified
content; they must not reopen mutable predecessor requirements as candidate code.

Expose a read-only compatibility description derived through existing manifest,
role-environment and adapter logic. Compare the candidate's generation fingerprint
and every affected existing role-binding fingerprint before effects become
possible. Missing provider-negotiation inputs leave compatibility unestablished;
do not substitute guessed capabilities or start a domain turn to obtain them.
Protocol initialization/negotiation, if necessary, is distinct from domain work
and must not deliver role input or start semantic recovery.

Stable host helpers and generation worker modules must come from the admitted
executable source closure. The closure may distinguish stable-host and snapshot
components, but both need exact source identity and integrity checks. Operational
targets remain independently configured. Audit imports/subprocesses exercised by
the bounded capability fixtures; an uncaptured dependency fails the source proof.

### 2. Inert staging and admitted lifecycle

Split immutable environment description/validation from creation of active live
services. Validation cannot open a writable live semantic store, install active
telemetry/retry consumers, perform provider-turn requests, recover queued inputs,
or start campaign services. Read-only projections and isolated fixture state are
permitted. Do not add a permissive flag whose default lets a staged candidate
perform effects before its owner admits it.

Use worker lifecycle callbacks and the parent dispatch gate to enforce the
distinction. Add explicit, idempotent recovery enablement at the retained/live
host boundary; creating an object is not recovery authorization. Queue authority
continues to come exclusively from custody, not from the new lifecycle method.

### 3. Durable selection, operating mode and crash behavior

Generation store owns the exact source/realization maintenance record and its
CAS publication. Bind predecessor, successor, relevant state revisions,
authorization reference, compatibility evidence identity, and operating mode.
Maintenance-only mode must survive restart and admit only read-only administrative
status, not domain delivery or lifecycle/queue recovery. A future change to normal
operation requires its own exact owner-authorized transition; it cannot fabricate
incident-488 release authority or be inferred from technical readiness.

Worker readiness before CAS remains inert. After CAS, startup/reload completes
the recorded successor activation; admission stays fenced until runtime and
durable selection agree. Cover initial initialization, compatible startup reload,
in-process reload, and recovery of an interrupted activation. If activation fails
after selection, record/report the selected-but-unavailable state and keep effects
fenced; do not apply the precommit failure path or silently reinstate a predecessor.

Expose source revision/digest, generation identity, realization identity,
activation state, and maintenance mode in read-only status. Do not claim
`active_unexercised` proves retained-role continuation. Stable-host changes require
a process restart; in-process reload cannot silently replace stable code.

## Exact allowed production/document paths

Paths below are an upper bound, not a requirement to edit every file. A new path
or semantic owner outside this boundary requires a reported plan amendment.

1. `app-server/scripts/app-server-proxy.mjs`
2. `app-server/src/executable-generation-bootstrap.mjs`
3. `app-server/src/executable-generation-store.mjs`
4. `app-server/src/executable-generation-manager.mjs`
5. `app-server/src/executable-generation-worker.mjs`
6. `app-server/src/executable-generation-worker-runtime.mjs`
7. `app-server/src/default-executable-generation-worker.mjs`
8. `app-server/src/executable-generation-role-environment.mjs`
9. `app-server/src/runtime-manifest.mjs`
10. `app-server/src/codex-app-server-adapter.mjs`
11. `app-server/src/retained-role-live-host.mjs`
12. `app-server/src/local-semantic-live-host.mjs`
13. `app-server/src/services/slice-campaign/capability-host-runtime.mjs`
14. `app-server/src/services/slice-campaign/legacy-control-adapter.mjs`
15. `app-server/src/services/operational-coordination/chatboard-adapter.mjs`
16. `app-server/src/executable-generation-realization.mjs` (new, optional pure
    normalization/compatibility helper owned by generation bootstrap)
17. `app-server/README.md`

Adapter changes are limited to code-source versus operational-target wiring,
not claim, receipt, review, publication, or campaign semantics. The Python backend
already resolves its helper code relative to its own file; exercise it from the
admitted source with the correct target rather than changing its protocol.
Do not change role-binding-registry, context-input-custody, SQLite custody schemas,
runtime manifest YAML, canonical skills, or credentials to satisfy this slice.

## Exact allowed tests

Under `app-server/tests/`:

- `executable-source-maintenance.test.mjs` (new vertical)
- `executable-generation-realization.test.mjs` (new pure-contract tests)
- `executable-generation-manager.test.mjs`
- `executable-generation-worker.test.mjs`
- `runtime-manifest.test.mjs`
- `codex-app-server-adapter.test.mjs`
- `retained-role-live-host.test.mjs`
- `supervisor-campaign-capabilities.test.mjs`
- `supervisor-operational-coordination-integration.test.mjs`
- `supervisor-native-review-hosting-integration.test.mjs`

Keep new fixtures self-contained in those tests, under isolated durable scratch
roots when retained evidence is needed. Never run tests against live v20 state.

## Acceptance proof and review

The vertical starts from a synthetic predecessor with an existing binding,
delivery, attachment, closed queue, and known failure/recovery state. Stage code
from another root with the preserved realization. Assert actual module/helper
origins, actual repository/artifact targets, both fingerprint equalities, no
registry/custody mutation, and zero domain requests or recovery activity during
preparation and maintenance activation. Separately authorize a synthetic normal
continuation and demonstrate original-thread reuse without legacy-input replay.

Include a queue with otherwise valid recovery authority to expose construction-
time effects; an unreleaseable incident-488 fixture alone would mask that bug.
Also cover writable-store creation/migration attempts, timer/subscription activity,
stale CAS and mappings, source mutation, changed identity/requirements/capabilities,
symlink escape, true target changes, and mismatched provider options.

Inject failures before and after selection, before and after worker acknowledgment,
and on restart. Demonstrate durable maintenance-mode preservation, no mixed
identity admissions, idempotent retries, and truthful unavailable outcomes.
Retain normal compatible reload and authorized recovery regressions.

Run the new vertical and affected focused suites, `npm run test:app-server`,
and `git diff --check` on the immutable candidate. Classify baseline/unrelated
failures instead of silently excluding or counting them as passes. Live-provider
integration is not authorized by these test commands. Use fake transports and
bounded fixtures; any additional live exercise requires separate approval.

After deterministic gates, obtain independent implementation review of the exact
candidate covering source/identity, authority/custody, activation ordering and
crash consistency. Existing subscription Claude authorization does not grant paid
fallback. Preserve findings and remediation lineage. Implementation completion
requires accepted dispositions and truthful test/review limitations, not merely
a running proxy or a model's assertion that the design is sound.

## Work and authority boundaries

Implement in an isolated worktree from the named baseline. Preserve the original
worktree/index and all dirty PPCE artifacts. Capture immutable candidate evidence
and report exact changes; do not commit to main or activate the live runtime under
implementation-plan acceptance alone.

Stop for a scoped plan decision if existing fingerprints cannot remain equal,
requirements verification needs a new identity contract, stable helper closure
cannot be attributed within these owners, or custody semantics must change.
Broader runtime-port architecture, automatic credential refresh, PPCE repair,
and incident recovery are excluded.

After reviewed publication and separate activation authorization, live admission
must bind then-current state and the new source revision. First verify running
identity and unchanged custody in maintenance mode. PPCE reconciliation and
incident 488 recovery remain later, separately authorized work.
