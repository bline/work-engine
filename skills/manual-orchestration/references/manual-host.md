# Bounded outer manual host

`work-engine-manual-host` performs deterministic dispatch, observation and
collection for one explicitly bound outer-workflow objective. The root still
owns scope, model/profile decisions, capacity judgment, acceptance, integration
and user communication. It does not replace product lifecycle/task/review hosts.

## Provision and version

Run `scripts/provision_manual_host.sh /absolute/repository`. It builds through
custody with an isolated target and emits a content-addressed binary plus an
adapter bundle manifest and build custody path. The bundle preserves the scripts'
import layout, including `codex_app` and `durable_state`; it does not import those
modules from a mutable working tree. Every operation verifies its manifest and
script/binary digests. The context reader remains separately pinned by
`provision_context_status.sh`.

This prepares a private candidate. Global skill activation and deployment retain
their own authority. Keep the prior binary/bundle and unresolved state during a
cutover. `schema` emits generated versioned public request/closure schemas; the pin includes
`schemas.json` and the provisioning wrapper source. State schema `work-engine.manual-host.v1` binds the exact executor digest;
another binary refuses mutation rather than guessing a migration. `inspect` can
read the raw state across versions. Rollback runs the original pinned binary
against its original state. This pilot supplies no cross-binary state migration.

## Request contract

For ordinary use, supply a draft JSON object to `dispatch --draft FILE --wait`.
Use the full shape below, but give `authority` and `packet` as absolute path
strings, and replace `tools` with `{ "bundle": "/absolute/pin", "python":
"/usr/bin/python3", "context_binary": "/absolute/context-pin/work-engine-context-status" }`.
The host binds these references and all tool/dependency digests itself; the root
does not write hashing code. `dispatch --draft FILE --bind-only` emits the exact
bound request for review without launching work or creating host state.
Root-declared `artifacts`/`custody` may likewise use paths for automatic binding.
Authorization meaning still belongs to the root; automatic hashing does not
approve a packet or confer launch authority.

A bound request is a JSON object; unknown fields fail. Paths are absolute. For example
(the angle-bracket values must be replaced with exact observations):

```json
{
  "schema": "work-engine.manual-host.v1",
  "operation_id": "task-1",
  "revision": 1,
  "state_dir": "/absolute/objective/manual-host",
  "repository": "/absolute/repository",
  "cwd": "/absolute/task-worktree",
  "authority": {"path": "/absolute/approved-packet.md", "sha256": "<digest>"},
  "packet": {"path": "/absolute/task.md", "sha256": "<digest>"},
  "assignment": "/absolute/objective/assignment.json",
  "objective_id": "<existing-scope_budget-objective_id>",
  "root": {"session_id": "<verified-root-SID>", "rollout": "/absolute/root.jsonl"},
  "tools": {
    "python": "/usr/bin/python3",
    "bundle_manifest": {"path": "/absolute/pin/manifest.json", "sha256": "<digest>"},
    "role_host": {"path": "/absolute/pin/skills/manual-orchestration/scripts/role_host.py", "sha256": "<digest>"},
    "scope_budget": {"path": "/absolute/pin/skills/manual-orchestration/scripts/scope_budget.py", "sha256": "<digest>"},
    "usage_ledger": {"path": "/absolute/pin/metrics/scripts/usage_ledger.py", "sha256": "<digest>"},
    "child_context": {"path": "/absolute/pin/skills/manual-orchestration/scripts/child_context.py", "sha256": "<digest>"},
    "context_binary": {"path": "/absolute/context-pin/work-engine-context-status", "sha256": "<digest>"},
    "chatboard": {"path": "/absolute/pin/skills/durable-state/scripts/codex_chatboard.py", "sha256": "<digest>"},
    "task_profile": null
  },
  "model": "gpt-6.1-sol",
  "effort": "medium",
  "service_tier": "default",
  "sandbox": "read-only",
  "require_read_only": false,
  "bootstrap_context": true,
  "retained": null,
  "result_from_turn": false,
  "result_path": "/absolute/task-output/result.json",
  "artifacts": [],
  "custody": [],
  "claim": null,
  "profile": null,
  "poll_seconds": 5,
  "wait_seconds": 300
}
```

