## Closure review: review 3 remediation (`7c103ad2`..`9b289171`) and the dev vertical's final record

Reviewer: Fable, fresh context, read-only. Nothing was built, run or modified; no credential file was read. The subject was read from `subject.diff` (all 2,747 lines) and the working tree; evidence from the r8, c3 and regression run directories. Line numbers are working-tree lines unless marked "diff".

Shorthand: `hosted.rs` = `rust/crates/work-engine-manual-host/src/pi_coding_hosted.rs`, `https.rs` = `…/pi_coding_https.rs`, `provider.rs` = `…/pi_coding_provider.rs`, `pi_stream.rs`, `pi_docker.rs`, `pi_coding.rs` likewise; `job.rs` = `rust/crates/work-engine-server/src/job.rs`; `prepare.rs` = `…/src/bin/coding-job-prepare.rs`; `README` = `records/dev-vertical-hosted-qualification-20261117/README.md`; `c3` = `~/.local/state/work-engine/dev-vertical-cutover-20261117/c3`.

## 1. Disposition

**The milestone can close with residuals, after two cheap record actions.** The remediation is correct where I read it: no path releases money without a definitive refusal or confirmed non-dispatch, no unsettled attempt authorizes another, the new body policy and credential hardening fail closed, and the M3/M4 changes do not alter any completed-path settlement (section 3 explains why). I found no P0 and no code P1.

Required before closure (record, not code):
1. **Run and record the unit suites at the remediation commit.** The README's "Tests" section still reports runs at `8fa71188`/`cab80a3a`; nothing records that the new or changed tests (M2, R-a, F7 loop, M4 assertion, the F4 helper changes in `tests/coding.rs`, the server fixture changes, the new VM test, the restored Python test) pass at `7c103ad2` or later. Only `coding_job` `server_stop_…` (5/5) is recorded post-remediation. Supervisor should run: `cargo test -p work-engine-manual-host -p pi-pressure-context-host -p work-engine-server`; `node --experimental-vm-modules --test rust/crates/work-engine-manual-host/tests/*.test.mjs`; `python3 -m unittest discover skills/session-events/tests`.
2. **Correct the R-d claim and the stale D4 line** (findings N1, N4): the relay bound is 32 MiB, but the receiver's stdout is bounded at 8 MiB for the whole job and grows per relayed chunk, so the effective output ceiling is much lower than "128k output tokens"; README:293 still says "The 1 MiB response bound".

Residuals that may close with the milestone (all operator-deferred or follow-up, consistent with `docs/dev-vertical-open-items.md`): negative tests for the new M5/F6 refusals (N3), the F3/F7 test cases not ported, F5 dead code, R-c/R-e/R-g, K-c, registration/permission, unexercised live failures, `owner_pending`.

Gates this review still cannot close: this is again a same-provider review (packet states it); owner acceptance; a green broad gate (inherited failures, section 4).

## 2. Status of each review 3 finding

### Fable M1–M6

