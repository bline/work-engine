# Settled-fact contradiction classification packet — Work Engine Rust lifecycle S4–S8

## Your task

You are an independent classifier. Another model proposed the hypothesis below and classified one slice itself; you have **not** been shown its labels. Classify from the evidence in this packet only.

**Settled-fact contradiction:** an attempt, contract/route revision, or review finding whose failed premise or defect contradicts a fact that
(a) was authoritative before that attempt began,
(b) applies to the attempt's subject or boundary, and
(c) remained valid throughout the attempt.

"Authoritative" means established by an earlier accepted/published slice, an accepted plan or contract the builder was working under, or the committed code/schema the attempt built on. A fact discovered only by running or testing during the attempt is not settled.

Classify each item into exactly one bucket:
- **clear** — the defect or failed premise directly contradicts such a settled fact; name the fact and where it was established.
- **arguable** — likely preventable by better contract formation or by giving the builder the relevant settled facts, but no single settled fact was directly contradicted.
- **novel** — genuine discovery: environment/provider behavior, a defect only found by running, new coverage, or a real design gap nobody had settled.
- **undetermined** — the packet does not contain enough to decide. Prefer this over guessing.

Also mark **dependent-on: <item>** when an item exists mainly because an earlier item's fix caused it (e.g. a revision correcting a previous revision).

### Items to classify
- Every review finding (IDs like `S4C1-F1`, `S5C-F3`, `S6-C1-F1`, `S7-C1-F2`, `AIR-...`).
- Every candidate attempt after the first (why was candidate N+1 needed?).
- Every S6/S7 shared-contract-freeze revision and every S8 admission-contract revision.

### Output format
One table per slice: `item | bucket | settled fact contradicted (and source) or discovery | dependent-on | confidence (high/med/low) | one-line rationale`.
Then a summary: counts per bucket per slice, and your overall view of how much churn was avoidable settled-fact rework versus productive discovery. Note any item where you think the hypothesis itself is poorly defined.

Severity labels in reviews: P1 = blocking/high, P2 = medium. "Candidate" = an immutable implementation attempt submitted for gate and review.


## Background: published slices before and during this work

The Rust context-lifecycle replacement was built in slices S0–S10. Each line is a published commit (date, subject, body).

```text
2026-10-07 84be669 Establish Rust context lifecycle foundation contracts
Add an explicit seven-package Rust workspace with versioned identity and codec types, strict public command DTOs, private store entry authority, runtime and provider ports, and a read-only client interface. Bind controlled text bytes and their verified digest across the plan, entry permit, and provider request; qualify the byte contract and authority boundaries with golden vectors and compile-fail tests.

The S0 gate passed 12/12 checks. Retained reviews verified all four contract findings resolved and found no instruction issues. Durable store admission, process and provider execution, public schema delivery, and integrated service behavior remain later work.
```

```text
2026-10-07 e7c1d88 Implement transactional Rust context lifecycle core and store
Implement pure command, queue, entry and observation reductions with a normalized SQLite store. Bind trusted authority and canonical commands; atomically persist results, custody and effect plans; fence competing writers and uncertain attempts; retain exact observation conflicts; and publish artifacts with coherent WAL recovery.

Validation: 43 passing core/store cases, 14/14 ordered gate checks, and retained contract review resolving all nine findings. This is the S1 module boundary; live service/provider integration remains later work.
```

```text
2026-10-07 b2efdbf Add owned asynchronous Rust lifecycle execution
Add an S2 runtime that reserves bounded task/result capacity before a store claim, eagerly owns each nonclone AuthorizedEntry, and reports exact correlated task and child facts even when an observer ticket is dropped. Add qualified provider ingress evidence, monotonic wait budgets, cancellation-safe bounded framing, subprocess supervision, and shutdown accounting. Register pinned Tokio/runtime dependencies and controlled-port/subprocess regressions. The final affected gate passed 8 checks with 164 workspace tests; one retained contract reviewer verified six findings resolved. S4 store/service composition and live provider settlement remain deferred.
```

```text
2026-10-07 634161c Add versioned lifecycle wire contract and passive Rust read client
Add strict protocol-v1 command parsing, stable wire errors, generated JSON Schema 2020-12 and compatibility fixtures. Add a read-only client that carries exact targets and remaining budgets through snapshots, wait hints, cursor-gap resync and reconnect, while preserving failed outcome, effect settlement, observation availability and current admission as separate facts. Register pinned schema dependencies. The 13-check S3 gate passed with 136 workspace tests; the retained contract review verified its four initial findings resolved.
```

```text
2026-10-07 41992a2 Add integrated Rust lifecycle service and read-only observation endpoint
Compose the published S1 store, S2 owned runtime, and S3 wire/client through a permissioned Unix socket service. Bind controlled text results and replacement transitions durably, preserve v1 behavior, and expose coherent historical snapshots, passive waits, reconnect and cursor-gap resync. Cover real process disconnect, crash, shutdown, security, schema, and packaged-profile paths. Native provider integration and live cutover remain deferred. Publication union onto df9ebb6e preserves the two separately published Cargo members and merges only the workspace manifest and lockfile; the accepted S4 domain source is byte-identical.
```

```text
2026-10-08 b4c6df9 Implement pinned native Codex lifecycle and bounded snapshot read
Add the opt-in pinned native Codex adapter, trusted finite workspace.snapshot.read/v1 operation, and generation-scoped session ownership. Persist native ingress, parent and child claims, result-before-reply custody, settlement, and query-only recovery in the S1 SQLite store. Retain trusted subject-clock observations across failed bound-grant entry attempts while keeping failed entries uncommitted.

Validate the exact accepted 25-path tree with isolated Rust checks and an installed, network-isolated native simulator vertical. The pinned simulator establishes the exercised route; live-provider behavior and S6/S7 runtime remain outside this slice.
```

```text
2026-10-08 d6ced89 Add Rust S6/S7 semantic preservation and lifecycle custody
Implement the accepted Rust S6/S7 semantic successor gate and offline lifecycle recovery, signed no-retry disposition, coherent export/adoption, fencing and current-state rollback. Include the three existing S67 preparation fixtures required by the new tests; retain their preexisting attribution. Preserve the already published E0/J1/E1 dependencies and other documentation.

Validation: exact accepted C4 Rust subtree; 12/12 affected gate and three retained reviews with nine findings resolved; isolated locked/offline publication projection metadata, S5 compatibility (2 tests) and adoption recovery (4 tests).
```


## Plan sections S4–S7 (the accepted plan the builders worked under)

### S4 — First integrated service and observation boundary

Depends on S1, S2 and S3. The named integration owner owns
`rust/crates/lifecycle-service/`, joined tests in `rust/tests/`, and common
`rust/crates/lifecycle-test-support/`. Lane owners fix their modules as needed
against the same interface revision; changes to shared contracts are explicit.

Deliver the permissioned local endpoint, trusted principal/grant resolver and
real core/store/executor composition. The controlled scenario enters through
public commands, disconnects its initiator, progresses the supported text
transition autonomously and observes its committed result using the shared client.
Provided continuation/controlled semantic responses establish only the bounded
profile, not the later semantic-quality claim. Unsupported production routes stay
disabled. Tests never edit lifecycle tables or invoke a hidden advance method.

Exercise actual-service crashes at all entry/result boundaries, stale and
conflicting commands, early success/failure, saturation, observation loss, clock
boundaries, process replacement and unsafe-close history. Derive proof facts from
this run's durable state and independent peer/child observations.

Exit: M1 and the read-only observation part of M2, with T01/T03–T07/T09/T11/
T18/T19/T21–T24/T27/T28/T32/T33/T36/T37 at the real composition. Freeze the tested
build, profiles, schema/client revision and evidence. U1 gives the UI owner a
controlled running service and reconnection contract. It is neither a production
proxy connection nor authority for UI commands. The UI owner separately accepts
and implements its integration; this plan does not claim or edit that workspace.

### S5 — Native provider and bounded owned operations

Depends on S4 and selection of the first operation profile and authorization
semantics. Own `rust/crates/lifecycle-codex/` and bounded provider-port/contract
extensions. Shared core/store changes go through the integration owner.

Preserve the pinned native protocol contract, raw ingress identity, history
association and operation-specific settlement. Test installed native execution
against isolated simulated inference where available, without credentials or live
network/provider entry. Preserve the native versus simulated evidence distinction.
No private/raw adapter path may bypass entry accounting.

Select one useful narrow tool profile, including its domain and execution owner,
entry-bound or commit-bound authority guarantee, outcome and settlement evidence.
Unenforceable revocation/descendant-effect guarantees disable that route; they do
not become a synthetic universal seal. Respect same-native-session requirements.
Normalize observed token/cache metrics without inventing measurements or adopting
a forecasting policy.

Exit: remaining M2 provider/profile evidence; T03/T06–T09/T15–T17/T28/T30/T33–T35.
Live provider verification remains separately authorized. This slice does not
adopt the unresolved general Authority Commit Gate proposal.

### S6 — Semantic preservation

Depends on S5; may overlap S7. Own `rust/crates/lifecycle-semantic/`, its material
ports, corpus and tests. The semantic verifier's quality/independence policy is
selected before this slice's implementation grant.

Build bounded canonical material projection, source/grant revisions, separate
compiler/verifier effects, adequacy challenges and successor continuation checks.
Use the same runtime/entry/result path as domain work; inference cannot create
authority. Include human answers arriving during capture, superseded decisions,
unresolved approvals and stale canonical references. Controlled responses test
orchestration and rejection, not the quality of an unrun live model.

Exit: M3's controlled composition and T13/T14/T15/T23/T35 evidence, with explicit
remaining live semantic-quality acceptance. A supplied digest echo is not success.
Corpus preparation can begin earlier without claiming this dependent slice done.

### S7 — Recovery, disposition and offline administration

Depends on S5; may overlap S6. Own `rust/crates/lifecycle-admin/`, admin scenarios
and proposed migrations. The integration owner serializes shared store/reducer
changes; concurrency never permits independently defined custody semantics.

Implement observation/reconciliation of uncertain entered attempts, exact-claim
authority-bound no-retry disposition, compatible startup, coherent backup/export,
inactive destination adoption and fenced source handoff. Preserve pending and
completed delivery identities in candidate-current rollback state. Ordinary
context cycling uses the stable store and never invokes offline migration.

Qualify only the historical codecs actually reached by the selected import,
using pinned old bytes/hashes and cross-language fixtures. Preserve old references
and explicit lineage to any new identity; reject unknown or mismatched codec
contracts instead of silently rehashing them. Claim/workspace compatibility
examples in S0 are not authority to migrate those services.

Exit: administrative components pass T21/T22/T24–T26/T35 using disposable state.
Production old-state import remains unestablished until S9. No actual queue,
including historical Q3/Q4, is retired, transferred or delivered by this slice.



## Plan section S8

### S8 — One retained workflow through the complete host path

Depends on S6 (therefore S5/S4); can overlap independent S7 work. The integration
owner composes the lifecycle service with a permanent Rust host and
`rust/crates/lifecycle-proof/`. Before slice admission, the lifecycle and host
owners agree the actual retained-workflow consumer, supported subject/profile,
entry path and exact source/contract revision. A proof-only host cannot replace
that consumer. Domain hosts remain external owners; this join neither absorbs
native-review authority nor grants edits to another host's paths. No legacy
bridge is assumed merely to keep the unused system running.

Run active work through pressure/yield, semantic capture, verified checkpoint,
successor readiness, custody and two queued inputs. Continue one delivery through
a second context change without replay. Exercise supported tools and a differently
named subject kind against the same kernel. Normal context replacement keeps its
host process alive when the selected provider contract permits it.

The controlled and later live proof use one installed service and scenario driver,
changing only authorized external-provider behavior and fault controls. Readiness
and final receipts project durable applied facts; caller completion, files and
test callbacks never advance production state. Validate before proof teardown;
perform any required offline preparation only after truthful quiescence.

Exit: M4; T02/T10–T17/T19–T22/T27/T29–T35 at the retained-workflow boundary.
All eight legacy proof responsibilities have an exercised supported path or an
explicitly justified replacement. Tool and semantic claims remain scoped to
the tested profiles; no real production work or live inference is implied.



---

## S4 — integrated service and observation endpoint

Accepted at candidate 3.


### S4 contract reviewer reports (chronological)

### 2026-10-08T00:08
Blocking verdict for `b5aacce41e78850db6e220cda7562273a2282a38`.

- **S4C1-F1 — P1:** Replacement can bind completed result A while newer attempt B remains entered. After B settles, the service can reconcile using A’s checkpoint and successor, ignoring contradictory B continuation facts. Bind continuation evidence to the applicable predecessor boundary. Evidence: `lifecycle-store/src/sqlite.rs:442–449,1254–1262,1320–1331`; `lifecycle-service/src/engine.rs:51–129`.
- **S4C1-F2 — P1:** SIGTERM queues shutdown behind transition execution; it cannot immediately fence the locally owned verifier supervisor. Releasing a launch barrier after SIGTERM still permits launch/activation. Share the close gate and preserve child/result custody independently of external waits. Existing launch-cut tests use SIGKILL, missing this race. Evidence: `lifecycle-service/src/socket.rs:73–97`, `actor.rs:136–176`, `engine.rs:155–204`; `rust/tests/s4_vertical.rs:884`.
- **S4C1-F3 — P2:** Enqueue replay reserves capacity before consulting durable command history. With capacity occupied, an identical replay returns `observation_unavailable` instead of its committed result. Resolve authenticated replay before reserving new admission capacity. Evidence: `lifecycle-service/src/actor.rs:195–220`; `lifecycle-store/src/sqlite.rs:331–355`.
- **S4C1-F4 — P2:** Server-capped waits become premature client deadlines. A 10-second caller budget with a 1-second server cap can return `DeadlineExpired` after one second. Continue bounded passive waits through the caller’s remaining budget. Evidence: `lifecycle-service/src/socket.rs:222–264`; `lifecycle-client/src/socket.rs:130–148`, `src/lib.rs:141–182`.

Published and read back durable revision **`e0a05525e4af095df71f97dd284afe66368741c3`**, phase `remediation`, all four findings open.

