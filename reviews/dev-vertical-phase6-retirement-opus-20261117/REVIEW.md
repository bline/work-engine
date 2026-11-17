# Review: retiring the credential-mounted live profiles and cutting over to the hosted profile

Subject: `8fa71188` (retirement and cutover) and the operating docs in `b50017ab`, compared against `38803cff`.
Reviewer: Claude Opus. Read-only review with fresh context. This is a same-model review, since an Opus session wrote the code.

## Disposition

**OK with notes. No P1 findings.**

- **Retirement holds.** No reachable path is left that mounts or projects provider access into a container, or that admits a retired schema.
  - Host admission refuses all four retired names before anything else.
  - Any provider plan forces the hosted schema.
  - The inference container always gets `--network none` and has no credential mount.
  - The receiver refuses any live configuration that is not hosted and on the lifecycle bridge.
  - The MCP helper refuses a subscription submit.
- **Provider-disabled profiles are unchanged.** The fixture, lifecycle fixture, claims bridge and signed fixture (with its `PiOwner`) behave as before.
- **What remains:**
  - **Docs:** the Host's own profile doc still describes the retired lane as current.
  - **Deleted tests:** three test files were removed while the behaviour they covered still exists.
  - **Precheck gaps:** the server and preparation prechecks are narrower than Host admission.
  - **Dead code:** some branches from the credential era are still there.

Severity scale: P1 = fix before relying on the cutover; P2 = fix before release; P3 = minor or cleanup.

## Limitation that affects every finding

**I could not run `git show`, `git diff` or `git log`.** This session had no shell: only file reads, grep and `watchdog_diff`, which only shows uncommitted changes.

- **What I reviewed:** the current state of HEAD.
  - From `.git/logs/HEAD`, HEAD is `03a981b8`. It contains `8fa71188`, `a0573ab2`, `cab80a3a` and `b50017ab`.
  - The working tree differs only in an unrelated file, `native-context.md`.
- **Removed files:** I identified them from `subject-manifest.txt` plus their absence at HEAD.
- **"Before" contents:** where I describe what a removed file contained, I used the older copies under `.claude/worktrees/agent-a830873f8b942e3e9/`. Those predate `38803cff`. Please confirm those claims with the `git show` commands listed at the end.

## Answers to the packet questions

### Q1. Is anything left that could mount or project provider access, or select a retired schema? No.

- **Host profile admission**
  - `pi_coding.rs:325-329` refuses `RETIRED_LIVE_PROFILES` by name, before any other check.
  - `pi_coding.rs:330-341` requires the hosted schema whenever a provider plan is present. A hosted schema without a provider fails the same check.
  - `pi_coding_provider.rs:426-429` makes `ProviderPlan::validate` refuse any profile that is not hosted.
  - `Admission` (`pi_coding_provider.rs:380-394`) uses `deny_unknown_fields` and no longer has any native-access fields.
- **Mounts**
  - `expected_mounts` (`pi_coding.rs:490-549`) gives the hosted profile only the stream adapter, `/config` and `/profile`. Only provider-disabled profiles get the fake `coding-canary` directory at `/auth`.
  - Nothing mounts `private-access`, which stays empty (`pi_coding_provider.rs:544-548`).
  - `verify` enforces the exact mount set (`:640-657`) and an environment allowlist (`:661-678`).
- **Network:** for every profile that passes validation, `base()` and the `NetworkMode` check resolve to `none`. The `bridge` branch is dead code (see F5).
- **Receiver** (`receiver/pi-coding.mjs`)
  - `:20-24` throws unless a live configuration is hosted and on the bridge.
  - `:26-30` replaces `fetch` for the hosted, bridge and signed paths.
  - `:251` allows only the hosted transport.
- **Binding and admission paths**
  - `bind_coding_effect_gate` (`pi_coding.rs:687-698`) accepts only provider-free fixture bridges, or the hosted profile with a provider.
  - `run()` requires both the gate and the hosted transport to be bound (`pi_docker.rs:975-990`).
  - `Host::admit` refuses the signed fixture without an owner (`pi_docker.rs:316-323`).
  - `admit_signed_fixture` refuses subscription owners (`:330`).
  - `examples/mcp_pi_coding.rs:87-92` refuses a subscription submit.
  - Docker child processes run with a cleared environment (`pi_docker.rs:567-569`).

