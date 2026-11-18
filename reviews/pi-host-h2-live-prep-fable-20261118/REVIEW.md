# Review: H2 live preparation (owner 5923), `git diff bf64a1e3 546a24a9`

Worktree `wt-pi-host-h2-live-prep-20261118`, branch `pi-host-h2-live-prep-20261118`, one commit `546a24a9`.
Read-only adversarial review; no repository file was modified. No live inference, no Docker/Q tests, `WORK_ENGINE_HOSTED_CREDENTIALS` never set.
Builds used `CARGO_TARGET_DIR=/home/bline/.cache/fable-h2prep-target`.

## Verdict: accept with changes

Nothing in the diff makes the first live run unsafe. The two P2 items are
test-only (one dropped offline assertion; one marker selection in the
post-run delivery check that can fail for the wrong reason on realistic live
data). The P3 items are record/precision notes.

## Findings

### P2-1 — Delivery check: the changed-read-content marker can be picked from a prompt occurrence, so the control fails for the wrong reason (not vacuously)
- `rust/crates/work-engine-task-host/src/research_delivery.rs:1707-1719` (predicate), `:1722-1745` (flip and `verify(v).err().unwrap()`).
- Evidence (reading): the predicate requires the 16-byte window to occur once in the projection and `bodies.iter().any(|b| count(b, w) == 1)`. `bodies[0]` is request-1's body, which precedes any `snapshot_read` result and contains only the system prompt and the task text. If the window also occurs in the task text (the arm's `facts`/objective are taken from the card's decision text, and the search starts at the middle of the projection, i.e. inside `card.json`), then every post-read body has count 2 and `bodies[0]` has count 1, which satisfies `any(...)`. The flip then hits the prompt copy in every body (`position()` returns the first occurrence, and the prompt precedes the tool results under the continuity rule), the projection is unchanged, `verify` returns `Ok`, and `.err().unwrap()` panics. The outcome is a spurious test failure on the live run's retained data, not a vacuous pass; the final `assert_eq!(refusals["changed-read-content"], ...)` at `:1758-1761` also guards the reason. No inference is wasted, but the first live evaluation would be misleading until fixed.
- Smallest fix: add `&& count(&bodies[0], w) == 0` to the predicate at `:1717` (the first body is the shared prefix of every body and contains no read content), and record `marker_offset` as now.

### P2-2 — Offline refactor dropped one previous assertion (`capture_host` / no `capture_codex`)
- Old `tests/pi_host_join.rs:467` (`git show bf64a1e3:...`): `assert!(kinds.contains(&"capture_host") && !kinds.contains(&"capture_codex"));`. New file: no occurrence of `capture_host` outside the kind mapping at `:643-644` (`grep -n capture_host`). The live test's doc comment (`:306-309`) and the record ("keeps the eligibility, settlement, no-container and failure assertions") describe the refactor as keeping the offline assertions except the read count; this one is gone from both the offline and the live path.
- Smallest fix: in `Run::assert_closed` (`:547`) add `assert!(self.kinds.contains(&"capture_host") && !self.kinds.contains(&"capture_codex"), "{:?}", self.kinds);`.

### P3-1 — F4/protocol: the request-level `not_dispatched` (subscription pre-dispatch refusal) has no attempt entry, so `attempts` omits it and the capture's reason does not name it
- `rust/crates/work-engine-manual-host/src/pi_coding_hosted.rs:985-1005`: when the subscription route gets a body that is not a Claude Code request, the Host sets `r.not_dispatched = Some("subscription_request_not_claude_code")` and `settled_microusd = Some(0)` with `r.attempts` empty (no `reserve_attempt`). `pi_host_serve.rs:960-963` then pushes only `"request {n} has no attempt"`; `attempt_records` (`:967`) gets nothing for that request, and `r["not_dispatched"]` is never read. The protocol sentence (`PI_HOST_PROTOCOL.md:110-112`, "every attempt ... including one never dispatched") is true only for reserved-then-not-dispatched attempts (`pi_coding_hosted.rs:1093`, `:1200`, which do create attempt records with `route`/`payer`). On the first live run this is exactly the path that fires if `claude_code_request` (`pi_coding_hosted.rs:463-475`) does not recognise the live body, and the capture would say only "has no attempt".
- Smallest fix: at `pi_host_serve.rs:962` include the reason: `format!("request {n} has no attempt{}", r["not_dispatched"].as_str().map(|s| format!(" ({s})")).unwrap_or_default())`; optionally push `{"request":n,"attempt":0,"disposition":"not_dispatched","route":null,"payer":null}` so the list is complete. The protocol wording can stay.