This was immutable-source review without test execution. T18 clock and T23 artifact-owner evidence remains qualified component evidence; it does not establish the failing integration paths above. Graph generation `2026-10-07T23:51:10Z` lacked new service coverage; exact Git source supplied fallback evidence.

Host-bound reviewer: `gpt-6-astra/xhigh`, fresh entry without builder history; builder `gpt-6-sol/high`. Same-provider different-model review; `independence_claimed: false`.

### 2026-10-08T00:30
Candidate2 `049016a41d0c03708eb2e2d651cc0362f1d656fe` remains **blocked by S4C1-F2**.

- **F1 verified resolved:** transition evidence now follows the newest entered attempt.
- **F3 verified resolved:** authenticated durable replay precedes capacity reservation.
- **F4 verified resolved:** capped socket waits continue through the caller’s budget.
- **F2 partially repaired, unresolved:** shared gates now fence launch/activation, with SIGTERM barrier tests. However, an already-active verifier can wait up to 60 seconds while socket shutdown abandons the actor after 30 seconds. `main` then calls `process::exit(1)` before child cleanup and durable close reporting. Evidence: `socket.rs:45–48,102–105`; `engine.rs:269–277,320–331`; `actor.rs:139–196`; `main.rs:20–23`.

Coordinate shutdown with the active verifier’s bounded drain and preserve its terminal or unresolved custody before process exit.

Published and read back revision **`f13b2919a69faa29a77e578bc242f984514479de`**, phase `remediation`, same episode/generation. No new findings.

Limits: immutable-source continuation and supplied 10/10 gate evidence; no tests rerun. The added replay test does not actually saturate both slots, so F3 closure rests on verified source ordering. Prior T18/T23 qualifications remain unchanged. No fresh-entry or independence claim.

### 2026-10-08T00:40
Candidate3 review passes for commit `f5c864d6ece2e1a398c3e758d6f66006c459da37`.

Durable result **`2673c2a2a7585af539afadeb4162ad5d384be9b7`**, read back successfully: phase `reported`, same episode/generation 1.

- **F2 verified resolved:** `socket.rs:106–118` now accommodates configured verifier cleanup and controlled barriers. `s4_vertical.rs:1158–1304` exercises SIGTERM beyond 30 seconds and checks exact child reaping, durable failed/unsafe/late verifier evidence, and service close.
- **F1/F3/F4 remain verified resolved.**
- No new findings or unresolved questions.

Verified the exact tree, all 46 receipt blobs, three-path delta, and supplied 9/9 affected gate. Tests were not rerun. Existing T18/T23 qualifications and F3 saturation-test limitation remain; external verifier waits still serialize the actor.

Retained same-provider different-model review: parent-observed reviewer `gpt-6-astra/xhigh`, builder `gpt-6-sol/high`; original fresh isolation without inherited builder context, current continuation, `independence_claimed:false`. No source edits, slice acceptance, or publication performed.


### S4 agent-instruction reviewer reports

### 2026-10-08T00:05
Applicability: **applicable**. Candidate `b5aacce41e78850db6e220cda7562273a2282a38` contains normative directions for agent maintainers/operators in the client and service READMEs.

**Findings: none. Advisory outcome: retain.**

Decisive evidence, bound to that commit:

- `lifecycle-client/README.md:5` explains the historical v1 manifest identity before directing isolated schema generation. The generator’s writes and frozen-manifest test support that preservation boundary.
- `lifecycle-service/README.md:3–11` scopes socket permissions, command replay, result custody, and controlled-profile operation. These match the accepted S4 contract; the text explicitly excludes native-provider, production provisioning, and deployment authority.
- Added API comments preserve exact-result ownership and unresolved-entry/no-resend distinctions. No material instruction ownership, precedence, or causal-exposure defect was identified.

Review state was published and read back at durable revision **`0af5801e438aafec7f11af27eb938c0b7428994b`**, phase `reported`, generation 1, with empty findings and unresolved questions.

Limitations: bounded instruction review, not generic implementation acceptance or test validation. The graph lacked new service paths; immutable Git source supplied the relevant evidence. No repository edits, tests, or live operations were performed.

Host metadata corroborates reviewer `gpt-6-astra/xhigh` versus builder `gpt-6-sol/high`: fresh entry without inherited builder history, **accepted same-provider different-model review**, `independence_claimed: false`.

### 2026-10-08T00:30
**Applicable; no findings. Advisory outcome: retain.**

Subject: Candidate 2 `049016a41d0c03708eb2e2d651cc0362f1d656fe`, tree `b5fe5eedd1d7883a79341a033b84c5b8aad30e5a`.

Both READMEs retain their exact Candidate 1 bytes. Their maintainer/operator directions remain materially normative. The seven-path repair delta preserves their scoped replay, shutdown, result-custody, and passive-read consequences without introducing broader authority or loading requirements.

Published and read back in the same episode/generation at **`79551bf2105b87ed942c5a0c2d99a19853a12280`**, phase `reported`; findings and unresolved questions are empty.

Limitations: bounded instruction review, not generic implementation acceptance or closure of another reviewer’s findings. Immutable Git source supplied evidence where graph coverage was missing. No tests, source edits, or live operations. Same-session continuation; no new freshness or independence claim.

### 2026-10-08T00:38
**Applicable; no findings. Advisory outcome: retain.**

Subject: Candidate 3 `f5c864d6ece2e1a398c3e758d6f66006c459da37`, tree `fe685e4ff74b8d09d29f4708d29922d85b2a96dc`.

Both normative READMEs remain byte-identical. The shutdown distinction remains supported:

- `README.md:3` corresponds to the unchanged five-second executor drain in `actor.rs:176–197`.
- `README.md:9` separately exposes configurable verifier cleanup deadlines.
- `socket.rs:102–121` now preserves time for verifier cleanup and controlled barriers before process exit.

The five-second clause does **not** establish a total service-exit deadline. No material instruction authority, loading, ownership, or causal-exposure defect was identified.

Recorded and read back at **`87f4fcd99f51840fe2b92868fbbf3ebe0dfefdb0`**, phase `reported`, with empty findings and unresolved questions.

Bounded instruction review only; no generic acceptance or other reviewer’s finding closure. Immutable source used where graph coverage was missing. Same-session continuation; no tests, source edits, live operations, or new independence claim.


### S4 placement falsifier report

### 2026-10-07T22:33
At checkout `b2efdbfa`, the existing v1 public enqueue path cannot satisfy the accepted core admission contract unchanged. The strict [wire payload](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-wire/src/lib.rs:49) has only `input_id`, and [wire parsing](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-wire/src/lib.rs:219) verifies a digest over that shape. [Core admission](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-core/src/reducer.rs:145) requires four payload fields—`input_id`, `producer_ref`, `text`, and `text_digest`—and exact agreement with `ControlledTextInput`. The [store](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-store/src/sqlite.rs:369) durably inserts those values from the checked admission. Trusted staging keyed by `input_id` cannot bridge both digest checks. Changing v1 in place would change its checked-in strict schema and compatibility contract. A new versioned enqueue shape and corresponding trusted command codec/admission path are the sound placement; retaining the v1 parser for historical compatibility is possible.

The planned service owner is appropriate for socket authorization, principal/grant resolution, autonomous coordination, and redacted projection. The S4 plan explicitly assigns [lifecycle-service](~/code/work-engine/experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md:236) that composition; the crate is absent at this checkout. It should not assemble snapshots from multiple calls: [recovery](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-store/src/recovery.rs:9) exposes counts, while [effect_projection](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-store/src/sqlite.rs:849) exposes one effect. Neither yields the subject, transition, delivery, and cursor from one consistent read required by the [design](~/code/work-engine/experiments/context-lifecycle-rust/DESIGN.md:826). A subject-scoped transactional read belongs in lifecycle-store; service maps it to the existing wire snapshot and the [passive client transport](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-client/src/lib.rs:65).

`final_text` needs a narrower decision. The [provider port](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-provider/src/lib.rs:27) returns it, but [runtime](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-runtime/src/executor.rs:496) discards it when constructing `TaskResult`; [core observation](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-core/src/contracts.rs:239), [store observations](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-store/migrations/0001_initial.sql:115), and [wire delivery](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/worktree/rust/crates/lifecycle-wire/src/observation.rs:64) have no text result. If S4’s “committed result” means the documented delivery outcome and settlement, no text-result schema is necessary. If exact response text must survive a crash or drive subsequent transition work, runtime must carry it to the store, and the store must bind its bytes or digest to the accepted source/attempt in the same durable result transaction. That can remain a typed result beside `EffectObservation`; placing text into the accounting observation or public snapshot is unnecessary unless an authorized consumer actually needs it. A service side map or separate later artifact publication would leave an unowned crash gap.

This is source and contract analysis, not an implementation finding. Graph generation `2026-10-07T22:18:38Z` reported matching metadata and no recorded gaps for cited Rust files except partial SQL parsing; I read the SQL directly. No builds or tests were run.


### S4 builder reports

### 2026-10-07T22:35
The read-only S4 plan is ready: [s4-plan-v1.md](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-plan-v1.md) (`sha256: 71c80e77e4daa9d70ade8bb97892d2410bf4ed33c970fab2a9252459e89e6dad`). Its [receipt](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-plan-receipt-v1.json) records the evidence and pending decisions.

Fresh placement checks confirmed that published v1 enqueue cannot satisfy S1’s text-bound admission contract. The plan preserves v1 and proposes an explicit versioned command seam, store-owned coherent projection and durable results, and a service-owned local endpoint and autonomous controlled transition. It maps the S4 scenarios to actual-process proof and ordered gates.

The isolated checkout remains clean. No implementation, build, test, provider call, or review was run. Final amended roadmap hashes and supervisor plan acceptance remain pending.

### 2026-10-07T22:46
The revised read-only S4 plan is ready: [s4-plan-v2.md](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-plan-v2.md), SHA-256 `d76f33be98b6624a10c4f749f296d1ec925f4dce3a2d07d5c4f87246aa6f7e8f`. The [receipt](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-plan-receipt-v2.json) contains the exact 49-path manifest and amended document hashes.

The plan preserves published v1 bytes, defines explicit v1 enqueue refusal and v2 text admission, and requires actual-process controlled proofs for T07, T18, T21 and T22. Its earliest binary vertical must confirm that continuation consumes the exact peer response before the result-text seam proceeds. The pinned Rust 1.92 toolchain stays unchanged; changed lock dependencies require license and RustSec advisory closure.

The detached checkout is clean. No source changes, builds, tests, provider calls or reviews were performed. Implementation awaits supervisor plan acceptance.

### 2026-10-07T23:36
The S4 implementation candidate is ready for the supervisor’s final-gate decision. It changes 46 of the accepted 49 paths and none outside the boundary. The [file manifest](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-implementation-file-manifest.json) and [readiness receipt](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-implementation-readiness.json) are bound to active-slice revision `acbd19aa38702a5290c00b845472ee801165133b`.

Targeted actual-process service tests passed: 6 vertical, 1 four-cut crash, 6 security, and 2 contract tests. T23 combines unchanged S1 artifact-owner tests with an actual service-produced coherent SQLite image and immutable-base v1 schema refusal; it makes no service blob-publication claim.

The ordered final gate, dependency license/advisory closure, immutable checkpoint, and adversarial review remain pending supervisor release.

### 2026-10-07T23:52
S4 is deterministically ready. The [gate receipt](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-deterministic-gate-receipt.json) records 15/15 passing continuation checks; the [final manifest](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-gate-candidate-manifest-v2.json) binds 46 changed paths, none outside the accepted 49. Active-slice revision is `6a06d375b6852d00bcb8ec46cca23eb11b6889e0`.

The full workspace passed before a type-alias-only Clippy repair. Final-candidate S1 and S4 tests, Clippy, release profile launches, schema checks, license closure, and a fresh RustSec audit passed. Remaining work is the supervisor-owned immutable checkpoint and review. The [review projection](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-review-ready-projection.json) marks agent-instruction applicability **uncertain** because the changed READMEs contain operational directions that agents may read.

All work after the no-`/tmp` instruction used the verified S4 subtree under `~/code/.work-engine-tmp`.

### 2026-10-07T23:56
Private S4 Candidate 1 is frozen and adapter-validated. The [full checkpoint receipt](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-candidate1-checkpoint-receipt.json) has SHA-256 `2199918b0ed05631245d7cbd08371d44098b5c101c555c7ff4c3b94cee9ce39a`. Its private commit is `b5aacce41e78850db6e220cda7562273a2282a38`; the seven-field subject and artifact bindings are in the [review projection](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-candidate1-review-projection.json) (SHA-256 `bd5a7edfd57e6517a161c32560ac861f1142ea37d2936f54be345e09abbf5fa2`).

All 46 paths are attributed and content-bound. The detached HEAD, real index, worktree status, and source bytes are unchanged. No review, acceptance, or publication was started.

### 2026-10-08T00:46
S4 is privately accepted at checkpoint `6e3d1c1b9a92830ba8c2ef58ff393b0899c56611`. All four contract findings are verified resolved; the retained instruction review found none. The final affected gate passed 9/9 checks. No ordinary commit or live cutover occurred.

The terminal artifacts are ready:

- [Schema-v5 audit](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-semantic-audit-receipt-v5.json) — SHA-256 `0e12664a3f6c397f1eee2e65a1b7882a57834826adcf295211cb67c8785c2c63`
- [Compact handoff](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-handoff-receipt.json) — SHA-256 `175c48fa5908ea92afa4f919e19b41322fb268252d1eeb31deb6eead5cd23016`
- [Schema-v2 commit proposal](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-completing-builder-commit-proposal-v2.json) — SHA-256 `1d16386df0556363878fee229f1ed0d0d2133caa34f6e500d9a99e792efe0eb4`
- [U1 service handoff](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-u1-service-handoff.json) — SHA-256 `af767e06d5b212e4eb80b2547240dba1741dc4b55879346fc8943a9a28e5692a`

The audit and proposal pass their schema validators. The parent retains telemetry assembly, terminal finalization, and the completion offer.

### 2026-10-08T01:01
The S4 publication union is ready in an isolated clone. Against shared parent `df9ebb6e`, its immutable candidate is `e3f1554f` (tree `dfedfdbf`), with exactly the accepted 46-path delta. The 44 paths outside Cargo root and lock match accepted S4 bytes; the Cargo files retain both newly published workspace members.