Authority and packet references are root-supplied authorization evidence, not
machine interpretation of human prose. The existing budget record must be open,
match the objective, and include the verified root. Admission requires current
root context, complete cost coverage, an actual configured dollar control with
headroom, and the existing scope-budget policy check. Bindings and close mutate
only through scope_budget's serialized owner. API-equivalent cost, account meter
freshness and unavailable coverage retain their existing meanings.

`bootstrap_context: true` is the root's explicit fresh-thread allowance. The host
records initial unavailability and tolerates at most 30 seconds awaiting the first
actual child context observation during foreground collection. It never reports
that period as healthy enrollment. A retained thread cannot use the allowance.
A pressure reading remains evidence for root judgment; the host cannot replace
context. The authorized dispatch expresses the root's scoped capacity judgment. The
reader's heuristic recommendation never grants or vetoes admission, enrollment
or continued collection; usable bound observations are required, while pressure
and observation loss remain visible to the root.

`require_read_only: true` requires actual returned `thread/start` sandbox evidence
of type `readOnly` before sending work. Missing evidence leaves the requirement
unestablished and starts no turn. `false` does not assert read-only attestation:
the sandbox request is recorded, but a downstream reviewer isolation requirement
remains unmet until its owner has admissible evidence. Stronger independence
claims are never inferred from this field or a session being separately hosted.
The adapter preserves the complete returned realization for inspection. This
field establishes only the reported native filesystem sandbox policy. It cannot
attest connector/tool mutation grants, prior exposure or the wider independence
contract. Those required reviewer properties remain unestablished here and must
not be accepted merely because this narrower check passed.

An optional host-owned claim uses exactly
`manual-host:<objective_id>:<operation_id>` and carries `claim_id` and positive
`ttl_seconds`. Other root, review and integration claims remain outside this
owner. A profile is an existing root-selected `{run_dir, episode, role}` with a
pinned `task_profile` script; the host records its assign operation without
choosing eligibility, an experimental arm or a model. It never inserts profiling
sidecars into the child's prompt.

For retained work, supply `retained: {session_id, rollout}` from an already
collected operation in the same state. Use a new operation ID/result path and
`bootstrap_context: false`. Another outstanding assignment for that SID refuses
admission. Requested model/effort/tier must match its observed settings before
start. Retained read-only realization currently remains unestablished and refuses
when that pre-execution claim is required; ordinary retained delivery makes no
stronger grant claim.

## Operate and collect

```sh
work-engine-manual-host dispatch --request /absolute/request.json --wait
work-engine-manual-host inspect --state /absolute/objective/manual-host
work-engine-manual-host collect --state /absolute/objective/manual-host \
  --operation task-1 --wait --ack-receipt '<previous-receipt-id>'
work-engine-manual-host close --state /absolute/objective/manual-host \
  --request /absolute/close.json
```

Use the foreground waiting form through an active root tool invocation. A yielded
exec session can keep running while the root handles independent semantic work;
its tool-session collector remains with the root. Local process polling happens
inside the host. Meaningful budget/context changes, unavailability, exact turn
termination or the bounded wait deadline return a compact receipt. It does not
wake an ended root. The next meaningful command explicitly acknowledges the
receipt ID. Stdout alone leaves delivery pending; lost output can be collected
again. A returned receipt ends that foreground observation scope; its monitoring
gap is visible, and the next admission re-establishes current observations.

Accounting keeps the budget reader's canonical lineage sources, including
externally launched root-owned roles, and adds hosted children through existing
scope_budget bindings. The host's private dispatch ledger records custody and
never substitutes for the accounting owner's ledger.

