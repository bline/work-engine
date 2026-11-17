# M4 tool registry evidence: one bounded slice plan (S1)

2026-11-16, revision 1. **This is a planning candidate for root review. It does not
implement anything, select a native build, install anything, run inference or grant
authority.** Packet:
`/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/001-plan/PACKET.md`.
Root and collector: Pi session `01a1e56d-7ceb-71fc-9c46-c9fcd9618234`. Planner: Pi
session `01a1e575-4d4f-72f6-8f77-07aa21d0f515`. Shared branch HEAD when read:
`9d24449c`.

**Recommendation.** Extend the same private native producer that the accepted input
capture unit patched. In strict mode it should durably publish, before sampling, one
per-step record of the finalized tool router. Each tool dispatch should then cite the
router snapshot it ran against. A Work Engine consumer in `lifecycle-codex` checks
those records against the tool set Work Engine itself requested, and against the
`tools` field in the strict request bytes that were already captured. The existing
`eligible` flag carries the result to the task-host preparation fence, which already
refuses ineligible input. This needs one new private native build. It does not change
production startup refusal.

## 1. Claim and consumer

**Claim.** This claim covers one controlled native turn of the private build in strict
mode. For every step, the native producer published a checked record of the finalized
tool router before that router was used for sampling or dispatch. Each such record
names exactly the expected tool set, with no extra registered, hosted or cached
runtime. Every tool dispatch in the turn, whether direct or nested through code mode,
cites a snapshot whose record matched. The model-visible tool specs in each snapshot
are also the `tools` content of the strict request bytes sent for that step. If any
of these facts is missing, mismatched or unpublished, the turn's native input is
ineligible.

The claim does not make execution impossible. It ensures that a turn whose executable
tools differed from the reviewed registry cannot be used. Pre-execution prevention is
a separate question, listed under open decisions.

**Expected tool set.** The plan settles the reserved semantics so that the builder does
not have to. The expected set is exact. It contains the dynamic tools that
`lifecycle-service/src/operations/mod.rs` sends in `thread/start` (`dynamicTools`: the
`workspace` and `task` namespaces and their functions), each with the schema digest
and exposure Work Engine requested. Because the selected policy sets
`code_mode_host: true`, it also contains the code-mode `exec`/`wait` pair, and the
nested code-mode names must be exactly those dynamic tools. Any other registered tool
is a mismatch: a native core utility, an MCP, plugin or extension executor, a hosted
spec, a cached runtime, or a `tool_search` entry that Work Engine did not request.
Missing or colliding tools are also mismatches.

**If the real native build registers additional tools under the selected
configuration, the builder must stop and report the observed set to root.** The
builder must not widen the expected set. Root or the operator then chooses between
disabling those tools in configuration and approving them by name.

A dispatch that the native registry rejects because the name is unknown is consistent
with the claim, provided the rejection is recorded against a matching snapshot and no
handler ran. The registry already rejects absent names (`registry.rs` 615–672; the
trace starts at 649, before lookup).

**Consumer.** The checked evidence is a new module, `lifecycle-codex/src/tool_registry.rs`.
It is called from `compiled_input::acquire_checked` for the same thread and turn. It
reads a strict trace file `strict-tool-registry.jsonl` beside
`strict-transport-input.jsonl` and returns a `CheckedToolRegistry` value or a refusal.
`AcquiredNativeInput::eligible()` becomes false unless that value is present and
matches. Refusal then flows through the existing chain:

1. `actor.rs` (~1531) passes `observation.eligible()` into
   `lifecycle_store::retain_native_input`.
2. `native_causal.rs` (56, 202) refuses qualified causal evidence for ineligible input.
3. The task-host preparation fence in `actor.rs` (~2712–2724) returns
   `StoreError::Rejected` when `!input.eligible`.

The checked snapshot bytes are retained beside the native input so reviewers can read
them later. `native_startup::publish` keeps refusing non-controlled startup. This unit
does not qualify the protection or access profile, so its comment stays true for
production.

## 2. Alternatives considered