### Q2. Did removal break or weaken a provider-disabled profile, the signed fixture or the lifecycle gate? I found no regression.

- The fixture inference arguments are unchanged: `PI_OFFLINE=1` and `PI_CODING_AGENT_DIR=/tmp/pi-empty` (`pi_coding.rs:813-840`).
- `signed_pi()` now matches only `SIGNED_PROFILE`, and the receiver's `signed` is now the same as `fixtureSigned`. The `PiOwner` seal, lease and fake-provider path is intact (`pi_coding_provider.rs`, `coding_fake_provider_frame`).
- The lifecycle gate refuses a hosted policy in fixture mode (`coding.rs:373-376`) and refuses the Sol-high profile digest (`coding.rs:356`).
- Weaknesses are listed separately: F4 (gate still accepts live mode without a hosted policy) and F7 (thin replacement tests).

### Q3. Are the server and preparation checks consistent with Host admission, and is the profile `env` safe?

- **Consistency:** mostly. The server and preparation bounds match the Host's or are stricter:
  - context bytes at most 24000, budget at most 700000, task at most 8192 bytes, exactly four requests;
  - the route table is bound through `hosted_routes_sha256`;
  - a retired schema yields no input digest.
  - Some Host checks are not mirrored (F6). All of them fail closed at the Host, but only after the one-shot authorization has been used.
- **`env`: safe.**
  - `WORK_ENGINE_HOSTED_CREDENTIALS` passes the profile's rules (`job.rs:449-455`: key is an identifier containing neither `TOKEN` nor `KEY`; value at most 1024 bytes with no NUL).
  - Its value is a path to a file that only names where each payer's access comes from; the path is not a secret.
  - Preparation checks that the file is canonical, owner-only and owned by the current user (`coding-job-prepare.rs:177-186`).
  - One note on `api-key-env` is in F6.

### Q4. Were deleted tests covering behaviour that still exists? Yes, in three places (F2, F3, F7).

### Q5. Do the operating guide and preparation doc match the code?

Mostly yes. The Host's own doc and the server README do not (F1, F8).

## Findings

### F1 (P2): The Host's profile doc still presents `pi-coding-sdk.v1`, its native-access admission and bridge networking as current, and never describes the hosted profile

- **Evidence**
  - `rust/crates/work-engine-manual-host/docs/pi-coding.md:14-19` presents `work-engine.pi-coding-sdk.v1` and its `access_adapter` with no retirement banner.
  - `:22-31` lists the admission as requiring `native_auth_source` and `allow_native_access_projection:true`.
  - `:33-58` describes reading the native source, staging access in a 0600 file and mounting it.
  - `:74-78` says "Live inference uses bridge networking for the fixed SDK subscription endpoint".
  - `:117-121` and `:159-166` still describe the access projection as present.
  - Only the signed-subscription section (`:176`) got a "Retired" banner.
  - A search of `work-engine-manual-host/**/*.md` finds no description of `work-engine.pi-coding-hosted.v1` other than that banner.
- **Why it matters**
  - An admission document written from this doc is refused by `Admission`, which rejects unknown fields (`pi_coding_provider.rs:380`).
  - The doc contradicts the main security property of the cutover: no network, no credential mount.
  - The only live profile has no Host-side contract document.
- **Minimal change**
  - Add the same "Retired" banner above `:14`, `:22` and the networking sentence at `:76-77`.
  - Add a short hosted section covering the schema, the `Admission` fields including `hosted_routes_sha256`, the `--network none` container with only `/config`, `/profile` and the adapter mounted, and the Host-held transport. Link the qualification record.

### F2 (P2): Deleting `test_pi_subscription.py` removed the only coverage of a coding-helper switch that still exists, and session-events still offers the retired subscription transport