| Item | Status | Evidence |
| --- | --- | --- |
| **M1** receipt labels | **Closed.** | `pi_docker.rs` diff hunk @1322: `provider: "hosted-routes"`, `provider_attempts` built from `provider_requests` joined to `plan.hosted.routes`, `model: plan.model`, cost label "pinned openai-codex catalog rates … a proxy, not provider billing", `container_provider_identity: "openai-codex"` kept, `hosted_transport` included. In c3: `state/coding-host-result.json:1171-1200` lists four `{request, attempt, route: primary, payer: chatgpt-subscription, endpoint: https:chatgpt-codex, disposition: completed}`; `:462-463`, `:497-499`. |
| **M2** connect-failure test | **Closed.** | `hosted.rs:2048-2097` `connect_failure_is_a_confirmed_non_dispatch_…`: `FakeTransport.unreachable` → asserts `route_unreachable` (`:826` is the production answer), `NotDispatched`, `!gate_entered`, reason "route connection failed", `charged_microusd()==0`, no gate intent, nothing opened, `provider_failed`, no `stream.sse`, and `reserve_attempt` for a second attempt errs. `check.py:60-62` exempts `not_dispatched`; `matrix.json` renamed check present in all 19 scenarios. Not run by me. |
| **M3** unfinished-drain bytes | **Closed.** | `pi_stream.rs:645-651` `into_relay`: when `!drained`, `unforwarded` is taken into `held_dropped`; `held` is always taken. `hosted_close_uncertain` goes through `into_relay` (`hosted.rs:1341`), so `evidence()` emits no `unforwardedSha256` for bytes never kept. No completed-path regression: `unforwarded` is populated only after entering `Draining` (`:729` seed, `:834` accumulate) and `drained` is set only by `drain_done` (`:630`); `finish()` already takes `held` (`:676`), so the second line adds 0 on every finished session. |
| **M4** drain error can still complete | **Closed (conservatively), test residual.** | `provider.rs:157-159`: `terminal_last` now requires `relay.drain_error.is_none()`. `pi_stream.rs:837-840` sets `drain_error` only on `Some(Err(code))`; the time bound (`:841`) and size bound set none, so K-a (`late-close-long`, r8 pass) is unaffected. Residual: the fix drops the chunk's bytes as before and marks Uncertain, rather than the suggested terminator-at-next-read; the only test is a synthetic `Relay` with `drain_error = Some(..)` in `provider.rs` attempt tests (diff @1441), not a `pi_stream` drain-error test. Acceptable. |
| **M5** loopback admitted in live class | **Closed.** | `job.rs:219-229` requires every endpoint to parse to `ChatGptCodex \| OpenAiResponses` (`hosted.rs:128-131`); `prepare.rs:177-185` same; `job.rs:481-487` requires `WORK_ENGINE_HOSTED_CREDENTIALS` as an absolute path in a live profile. With the variable set, the HTTPS transport refuses loopback (`hosted.rs:299`, unchanged). |
| **M6** record inaccuracies | **Closed, one residual.** | README diff: 31 KB largest response; "synthetic invalid" transition with the refusal text; `live_check.py:38` now writes `live-check.json`, present in all five live run dirs (`live-1`, `live-transition-1`, `c1`, `c2-transition`, `c3`); cutover's persisted API key documented (README and `cutover.py:54-55`); `qualify.py` docstring and `coding-qualification.rs:90-91` fixed. Residual: `c1` still has no harness summary (`c2-transition.json`, `c3.json` exist; no `c1.json`); the c1 row rests on its run directory. |

### Fable R-a..R-g

| Item | Status | Evidence |
| --- | --- | --- |
| **R-a** body fields | **Closed as decided.** | `hosted.rs:572-589`: `store == false` required, `background` null/false, `service_tier` null/"auto"/"default", `tools` absent or all `type == "function"`. Test `:2098-2121` covers seven refusals and one acceptance. The real Pi body in c3 (`state/0005-rpc-0.stdout` line 9, `provider_fetch.body`) decodes to `{"model":"gpt-6-luna","store":false,"stream":true,…,"tool_choice":"auto",…}` and passed. The policy also applies to loopback routes; r8 passed with the real Pi container. |
| **R-b** payer provenance | **Closed as recorded (not pinned).** | `hosted.rs:196-198` `identity()`; saved to the journal at bind (`:420-423`); `https.rs:122-144` builds `{transport, credentials_file_sha256, payer_sources}`; c3 `state/pi-state.json:908-910` and the receipt carry it. The admission does not bind the digest (the alternative offered); record and open-items say "recorded", which is accurate. |
| **R-c** price parity | Open by decision; `docs/dev-vertical-open-items.md` row. |
| **R-d** response bound | **Changed, but the claim is not supported end-to-end.** See N1. `pi_stream.rs:19-21` 32 MiB; consumers `:96`, `:139`, `:869`. |
| **R-e** live fragility | Unchanged; README "Not changed" lists it. |
| **R-f** hardening gaps | **Closed.** | `https.rs:64-79` `owner_only` (regular file, size, mode `& 0o077 == 0`, euid); applied to the Pi auth store (`:82`, 1 MiB), the credentials file (`:116`, 64 KiB) and the API key file; account-id charset `[A-Za-z0-9_-]` (`:93-99`). `chatgpt-invalid-for-qualification` still ships; its kind is now visible in `payer_sources`. |
| **R-g** | Unchanged, as recorded. |