| Route | What it gives | Why it was not chosen, or is only partly used |
| --- | --- | --- |
| Thread-start request, config and ACK (current) | Proves what was submitted and accepted | Overnight 003 showed that finalization happens later, in `capture_step_context_inner` → `built_tools` → `build_tool_router`/`finalize_tool_router`. Conversion can drop tools and collisions can occur. Insufficient. |
| Static Work Engine config constraint only (disable shell, MCP, plugins, apps, web search; empty environments) | Already in place and checked by `post_login_policy_readback_refuses_tool_and_sandbox_drift` | It constrains construction but does not read back the result. Core utilities, tool policy from extension data and code-mode or cached machinery are not identified by config (003, `spec_plan.rs` 1209–1455). It stays a precondition, not the evidence. |
| Existing ordinary rollout-trace dispatch events (`tool_dispatch.rs`, `tool_dispatch_trace.rs`), already consumed by `causal.rs` | Direct and code-mode call identity per turn | Writes are best effort, and the events carry no router identity or registry contents. Reused as the dispatch locus, but strict rows are needed. |
| Registration metrics (`router.rs` 398–435) | Counts of exposures | Counts do not give the exact registry or show what was excluded (003). |
| **Per-step strict router snapshot plus dispatch citation through the rollout-trace owner (chosen)** | Exact finalized registry per step, bound to dispatch and to the captured request bytes | Same owner, writer and strict publication pattern as the accepted input capture unit. No new protocol or service. |
| Dispatch-time allowlist enforcement in the native router | Prevents unlisted execution | Changes native trust semantics: Work Engine policy would be enforced inside a modified native TCB. That is larger, needs an owner decision, and still needs evidence to be reviewable. Deferred. |
| Narrower claim (dynamic tools only, or direct dispatch only) | Smaller patch | Leaves extra core, hosted or cached tools and code-mode nesting unproven, which is exactly the 003 gap. Not sufficient for A5 refusal. |

The chosen route is enough because the consumer needs three things, each from
native-produced bytes: the registry the router actually finalized, the snapshot every
executed call ran against, and the advertisement actually sent. It is smaller than
enforcement because it adds records at two existing boundaries and changes no
dispatch policy.

## 3. Ownership

One builder owns one new private source tree and its processes:
`/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/002-build/producer/`.
It is a copy of the accepted 008 source (manifest `019788ae…3263`). The held 004/008
tree stays immutable.

| External native path (relative to `codex-rs/`) | Responsibility |
| --- | --- |
| `core/src/session/mod.rs` (`capture_step_context_inner`, about 3863–4012) and `core/src/session/turn.rs` (`built_tools`, 1892–1971) | After finalization, publish the strict snapshot before the `StepContext` is returned. In strict mode, a publication failure fails the step and no router is used. |
| `core/src/tools/spec_plan.rs` (`build_tool_router` 121–183, `finalize_tool_router` 368–545, code-mode 1012–1105) | Expose the finalized facts read-only to the snapshot builder: registered names, kind, exposure, schema digest, nested code-mode names, cached runtimes, hosted specs, collisions and dropped dynamic entries. No policy change. |
| `core/src/tools/router.rs`, `core/src/tools/registry.rs` (`dispatch_any_with_state` 615–672), `core/src/tools/tool_dispatch_trace.rs`, `core/src/tools/code_mode/{mod,delegate}.rs` | Give the router a snapshot identity. Every dispatch, including an unknown-name rejection and nested or surviving-cell calls, writes a strict row citing it before the handler runs. Publication failure refuses the dispatch. |
| `rollout-trace/src/{tool_dispatch,writer,payload,lib}.rs` and a new `rollout-trace/src/tool_registry.rs` if cleaner | Strict snapshot and dispatch row kinds with checked publication, reusing the strict writer that 008 repaired. |
| `qualification/` harness, a copy of the 008 harness | Fixture modes for section 4, and causal oracles. |