- **Evidence**
  - `skills/session-events/tests/test_pi_subscription.py` is in the manifest and is absent at HEAD.
  - Its last available copy contains `test_disabled_by_default_and_conflicting_switches_fail`. That test covers `CodingHelper.from_environment`: disabled by default, conflicting `PI_CODING_FIXTURE_ENABLED` and `PI_CODING_SUBSCRIPTION_ENABLED` raise an error, and a switch without pins raises an error.
  - That code is unchanged and still selects `SubscriptionCodingHelper`: `skills/session-events/scripts/session_events.py:2698-2709`, `:2717-2719`.
  - It still advertises an MCP tool saying "Submit the exact signed Pi subscription backend candidate" (`:2948-2951`).
  - A search of `skills/session-events/tests` finds no other `CodingHelper.from_environment` test.
  - The Rust helper's new error code `pi_subscription_profile_retired` (`mcp_pi_coding.rs:91`) is not in `CodingHelper.error_reasons` (`:2696`), so it is collapsed to `coding_unavailable` (`:2414-2417`).
- **Why it matters**
  - The fixture's enable/disable logic is now untested.
  - Operators can still enable a transport for a retired profile. It fails with an unhelpful error, after the helper has already done registry writes (see F5).
- **Minimal change**
  - Either remove the subscription switch, `SubscriptionCodingHelper` and its branch in `coding_tools`, or have `from_environment` refuse `PI_CODING_SUBSCRIPTION_ENABLED`.
  - In either case, restore the disabled-by-default and conflicting-switch assertions for the fixture helper in `test_pi_coding.py`.

### F3 (P2): Deleting `pi-signed-subscription.test.mjs` removed receiver VM regressions for provider-disabled paths and left a dangling README reference

- **Evidence**
  - `rust/crates/session-task-admission/README.md:112-116` still says: "Provider-disabled deadline regressions run the fixed receiver with VM SDK, clock and IPC fixtures: `node --experimental-vm-modules --test rust/crates/work-engine-manual-host/tests/pi-signed-subscription.test.mjs`". That file was deleted.
  - Its last available copy includes `composed receiver retains provider-disabled pressure observation and checkpoint insertion`. That test exercises receiver code that still exists:
    - `pi-coding.mjs:101-105` (`pressure_context`);
    - `:347-352` (checkpoint insertion and `pressure_inserted`);
    - `:301-304` (`pressure_request`).
  - It also includes the consumption-deadline cases, which share `consumptionDeadline` and `checkConsumption` (`:38-45`, `:122-124`) with the signed fixture.
  - Searching `rust/**/tests/**` for `pressure_inserted` or `pressureCheckpoint` finds nothing.
  - `pi-coding-hosted.test.mjs` covers only the hosted path.
  - I could not diff the `38803cff` version directly; see the verification commands at the end.
- **Why it matters:** the receiver's provider-disabled pressure path, and the deadline checks the signed fixture still relies on, have no receiver-level test left.
- **Minimal change**
  - Keep a provider-disabled VM test: port the pressure case, and one delayed-initialization deadline case, to `pi-coding-hosted.test.mjs` or a new `pi-coding-fixture.test.mjs`, driving the fixture and signed-fixture configurations.
  - Update `session-task-admission/README.md:112-116`.

### F4 (P3): The lifecycle gate still accepts the retired live-without-hosted configuration, and the lifecycle store keeps the Sol-high grammar

- **Evidence**
  - `CodingContinuity::new` (`pi-pressure-context-host/src/coding.rs:354-378`) accepts `task: Some, hosted: None`. That is the old `lifecycle-live` composition.
  - Production never builds that configuration: `coding-qualification.rs:144-157` always sets `hosted` in live mode.
  - Tests still treat it as supported (`tests/coding.rs`):
    - `live_continuity_owner_derives_checkpoint_…` (`run_complete(true, false)`);
    - the comparison inside `hosted_attempts_leave_the_settled_event_grammar_unchanged…`.
  - `lifecycle-store` still recognises Sol-high digests and kinds: `src/pi_coding.rs:27-52, 255-258, 348-349`, `src/native.rs:2484-2489, 3026-3027`, `tests/pi_multiturn.rs:876`.
- **Why it matters:** this is not reachable with real access, because the Host only binds a live gate to the hosted profile. But the trusted library still encodes the retired composition, and the Sol-high removal is only enforced at the gate.
- **Minimal change**
  - In `CodingContinuity::new`, refuse `config.task.is_some() != config.hosted.is_some()`.
  - Default the test helpers to `hosted_policy()` in live mode.
  - Record lifecycle-store Sol-high removal as follow-up work.