### Opus F1–F8

| Item | Status | Evidence |
| --- | --- | --- |
| **F1** Host profile doc | **Closed, minor residual.** | `pi-coding.md:14` retired banner; `:24-47` hosted section (schema fields, the eleven admission keys — they match `Admission` at `provider.rs:382-396` — network-none, mounts, transport, body policy, record link); `:48` heading marks the old lane as history; `:101-103` networking sentence corrected. Residual: later sections ("Catalog proxy budget…", last paragraph of "Opt-in persistent source-file containment") still describe the projected-access flow without a banner. |
| **F2** session-events switch | **Closed per decision; switch left.** | `skills/session-events/tests/test_pi_coding.py:176-190` restores disabled-by-default, switch-without-pins and conflicting-switch cases; they match `session_events.py:2698-2709` (unchanged, digest-pinned). Open-items doc records the follow-up. |
| **F3** receiver VM test | **Partly closed.** | New `tests/pi-coding-fixture.test.mjs` drives the provider-disabled receiver: asserts no fetch/access reads, one `pressure_context` with `idle:true`, `pressure_inserted` with `details.authority == 'observation_only'` (receiver `:351-352`), one `pressure_request` (`:303`), final `provider: 'disabled_fixture'` (`:356`). `session-task-admission/README.md:112-117` updated. Not ported: the delayed-initialization deadline case. |
| **F4** gate accepts live without hosted | **Closed.** | `pi-pressure-context-host/src/coding.rs:355-359` refuses `task.is_some() != hosted.is_some()`. `tests/coding.rs:278-282` defaults a hosted policy for live tests that choose none, so `run_complete(true, …)` callers (`:860`, `:889`) still construct; `:1098-1102` asserts the refusal at construction. Lifecycle-store Sol grammar → documented follow-up. |
| **F5** dead code | **Partly closed.** | Network always `none` in `base` (`pi_coding.rs:438-440`) and `verify` (`:604`); MCP helper refuses the retired schema before `job.verify()`/`PiOwner::open` (`examples/mcp_pi_coding.rs:82-86`); `reasoningEffort`/`payerAccountId` dropped (`pi_coding.rs:1386-1389`). Remaining (all unreachable): receiver `pi-coding.mjs:188` stale comment, `:121`/`:126` `/auth/access.json` reads, `pi_coding.rs:1662` `private-access/access.json` cleanup. README's F5 row claims only what was done. |
| **F6** precheck narrowness | **Closed for the named checks.** | Eleven-key check in `job.rs:234-253` (set equality) and `prepare.rs:308` (count plus every key by name); HTTPS-only endpoints (above); `api-key-env` documented as unusable through the server (`coding-preparation.md:92-96`). Optional credentials-file coverage check not done. |
| **F7** thin tests | **Partly closed.** | `provider.rs:961-973` loops all four retired names with and without a provider (`pi_coding.rs:329-332` refuses by name first, so both branches hit "retired"); `tests/coding.rs:922-928` checks construction; `tests/pi_signed_subscription.rs` deleted (an offered option). Missing: the negative cases in `job/tests.rs` (retired schema, `hosted_routes_sha256` mismatch), `tests/preparation.rs` (group-readable or symlinked credentials file), the receiver guard. See N3. |
| **F8** stale docs | **Closed.** | `README-coding.md:75-79`; server `README.md:283-284`, `:291-292`; `coding-preparation.md:92-96`; `job.rs:171`; `coding-qualification.rs:90-91`; `prepare.rs:424` (`let live`); `docs/repository-claims-operator-guide.md:242` (wording "fixture/hosted live profile retain" is ungrammatical; trivial). |