| Work Engine path | Responsibility |
| --- | --- |
| `rust/crates/lifecycle-codex/src/tool_registry.rs` (new) and `src/lib.rs` | Strict parse, using duplicate-key rejection via `lifecycle_wire::parse_strict_json` and exact field sets. Join snapshot to expected set, dispatch rows and request `tools`. |
| `rust/crates/lifecycle-codex/src/compiled_input.rs` | Call the consumer from the acquire path and fold it into `eligible()`. Carry the checked snapshot bytes. |
| `rust/crates/lifecycle-service/src/operations/mod.rs` | Expose the exact requested `dynamicTools` value, or a function that derives it, so the consumer compares against the same bytes that were sent. No change to policy content. |
| `rust/crates/lifecycle-service/src/actor.rs`, `rust/crates/lifecycle-store/src/native_input.rs`, plus one migration `0012_native_tool_registry.sql` | Retain the checked snapshot bytes immutably with the native input record, following the 0010 pattern. No change to the fence logic itself. |
| `rust/crates/lifecycle-service/tests/native_input_capture.rs` | Controlled native qualification modes that pass through the real consumer and fence. |

**Out of scope:** `native_startup::publish` and production startup; protection and
access profile; helper and proxy; production pins and profile selection; shared
`Cargo.toml` changes beyond what the new module requires; new crates; Codex dispatch
policy or enforcement; MCP, plugin or extension enablement; installation; live provider
use; authority; real H2.

## 4. Decisive joined signal and negative controls

**Decisive signal.** A built app-server runs a controlled turn under the 008 Docker
confinement against the scripted peer. The turn includes a direct `task.result_publish`
call and a nested call through code-mode `exec`. For each step it yields a published
strict snapshot equal to the expected set. Every strict dispatch row cites that
snapshot. The decoded strict request bytes for the step carry a `tools` value equal to
the snapshot's model-visible specs. The Work Engine consumer marks the retained input
eligible, and the task-host fence admits controlled preparation.

| Control | Boundary consequence | Expected |
| --- | --- | --- |
| Positive (above), HTTP plus code mode | Full join | Eligible; fence admits |
| Extra registered tool (fixture config enables one native utility that the selected config leaves off, or the test thread start adds one extra dynamic tool) | Registry contents | Ineligible; fence rejects; startup ACK still succeeded |
| Dynamic tool dropped by schema conversion, or a name collision | Finalization differs from submission | Ineligible, with the exact reason |
| Model calls an unregistered name | Native rejection is recorded | Eligible only if the rejection row cites a matching snapshot and no handler ran |
| Simulated snapshot publication failure on step 2 (same one-way fault style as 008) | Strict fail-closed | Step fails; no dispatch on that router; ineligible |
| Code-mode cell dispatching after its turn's terminal, or citing an unknown or stale snapshot | Currentness | Ineligible |
| Work Engine unit controls: tampered snapshot, duplicate keys, an orphan dispatch row, request `tools` ≠ snapshot specs, missing file | Consumer hostility | Refused |

This is a set of distinct boundary consequences, not a field matrix. Hosted tools need
no native dispatch control here. The selected config disables web search, and a
hosted spec appearing in the snapshot or in the request `tools` is already a mismatch.

## 5. Reuse and build

- **Source:** official Codex `83086188ae0614bd882818ff1aa23f3194962012`, as patched and
  accepted at 008 (`NATIVE-ROOT-ACCEPTANCE.json`, sha256 `5e426ad0…173c`). The new
  patch is layered on top, so strict input capture and registry evidence share one
  binary. The request `tools` join depends on that.
- **Toolchain and build:** the Rust 1.95 direct binaries already verified, with a
  private `CARGO_HOME`, rustup directory, empty HOME and target, default app-server
  features and `-j4`, all under `custody.py run`. Earlier builds took 1032 s cold and
  521 s warm. Reuse the vendored lock. The rollout-trace `sha2` amendment already
  exists, and no new external crate is expected. Any lock delta needs root approval,
  as at 004.
- **Confinement:** the existing Docker image `4b150b48…`, with network none, read-only
  root and `/usr`, user 1000, `--cap-drop ALL`, no-new-privileges, empty homes and no
  credentials or sockets, and native processes waited after their terminal. The
  bwrap, netadmin and `unshare` routes failed with EPERM at 004. Do not retry them.
- **Custody and freeze:** `custody.py freeze` for the source, harness and composition.
  Use a `COLLECTION.json` with zero leftovers, as at 008. Keep failed runs immutable.
  Keep the known 6 TLS baseline failures distinct.