The combined workspace passed serial all-features tests, formatting, Clippy, both packaged release profiles, and the pinned RustSec check. A parallel all-features run exposed a process-global fault-variable race in two published HP1 tests; the focused and full serial runs passed. Details and exact hashes are in the [publication handoff](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-publication-union-handoff.json), with the [validation receipt](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-publication-union-validation-v1.json) and [completion request draft](~/.local/state/work-engine/rust-context-lifecycle-s4-20261007/s4-publication-union-completion-request-draft.json).

The parent can import the candidate object from the isolated clone and use the existing serialized publication route. I made no shared branch or index change and no ordinary commit.


---

## S5 — native provider and bounded owned operation

Accepted at candidate 5 (a second builder at higher reasoning effort also ran).


### S5 record `review-finding-disposition-c1.json` (excerpt)
```json
{
 "air_episode_revision": "4132ef0a9731b7ac290505f10d5cad4df6a7b216",
 "candidate_commit": "3c40bbfd7d38ad078c76c4db7403ea071c2f861f",
 "contract_episode_revision": "7f434c211a16957f6ea487a5ca93a388e751e01a",
 "disposition_owner": "retained-s5-builder",
 "findings": [
  {
   "anticipated_existing_scope": "store/native.rs, service/actor.rs",
   "consequence": "Current-at-entry transaction must revalidate subject owner/context, trusted principal/build/proof, grant/profile and rollback fence; preserve exact D2 replay.",
   "disposition": "valid",
   "id": "S5C-F1"
  },
  {
   "anticipated_existing_scope": "store/native.rs, store/recovery.rs, store/sqlite.rs, service/actor.rs, service/socket.rs, core state/reducer if needed",
   "consequence": "Native custody/recovery is separate from canonical lifecycle projection; implement native generation handoff and inclusion in recovery/reconciliation through same store/service owner. Prior T15 proof is limited to allocation/direct subject mutation.",
   "disposition": "valid_architecture_gap",
   "id": "S5C-F2"
  },
  {
   "anticipated_existing_scope": "store/native.rs, service/actor.rs",
   "consequence": "Track and reconcile started but uncompleted tool items; reject unsafe full-history item variants and result/status mismatches before settlement.",
   "disposition": "valid",
   "id": "S5C-F3"
  },
  {
   "anticipated_existing_scope": "service/actor.rs, service/operations/mod.rs, runtime/native_stdio.rs if needed",
   "consequence": "Retain child cleanup owner until observed reap or durable unsafe close, and preserve cleanup failure as data.",
   "disposition": "valid",
   "id": "S5C-F4"
  },
  {
   "anticipated_existing_scope": "service/operations/mod.rs",
   "consequence": "Persist every received raw terminal coordinate before fallible decode/qualification.",
   "disposition": "valid",
   "id": "S5C-F5"
  },
  {
   "anticipated_existing_scope": "service/operations/mod.rs, lifecycle-codex transport/protocol and installed proof harness as needed",
   "consequence": "Current native configuration/probe does not establish selected profile exclusion: real functions.exec/wait and async question inventory observed. Qualify actual route and enforce exclusion or retain Unsupported/unaccepted; no fixture substitution or acceptance waiver.",
   "disposition": "valid_unmet_installed_route",
   "id": "S5C-F6"
  },
  {
   "anticipated_existing_scope": "store/native.rs, service/actor.rs, service/socket.rs, core/contracts.rs if needed",
   "consequence": "Expose bounded versioned native history/material reader and allocation-only recovery; support distinct authorized purpose accounting without S6/S7 semantics.",
   "disposition": "valid",
   "id": "S5C-F7"
  }
 ],
 "next": "Perform bounded in-scope remediation after checking exact current source and route qualification; create new private candidate and return to supervisor for rebind before retained reviewer continuation.",
 "overall": "blocking; no acceptance waiver; accepted S5 objective remains",
 "path_scope": "All anticipated implementation paths listed below are within previously parent-authorized task-owned-manifest-v4; no new file path requested yet. Any actual expansion must return to parent before edit.",
 "recorded_at": "2026-10-08T04:01:04.321381+00:00",
 "route_revision": "C1 positive native snapshot read and network isolation remain evidence; C1 claim of accepted D3 generation switch and selected installed capability exclusion is stale/unestablished. Native custody must join canonical lifecycle/recovery owner; actual installed tool routes must be proven or profile stays Unsupported.",
 "schema_version": 1
}
```


### S5 record `f6-route-clarification-c1.json` (excerpt)
```json
{
 "conclusion": "For exact pinned config code_mode_host=false, valid functions.exec custom route is denied before execution; zero starts and absent marker. A denied native-internal attempt need not be fabricated as lifecycle ingress. Valid async question route emits unsupported raw agentMessage and remains entered_uncertain. Initial F6 unsupported-status conclusion is superseded only on these tested route/config facts, pending formal retained re-evaluation on new bound candidate.",
 "evidence": [
  "~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/proof_harness/actual-custom-exec-route-result.json",
  "~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/proof_harness/actual-async-valid-route-result.json"
 ],
 "limitations": [
  "No universal hidden-route absence claim",
  "Exact service-binary-to-candidate build binding supplied by prior gate, not newly established by reviewer",
  "F1-F5 and F7 unchanged",
  "C1 finding remains preserved and open"
 ],
 "recorded_at": "2026-10-08T04:10:17.304035+00:00",
 "retained_contract_episode_revision": "7f434c211a16957f6ea487a5ca93a388e751e01a",
 "schema_version": 1,
 "status": "reviewer_clarification_not_formal_finding_transition",
 "subject_commit": "3c40bbfd7d38ad078c76c4db7403ea071c2f861f"
}
```


### S5 contract reviewer reports

### 2026-10-08T03:59
**Verdict: blocking** for candidate `3c40bbfd7d38ad078c76c4db7403ea071c2f861f`.

Seven findings were recorded and read back at durable revision `7f434c211a16957f6ea487a5ca93a388e751e01a`. The episode remains active for retained remediation.

1. **S5C-F1 — High: admission does not enforce current authority.**  
   `lifecycle-store/src/native.rs:375,549,1049` checks cached session context and omits principal/build/proof and current lifecycle ownership. Retained handles can admit new work after fencing or generation change. Revalidate these bindings transactionally for every new parent/child entry, preserving exact D2 replay separately.

2. **S5C-F2 — High: native custody is disconnected from lifecycle transitions.**  
   `native.rs:393,949`, `recovery.rs:49,94`, and `sqlite.rs:1365` leave native work outside canonical unresolved/reconciliation accounting. `lifecycle-service/src/socket.rs:181` also rejects native lifecycle commands. The accepted generation-switch route is absent, rather than merely untested. Integrate native custody, results, recovery, and generation handoff with the lifecycle owner.

3. **S5C-F3 — High: settlement can overlook outstanding or unsafe activity.**  
   `actor.rs:399` qualifies started snapshot tools; `native.rs:740` examines only completed items. History checks at `native.rs:927` ignore unknown/remote/question variants and detailed tool status. A started-but-uncompleted tool or adverse history-only item can escape settlement checks. Reconcile every relevant started item and validate full-history semantics.

4. **S5C-F4 — High: shutdown can discard cleanup ownership.**  
   `actor.rs:570,586` records the controlled close report before native cleanup, then aborts the native owner on timeout without observed reaping or durable unsafe-close status. `operations/mod.rs:399` also discards cleanup errors. Retain exact child cleanup custody and record unresolved cleanup distinctly.

5. **S5C-F5 — Medium: malformed terminal evidence can be lost.**  
   `operations/mod.rs:132` interprets terminal fields before persisting raw ingress. Missing or malformed terminal identity exits without durable frame custody. Persist raw coordinates before fallible interpretation.

6. **S5C-F6 — High: required installed-profile exclusion remains unestablished.**  
   The installed inventory advertises asynchronous question tools. The recorded shell probe tested nonexistent `workspace.shell`, whose output was `unsupported call: workspaceshell`; it did not establish native shell or accepted-async exclusion. Qualify the actual reachable inventory and settlement boundary, or retain Unsupported/unaccepted status. This does **not** assert hidden shell execution occurred.

7. **S5C-F7 — Medium: required observational handoff is missing.**  
   `native.rs:205,364,960` provides no usable versioned history/material reader or allocation-only recovery observation, and rejects every invocation purpose except DomainWork. Implement the accepted H3/H6 observation and distinct-purpose accounting ports without adding S6/S7 semantic behavior.

All 24 candidate file hashes matched the attributed manifest. Findings use immutable source and supplied evidence; no additional executable tests or live inference ran. Positive snapshot-read, same-thread follow-up, and network-isolation evidence remain valid. Mutable graph evidence was background only; SQL was read directly.

Requested and host-observed reviewer: **gpt-6-astra / xhigh**, fresh entry without builder history. Builder: **gpt-6-sol**. Same provider, different model; `independence_claimed: false`.

### 2026-10-08T04:10
**Yes. These proofs support the narrowed configuration contract; observing every denied attempt is not independently required.** My earlier F6 wording was too broad if read as requiring all advertised tools to disappear or every refusal to enter the lifecycle ledger.

- **Advertised `functions.exec`: denied before execution.** The native return is `code-mode host is disabled`, effective configuration has `code_mode_host=false`, the marker is absent, and no operation was admitted. Subsequent parent settlement is compatible with T16. Missing tool ingress/history for this refusal does not establish unresolved activity.
- **Valid asynchronous question: accepted native activity, correctly fenced.** Raw ingress sequence 13 records `agentMessage` with `delivery:"async"` and questions as `unsupported`. The parent remains `entered_uncertain`, with no final result and zero operation starts. This supplies actual installed-route evidence that was missing from the original F6 assessment.

The owning requirement concerns unauthorized execution and false settlement of accepted unresolved activity. The raw-ingress requirement covers frames emitted to the adapter; it does not create a requirement to fabricate lifecycle events for native-internal denials. A pinned binary/configuration boundary, supported by these route observations, can establish that property without a new mediator.

Limits: these observations cover the exact configurations and routes tested, not arbitrary configurations or hidden universal behavior. The marker alone would be insufficient; the explicit disabled-host response and effective configuration supply the decisive evidence. I verified both proof, harness, and service-binary hashes against their metadata; candidate-to-binary build attribution remains supplied by the parent.

**Correction to F6:** these new proofs resolve its identified exec/async evidence gap. F6 should no longer demand Unsupported status merely because those tools remain advertised or the exec refusal lacks a tool event. Formal retained re-evaluation can close F6 after binding these evidence references to the reviewed build. F1–F5 and F7 are unchanged.

The durable finding remains unchanged at `7f434c211a16957f6ea487a5ca93a388e751e01a`, as requested; this was clarification, not a formal re-evaluation.

### 2026-10-08T05:00
C2 `9f29b45e`: **changes required**.

- **F1 high:** `native.rs:1358` still omits the durable clock rollback fence. Add transactional high-water validation/update without changing exact committed replay.
- **F3 high:** `native.rs:1219` accepts history with conflicting tool arguments/results when call ID and success match. Reconcile exact result semantics.
- **F7 medium:** `native.rs:275,341` omits persisted acquisition time from material readers. Expose it with the evidence coordinate.

**F2/F4/F5/F6 resolved.** Generation proof establishes mechanical handoff, not semantic adequacy. Cleanup and malformed-terminal adverse cuts remain source-qualified; route exclusion remains limited to the tested pin/config. No new findings.

[Detailed dispositions](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/reviews/s5-contract/re-evaluation-candidate2-g1.json) published and read back at **`cb789644e7053f1512bc081784ca92087e88f2e1`**. Episode remains active, generation 1.

Requested/observed: Astra xhigh; same provider, different model from builder Sol. No renewed freshness or independence claim.

### 2026-10-08T05:14
C3 `a751eef7`: **changes required**.

**F1 remains high:** native observations and settlement do not advance the subject’s clock high-water (`native.rs:673,1266`). After entry at 100 and settlement at 300, rollback to 150 can admit a follow-up under a grant expiring at 250. Preserve bound observation/settlement time and test that sequential route.

**F2–F7 verified resolved**, retaining the cleanup, malformed-terminal, pinned-route and semantic-adequacy limits. No new finding IDs.

[Detailed result](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/reviews/s5-contract/re-evaluation-candidate3-g1.json) published and read back: **`1f24197c2979e07f7328f2a121d7c5a2efe4b5e7`**.

Episode remains active, generation 1, Astra/xhigh. No renewed freshness or independence claim.

### 2026-10-08T13:44
C4 `bdc83d8`: **changes required**.

**F1 remains high:** `native.rs:637–640,910–924` discards the clock update when a valid-grant admission later fails on active-work, budget or insertion conflict. Preserve that trusted time while keeping the failed entry uncommitted.

The prior sequential observation/settlement gap is fixed. **F2–F7 remain resolved.** This residual is source-qualified at the store port; no installed overlapping-turn behavior is claimed.

[Review receipt](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/reviews/s5-contract/re-evaluation-candidate4-g1.json) published and read back at **`abf108420df88b829651767822fdf851d1570e83`**.

Same active episode, generation 1, Astra/xhigh; no renewed freshness or independence claim. Prior evidence limits remain.

### 2026-10-08T14:15
C5 `a211ca3`: **no unresolved material findings**. **S5C-F1–F7 are verified resolved**; no new material defects found.

Published and read back revision: `59a1bd07a95857cb3effdca57563b8859d995be3`.

[Review receipt](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/reviews/s5-contract/re-evaluation-candidate5-g1.json)  
SHA256: `804096ddf41f4100460ade73d41ef966d146642f858fc60403fbdda5ebfe3336`

Prior limits remain: C4 generation proof was not rerun; F4/F5 adverse cuts are source-qualified; installed simulation is not live-provider or semantic-adequacy evidence. The receipt records the nonblocking history-fixture limitation.

Same episode/generation remains active and reported. Astra/xhigh; no renewed freshness or independence claim. Parent owns acceptance.


### S5 agent-instruction reviewer reports