## 3. New findings on the remediation

**N1 (P2). The R-d claim is not supported end-to-end: the receiver's stdout, bounded at 8 MiB for the whole job, grows per relayed chunk and will end a long response well before the 32 MiB relay bound.**
- Evidence: `pi_docker.rs:32` `LIMIT = 8 MiB`; `:636-641` the run loop fails the process ("process output exceeded bound") when the RPC process's stdout exceeds it, which exits the loop and closes the attempt Uncertain via `coding_hosted_abort` (`:804`). That stdout is the inference container's channel to the Host for the whole job, not per attempt. In c3 (`state/0005-rpc-0.stdout`, 246 lines for 138 output tokens over four requests) every relayed chunk costs one ~330-byte `pull` frame (lines 10, 12-14), plus the `native_event` frames the receiver emits for every SDK event while a turn is active (`pi-coding.mjs:186`), plus each `provider_fetch` frame carrying the full base64 request body. c3 request 1 relayed 32,248 bytes in 29 chunks, mostly one ~250-byte delta per chunk (`state/coding-hosted-1-1.relay.json`). So stdout bytes per output token are at least of the same order as relayed bytes per token, which puts the job-wide ceiling in the low tens of thousands of output tokens, not 128k per response. The 8 MiB `MAX_AGGREGATE` (`pi_native.rs:18`) does not apply: hosted frames are written directly to stdin (`pi_docker.rs:696-702`), and the adapter's 2 MiB bound (`pi-native-tools.mjs:6`) is per frame.
- Why it matters: the outcome is fail-closed (Uncertain, full hold, job failed), so money is safe. But the README's R-d row ("covers the catalog's 128k output tokens") and the removal of D4 from the open items describe a ceiling that does not exist in the pipeline. The failure mode also changed name: it will read as a process-output failure, not `upstream_response_bound`.
- Not measured: the stdout file's byte size and the per-delta size of Pi's `message_update` events (if they carry the accumulated message, growth is quadratic). Both are readable from `0005-rpc-0.stdout`.
- Minimal change: measure stdout bytes per output token from c3; state the effective per-job output ceiling in the README and open-items doc; then either raise the hosted RPC stdout bound to match the relay bound or keep D4 open. One sentence each until measured.

**N2 (P1 for the record, not the code). No recorded test run at the remediation commit.** README "Tests" still reads "At `8fa71188`/`cab80a3a`". The regression logs are at `cab80a3a` (`r2.head`). The diff changes test helpers shared by many tests (`tests/coding.rs:278-282`), adds four Rust tests, one VM test and one Python test, and changes server fixtures. The integration evidence (r8, c3) does not exercise them. Fix: run the three commands in section 1 and add one line to the README.

**N3 (P2). The new fail-closed checks have no negative tests.** `job/tests.rs` (diff @1127) and `tests/coding_job.rs` (diff @264) only add the env var to fixtures. Nothing asserts: a loopback endpoint → `capability_mismatch`; a twelve-key or ten-key authority → `capability_mismatch`; a live profile without `WORK_ENGINE_HOSTED_CREDENTIALS` or with a relative path → `invalid_job_profile`; the same two in `prepare.rs`. Minimal change: five small tests next to the existing live-class tests.

**N4 (P2, record). README:293** still lists "The 1 MiB response bound (D4)" under "Not exercised" after R-d raised it; with N1, the line should name both bounds.

**N5 (P3, record). r8's binding to `7c103ad2` is asserted, not shown.** `matrix.json` says `subject: 20bb335c`; `matrix-r8.sh:2` says "code 81c68b3b + 20bb335c"; the README says "r8 on `20bb335c` (code `7c103ad2`)". `20bb335c` is not one of this vertical's commits and I could not relate it to `7c103ad2`. Mitigating: r8 and c3 ran the same binary (`cb73a44a…` in `matrix.json` and `c3.json.host_sha256`), so the offline and live evidence at least share code. Fix the script comment and say what `20bb335c` is.