- **A new native build is required**, because core session and tools code changes.
  Re-run rollout-trace (65 tests at 008), API (190) and the selected core tests. One
  strict-capture positive must still pass to show the earlier claim was not
  regressed.

## 6. Validation and review

The builder runs the following:

1. Native focused tests for the snapshot builder, strict publication failure and
   dispatch citation.
2. Rollout-trace, API and selected core suites.
3. The patched app-server build.
4. The joined fixtures from section 4 plus one 008 strict-input positive.
5. Work Engine `cargo test -p lifecycle-codex -p lifecycle-store -p lifecycle-service`
   for the affected tests, including `native_input_capture` controlled modes, and
   `cargo fmt --check` on the changed files.

All of these run under custody. The builder commits the Work Engine change only on a
private slice-checkpoint ref; it does not commit to the shared branch.

A separate reviewer (Claude through Pi, same provider unless the operator approves an
outside reader) must check:

- that the snapshot is taken after `finalize_tool_router` and before any sampling or
  dispatch on that router, with fail-closed behavior;
- that every dispatch path cites a snapshot: direct, parallel, code-mode nested, cached
  runtime and surviving cell;
- that the expected set is derived from the bytes actually sent in `thread/start`, not
  from a second hand-written list;
- that request `tools` equality uses the strict request bytes;
- that ineligibility reaches the fence with no bypass;
- that the native diff introduces no change to dispatch policy;
- the custody, freeze and no-leftover record.

## 7. Budget and stop conditions

Figures are API-equivalent dollars against about $97 remaining.

| Part | Expected | Cap |
| --- | --- | --- |
| Builder (native patch, build, fixtures, Work Engine consumer) | 18–24 | 30 |
| Separate review | 5–7 | 9 |
| Repair and recheck | 4–6 | 8 |
| Root collection and acceptance | 2–3 | 4 |
| **Total** | **29–40** | **51** |

The builder stops and reports in any of these cases:

- the real native build registers tools outside the expected set under the selected
  configuration;
- a snapshot cannot be taken before sampling without changing dispatch policy;
- some dispatch path (cached runtime or extension) cannot cite a snapshot within the
  listed paths;
- a lock delta or new crate is needed;
- Docker confinement or the build is unavailable;
- the builder reaches about $30.

Reaching a cap is not acceptance. Unfinished state and processes must be preserved.

## 8. What remains open after this unit

- **Pre-execution prevention** versus refusal of use after the fact. This is an
  operator or owner decision, and is needed only if A5 requires that an unlisted tool
  never runs.
- **Production access and protection profile**, the protected set and descendants, and
  sockets, proxy and helper.
- **Production selection** of any modified native build, and source-to-binary
  attestation. `native_startup::publish` stays Unavailable.
- **Routed, HTTPS, proxy and HTTP2 transport** for the strict request join, unchanged
  from 008.
- **Actual research, execution and inference authority**, real H2, M5 and acceptance of
  the whole original M4.
- **Rehydration, substitution and ticket experiments** listed in the M4 completion
  table.

No different unit needs to come first. The protection profile is independent and
remains open whether or not this unit runs.

## Evidence used

M4 completion plan (this repository); overnight 003 `RESULT.md`; m4-completion 004 and
008 `RESULT.md`, `NATIVE-ROOT-ACCEPTANCE.json` and `CLOSURE.json`. Direct reads of the
held producer tree under `004-native-input-capture/producer/codex-rs`: `tools/registry.rs`,
`router.rs`, `call_trace.rs`, `tool_dispatch_trace.rs` and
`rollout-trace/src/tool_dispatch.rs`. Work Engine sources: `native_startup.rs`,
`isolation.rs`, `compiled_input.rs`, `causal.rs`, `actor.rs`, `native_causal.rs`,
`native_input.rs` and `operations/mod.rs`. Line numbers outside the files read here
come from 003 and were not re-verified. Not read: `PLAN-R2.md` and the ingestion plan
r2, because the consumer chain was confirmed directly in the merged source. No build,
test or native run took place.