### F5 (P3): Dead code from the credential era in the Host, receiver and MCP helper

- **Evidence**
  - **Bridge network branch:** `pi_coding.rs:434-443` (`base`) and `:606-614` (the `NetworkMode` check) still pick `bridge` when `provider.is_some() && !is_hosted`. Validation makes that impossible.
  - **Non-bridge live branches (Host):** `:1066-1077` (`task_start` on the non-bridge `ready` path).
  - **Non-bridge live branches (receiver):** `pi-coding.mjs:106`, `:345`, `:354`.
  - **Stale Sol-high and access fields:** `pi_coding.rs:1400` still copies `reasoningEffort` and `payerAccountId`.
  - **Leftover cleanup:** `:1671-1679` deletes `private-access/access.json`, which is never written.
  - **Stale comment:** `pi-coding.mjs:188` ("the direct live profile").
  - **Late refusal in the MCP helper:** `mcp_pi_coding.rs:84-92` refuses a subscription submit only after `PiOwner::open`. That call ingests control and publishes to the registry: `pi_pilot.rs:225`, `:817-826`.
- **Why it matters:** none of it is reachable with access today. But `bridge` networking stays one future schema mistake away, and a retired submit still writes to the registry.
- **Minimal change**
  - Make the inference container's network always `none` in both `base` and `verify`, and drop the dead branches and fields.
  - In `mcp_pi_coding.rs`, move the retired check before `PiOwner::open`, after the read early-return at `:79`.

### F6 (P3): Server and preparation prechecks are narrower than Host admission

- **Evidence**
  - `verify_live_job` (`job.rs:198-275`) and the preparation admission check (`coding-job-prepare.rs:299-313`) look up named keys with `Value`. The Host's `Admission` rejects unknown keys (`pi_coding_provider.rs:380`).
  - So a pre-K-f admission that still carries `native_auth_source` or `allow_native_access_projection` passes both, then fails at `Host::admit`.
  - Host checks that neither of them mirrors (`pi_coding_provider.rs:430-468`):
    - the `pi_config` file set (`settings.json`, and `models.json` equal to `{}`);
    - catalog path equal to `runtime/CATALOG`;
    - reservation not exceeding budget (the server does not check this; preparation does);
    - `pi_config` not overlapping `state_dir`;
    - credential coverage of every route payer.
  - `HostedRoutes::validate` accepts `loopback:<port>` (`pi_coding_hosted.rs:128-137`), so the live class accepts loopback routes. Without the env var, `LoopbackTransport` then serves them with synthetic access.
  - **Cost of a late refusal:** the server binds and writes `entry-attempt.json` before the Host child runs (`job.rs:672-710`, `:815-821`). If the Host refuses, the operation stays `entry_uncertain` and the authorization is consumed, contrary to the guide's "refusal leaves the authorization usable" (which promises this only for checks 1 to 7).
  - **`api-key-env`:** the server clears the child environment (`job.rs:845-847`). So `api-key-env` (`pi_coding_https.rs:31-32, 104-110`) can only work if the secret sits in `profile.env` under a name avoiding `KEY` and `TOKEN`, which bypasses the rule's intent.
- **Minimal change**
  - In both prechecks, require exactly the 11 admission keys (or deserialize into a mirror struct that rejects unknown fields), and in the live class require `https:` endpoints.
  - Optionally have preparation parse the nonsecret credentials file and check payer and endpoint coverage.
  - Either document `api-key-env` as unsupported through the server, or refuse it when loaded from a server-launched Host.

### F7 (P3): Some retained or replacement tests are thin

- **Evidence**
  - **Passes for an unrelated reason:** `work-engine-manual-host/tests/pi_signed_subscription.rs:4-16` still uses `pi-coding-signed-subscription.v1`. Since `Host::admit` no longer checks that schema (`pi_docker.rs:316-323`), the test fails at "image must be digest pinned" (`pi_docker.rs:243-250`, because the image is `"untrusted"`). It never reaches the retirement check.
  - **Only one retired name tested:** `pi_coding_provider.rs:958-963` checks just `RETIRED_LIVE_PROFILES[1]`.
  - **Refusal reason not pinned:** `tests/coding.rs:917-922` (`retired_sol_high_profile_cannot_start_continuity`) asserts only `is_err()` from a full run. It does not check that construction refused.
  - **Missing negative cases:**
    - in `job/tests.rs` (`:1131-1178`, `:1232-1254`): a retired `coding.schema`, and a `hosted_routes_sha256` mismatch, in the live class;
    - in `tests/preparation.rs`: a group-readable or symlinked credentials file;
    - in `pi-coding-hosted.test.mjs`: the receiver guard at `pi-coding.mjs:23` (non-hosted live, or hosted without `--lifecycle-bridge`).