**N6 (P3). Availability coupling in `owner_only`.** `https.rs:82` now refuses the whole credentials file (every payer, not just the subscription one) if Pi ever rewrites `~/.pi/agent/auth.json` with group/other bits, since `from_file` errors on the first bad source. Fail-closed and intended by R-f; worth a sentence in the operating guide so the operator recognises the refusal text.

**N7 (P3). `metrics/probes/hosted_live_observations.py`** matches the journal (`failures` top-level, `phase`, `provider_requests` keyed by string, attempt `disposition` snake_case, `"cancellation requested"` exact at `pi_docker.rs:791`). The four logged entries are the two synthetic transitions, as the doc says. Nit: the `job_kind` dedup key reuses `state_dir` only, so a reconciled-then-rerun directory logs once; acceptable.

No other new defects. Specifically checked and clean: `into_relay` on completed paths (section 2, M3); `is_none_or` already used in this workspace (`pi-pressure-context-host/src/coding.rs:436`); `Endpoint::parse` returns `Option` (`hosted.rs:128`); `continuity_try_full` lifetimes; the MCP helper's early return precedes any registry write; `verify()` and `base()` both force `none`.

## 4. Are the record's claims supported?

Supported, with the evidence I checked:
- r8 19/19: `matrix.log` ("ALL-DONE all_pass=True", 19 "done 0"), `matrix.json` `all_pass: true`, binary `cb73a44a…`.
- c3 row: `c3.json` (collected, `component_passed`, four completed primary attempts, settled 271/274/205/205, one tool effect); `live-check.json` (154 files scanned, no secrets, network none, no auth mounts); receipt and journal fields as in M1/R-b; the real Pi body passed the policy.
- Secret scan retained: five `live-check.json` files.
- Broad regression: `r2.head` = `cab80a3a`; the 29 `test result: FAILED` lines in `r2.log` sum to exactly 141 failed. I did not sum passes or check the attribution per crate or the baseline log.
- Review 3 remediation table: each row's code change exists as described (section 2). The rows are accurate about scope, including the partial F3/F5/F7 rows.
- "Largest relayed live response about 31 KB": c3 request 1 is 32,248 bytes, so the superlative now belongs to c3; immaterial.

Not supported or stale: the R-d row's "covers … 128k output tokens" (N1); "Tests" section (N2); README:293 (N4); r8 commit binding (N5). Not verifiable by me: the reviewer-cost figures ($7.36/$7.92); that the five `live-check.json` files were produced from unchanged run directories.

## 5. Not verified

- **Commit identity.** No git tooling; `subject.diff` and the manifest were taken as given. I did not confirm the working tree equals `9b289171` for these paths, nor what `20bb335c` contains.
- **Runtime behaviour.** No test was run; test correctness is from reading. The M2 and R-a tests reference helpers (`tick_until`, `has`, `request`, `push`) that exist (`hosted.rs:1639-1680`), nothing more.
- **N1's magnitude.** Derived from frame shapes in c3; stdout byte size and `message_update` sizes not measured.
- **Secrets.** I read no secret and did not scan the run directories myself; I relied on `live-check.json`.
- **Backend behaviour** for the body policy beyond c3's four requests (one model, reasoning off, one task); whether the backend honours `store:false`.
- **Pi auth store mode in practice** (R-f) — not read.
- **Regression attribution** of the 141 failures to inherited suites; the baseline log; the `TMPDIR` reruns.
- **Not read in full:** `matrix.json` beyond headers and check flags (diff showed only expected churn: timestamps, body digests, elapsed); r8 per-run state; `tests/pi_stream.rs`; `pi-coding-hosted.test.mjs`; the Opus-era removed files.