### P3-2 — M1 attribution: the count is job-cumulative and sampled at frame times; a stray refusal between the history frame and the next provider request lands on the next turn
- `pi_host_serve.rs:539-553` keeps one `forwarder_refused` for the job (never reset per turn), and `:935-939` turns any non-zero value into the reason. A refusal in turn 1 fails turn 1 and (if the thread continued) every later turn with the job-wide count in the message. This is fail-closed, and after a failed turn `thread_failed` ends the thread anyway (`:681`, `:713`), so only the wording is imprecise.
- The receiver's Claude Code process persists across requests and turns (pinned `pi-claude-subscription/src/provider.ts:37`: `idleMs: 20 * 60_000`, `maxSessions: 3`; "a pooled session parked on tool calls waits for pi's results" `:485`). A request it makes after the `coding_history` frame is counted (`pi-coding.mjs:128`) and reported on the next `coding_provider_request` frame (`:312`) or the final `counts` (`:459`, checked by `pi_coding_analysis.rs:457-475`), so the job still cannot complete; the already-settled turn is unaffected because its answer preceded the refusal. Record this window in the record's M1 row; no code change needed for the live run.

### P3-3 — R1: a failed record write replaces the finish error and the stderr record
- `pi_host_serve.rs:1227-1230`: `serve_record(...)?` runs before `finished?`. `create_only` (`:848`) fails with `AlreadyExists` if `pi-host-serve-record.json` is already there (e.g. a replayed launch into a used state directory, where admission refuses with "state directory must be fresh", `pi_docker.rs:369-376`). The bin then prints the I/O error and exits 1 (`bin/work-engine-pi-host.rs:21-27`), and neither the run error nor the attempt log reaches stderr. Outcomes are not changed (the receipt and lifecycle settlement are already durable, and the runner does not read the Host's exit status: `lifecycle-service/src/operations/pi_host.rs` has no `wait`/`status` handling, only `Command::new` at `:609`), but the error is masked.
- Smallest fix: in `serve_record`, on write failure set `record["record_error"] = json!(e.to_string())` and return `Ok(record)` instead of `?`.

### P3-4 — `cargo fmt --check -p work-engine-task-host` is not clean (pre-existing)
- Diff reported in `work-engine-task-host/src/observation.rs:326`, a file this diff does not touch (`git diff bf64a1e3 546a24a9 --stat -- .../observation.rs` is empty; last touched by `0e467aa1`). The record's fmt claim is scoped to the two crates of the earlier section; the H2 section makes no fmt claim. Note only.

## Answers to the questions, with evidence

1. **M1 correctness / fail-closed.**
   - Receiver: `reportRefused = claudeSdk && relay` (`pi-coding.mjs:127`; `relay` at `:25`, `claudeSdk` at `:37`). The count rides on every `coding_provider_request` (`:312`) and `coding_history` (`:456`); `forwardRefuse` increments it on every local answer, including 413/404/503/415 and the "host relay failed" 502 (`:128`, `:132`, `:136-140`, `:153`).
   - Host gate: `forwarder_count` (`pi_host_serve.rs:539-553`) is called in `provider()` after the ordering check and before `provider_calls += 1` (`:556-561`) and in `history()` before `finish_turn` (`:580-590`). Absent or non-u64 → `refused`, a decrease → `refused`; on a non-`claude-sdk` provider the field is refused. A gate `Err` propagates through `pi_coding.rs:902` (`.frame(&frame)?`) and ends the Host job; `Controller::finish` (`:296-320`) then reports an active turn as failed ("Host job failed: ..."), so a frame without the field cannot settle a turn.
   - Timing: the grant precedes Claude Code's request (`forwardSlot` is set at `pi-coding.mjs:328` before `original(...)` is awaited at `:330`), so the first request of a Pi call cannot be refused for lack of a grant. A refusal during the last request precedes the history frame (emitted after `session.prompt` resolves, `:453-457`), so `:935` makes the turn ineligible and `finish_turn` fails it (`:669-676`, `:704-706`), tested by `forwarder_refusal_in_the_turn_fails_it` (`:1407-1416`). Refusal after the history frame: see P3-2.
   - Monotonic check: the receiver's counter only increments and there is one receiver per relay job (`resets` must be 0, `pi_coding_analysis.rs:473`), so `n >= self.forwarder_refused` is sound; the test at `:1561-1575` covers absent, string, increase, decrease, and the other-provider refusals.
   - Analysis profile: `reportRefused` is false (`relay` false), so neither frame carries the field and the retained `*-rpc-*.stdout` bytes are unchanged; the final `counts.forwarderRefused` was already present on `claude-sdk` (`:459`, unchanged). The Host's `forwarder_count` lives only in the relay gate; the analysis path has no gate call. The record's Open item correctly notes the analysis profile keeps the per-turn gap.

2. **R2/F4 attempts.** `build_capture` lists every entry of `r["attempts"]` for every request of the turn that has a ledger record (`pi_host_serve.rs:958-967`), before the `not_dispatched` `continue` (`:974`), so reserved `not_dispatched`, `refused`, `uncertain`, `pending` and failover attempts all appear with `route`/`payer` (fields set by `reserve_attempt`, `pi_coding_provider.rs:369-400`). Verified by running `capture_lists_every_attempt_and_forwarder_refusals` and by the r10/r9 summaries (request 1: `refused` primary then `completed` secondary). Gap: the request-level `not_dispatched` path (P3-1). Runner copy: `lifecycle-service/src/operations/pi_host.rs:534` adds `attempts` to the metadata inside the existing 128 KiB check (`:538`); size is bounded by `MAX_TURN_SENDS` (64) entries of ~100 bytes. Consumers: the metadata is a JSON value parsed by `lifecycle_wire::parse_strict_json` (`research_delivery.rs:33-35`), no `deny_unknown_fields` struct reads it; `metadata["wire"]` is the only key touched (`:1691`). Test `check_capture` with `attempts` passes (`cargo test -p lifecycle-service --features controlled-proof --lib operations::pi_host`, 3/3).

3. **R1 serve record.** Written once, create-only, after `controller.finish` (`:1227-1252`); admission requires a fresh state directory (`pi_docker.rs:369-376`), so no collision in a normal launch; it does not change the Host receipt, the lifecycle settlement or the controller protocol. Masking on a replayed launch: P3-3. The record contains the receipt, run error and attempt intents (route/payer), no credential material (the receipt carries `credentials_file_sha256` only, as before on stderr). Present in r10 (`served`, 5 attempt-log entries) and q21 (`native/pi-host-state/pi-host-serve-record.json`).

4. **C1(vii) reachability and bypass.** `ProviderPlan::validate` (`pi_coding_provider.rs:530`) is called from the coding profile's `validate` (`pi_coding.rs:461`), which `check_job` calls for every job with a coding profile (`pi_docker.rs:272-274`), i.e. analysis and relay alike; `Host::admit` in `serve()` (`pi_host_serve.rs:1212`) goes through it. `claude_code_pinned` (`:643-653`) requires `Entry::File { sha256 }` equal to the constant; `Entry` is `#[serde(tag = "kind", deny_unknown_fields)]` (`pi_docker.rs:35-40`), so a `symlink` entry, a different digest or an absent key refuses (test `claude_code_binary_entry_is_the_pinned_file`, run, passes). Path variants: manifest keys come from `inventory` (`pi_docker.rs:44-60`, `strip_prefix(root)`, UTF-8, no `./`), and `Tree::verify` (`:98-105`) refuses any tree whose inventory differs from the manifest, so the pinned entry is the file on disk. Checked the pinned runtime: `runtime-manifest.json` (`52ace834…`, 19,014 entries) has the key as `{"kind":"file","sha256":"0dc4f4a9…"}`, and no symlink under `@anthropic-ai`.

5. **Protocol text.** Digest recomputed: `sha256sum PI_HOST_PROTOCOL.md` = `56c00a88…` = `PROTOCOL_DIGEST`; `native-realization` 6/6 including `protocol_digest_is_the_spec_digest`. The eligibility sentence matches `:935-939` plus the frame checks. The `attempts` sentence over-states one case (P3-1).

6. **Live variants.**
   - `pi_host_live_arm` (`tests/pi_host_join.rs:313-363`): credentials `expect` at `:314-315` and file check `:316-319`, arm parse, then the https check `:324-331`, all before `out_root()` (`:341`) and `drive` (`:352`); nothing is written before refusal (matches the record's two refusal runs). The runner's own https gate (`operations/pi_host.rs:177-185`) admits loopback under `controlled-proof`, so the test's assert is the effective guard in that build; it is present.
   - Offline refactor: retained `assert_closed` (no container, no failures, `:547-554`), eligibility/protocol/wire/causal page/`CompletedSettled` in `settled()` (`:699-718`), the read count for the research run (`:528`), and the retained digest (`:742`). Dropped: P2-2.
   - Grants: `config()` takes `expires_wall_ms = deadline * 1000` (`:613`), the same unix deadline given to the job (`:606-612`); `grants.json` is `serde_json::to_vec(&config.grants)` of `Vec<GrantConfig>` (`lib.rs:437-445`, all five fields serialised); `summary.json` records `grants_sha256`.
   - Arm schema: `Arm`/`ArmMember`/`JobParams` are `deny_unknown_fields`; member bytes are checked against the pin (`:337`), then sorted (`:341`). Duplicate or unsorted ids cannot reach the snapshot: `workspace_snapshot_read::prepare` refuses `previous >= member.id` (`:82-87`), so a duplicate id is `Rejected` at composition (before any Host launch). `job.routes` is arbitrary JSON, but the Host's `hosted.validate()` and the route table admit it.
   - Marker soundness: P2-1.

7. **R3 measurement.** `MemorySampler` (`serve.py:238-258`) reads `pi-state.json`'s `container_id`, which the Host sets for the one relay container (`pi_docker.rs:1008`, `pi_coding.rs:790`), the container in which the receiver, its loopback forwarder and Claude Code all run; `memory.peak` is monotonic for the cgroup's life, so the last sample before removal is a lower bound (39 samples, no OOM). `container_frames` (`:260-272`) reads the retained `*-rpc-*.stdout`, i.e. the receiver's own frames, and `contextBytes` is the same quantity the Host bounds (`pi_coding.rs:1568`). I re-read `serve/r10-treatment/summary.json` and the per-type breakdown of its 4,628,136-byte stdout (1.59 MB `coding_provider_request`, 1.06 MB `provider_fetch`, 0.57 MB `coding_history`, ~1.1 MB native events). The relay stdout bound is `OUTPUT_BYTES` = 64 MiB (`pi_docker.rs:671-675` with `analysis_profile()` ⊇ relay, `pi_coding_analysis.rs:230-236`, `pi_coding.rs:56-58`), so a ~19-request live turn (~12 MB by extrapolation) is well inside it; the 8 MiB `LIMIT` applies to stderr and to the non-coding `verify_rpc` path only (`:792`, `:884`).

8. **Other live-run risks considered and found bounded.** History frame ≈ 2× final context (r10: 565,564 B at 280,483 B context): at the 400,000 cap plus a 65,536 B answer that is ~930 KB, under the 1 MiB line; the sizing's expected ~300 KB context leaves ample margin. `thread/read` reply 272,539 B vs `HISTORY_FRAME_LIMIT` 1,000,000. Receiver `relay` gating, `forwardSlot` ordering and the Host-side failover (request 1 primary 429 → secondary, `forwarderRefused` 0) behave as recorded.

## What I ran (all with the separate target dir)

| Command | Result |
| --- | --- |
| `sha256sum PI_HOST_PROTOCOL.md` vs `PROTOCOL_DIGEST` | equal, `56c00a88…` |
| `cargo test -p native-realization` | 6 passed |
| `cargo test -p work-engine-manual-host --lib pi_host_serve` | 10 passed |
| `cargo test -p work-engine-manual-host --lib claude_code` | 3 passed (incl. `claude_code_binary_entry_is_the_pinned_file`) |
| `cargo test -p lifecycle-service --features controlled-proof --lib operations::pi_host` | 3 passed (lib test build compiles `tests/pi_host_join.rs` via `actor.rs:5177`) |
| `cargo test -p work-engine-task-host --features controlled-proof --lib research_delivery` | 6 passed, 3 ignored (both delivery tests compile) |
| `cargo fmt --check` (4 crates) | one pre-existing diff in `work-engine-task-host/src/observation.rs` (P3-4) |
| Read `serve/r10-treatment`, `serve/r9-relay`, `q21-research` state and summaries | numbers match the record; serve records present |
| Pinned runtime manifest check (`python3 -I`) | entry present as file `0dc4f4a9…`, no claude symlinks |

Findings verified by running: digest, all test results, fmt state, r10/r9/q21 figures, manifest entry.
Findings verified by reading: everything cited by `file:line` above, including P2-1, P2-2, P3-1, P3-2, P3-3.

## Sound (checked, no issue)
- M1 receiver gating, frame placement, Host gate ordering, refusal → job end, capture reason, tests.
- Analysis profile frames and recorded bytes unchanged.
- R2 attempts for reserved attempts of every disposition; runner metadata copy within bound; no strict consumer breaks.
- R1 create-only record in a fresh state directory; no outcome change; no secrets added.
- C1(vii) reachable on both profiles; not bypassable by entry kind or path variant; the pinned runtime carries the entry.
- Protocol digest and eligibility sentence.
- Live arm: refuses before any write/inference on missing credentials or non-https routes; grants exact and bound to the job deadline; duplicate/unsorted members rejected at composition; member pins checked.
- R3 cgroup and frame sources; relay stdout bound 64 MiB; observer multi-tool `steps` shape (`pi_hosted_messages_observer.rs:276-292`) refuses mixed `tool`/`tools` steps.