The executor observes the existing context and budget readers directly with
owned one-shot subprocesses and the budget owner's cursor. No board/control
subscription is claimed. This is the smaller composition for a foreground scope;
board claims still use the existing board owner. The root does not perform the
reader polling, and no unused board cursor is manufactured. Each process records
argv, PID/start identity, exact stdout/stderr digests, exit, timeout and leftovers.
Normal cleanup kills and collects its own process group. Executor death kills
its immediate adapter through Linux parent-death signaling. On restart, unknown
launch or descendant custody remains visible and cannot be silently replaced.

Before an external effect the host fsyncs its attempt. A lost create response is
`create_unknown`; a lost start response is `start_unknown`. Repeated dispatch
cannot create a replacement. Exact known turns are read with complete needed
pagination; latest-thread idleness never establishes a selected turn's outcome.
Ambiguous unknown effects need the owning root's external reconciliation; this
pilot does not pretend to recover them automatically or offer a force-clear.

By default the task packet tells the child to write `result_path` as JSON.
For a physically read-only child, use `result_from_turn: true` and request one
final JSON answer instead. The host reserves a normalized output path for each operation, including
operations that have not yet produced a file. No second operation can reserve
that path or an alias. Native collection always validates the exact turn and
item even when an output file already exists. Its durable capture intent binds
operation/input, session, turn, item and content. A retained staging inode and
atomic hard-link publication distinguish the host capture from a foreign file;
restart revalidates that binding. The staging file remains part of custody.

The host collects that exact turn's full native items,
requires one explicit `final_answer` agent message, and writes the bound JSON
under its own custody. It never labels this as a child file write. Missing,
ambiguous or legacy phase-unknown output remains unestablished. It may contain
`artifacts` and `custody` arrays of `{path, sha256}`. Child-declared paths must
resolve inside the result directory; root-declared artifacts may reference
already-authorized external evidence. Custody references use the existing
`manual-orchestration.custody.run.v1` schema and must establish known exit plus
no remaining processes. The host freezes the bytes it read, checks for mutation,
and preserves result/turn/settings distinctions. A turn can fail while its
report is mechanically collected. Missing results or mismatched settings do not
become semantic success. Changed collected bytes invalidate later consumption;
the old receipt remains historical evidence.

## Explicit closure

A closure request contains `schema`, `operation_id`, an exact `authority` artifact,
`ack_receipt`, `objective_end`, `disposition` (`accepted` or `abandoned`) and `reason`.
It must acknowledge the latest operation receipt. Only an explicit objective-end
request closes scope_budget, after every other assignment is collected. A single
child ending never closes the objective. Task closure and objective closure have separate idempotent transitions. A
closed task can anchor a later explicit objective-end request with its own
authority and reason, without replaying native/claim effects. Repeated matching
closure has one effect; conflicting inputs within either transition fail. Objective accounting is observed again after close,
so attributable tails and unavailable state remain inspectable.

Accepted closure needs collected assignment evidence. Explicit abandonment can
close a known pre-execution refusal or a terminal assignment lacking a result,
retaining the missing evidence and without manufacturing acceptance. Unknown
external effects remain unresolved and retain custody. Only the host's own claim
is released when its work is quiescent. An objective-end request also carries the root's responsibility for any work
outside this host registry; the host verifies its own operations, not a global
census of every agent or process. No close operation kills a running native
turn, expands a grant or accepts an implementation.

## Turn configuration and accounting refresh

Admission of retained work can inspect the latest session configuration, but
that observation never establishes the new turn's execution settings. Collection
uses `role_host settings --turn-id` to bind matching applied settings to the
selected turn's attributed rollout interval and turn_context. Later turns cannot
replace that evidence; missing, stale, inconsistent or unattributed observations
leave the configuration-dependent claim unestablished.

`collect --state DIR --operation ID --reconcile-accounting` explicitly observes
current budget/account evidence after collection or closure, without replaying
task, claim or close effects. It emits a separate immutable accounting receipt
referencing the historical task receipt; historical task evidence is not
revalidated by this read. The budget owner's `snapshot.post_close` view preserves
closed-window totals and separately reports later candidate calls and settlement
uncertainty. Candidate tails are never silently added to settled objective totals,
and current account readings are independent of the closed cost window. An
accounting receipt can be acknowledged on the next meaningful command.