### 2026-10-08T03:59
**Applicable; no material instruction findings.** This is bounded advisory evidence, not S5 acceptance.

Subject: `3c40bbfd7d38ad078c76c4db7403ea071c2f861f`, tree `0676d123f01e9742719e1c8f280e48cb79dc1643`.

Inspected the namespace/function descriptions and required schema in [operations/mod.rs](~/code/.work-engine-tmp/s4-20261007/tmp/s5-review-candidate1-full-3c40bbfd/rust/crates/lifecycle-service/src/operations/mod.rs:249), lines 249–259, plus necessary context:

- Loading: same file, lines 215–305 and 450–542.
- Trusted configuration: `lib.rs:44–73`.
- Snapshot/parser semantics: `workspace_snapshot_read.rs:9–56,59–151`.
- Authority and dispatch: `actor.rs:153–185,477–558`.

The schema is normative because it governs agent tool calls. The descriptions expose a finite immutable-read capability without prescribing a workflow. Required IDs and byte-range fields express the implemented interface; grant selection and operation authority remain in trusted service composition. The model receives the approved/immutable/bounded scope at tool loading. Backend enforcement details are reviewer evidence, not assumed model exposure.

Full `AGENTS.md`, `DESIGN.md`, and `PHILOSOPHY.md` were read. DESIGN’s interface, minimal-command, and runtime-projection rules support this assessment; PHILOSOPHY was treated as non-normative. All four inspected implementation files match the candidate manifest hashes.

Limitations: source review only; no model-behavior test, live execution, arbitrary prompt-content review, architecture decision, or implementation-completeness claim. Graph generation `2026-10-08T03:42:58Z` reported no recorded coverage gaps, but isolated candidate bytes controlled.

Initial result was published and read back:

- Episode: `d6a3cee4-c119-4552-ac04-1b0222b80724`
- Durable revision: `4132ef0a9731b7ac290505f10d5cad4df6a7b216`
- State: `active / reported`; findings and unresolved questions empty; not retired.

Requested and host-observed runtime: **GPT-6 Astra, xhigh**, fresh entry without inherited builder history. Same provider/different model from builder GPT-6 Sol; `independence_claimed: false`. Reviewer retained for remediation.

### 2026-10-08T04:53
**Applicable; no material findings.** C1’s zero-finding result carries forward to C2. Finding IDs: none.

Subject: `9f29b45ed35175116ca7d0f88c0eec5c4ef6391c`, tree `19c7d1a604a29eb3890a2f7b20abb1da170e31bc`.

Decisive evidence:

- [operations/mod.rs](~/code/.work-engine-tmp/s4-20261007/tmp/s5-review-candidate2-full-9f29b45e/rust/crates/lifecycle-service/src/operations/mod.rs:253), lines 253–263: unchanged normative tool descriptions/schema.
- Same file, lines 271–334: predecessor text enters a distinct successor-rehydration turn as user text.
- `engine.rs:37–49,129–185`: accepted predecessor JSON has fixed Boolean/version fields and a successor-context string matched to trusted configuration; no free-form instruction field.
- `actor.rs:262–303`, `sqlite.rs:863–885`, `native.rs:586–611,1198–1212`: loading remains bound to committed predecessor bytes and exact successor input.

The loading delta does not introduce an undeclared instruction contract in this bounded JSON shape. The flags are predecessor data; tool grants remain separately owned and checked. This does not establish semantic readiness, model interpretation, or S5 acceptance.

All five inspected implementation files match C2 checkpoint hashes. Governing documents and specialist contracts are unchanged. Graph generation `2026-10-08T04:35:20Z` reported no recorded gaps; isolated candidate source controlled.

Same-episode result published **and read back**:

- Episode: `d6a3cee4-c119-4552-ac04-1b0222b80724`
- Revision: `af86de024f0f73f77501d22bc446189202fe348d`
- State: `active / reported`, not retired; findings and unresolved questions empty.
- Selection: [review-selection-candidate2.json](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/review-selection-candidate2.json)

Retained Astra/xhigh generation 1; same provider/different model from builder Sol. No renewed freshness or independence claim.

### 2026-10-08T05:11
**Applicable; zero material instruction findings.** C2’s result carries forward to C3; finding IDs: none.

Subject: `a751eef72ee210be5a95187c88b4a2af4ec4c6d6`, tree `d287693483777974762a9f6c38067618a2fb99de`.

Verified unchanged:

- `operations/mod.rs:253–263`: model-facing descriptions and required schema.
- `operations/mod.rs:271–334`, `engine.rs:37–49,129–185`, `actor.rs:262–303`: successor-input loading and closed JSON shape.

Reviewed the complete C2→C3 `native.rs`/`sqlite.rs` delta. Clock checks, history equality, and timestamp projections introduce no new agent directive, precedence rule, or instruction authority. All five implementation files match C3 checkpoint hashes.

Published and read back in retained episode `d6a3cee4-c119-4552-ac04-1b0222b80724`:

- **C3 revision:** `752526e5f4fbcbcde62da6bee3a1dca263d6fbed`
- **Preserved C2 revision:** `af86de024f0f73f77501d22bc446189202fe348d`
- State: `active / reported`; no findings or unresolved questions; not retired.
- [Exact C3 selection](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/review-selection-candidate3.json)

Bounded source review only; no S5 acceptance or semantic-adequacy claim. Same retained Astra/xhigh generation 1, same provider/different model from builder Sol; no renewed freshness or independence claim.

### 2026-10-08T13:44
**Applicable; no material instruction findings.** No omission. C3’s bounded result carries forward.

Reviewed exact Git candidate `bdc83d83637c649aa417bdb76fb4a61a9839ae28`, tree `5fefb07fd7e3f2f0d9294438517be75a5a0881ec`.

- Normative surface: `operations/mod.rs:253–263`, with loading at `271–334`; unchanged from C3.
- Necessary context: `engine.rs:37–49,129–185`, unchanged.
- C4 delta: `actor.rs:513–517` and production changes in `native.rs` bind ingress to the owned session and preserve clock observations. They introduce no agent-facing directive or changed instruction authority.

Inspected file hashes match C4’s checkpoint. This is source-only advisory evidence, not implementation acceptance or semantic-readiness certification.

Same retained episode `d6a3cee4-c119-4552-ac04-1b0222b80724`, generation 1:

- **Published and read-back revision:** `8deb8714c40739ba271caa7d5e4aa25f153258ee`
- State: `active / reported`; findings and unresolved questions empty; not retired.
- Receipt ref: `refs/work-engine/independent-review-state/1fba5eb2d44c0a1c8cc75b592e5644cce326b6bdcc8abbf8288704f9de37c689`
- [Exact C4 selection](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/review-selection-candidate4.json)

Retained Astra/xhigh; same provider/different model from builder Sol. No renewed freshness or independence claim.

### 2026-10-08T14:14
**Applicable; zero material instruction findings. No omission.** C4’s bounded result carries forward to C5.

Subject: `a211ca3044444eb26775b61e7e786bfd897ac824`, tree `58d4f5eae9517f1aac5a0de9d636841ca237287f`.

Verified unchanged tool descriptions/schema at `operations/mod.rs:253–263` and loading at `271–334`, with unchanged `engine.rs`/`actor.rs` context. Inspected the `native.rs:634–667,918–961` transaction delta: it adds no agent directive, precedence rule, or changed instruction authority. Exact inspected blobs match the checkpoint.

Same retained episode `d6a3cee4-c119-4552-ac04-1b0222b80724`, generation 1, was CAS-published and read back:

- **Revision:** `e7b294ed3a438c6767136e50d31a68d3e73445eb`
- **Payload SHA-256:** `26ac867bde4f16dd101b20ce2547767fd97dff6a058858306c0fb003bcb9f589`
- **Receipt ref:** `refs/work-engine/independent-review-state/1fba5eb2d44c0a1c8cc75b592e5644cce326b6bdcc8abbf8288704f9de37c689`
- State: `active / reported`; empty findings and unresolved questions; not retired.

Source-only advisory review, not S5 acceptance or transaction-correctness certification. Retained Astra/xhigh; same provider/different model from builder Sol. No renewed freshness or independence claim.


### S5 builder reports

### 2026-10-08T03:40
S5 is candidate-ready. The expanded gate passed **13/13**, including isolated native proof, serial all-feature tests, Clippy, and both release profiles. The [private candidate receipt](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/candidate-checkpoint.json) binds 24 attributed files to commit `3c40bbfd7d38ad078c76c4db7403ea071c2f861f`; an isolated archive of that commit also passed all-feature dependency checking.

The [candidate-bound review projection](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/candidate-review-projection.json) corrects the instruction-surface classification to **present** because the service sends model-facing tool descriptions and an input schema. The [gate consequence](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/gate-completion.json) retains the test evidence and explicit limitations. Review selection remains with the supervisor; no reviewer was launched.

### 2026-10-08T04:45
Candidate 2 is frozen at private commit `9f29b45ed35175116ca7d0f88c0eec5c4ef6391c`. All 15 gate checks passed, including the isolated two-process native generation switch; the immutable candidate also passes an isolated locked all-features check.

The [review projection](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/candidate2-review-projection.json) binds the exact tree, patch, manifest, gate, and remaining limitations for the supervisor’s rebind. Retained reviewers have not yet reviewed candidate 2. Forced-cleanup and malformed-terminal installed cuts remain source-qualified, without an acceptance waiver.

### 2026-10-08T05:15
C3 is frozen and **not acceptance-ready**. The retained contract review found one blocking F1 clock-boundary gap; F2–F7 are resolved with their stated limits. The instruction review found no material issues.

The [replacement handoff](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/replacement-handoff-c3.json) binds the exact 25-path candidate, review results, affected checks, clock-boundary map, and sequential regression. Source changes are paused pending the user’s effort-escalation decision.


### S5 second (xhigh) builder reports

### 2026-10-08T14:24
The accepted S5 checkpoint is bound to a validated schema-v5 semantic audit receipt: [audit receipt](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/audit-receipt-candidate5-semantic.json), SHA-256 `f178f53f838f3fc4c7bcf49a1925cfea6416ec38eec0ac8a0bdd9d7b51e504d8`. The [commit proposal](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/commit-proposal-candidate5.json) and [handoff](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/handoff-receipt-candidate5.json) are ready.

Both retained reviews cleared exact C5: F1–F7 resolved and agent-instruction review found no material issue. The receipt records the evidence limits, including the C5 history-fixture limitation and the C4 generation proof’s applicability. No ordinary commit was made.

### 2026-10-08T14:32
The publication union is frozen at private checkpoint `8aed0930727050b60c500f6df9efe67451265f51`, tree `968fa1ea6f4f87f24e276e58287927ef1a14ddcf`. Current HEAD differs from the S5 baseline on **none** of the 25 approved paths; the union preserves the 30 published CE3/SC2 path changes outside them.

[Validation](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/publication-union-validation.json) passed locked offline Cargo resolution and a workspace all-features check. The [checkpoint](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/publication-union-checkpoint.json), [schema-v2 proposal](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/publication-union-commit-proposal.json), and [publication request](~/.local/state/work-engine/rust-context-lifecycle-s5-20261008/publication-union-request.json) are ready. The branch, shared index, and shared worktree remain unchanged; ordinary publication belongs to the parent.


---

## S6/S7 — semantic preservation and recovery/admin (joint)

Accepted at candidate 4. Shared contract frozen and revised 14 times; each revision's stated reason follows.

- **v2** — Co-builders send any required shared type/signature amendment to integration and root; only integration edits shared files and republishes freeze with attributable revision. No unilateral narrowing of accepted outcomes. Revision 2 aligns method names with S7 co-builder request; no semantic/authority reduction.

- **v3** — Co-builders send any required shared type/signature amendment to integration and root; only integration edits shared files and republishes freeze with attributable revision. No unilateral narrowing of accepted outcomes. Revision 2 aligns method names with S7 co-builder request; no semantic/authority reduction. Revision 3 adds exact S7 selector and observation provenance fields, and clarifies component-only launch guard proof.

- **v4** — Revision 4 preserves revisions 1-3 verbatim and records exact additive implementation contract after co-builder findings; no accepted criterion narrowed. Subsequent changes require immutable revision 5.
  - additions: {"authority": "Human-approved existing S6/S7 objective; parent procedurally accepted bounded Yielded addition for A5, no new production operation/authority", "paths": ["rust/crates/lifecycle-core/src/contracts.rs", "rust/crates/lifecycle-core/src/reducer.rs", "rust/crates/lifecycle-store/src/admin.rs", "rust/crates/lifecycle-store/src/native.rs", "rust/crates/lifecycle-store/src/sqlite.rs", "rust/crates/lifecycle-service/src/material_port.rs", "rust/crates/lifecycle-service/src/lib.rs", "rust/crates/lifecycle-service/src/socket.rs", "rust/crates/lifecycle-service/src/actor.rs", "rust/crates/lifecycle-service/src/controlled.rs", "rust/crates/lifecycle-service/src/projection.rs", "rust/crates/lifecycle-wire/src/observation.rs", "rust/crates/lifecycle-test-support/Cargo.toml", "rust/crates/lifecycle-service/Cargo.toml"], "semantic_change": "Exact service material port and synthetic owner test seam; no semantic verdict acceptance yet", "admin_change": "Native ingress owner/session provenance, signed-retirement readback, coherent transfer retry and rollback lineage; explicit Yielded outcome preserved through existing effect observation and wire projection", "compatibility": "Existing Pending/Completed/Failed/Interrupted/Unknown outcomes retain their encoding and reducer behavior. Yielded is an additive terminal outcome on offline component scope only. Downstream wire/client compatibility and focused tests required before acceptance."}