- **Minimal change**
  - Rewrite `pi_signed_subscription.rs` to call `job.coding.validate` directly and assert "retired", or delete it.
  - Loop over all four retired names.
  - Assert `continuity_try_full(true, None, None, true, None, None, 1000).1.is_err()`.
  - Add the three missing negative cases above.

### F8 (P3): Other docs and comments that disagree with the code

- **Evidence**
  - `work-engine-server/README.md:283` says the live capability "uses the existing lifecycle-live Host profile". `:291` mentions "separately versioned Sol/high preparation".
  - `pi-pressure-context-host/README-coding.md:79-131` marks the whole live section "kept as history". But the hosted profile runs on the same v4 lifecycle digests and checkpoint derivation (`coding-qualification.rs:229-238`), so part of that text is the current contract. The "Status: … No Docker, SDK or provider operation has run" line (`:75-77`) is now false.
  - `docs/coding-job-operating-guide.md:55-56` and `coding-preparation.md:78-80` list only the `https:` endpoints. The code also accepts `loopback:` (see F6).
  - `coding-preparation.md:93-96` omits `api-key-env`.
  - Stale comments and names:
    - `job.rs:171` ("lifecycle-live class");
    - `coding-qualification.rs:91` ("auth-source path");
    - `coding-job-prepare.rs:414` (`let sol = …`);
    - `docs/repository-claims-operator-guide.md:242` ("live/Sol profiles").
- **Minimal change:** update these lines. In `README-coding.md`, split out the v4 lifecycle contract that the hosted profile still uses, so it is not labelled as history.

## Correct (verified)

- **Where retired schemas are refused, or now yield no digest:** Host profile validation, `ProviderPlan::validate`, the receiver, the MCP helper submit, the gate's Sol-high check, the server's `is_hosted` check and `host_input_digest`.
- **Container isolation:** the hosted container has no `/auth` mount, no access file and `--network none`. Mounts and environment are verified against Docker inspect.
- **Preparation output:** the hosted profile, 24000 context bytes, the stream adapter written from the compiled constant, reservation at most equal to budget, and `env` carrying only the credentials path.
- **Fail-closed transports:** with the env var unset, `LoopbackTransport` refuses `https:` routes (`pi_coding_hosted.rs:289-299`). `HttpsTransport::check` refuses any mismatch between a payer's credential kind and the route endpoint (`pi_coding_https.rs:155-176`).

## Not verified

- No builds, tests, Docker runs, network calls or credential files. Supervisor should run:
  - `cargo test -p work-engine-manual-host -p pi-pressure-context-host -p work-engine-server`
  - `node --experimental-vm-modules --test rust/crates/work-engine-manual-host/tests/*.test.mjs`
  - `python3 -m unittest discover skills/session-events/tests`
- No direct diff of `8fa71188` or `b50017ab` against `38803cff` (no git tooling). Confirm F2 and F3 with:
  - `git show 38803cff:rust/crates/work-engine-manual-host/tests/pi-signed-subscription.test.mjs | grep -n "legacyPressure\|provider-disabled"`
  - `git show 38803cff:skills/session-events/tests/test_pi_subscription.py | grep -n conflicting_switches`
  - `git diff 38803cff 8fa71188 --stat`
- Transport, ledger, streaming and evidence internals were left to the separate Fable review: the credentials-file handling in `pi_coding_https.rs`, `pi_stream`, settlement, and the observer example.
- The lifecycle-store Sol-high grammar was not reviewed beyond locating it.
- The qualification records, matrix and live run directories were not checked.

## Merge verdict

**OK with notes.** No P1 findings. Fix F1, F2 and F3 before release. F4 to F8 are cleanup and hardening.