- **v5** — Revision 5 supersedes implementation interface details from v4 without modifying any predecessor; additive third verifier session role and artifact refs avoid reusing an exited native child. No accepted criterion narrowed.
  - additions: {"authority": "Already accepted S6/S7 implementation boundary; parent confirmed semantic-enabled rehydrate seam fits accepted material/prompt bounds, no new policy choice", "migration": "0004_composite_s67.sql remains sole v4 migration; add semantic_successor_verify role and candidate_record/successor_prompt artifact refs before any publication", "service": "NativeConfig.semantic optional, run_with_material_port injects canonical owner, S5 config path unchanged; three isolated control child roles via pinned app-server, no dynamic tools, exact native ingress/turn settlement", "store": "Semantic capture/result/checkpoint immutable artifact linkage; native control claim under semantic_control grant, exact prompt/history and no-replay; successor prompt conditional exact package readback", "cuts": "opt-in semantic capture published-before-link, control before-entry/after-entry/after-result, semantic_before_commit owner fence; artifact_after_blob_sync explicit owner hook; startup before-open/before-recovery/after-recovery opt-in", "limitations": "Joined happy path and negative acceptance still executing; this revision freezes APIs and provenance, not an acceptance claim."}

- **v6** — Revision 6 supersedes shared implementation readback and proof-cut details from v5 without modifying any predecessor; no accepted criterion narrowed.
  - additions: {"authority": "Already accepted S6 A2/A6/A8 owner currentness, grant, and explicit incomplete-material criteria; no new production operation or human policy choice", "native_exposure": "Each host-stamped semantic exposure_id is derived from exact invocation_id; no placeholder identity is persisted.", "schema": "semantic_results.phase is explicit candidate_verify|successor_adequacy; basis artifact persists issue-bearing source projection as unresolved capture, with no control admission. Exact source change marks capture and derived results stale, keeping immutable bytes/findings.", "shared_paths": ["rust/crates/lifecycle-store/src/semantic.rs", "rust/crates/lifecycle-service/src/semantic.rs", "rust/crates/lifecycle-service/src/semantic_control.rs", "rust/crates/lifecycle-service/src/engine.rs", "rust/crates/lifecycle-store/migrations/0004_composite_s67.sql"], "test_only_grant_cut": "With native-sim-proof and explicit point.revoke_grant file at semantic_compile_before_entry or semantic_compile_after_entry, service calls real SqliteLifecycleStore::revoke_trusted_grant for exact configured semantic_control grant/revision while owning sole writer. Feature absent means no hook; no result/stage injection.", "validation": "cargo check -p lifecycle-service --features native-sim-proof --offline passes; cargo test -p lifecycle-store -p lifecycle-admin --offline passes default tests; joined S6 A6/A8 proof pending."}

- **v7** — Revision 7 closes a stale semantic-package fallback within the previously accepted authority fence; v6 and all predecessors remain immutable.
  - additions: {"authority": "Parent confirmed within accepted S6 no-bypass authority/currentness fence; no new human decision or production operation", "paths": ["rust/crates/lifecycle-store/migrations/0004_composite_s67.sql", "rust/crates/lifecycle-store/src/semantic.rs", "rust/crates/lifecycle-store/src/sqlite.rs", "rust/crates/lifecycle-store/src/native.rs", "rust/crates/lifecycle-service/src/actor.rs", "rust/crates/lifecycle-service/src/engine.rs", "rust/crates/lifecycle-service/src/semantic.rs"], "result": "Durable semantic_required marker selects S6 guards independently of current package. Service rejects semantic configuration without canonical material port, and cached accepted successor result is rechecked against current owner dependencies before use.", "trigger": "After a candidate checkpoint becomes stale, semantic_checkpoint returns None; package-presence guards alone could take S5 fallback at restricted transition or native rehydration API.", "validation": "cargo check -p lifecycle-service -p lifecycle-store --features lifecycle-service/native-sim-proof --offline passed; cargo test -p lifecycle-store -p lifecycle-admin --offline passed all default tests. Direct stale-package restricted fact and export/adoption marker regressions pending joined worker tests."}

- **v8** — Revision 8 adds explicit unsupported-material negative custody and early successor currentness checks within accepted S6 A2/A7/A8; all predecessor freezes remain immutable.
  - additions: {"authority": "Accepted S6 A2/A7/A8 negative and currentness evidence; no new production operation, policy choice, or inference provider", "crash_cuts": "Optional native-sim-proof semantic_after_candidate_result and semantic_after_successor_result barriers occur after durable result row before next stage fact, with no production result injection.", "paths": ["rust/crates/lifecycle-semantic/src/policy.rs (S6 owner exact policy fix)", "rust/crates/lifecycle-semantic/src/material.rs (S6 owner typed UnsupportedPolicy)", "rust/crates/lifecycle-store/migrations/0004_composite_s67.sql", "rust/crates/lifecycle-store/Cargo.toml", "rust/crates/lifecycle-store/src/semantic.rs", "rust/crates/lifecycle-store/src/native.rs", "rust/crates/lifecycle-store/src/lib.rs", "rust/crates/lifecycle-service/src/actor.rs", "rust/crates/lifecycle-service/src/semantic.rs"], "policy": "Only exact s6-strict-synthetic-v1 schema1 accepted by S6 pure projection; unsupported policy/schema returns typed UnsupportedPolicy. Shared store records exact digest-bound unresolved rejection with no control admission.", "successor_currentness": "Current canonical dependencies checked at gen2 startup, BindThread prompt and SuccessorRehydrate PrepareTurn; stale source marks capture/results ineligible and refuses entry. Store successor session reservation requires current package whenever semantic_required=1. Final activation still holds owner fence through commit.", "validation": "cargo check -p lifecycle-service --featur

- **v9** — Revision 9 adds the exact semantic staged-before-publish crash cut and records joined executable validation of the v8 contract; predecessors remain immutable and no accepted criterion narrowed.
  - additions: {"authority": "Accepted S6 A5 staged/published crash qualification and A2/A6/A7/A8/A9 joined proof, no new production operation", "cut": "semantic_after_capture_staged_before_publish is optional and pauses after real stage_artifact before publish_artifact; semantic_after_capture_published_before_link remains after durable publication before DB linkage. Both use actual store owner path, not outcome injection.", "joined_validation": "cargo test -p lifecycle-service --features native-sim-proof --test s6_semantic_preservation --offline -- --test-threads=1: 14 passed, 0 failed, 0 ignored, 406.75s. Includes candidate/successor C1, omission/echo, staged/published capture, A2 four timing cuts and T9, A6 grant cuts, A7 four crash cuts, A8 incomplete/unknown policy, A9 final fence and direct no-fallback.", "paths": ["rust/crates/lifecycle-store/src/semantic.rs", "rust/tests/s6_semantic_preservation.rs (S6 worker owner)"], "remaining": "Composite deterministic gate and candidate/review binding remain root-authorized later; this freeze is implementation contract/evidence, not acceptance or publication."}

- **v10** — Revision 10 retires the v9 typed wire-v1 Yielded projection premise after direct generated-schema gate failure and parent decision; exact core/store/admin Yielded custody remains, while published wire v1 uses explicitly qualified Unknown without new authority.
  - additions: {"authoritative_owner": "Core/store/admin retain exact Yielded, still-running false, original settlement and no-retry custody across transfer/rollback.", "authority": "Parent procedural decision within human approved offline S6/S7; no v2, UI edit, production operation or publication.", "decision": {"path": "~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/boundary-decision-wire-yielded.v1.json", "sha256": "25733bcdd4e2305b26777d9dbd3350c93f83be1951c9691bfe84630dbebd8036"}, "source_paths": ["rust/crates/lifecycle-service/src/projection.rs", "rust/crates/lifecycle-wire/src/observation.rs"], "validation": "Focused lifecycle-wire schema_compat 6/6, service qualified projection unit 1/1, standalone other-owner terminal-ui package 47/47 pass before final subject gate; client wait and S7 exact roundtrip included in final gate.", "wire_v1": "Published v1 enum/schema/manifest/client hash remain byte-identical; known Yielded is represented conservatively as Unknown only with yielded_outcome_not_representable_v1 blocking_reason and stable original delivery/effect/attempt IDs, settlement, provenance and evidence refs. Independent unresolved_entered_attempt marker remains visible. No typed Yielded-v1 claim."}

- **v11** — Revision 11 adds only Candidate1 review repairs within the accepted S6/S7 boundary. Candidate1 stays immutable; affected gate and Candidate2 selection are required before retained reviewer consumption.
  - additions: {"AIR_C1_F1_F2": "Pure prompt contract advertises exact consumed JSON shapes and verifier enum spellings. Owner-supplied bounded verifier_scope and exact source references expose the obligation target, while private expected answer/description remains withheld.", "S6_C1_F1": "When a transition durably requires S6 semantics, service without configured semantic runtime/material port remains query-only and cannot fall through to S5 ReadyToCommit activation. Direct store package guard remains; controlled restart regression proves no advance after owner revision change.", "S6_C1_F2": "C1 challenge is frozen answer-free; joined test decodes exact native successor input and rejects leaked obligation answers. Successor adequacy remains independently verified.", "S7_C1_F1": "Destination adoption has a durable exact identity intent outside copied database before copy. Ordinary open refuses pending destination; exact retry reconstructs partial copy. Complete intent publication is atomic before destination creation; process crash after DB copy and wrong-identity probes are covered.", "S7_C1_F2": "Transactional custody revision covers mutable owner tables across native, semantic, recovery and administration. Export/adoption/fence bind the exact clock and stale capsule/retry rejects after intervening mutations. v4 coherence requires clock and complete trigger set.", "S7_C1_F3": "Native grant/entry and unsent send intent require current active ownership. Public retained-parent and unsent-tu

- **v12** — Revision 12 records Candidate2 retained S7 review residuals within accepted A2/A4 and exact in-scope F4/F5 repair. Candidate2 remains immutable; C3 affected gate and root selection rebind required.
  - additions: {"S7_C1_F4": "A qualified fresh-coordinate turn/interrupted after completed_settled is contrary terminal evidence, alongside turn/failed, and creates durable incident/subject fence without replacing immutable original retirement/outcome/settlement.", "S7_C2_F5": "Adoption publishes exact pending-intent bytes atomically and holds that inode lock through copy/transaction/cleanup; overlapping retry returns Unavailable. Complete copied bytes have a separate durable marker; when marker exists, I/O unavailability preserves destination. Partial reconstruction requires exact intent plus destination writer lock, and pre-marker crash is retryable.", "lane_test_paths": ["rust/crates/lifecycle-admin/tests/late_conflict.rs", "rust/crates/lifecycle-admin/tests/review_f1_partial_adoption.rs"], "review_episode": "S7 retained same-session Candidate2 revision 2f787a35dc129aa64978fe8dff5f9a4d10e5796a, integrity 5d8aa0c8afba5d32dfbeaa4734045d6c121510c643964785f9812fd5f44ce924", "scope": "No schema/version or published wire change; no new grant, remote operation, live inference or production selector authority.", "shared_paths": ["rust/crates/lifecycle-store/src/native.rs", "rust/crates/lifecycle-store/src/admin.rs"], "validation": "S7 v10 focused public owner/process regression plus affected isolated C3 gate pending."}

- **v13** — Revision 13 records Candidate3 S7-C2-F5 residual within already accepted A4 custody: complete copied/admitted destination cannot be discarded on configuration or indeterminate readback. C3 immutable; C4 affected gate and root reviewer rebind required. No schema, wire, authority or API expansion.
  - additions: {"finding": "S7-C2-F5 retained C3 episode revision 7de41af7e56f061576f283c388e5eb58e6c66817; original F1-F4 resolved, F5 remains P1 for generic Rejected/Unavailable readback cleanup.", "lane_test_paths": ["rust/crates/lifecycle-admin/tests/review_f1_partial_adoption.rs"], "old_premise": "Revision12 protected copy-marker + Unavailable only; handoff-query errors mapped Rejected and empty trusted issuer rejected before inspecting committed custody, both reached generic recursive deletion. C3 lock-contention/exit86/87 tests did not exercise post-commit readback/configuration failure.", "repaired_boundary": "Reject empty trusted issuer before sidecar mutation. On retry hold exact intent inode and destination writer lock; only delete when marker is provably absent before admission or complete copied image is positively read as active source identity with no new handoff row. Any query/coherence/configuration error or inactive/committed identity preserves destination. Sync intent removal before marker cleanup so marker absence with intent cannot denote completed admission.", "scope": "No schema/version/published wire change; no new production authority, live inference, actual saved-state mutation, ordinary commit or push.", "shared_paths": ["rust/crates/lifecycle-store/src/admin.rs"], "test_cuts": ["adoption_after_inactive_commit exit88 before cleanup", "adoption_readback_unavailable post-open fault"], "validation": "S7 v11 red/green 4/4 exact process/public-owner F1 target plus tran

- **v14** — Revision 14 clarifies v13 descriptive wording only; production source, tests, schema, wire, authority and Candidate4 gate subject are unchanged. A failed first readback can be followed by a separate positive probe under both locks; cleanup is allowed only if that probe establishes unadopted copied-source state.
  - additions: {"no_authority_change": true, "operative_rule": "An initial error alone never authorizes deletion. Complete-marker cleanup requires positive coherent active copied-source identity and absence of new handoff. If the probe itself errors, returns inconsistent identity, or shows inactive/admitted custody, existing destination is preserved. Pre-open empty issuer is rejected before intent mutation.", "v13_overstatement": "The v13 sentence saying any readback query/coherence error preserves destination was overbroad. The actual code may retry an exact copy after an initial readback error when a separate read-only coherence/identity/handoff probe positively proves unadopted active copied-source state under the held writer lock.", "validation_binding": "C4 12/12 exact isolated gate receipt v4, no source change or rerun required for this wording clarification."}


### S6 contract reviewer reports

### 2026-10-08T18:03
**Blocking verdict** on Candidate1 `55203e9a1f7b8684c51aad2f01c61d10dd57dd91`, tree `e537e38213976968b56d48ce02abfc0d767e7cd4`.

- **S6-C1-F1 — P1:** A `controlled-proof` restart with `native=None` can advance a durable semantic-required `ReadyToCommit` transition without the canonical-owner fence. The engine selects that fence from current runtime configuration; the store checks cached eligibility and grants only. A changed external owner can therefore leave stale material eligible for activation. Evidence: `lifecycle-service/src/engine.rs:318`, `actor.rs:117,230,357`, `lifecycle-store/src/sqlite.rs:1580`. Require semantic capability from persisted transition state and add the configuration-downgrade restart regression.

- **S6-C1-F2 — P2:** The joined native challenge supplies “unfinished K-1 and U-1,” leaking answers the checkpoint must preserve. This contradicts S6-A1/A10’s answer-free challenge requirement. Evidence: `rust/tests/s6_semantic_preservation.rs:584,1213`; contrast `lifecycle-semantic/tests/continuation_assessment.rs:33`. Use the frozen generic C1 challenge and inspect its exact transmitted content.

Published and read back episode revision **`5b8d162d619b31ce828cc74bba18acb1a09956f4`**, awaiting remediation.

Requested and host-observed model: **Codex gpt-6-astra/xhigh**; launch confirms `fork_turns=none`. Same provider, different builder model; **no independence claim**. Findings derive from immutable source; tests were not rerun. Live semantic quality, held-out evaluation, and OS isolation remain unestablished. Retain this reviewer for continuation.

### 2026-10-08T18:46
Candidate2 `182ab795878be83a77374cdb840d387261b76ce2` (tree `f2f8717784d2dcc6885c6cea6c22c29d1b3ede49`): **no remaining material S6 findings**.

- **S6-C1-F1 verified resolved:** persisted semantic requirement fences advancement without the owner runtime (`engine.rs:130–136`); regression preserves DB-current `ReadyToCommit` across disabled restart (`s6_semantic_preservation.rs:1435–1564`).
- **S6-C1-F2 verified resolved:** frozen C1 challenge is used and checked in the transmitted successor envelope (`s6_semantic_preservation.rs:591,1083–1092,1299–1315`).

Published and read back revision `935efad3d08826384f433e456575122344ab9d3d`, SHA256 `17d9863e26dd605fa414e079d86017a5fb27b372c60a62d30eed61ee7ee200d3`, at:

`refs/work-engine/independent-review-state/14b02c80542f24d783a49786b1e615d48730d3338af8ddbd92cc46b8ef4b8298`

Phase `reported`; generation 1; `same_session`; original finding lineage preserved. Exact gate receipts record **16/16 passing**, including both-feature restart regression; I inspected these receipts without rerunning tests.

Host-observed runtime remains Astra/xhigh, thread `01a11ca6-81a6-7ab1-9d66-b09ea4d54f06`. No new freshness or independence claim; live quality, heldout performance, and OS isolation remain unestablished.

### 2026-10-08T19:17
Candidate3 `127040e0a1e4f54d81c46a2a4655498e113fa89e`, tree `8ed76bcaa4e045dc77e6c45cf6abd1dd7ccc46c1`: **no material S6 findings**. S6-C1-F1/F2 remain `verified_resolved`, with original lineage preserved.

The native custody and adoption changes preserve S6 fencing and custody. The **13/13 affected checks plus explicitly reused Candidate2 evidence suffice for this four-file delta**; they do not constitute a full Candidate3 gate rerun.

Published and read back:

- Revision: `7d21f92b58ebcbe75840ac250ab52e80adc0f0ab`
- State: `reported`, `active`, `same_session`, generation 1
- SHA256: `7a5bbce70cc904093861fda9e756d98d8ed3a930015420504f067bf09c67080b`
- Ref: `refs/work-engine/independent-review-state/14b02c80542f24d783a49786b1e615d48730d3338af8ddbd92cc46b8ef4b8298`

Evidence was inspected, not rerun. Retained host-observed Astra/xhigh runtime; no new freshness or independence claim. Live quality, heldout performance, and OS isolation remain unestablished.

### 2026-10-08T19:34
Candidate4 `080b9cf67a7671a95e2ec9bc612c8e0a2af3ef81`, tree `3469e092cdedb7a2d70e81e8494f0319e9ae28e1`: **no material S6 findings**. S6-C1-F1/F2 remain `verified_resolved`; original lineage is preserved.

The adoption-only delta preserves semantic/native custody. **C4’s 12/12 affected checks plus explicitly retained C3/C2 evidence suffice for this bounded review.** Freeze v14’s clarification matches the implementation.

Published and read back:

- Revision: `17c9dc1cd080f703f6c5b3bf0ca678ab2f92eb70`
- Phase: `reported`; continuity: `same_session`; generation: 1
- SHA256: `88ef9495c450fc3a145840852f152d807eee0389fe4320cc3467e357240a839a`
- Ref: `refs/work-engine/independent-review-state/14b02c80542f24d783a49786b1e615d48730d3338af8ddbd92cc46b8ef4b8298`

Receipts inspected, tests not rerun. Retained host-observed Astra/xhigh; no new freshness or independence claim. Live quality, heldout performance, and OS isolation remain unestablished.


### S7 contract reviewer reports

### 2026-10-08T18:03
**Blocking verdict** for Candidate1 `55203e9a1f7b8684c51aad2f01c61d10dd57dd91`, tree `e537e38213976968b56d48ce02abfc0d767e7cd4`.

Four P1 findings:

- **S7-C1-F1 — Partial adoption exposes active custody.** The destination receives the active database before the transaction makes it inactive and revokes copied grants. A crash in between leaves an admissible clone while the source remains active. Evidence: `lifecycle-store/src/admin.rs:1095–1159`, `1177–1204`; `sqlite.rs:111–261`. Publish partial adoption in a fail-closed form and test internal copy/commit cuts.
- **S7-C1-F2 — Source freshness omits custody mutations.** The transfer token counts journal entries and ownership revisions, but retirement, recovery observations and native writes do not advance it. Resuming an older capsule can discard intervening retirement or attempt state. Evidence: `admin.rs:1049–1067`, `803–817`, `1247–1250`; `native.rs:958–979`. Fence against a revision covering every custody mutation.
- **S7-C1-F3 — Native child/send boundaries bypass ownership fencing.** A retained parent can mint a snapshot-read child after source fencing; an unsent native turn can also advance its send intent. Evidence: `native.rs:1110–1304`, `984–991`, `1824–1896`. Enforce active ownership at these boundaries.
- **S7-C1-F4 — Late contradictions do not fence subsequent admission.** New-coordinate contrary ingress leaves cached completion intact. Same-coordinate conflicts block only the old turn, while later turn admission excludes only `entered_uncertain`. Evidence: `native.rs:1076–1106`, `1697–1756`, `953–979`. Preserve original facts while durably fencing affected admission.

Initial findings were published and read back at revision **`19b002ed31e0a135021b437b121a00400af69271`**, episode `s67-candidate1-s7-contract`, generation 1, phase `remediation`.

These are exact-source findings; I did not run new crash experiments or replay the gate. The qualified wire-v1 Yielded projection introduces no additional finding. Production trust, live recovery and S9 remain unestablished.

Host metadata verifies **Codex `gpt-6-astra` / `xhigh`**, exact target and `fork_turns=none`. Same provider/different model; no inherited builder context; **`independence_claimed: false`**. Retained identity remains available for remediation.

### 2026-10-08T18:53
Candidate2 `182ab795878be83a77374cdb840d387261b76ce2` remains **blocked by two P1 findings**:

- **S7-C1-F4 — open:** Fresh-coordinate `turn/interrupted` is qualified by `lifecycle-service/src/actor.rs:647–662`, but omitted from the contradiction predicate in `lifecycle-store/src/native.rs:1117–1120`. After completed settlement, it leaves cached completion and dependent admission unfenced.
- **S7-C2-F5 — new:** `lifecycle-store/src/admin.rs:1120–1125` recursively deletes the adoption destination after any non-`ClaimConflict` opening error. Writer-lock contention returns `Unavailable` at `sqlite.rs:152`, so overlapping same-handoff adoption can unlink another adopter’s lock and replace its destination identity.

**F1–F3 are verified resolved.** Original finding text remains unchanged. F3’s child-claim subclaim requires valid parent/grant/ingress and no unsettled child; immutable Candidate1 supports that case. Candidate2 checks active ownership and tests those preconditions. The unsent-send gap is also closed.

The qualified wire-v1 Yielded projection remains consistent with the accepted boundary: explicit `Unknown` limitation, exact authoritative Yielded custody, and preserved identifiers, settlement, evidence and independent blockers.

Published and read back:

- Revision: `2f787a35dc129aa64978fe8dff5f9a4d10e5796a`
- Predecessor: `ad1d8d3200772241b4cc5de92d286781dc437a66`
- Ref: `refs/work-engine/independent-review-state/6b063b206599e33b72e89cd44e27d1a6b625d296045026aeb422bbef9aeeedb9`
- Integrity: `5d8aa0c8afba5d32dfbeaa4734045d6c121510c643964785f9812fd5f44ce924`
- Phase: `remediation`; generation 1; same-session continuity. All 20 cited source blobs and line ranges verified.

Reviewed supplied 16/16 gate results; no broad suite or new concurrency experiment run. Host metadata confirms retained thread `01a11ca6-9cd0-75d3-a142-f3fb57265994`, Astra/xhigh. No new freshness or independence claim.

### 2026-10-08T19:18
Candidate3 `127040e0a1e4f54d81c46a2a4655498e113fa89e` remains **blocked by S7-C2-F5 (P1)**.

F1–F4 are verified resolved. F5’s lock-contention case is fixed, but accepted adoption can still be deleted after indeterminate readback:

- `admin.rs:1384–1387` converts every handoff-query error into `Rejected`.
- Retry preserves only `Unavailable` at `1179–1180`; `Rejected` reaches recursive deletion at `1196`.
- After inactive commit and a crash before marker cleanup, a query failure therefore permits replacement of committed destination identity. Empty `trusted_issuer` provides another rejection-before-inspection path.

Cleanup needs positive proof of unadopted partial state, with configuration/readback failures preserving existing custody.

The 13-check affected gate reasonably reuses unchanged surfaces, but its passing lock/crash tests do not cover this remaining failure. C2’s 16 checks remain prior evidence. Qualified wire-v1 Yielded handling remains consistent with the accepted boundary.

Published and verified:

- Revision: `7de41af7e56f061576f283c388e5eb58e6c66817`
- Predecessor: `660dfce37509beb2b32b21fcba2f5f5a3c3eec75`
- Ref: `refs/work-engine/independent-review-state/6b063b206599e33b72e89cd44e27d1a6b625d296045026aeb422bbef9aeeedb9`
- Integrity: `72fd062ab59232f983c42f3a77ec57e9eab88e48535c1806497020d741059828`
- Phase: `remediation`; four resolved, one open; original IDs/text preserved.

F3 retains its valid-parent/grant/ingress and zero-unsettled-child qualification. Same retained generation/runtime; no new freshness or independence claim. Source-established findings; no new fault experiment.

### 2026-10-08T19:35
**No remaining material S7 findings on Candidate4** `080b9cf67a7671a95e2ec9bc612c8e0a2af3ef81` (tree `3469e092cdedb7a2d70e81e8494f0319e9ae28e1`).

All five S7 findings are **verified resolved**, with original IDs/text preserved. F5 now requires positive unadopted-copy evidence under both locks; committed or indeterminate state survives readback failure. Empty issuer rejects before mutation, and durable cleanup ordering preserves the marker invariant. Exit86/87/88, contention and post-open fault tests cover the repaired boundaries.

F1–F4 closures remain valid, including F3’s valid-parent/grant/ingress and zero-unsettled-child qualification. The qualified wire-v1 Yielded boundary remains intact.

The **12/12 affected gate is sufficient for this two-file delta**; C2/C3 checks remain explicitly historical evidence. I did not rerun the suite.

Published and read back:

- Revision: `5fa59cf6387f19e1cdfb583addb07f2452ca79e3`
- Predecessor: `dd03b4f774b617470580fa3e90265f8407a99bb8`
- Ref: `refs/work-engine/independent-review-state/6b063b206599e33b72e89cd44e27d1a6b625d296045026aeb422bbef9aeeedb9`
- Integrity: `553f2bfc1f4160b1f89a50a497c70a380e4df465b5413981b929225cf1882904`
- Phase: `reported`; no unresolved questions; 37 cited source blobs/ranges verified.

Same retained generation/runtime; no new freshness or independence claim. Brief peer-status summaries embedded in the requested gate receipt were incidentally exposed, disclosed durably, and excluded from S7 judgment.


### S6/S7 agent-instruction reviewer reports

### 2026-10-08T18:12
Applicable: **two open advisory findings** on Candidate1 `55203e9a1f7b8684c51aad2f01c61d10dd57dd91`, tree `e537e38213976968b56d48ce02abfc0d767e7cd4`.

- **AIR-C1-F1 — High, restate.** `continuation.rs:430` requests `supported/contradicted`, but `assessment.rs:9–15` accepts `satisfied/violated/indeterminate`. Compiler/verifier prompts also omit their required output schemas. The isolated loader supplies no supplemental schema. Project the consumed interface and explain rejection of incompatible output.
- **AIR-C1-F2 — Medium, split.** `continuation.rs:428,543` exposes only obligation IDs/categories, withholding their source mappings. Controlled C1/C5 obligations share categories despite distinct meanings; test reports access mappings unavailable to the verifier. Expose distinguishable, non-answer-bearing obligation scope while keeping expected answers private.

Published and verified by readback:

- Revision: `171134163670fef46445d1140815ab530164658c`
- Ref: `refs/work-engine/independent-review-state/80534479009f506c80e7d0fbd60e70b252c3ace201f2d21d15181eb8f1145d7a`
- Phase: `remediation`; next action: `await_remediation`

Detailed finding-contract records and immutable evidence references are preserved there. Host-observed `gpt-6-astra/xhigh`; fresh no-inheritance launch, no independence claim. No tests or live inference performed. Retain this reviewer for remediation; acceptance remains with the supervisor.

### 2026-10-08T18:50
Applicable on Candidate2 `182ab795878be83a77374cdb840d387261b76ce2`, tree `f2f8717784d2dcc6885c6cea6c22c29d1b3ede49`.

- **AIR-C1-F1: verified_resolved.** `continuation.rs:215–257,314,496–499,612–613` now projects output structure, correct enums, and rejection reasons.
- **AIR-C1-F2: verified_resolved.** `continuation.rs:192–213` projects owner-authored obligation scope and exact citations; private answers remain excluded. Controlled C1/C5 scopes distinguish their obligations.
- **No new material findings** in the bounded instruction/loading delta.

Published `record_re_evaluation` and verified exact readback:

- Revision: `50988404fd498112661779e58553f032f28a774f`
- Ref: `refs/work-engine/independent-review-state/80534479009f506c80e7d0fbd60e70b252c3ace201f2d21d15181eb8f1145d7a`
- Phase: `reported`; continuity: `same_session`.

Original finding lineage is preserved. Gate evidence reports 16/16 passes; I performed source review, without new test execution or live inference. No renewed freshness, independence, or broader S67 acceptance claim. Coordination claim released; reviewer retained.

### 2026-10-08T19:22
Applicable on Candidate3 `127040e0a1e4f54d81c46a2a4655498e113fa89e`, tree `8ed76bcaa4e045dc77e6c45cf6abd1dd7ccc46c1`.

- AIR-C1-F1/F2 remain **verified_resolved**.
- Prompts, schemas, effective loader, and instruction tests are byte-identical to C2.
- Four changed store/admin/test files introduce no new material instruction consequence.
- C3’s 13/13 affected checks are sufficient for this bounded confirmation; C2’s broader gate remains separate prior evidence.

Published and read back:

- Revision: `bfe0818bae0c8a79c28916f3ad342506fa7451fd`
- Ref: `refs/work-engine/independent-review-state/80534479009f506c80e7d0fbd60e70b252c3ace201f2d21d15181eb8f1145d7a`
- Phase: `reported`; continuity: `same_session`.

No new findings, freshness, independence, or broader S6/S7 acceptance claim. Builder’s coordination claim remains for builder release.

### 2026-10-08T19:37
Applicable on Candidate4 `080b9cf67a7671a95e2ec9bc612c8e0a2af3ef81`, tree `3469e092cdedb7a2d70e81e8494f0319e9ae28e1`.

- AIR-C1-F1/F2 remain **verified_resolved**; no new material findings.
- Prompts, schemas, effective loading, and instruction regressions are byte-identical to C3.
- The two-file adoption/test delta adds no instruction consequence.
- C4’s 12/12 affected checks suffice for this bounded confirmation. Prior gates remain separate; freeze v14 supplies no waiver.

Durable result, exact readback verified:

- Revision: `86b4b85c2c791502aab19b3a7781993f1f6406ad`
- Ref: `refs/work-engine/independent-review-state/80534479009f506c80e7d0fbd60e70b252c3ace201f2d21d15181eb8f1145d7a`
- Phase: `reported`; continuity: `same_session`.

No new freshness, independence, or broader S67 acceptance claim. Builder’s claim remains for builder release.


### S6/S7 integration builder reports

### 2026-10-08T17:15
The S6/S7 implementation phase is published at durable revision `f009e473312de57d480845215a55b0db0acde9a5`. The [composite receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/composite-implementation-receipt.v2.json) binds all 18 acceptance items, source hashes, lane evidence, and a nine-step isolated gate proposal.

S6 passed 15 pure tests and 14 installed joined tests; S7’s offline and process evidence is recorded. The final gate, checkpoint, and reviews have not run. The gate must freeze other-owner build dependencies and rerun the costly joined proof unless exact source and profile applicability is established.

### 2026-10-08T17:49
The S6/S7 deterministic gate passed **16/16 checks** on isolated tree `e537e38213976968b56d48ce02abfc0d767e7cd4`. I published the gate consequence to the active slice.

The [gate receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/composite-gate-receipt.v1.json) and schema-validated [checkpoint request](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/checkpoint-candidate-request.v1.json) bind the exact 90-path subject. The [review projection](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/review-selection-projection.v1.json) identifies the contract changes and normative instruction surface. Published wire-v1 schema tests passed 6/6; the unchanged, separately owned terminal UI passed 47/47.

Checkpoint creation and reviewer selection remain with the parent. No ordinary commit, push, or review was launched.

### 2026-10-08T19:56
Candidate4 is privately accepted. All 18 S6/S7 criteria have mapped evidence, the affected gate passed 12/12, and all nine findings are verified resolved across three retained reviews.

I delivered a validated [schema 5 audit receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/candidate4-audit-receipt-semantic.v1.json) and [compact handoff](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/candidate4-handoff-receipt.v2.json). The active-slice consequence is published at revision `357c185b6cd719de789ba2a7e23b47f3f77832a7`.

The [completion proposal assessment](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/candidate4-completion-proposal-unavailable.v2.json) is unavailable under current fixture and publication constraints. No commit adapter was invoked; no commit or push occurred. Root owns authoritative telemetry assembly and terminal retirement.

### 2026-10-08T21:23
Both authorized commits are on `rescue-detached-20260911`: S6/S7 [`d6ced89`](https://example.invalid) and the separate role-ID repair `c3cfd7e`. No push occurred. The unrelated staged entries and dirty worktree bytes were preserved.

Publication receipts: [S6/S7](~/.local/state/work-engine/rust-s67-publication-20261008/s67-local-publication-receipt.v1.json) and [role-ID repair](~/.local/state/work-engine/rust-s67-publication-20261008/role-id/role-id-local-publication-receipt.v1.json).


### S6 lane builder reports

### 2026-10-08T17:13
S6 implementation and joined proof are complete. Pure tests passed (15/15), installed-path tests passed (14/14), and focused Clippy checks passed.

The [immutable S6 receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s6/implementation-receipt.v3.json) has the exact 13-file manifest and A1–A10 mapping; SHA256: `edf33e5b34f91c58e21c137bede82be2572ec7c9b4864484e26b3d2131ba2f01`. Composite review and publication remain with the integration lead.

### 2026-10-08T17:24
Added [S5 native compatibility test](~/code/work-engine/rust/tests/s6_s7_s5_native_compat.rs). It passes with semantic configuration disabled: **2 tests passed**, proving successor rehydration and reconciliation with no semantic capture or control state.

The additive [S6 v4 receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s6/implementation-receipt.v4.json) is immutable (SHA256 `b577118db355e4dacf90163084e79f26019e9e071d94428451f68ce68e70bdee`). Integration owns the composite gate.

### 2026-10-08T17:26
Corrected the format-only drift in the [S5 compatibility test](~/code/work-engine/rust/tests/s6_s7_s5_native_compat.rs); targeted `rustfmt --check` passes. Published immutable [S6 v5 receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s6/implementation-receipt.v5.json) (SHA256 `9cb7a27c621f4e5da22933334c999b6e631b302604bb88cd5ce6d1842278b0d8`). Integration can rerun the composite gate.

### 2026-10-08T18:22
Completed the S6-owned candidate1 remediations for S6-C1-F1/F2 and AIR-C1-F1/F2. Pure tests passed **16/16**; both focused installed tests, targeted Clippy, and rustfmt passed.

The immutable [v6 remediation receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s6/implementation-receipt.v6.json) records exact hashes and limits (SHA256 `8f5eb38fd3dc2e8c0db31ae8ca1ee9ab703228c50bd59190d085d039183af882`). Integration owns the composite gate and review disposition.

### 2026-10-08T19:51
Updated review-selection validation so distinct S6 and S7 roles can retain the real `codex-adversarial-review` skill with unique IDs. Legacy entries remain valid, and AIR remains exactly once. Metrics tests passed **48/48**; finalizer tests passed **3/3**.

The separate [repair receipt](~/.local/state/work-engine/slice-supervisor-review-selection-role-identity-20261008/implementation-receipt.v2.json) binds exact hashes and the [bounded delta](~/.local/state/work-engine/slice-supervisor-review-selection-role-identity-20261008/repair-only.patch). Its pre-edit files are explicitly labeled as reconstructions, not independently captured snapshots.


### S7 lane builder reports

### 2026-10-08T16:39
S7 offline admin implementation is complete in `lifecycle-admin`; I made no shared-file edits. The [immutable v4 evidence manifest](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v4.json) has SHA256 `7f119cf82fa0daca05993b4db476688297a40f36279767df9be9d4d46db81ddd`.

Validation: 14 default tests passed; four separately run tests using pinned S5 and current-service binaries passed. They cover recovery and signed no-retry disposition, coherent export, current-state rollback, crash cuts, and the selected-destination startup join. The composite gate and review remain with root.

### 2026-10-08T16:54
The S7 follow-up is complete. A public-API test confirms that `semantic_required` survives coherent export, inactive adoption, and current-state rollback.

The admin suite passes: 15 tests, plus 4 separately run pinned-binary process tests. The [immutable v5 evidence packet](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v5.json) has SHA256 `8119f506ac0436efa2752dbd4ac94006de088c34089c45e2c2dd6f43720b9608`. No shared files were edited.

### 2026-10-08T17:02
The A1 gap is closed in [recovery.rs](~/code/work-engine/rust/crates/lifecycle-admin/tests/recovery.rs). After reopen, a failed effect retains unresolved settlement, while a separate native process exit does not imply settlement or permit replay. The focused recovery tests pass (3/3); the full S7 suite was not rerun, as requested.

The [immutable v6 evidence packet](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v6.json) has SHA256 `7c28b30db44475d84abfbdfa0e44feb8dcc270359adb13f51cd666dd500f786c`.

### 2026-10-08T17:28
The S7 Clippy fix is complete. All-target Clippy passes with `-D warnings`, and the affected transfer and crash tests pass (4/4). The [immutable v7 receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v7.json) has SHA256 `2b4fe1a36ebb9f7baa1c2860410aa0816c92fe6939e96d3b68d968af7591e28c`.

### 2026-10-08T18:16
S7’s Candidate1 F1–F4 regressions pass in the live worktree: 11 focused tests, admin all-targets Clippy, and formatting. The immutable [v8 component receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v8.json) has SHA256 `b363fc9429f6e200694a709bec57a9ffe428b32427bac6bd99967c9ac371a3d3`.

The integration lead owns the remaining composite gate and reviewer continuation.

### 2026-10-08T18:57
Candidate2’s S7-C1-F4 and S7-C2-F5 regressions were captured failing, then passed after the shared fixes. The bounded admin suite passed 13/13; Clippy and formatting passed.

The immutable [v9 receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v9.json) has SHA256 `75fe14711e1ff7244a1be74ad9adde4f084532b25946d383c174c6af1817d2e9`. Reviewer re-evaluation and the composite gate remain with the integration lead and root.

### 2026-10-08T18:59
The pre-copy-marker crash test passes alongside the two existing partial-adoption tests (3/3). Admin all-targets Clippy passes.

The immutable [v10 receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v10.json) has SHA256 `a2c135f9fabaf7d0ed53fa99658bf5aad7b6cc05617d37bffc46e43cb2daeaf0`. Composite and reviewer gates remain with the integration lead and root.

### 2026-10-08T19:24
Candidate3’s S7-C2-F5 regression now passes: rejected and unavailable retries preserve the adopted destination, and an exact retry completes the same physical adoption. The affected tests pass 8/8; admin Clippy and formatting pass.

The immutable [v11 receipt](~/.local/state/work-engine/rust-context-lifecycle-s67-20261008/s7/component-manifest.v11.json) has SHA256 `615cae170074d461b2748f4d28a19c52d35ce342ee139a7afa32b6d671a3834b`. Reviewer re-evaluation and the composite gate remain with the integration lead and root.


---

## S8 — retained workflow (in progress, not accepted)

Admission contract v1 then 10 revisions within about two hours. Opening of each revision:


### S8 admission contract v1
#### S8 admitted implementation contract (2026-10-08)
Status: frozen for engineering implementation, subject to the approved plan and supervisor phase control. This record specializes the human-authorized `docs/rust-context-lifecycle-s8-implementation-plan.md` (SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7`); it does not authorize live provider entry, migration, activation, publication, or S9/S10. Source baseline is `c3cfd7ec191a9cc06c718444d8855ae919e5d6ef`; published S67 baseline includes `d6ced89bfe6be964a095bc88cdac82cd69457085`.
#### Owner and permanent consumer


### S8 admission contract v2
#### S8 admitted implementation contract v2 (2026-10-08)
Status: frozen for engineering implementation, subject to the approved plan and supervisor phase control. This record specializes the human-authorized `docs/rust-context-lifecycle-s8-implementation-plan.md` (SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7`); it does not authorize live provider entry, migration, activation, publication, or S9/S10. Source baseline is `c3cfd7ec191a9cc06c718444d8855ae919e5d6ef`; published S67 baseline includes `d6ced89bfe6be964a095bc88cdac82cd69457085`.
#### Owner and permanent consumer


### S8 admission contract v3
#### S8 bounded implementation route v3 — native attempt lineage
This immutable correction specializes the human-authorized S8 plan (SHA-256 c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7) and admitted v2 contract (SHA-256 151c55e3d81b4dbf094f5d28dad525ad4f33fd8b66fa9d473b05c24ab9d08ab8). All v2 owner, operation, provider, proof, authority, lock-order, scope, and stop boundaries remain. It does not waive any named S8 or legacy test.
The failed representation premise is that one existing S1 `attempts` row can be observed Yielded and later observed Completed under a new native thread. Baseline migration 0001 has `attempts.effect_id UNIQUE` and `one_active_effect_per_input`, while `reduce_observation` fences different provider thread/outcome on the same attempt; migration 0003 has `native_turns.attempt_id UNIQUE`. Those structures cannot express successive native attempts by replaying one queue observation or by creating a second active queue effect. The alternative of treating a yielded attempt as an unobserved native terminal would erase an essential consequence.
The retained logical delivery remains the one lifecycle input/effect/queue custody, FIFO sequence, and task-owner cursor. A new lifecycle-owned `native_delivery_segments` ledger joins each native invocation and distinct native attempt ID to that same queue attempt and immutable input, with ordinal, predecessor invocation, authorized transition and explicit entered/yielded/completed settlement. `native_turns` remains the durable native send-intent/terminal authority for every segment. Segment 0 starts only after the existing FIFO queue claim. A yielded segment requires a previously reserved transition bound to that exact native basis source; its terminal/history and yielded segment commit are durable while queue custody remains Entered. A successor segment gets a new native attempt ID under the newly committed context and current grant, with a predecessor→transition→successor link; it cannot start Q1 or create another queue effect. Final segment completion joins the original queue attempt for the one qualified delivery observation and then permits Q1. Native segment attempt IDs are physical


### S8 admission contract v4
#### S8 bounded implementation route v4 — qualified recorded host effects
This immutable route specializes the human-authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7` and accepted v3 native-attempt contract SHA-256 `3b5617bb00440ee7ac7ef9b23b51a8885ee0e8eddeb2dd7cbf5546c17ae46388`. All owner, grant, FIFO, attempt, proof, model, scope, and stop boundaries remain. It changes the semantic evidence source for one selected controlled-profile predecessor; it does not waive M4, T16, T35, any other named test, or the later live-provider qualification. Reviewers must inspect this change to source/completeness semantics.
#### Observed capability limit and preserved evidence
The selected local Responses provider and pinned Codex app-server profile use `historyMode=legacy` and `thread/read` with `itemsView=full`. In retained early-pressure root `~/code/.work-engine-tmp/s4-20261007/tmp/s8-external-proof-DPJcwE/lifecycle-store/lifecycle.sqlite`, the exact domain turn settled with `history_status=observed_tool_omitted`, one qualified `item/tool/call`, one settled snapshot child, a qualified terminal, and a same-turn history response at transport sequence 23. That response's matching turn contains `userMessage, agentMessage`; it omits the `dynamicToolCall` present as exact qualified `item/started` and `item/completed` raw ingress. The corresponding completed dynamic item has the admitted snapshot call/result. The provider fixture was changed to standard Responses `function_call`/`function_call_output` form and omission persisted. This is an observed selected app-server history capability, not permission to relabel the history as `observed_complete` or to silently repair the fixture. The D0 installed host/tool receipt is separate historical evidence, not the transition proof.
The isolated native profile pins `modelProvider=local_probe`, `approvalPolicy=never`, `sandbox=read-only`, `historyMode=legacy`, no environments, and only `workspace.snapshot_read` plus (S8 only) `task.question_open` and `task.result_publish` dynamic tools. Effective config must read back `web_search=disabled`, `shell_tool=false`, `multi_agent=false`, `code_mode_host=f


### S8 admission contract v5
#### S8 bounded implementation route v5 — two-cycle authority map
This immutable route specializes the authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7` and v4 recorded-effect source route SHA-256 `2afeb81499867f59e5d3b21ce45ce5d7dc61cf9cf5b79702cee09cd497e491b1`. It adds no request-supplied authority, provider loop, proof owner, or waiver. V4's `observed_tool_omitted` limitation and all v1–v3 custody/lock/attempt terms remain.
The trusted lifecycle startup supplies one unique grant per subject, current context, and operation scope. The installed task host reads the current lifecycle subject context/revision and selects that trusted grant for an ordinary `task.submit.v1` (`enqueue_input`) or `task.request_replacement.v1` (`request_replacement`) command. The g0 startup grant references must match their exact initial bindings. Missing, duplicate, wrong-context, wrong-scope, revoked, or stale grants fail at lifecycle admission; host preflight may reject missing/duplicate bindings before owner publication. Once an owner input is published, a lost reply is reconciled by exact command/result readback, not by retrying because the reply is absent. Request bytes never select a grant or context.
The native task profile's initial `semantic` configuration belongs to predecessor context g0. Optional trusted `native.semantic_contexts` entries pair a unique `predecessor_context` with a complete, distinct `SemanticNativeConfig` for each later transition, including capture, compile/candidate/successor session and attempt identities, challenge and the two scoped semantic grants. The lifecycle actor selects the cycle only from its durable current subject context while a transition owns the subject. Missing/duplicate cycles refuse. In the controlled two-cycle scenario, transition A compiles/verifies with g0 semantic grant and tests successor use with g1 grant; transition B compiles/verifies with g1 semantic grant and tests successor use with g2 grant. Native domain attempt and snapshot grants likewise resolve from the current trusted context. Each successor remains a new admitted native attempt in the same logical D0 delivery; three configur


### S8 admission contract v6
#### S8 bounded implementation route v6 — concurrent grant contraction
This immutable engineering correction specializes the authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7` and v5 authority-map route SHA-256 `33e6d653bba08d89dbd3de968140d4d7e4f7c17a8807e627b2a73fe27243b230`. All v1–v5 owner, source, grant, logical-delivery, profile and stop boundaries remain. It does not grant a proof writer, new public command, direct-table client, live provider, or generic lifecycle mutation capability.
The failed premise is that `task_before_owner_commit` could race an ordinary `task.contract_grant.v1` while native tool handling and `contract_grant` requests share one actor loop. The actor executes the task-owner callback synchronously; while held at that barrier it cannot process the queued contraction. Releasing the barrier allows owner commit before the queued command. Command arrival is not a committed invalidation, so that ordering cannot establish T35's required prior-contraction-wins branch. Merely making the proof barrier asynchronous would not make the synchronous owner commit interruptible. Moving all actor, native and tool custody into shared asynchronous tasks would be a broader refactor with more admission and recovery surfaces than this grant-only lane.
The one owning lifecycle-service incarnation now derives a bounded grant-control connection from its already-held writer-lock file descriptor and exact root/database identities. The handle exposes only the existing grant contraction CAS and exact readback; it cannot admit commands, effects, native entries or arbitrary SQL. The same authenticated Unix socket verbs `contract_grant` and `grant_result`, peer UID, command validation, canonical command bytes, durable `grant_contractions` row, revision and reply mapping are shared with the actor. The installed host continues to issue ordinary `task.contract_grant.v1`; no request chooses a separate path. Only this S8 profile constructs the auxiliary handle; S7 profiles retain their prior single actor path.
Every S8 contraction runs under the task owner's `BEGIN IMMEDIATE` currentness fence, then the lifecycle grant transaction: 


### S8 admission contract v7
#### S8 bounded implementation route v7 — readback after killed native owner
This engineering correction specializes the authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7` and accepted v1–v6 contracts. It grants no replay, native-session adoption, provider entry, generic writer, or new public mutation verb.
The failed premise is that a fresh host incarnation can reopen the same controlled native directory and session to expose a task-owner commit after SIGKILL. Preserved diagnostic root `~/code/.work-engine-tmp/s4-20261007/tmp/s8-external-proof-Wg8tTk` shows a committed `task.result.publish.v1` in the canonical task owner while the lifecycle child and native turn remain `entered_uncertain`; the old native session is bound to the killed incarnation and has no service-close receipt. `prepare_native_launch` correctly refuses the existing native user/codex/work directories before the socket is bound. Bypassing that check and calling `reserve_native_session` would conflict with the prior incarnation. Neither refusal is permission to recreate the native turn.
For the retained S8 profile only, startup first reads exact lifecycle recovery and the configured native session. If a prior native session or entered/uncertain native attempt belongs to an earlier incarnation, it enters **recovery-only observation mode**: it opens the owning lifecycle store and ordinary host socket but does not prepare native directories, reserve/bind a session, spawn a native child, send provider bytes, schedule queue work, or advance a transition. New task/lifecycle mutation commands are refused before owner mutation; exact existing command/result readback and task observation remain available. The old native attempt, send-intent, child and unsafe-close facts remain visible and unresolved. This is not adoption or a clean service close.
The recovery-only service may reconcile an already-entered `native_task_operations` row from the canonical task owner's exact operation ID and durable commit bytes. It verifies that the owner result is for that immutable operation and records only the corresponding child settlement. A missing owner row or mismatched bytes 


### S8 admission contract v8
#### S8 bounded engineering route v8 — grant drain before service close
This corrects only the v6 auxiliary grant-lane shutdown ordering under the authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7` and preserves the v7 recovery-only route SHA-256 `cfbe11a0b6cedcc98bf4a2c2e2bd284d921f3b99dbb326622fce1e2d64ab0276`.
The failed premise was that a closed grant admission gate followed by actor Shutdown was enough to make the service-close row final. The socket previously waited for `GrantLane.in_flight` only *after* the actor had recorded service close. An already-admitted task→lifecycle fence can commit after the gate closes. Its result would then postdate the supposedly clean close. A gate-close marker or a blocked request before lane entry did not prove drain.
On SIGTERM the socket closes both launch gates and stops listener admission, then waits up to the existing 60-second bound for the independent grant lane's in-flight count to reach zero **before** sending actor Shutdown. Only then may the actor record service close. If the drain times out, no clean service-close result is manufactured; the process reports failure and retains the prior durable grant/owner facts for readback. New work after gate closure is refused by the existing public authenticated command path; a request that entered the material fence before closure may truthfully commit, and its result precedes service close. The task→lifecycle lock order and sole lifecycle writer ownership are unchanged.
A controlled-only optional cut inside the existing material fence, after the open check and before grant commit, exposes the real in-flight race without advancing state for proof. The installed host fixture must hold an ordinary owner-authorized contraction there, deliver SIGTERM, observe gate closure, release the cut, and check the exact grant revision/result and service-close order through read-only owner/lifecycle evidence. A fresh authenticated request on an already connected public lifecycle socket, where supported, must yield unavailable with no later grant row; if connection closure prevents a typed reply, record that transport outcome and qualify the narrowe


### S8 admission contract v9
#### S8 bounded engineering route v9 — idle native runner shutdown
This specializes the authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7` and v8 grant-drain route SHA-256 `149639c1a0e2777235d543cd28de3a2dd89a933ad3888dd3e93f971986fba046`. It grants no native replay, session adoption, new provider entry, or proof-only writer.
The first v8 installed drain attempt stopped without a receipt because `service_closes.unsafe_at_close` was 1, while the fixture expected 0. The disposable root was removed on panic, so the exact database cause for that attempt is unobserved. Source inspection identifies a concrete close path: after an ordinary completed D0, the S8 native runner waits on its task-work receiver for the next FIFO item; actor shutdown closes the native gate but retains the sender while waiting seven seconds for the runner. The receiver cannot finish, so shutdown classifies cleanup as unobserved and unsafe. The test had observed a settled delivery/segment before signal, which makes this path applicable; the failed attempt remains diagnostic until a preserved rerun identifies its actual close row and native exit.
At actor shutdown, close the native gate and drop the actor-owned task-work sender before awaiting the runner. The runner rejects any buffered work it receives after the gate closes. It then reaches the existing `NativeHarness::terminate_and_reap` path, reports exact exit through the existing Done acknowledgment, and allows the actor to record a truthful close. A still-entered or unobserved native turn, failed reap, unresolved child, or missed acknowledgment remains unsafe. The v8 grant lane still drains before actor shutdown and close; this change does not alter task→lifecycle lock ordering or grant admission.
The next installed cut must preserve the disposable root on assertion failure, pin final source/build/features and v7–v9 contracts, replay only on a new disposable task, and verify: completed D0 before signal; held grant fence drains and commits before service close; native runner exits/reaps with an observed outcome; no late grant mutation; and `unsafe_at_close` is 0 only if native custody is fully settled


### S8 admission contract v10
#### S8 bounded engineering route v10 — native spawn custody cuts
This provides controlled observation for T22 under the authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7`, preserving v7 recovery-only and v9 shutdown/drain semantics. It does not grant a proof-only process launcher, provider replay, session adoption, or native mutation outside the installed owner.
The remaining T22 premise is unproven: positive installed runs show a durable native send intent and ordinary owned exit, but `kill_on_drop(true)` cannot by itself establish child death if the host is SIGKILLed. The previous crash receipts establish zero children started by a recovery-only successor, not that every original child exited. Likewise, the gap between launch-gate closure and native spawn and the gap after spawn before the runner registers its handle have not been observed.
Two opt-in `native-sim-proof` cuts are placed in the real lifecycle-owned native runner. `native_before_spawn` holds after startup allocation and before the pinned `NativeHarness::spawn`; release rechecks the shared native launch gate so a signal that committed gate closure cannot start a child. `native_after_spawn_before_registration` holds after the pinned binary has spawned but before the runner begins protocol entry; it writes and syncs the actual child PID into the private controlled barrier root before reporting reach. Release after gate closure skips native protocol entry and follows the existing terminate/reap and Done acknowledgment path. Cut errors fail closed. The marker is process evidence only, never entry authority or a lifecycle mutation.
An installed proof must bind the original host and actual pinned child PID plus Linux starttime; distinguish pre-spawn zero child from post-spawn owned child. For graceful signal, verify gate closure, no native protocol entry, exact child exit/reap and durable process outcome before service close. For host SIGKILL, observe whether the exact PID/starttime disappears within a bounded allowance and preserve absence of a clean close/unsafe prior session. If the child survives, the test must fail and clean up only the identity-matched cont


### S8 admission contract v11
#### S8 bounded engineering route v11 — distinguish live child from unreaped exit
This refines only the T22 observation in v10 SHA-256 `bc961a7c182688feedc88d52a543787b237685f3336eb11ce415bf2c8fb033ca` under the authorized S8 plan SHA-256 `c8b81575e8f29627664e5a07e76df5333dffc6d50cd802547dcf88f374be07b7`. It does not change native process or owner admission behavior and does not waive escaped-child absence.
The first pinned post-spawn SIGKILL cut failed because the exact child PID had not disappeared by five seconds. It has no receipt. Read-only follow-up found `/proc/<pid>` later absent without a cleanup signal; the retained lifecycle store has the original session intent but no native exit, turn or service-close row. The test's `observe_process` reads `/proc/<pid>/exe` to bind executable identity. That read can fail for a zombie even while `/proc/<pid>/stat` remains, so the failure established **bounded PID disappearance failure**, not that a child was still executing. The exact process state and PPID at the five-second boundary were not recorded. No prior result is relabeled.
The next proof cut retains the exact initial PID, Linux starttime and pinned executable SHA, then reads `/proc/<pid>/stat` state, PPID and starttime independently of the executable symlink. It must establish both (a) no identity-matched executing child by the fixed five-second execution allowance and (b) disappearance of that exact PID within a separately fixed 15-second reaping allowance. State `Z`/`X` is evidence of exited execution but not yet of PID disappearance. Any active identity-matched state at five seconds, a zombie still present at the reaping deadline, unreadable/ambiguous state, or changed identity without a witnessed exit fails closed. Active survivor cleanup may signal only the exact controlled child after a fresh starttime/executable match; a zombie is never called an executing escape. Preserve source/build/contract and the diagnostic root on failure.
A host SIGKILL still cannot produce an original-incarnation process-exit or clean-close receipt. Recovery-only readback preserves that unknown owner state and cannot infer adoption or replay. Passing the two process allowances
