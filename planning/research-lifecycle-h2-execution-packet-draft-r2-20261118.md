# H2 execution packet draft, r2: Pi Host controlled-proof composition (index)

**DRAFT — not approved.** This indexes draft bytes. It is not an approval, grant, installation, attestation or manifest publication. A holder named below comes from an operator or owner decision; naming the holder records no act of theirs. Governing plan: `planning/research-lifecycle-m4-s2-plan-20261117.md` §4 (sha256 `bcb6e1ce…`). r1 index: `planning/research-lifecycle-h2-execution-packet-draft-20261117.md`, which stays as written. Worker: S2 r2 child of the research-lifecycle root (`01a1e56d-7ceb-71fc-9c46-c9fcd9618234`). Source: committed tree of `rescue-detached-20260911` at `5fb035263c62d8749db46ca0a29aae09d733440b`. Artifacts: `/home/bline/cbm-work/research-lifecycle-tool-registry-20261116/016-s2-r2/`. r1 artifacts in `007-h2-artifacts/` were not modified.

## Artifacts

| File | SHA-256 |
| --- | --- |
| `gate-trace.md` | `b8707a6cd868e7fafe9b34017186364bb6944a3d0db3dc8c2c433bc63474f354` |
| `gate-loci.json` | `8804a92be22f62adb8accc7f6308955ea3947f442aa200adde1b9c83c6d49057` |
| `audit-request.json` | `b1917792d744480e75227e7df9d307dbb8ea81c2893b7ef798da4f1fb0b7d8b5` |
| `snapshot-manifest.json` | `a383ac1c3fa6964f5ec6201c02920e18fdfcbb69a5f532fce3aec403b931e2d0` |
| `rubric.md` | `ba6d3273a45973efd3a1297dd5c95e379aecd4e999af734d1d82bf279adca69c` |
| `lc7-protocol.json` | `8b4c42adb7142ee20f1f354b76690966d2c5df37221e00523cd7bbf75c1a1aca` |
| `advisory-input.md` | `df3a7ea8439bf240a7a1380b57a7436b7bd48b35d6948be0fecf3d8012ad7646` |
| `authority-skeletons.json` | `e60ffc2c212fa763df45a460bc93a08a4534a50ec523e6ef92b446052b950914` |
| `completeness.md` | `4530153b75bd1256963ff7e5a2779c89b92101bfd6fe69fbe4a1665969b9a138` |
| `check.py` | `b1dc62503d7d8a67cb01dc716b413141d1503c0bdfa1feb5aaf90944496b85a4` |

`check.py --index <this file>` recomputes every digest above, every git-pinned member, every register digest and 108 gate loci. It also checks slots, DRAFT marks, settled holders and the Pi Host constants. Custody records are in `016-s2-r2/custody/` (r2) `016-s2-r2/custody-repair/` (after the review repair) and `016-s2-r2/custody-ruling/` (after the grant-issuer ruling).

## Settled inputs used (operator or owner decisions; not reopened)

- Realization: the Pi Host (route `pi-host-v1`, protocol `9a33912a…`; record `records/dev-vertical-pi-host-realization-20261118/README.md`), not Codex and not a Claude Host (board 5874).
- Model for every arm: `claude-sdk/claude-haiku-5-5`, thinking adaptive at effort low, tier n/a (R6; 5886).
- H2 class: controlled-proof composition on live inference, no L4 startup hold (5909).
- Fresh start only (R3); narrow claim (R2); protection by the Docker profile (R9′, 5904). Tools are `snapshot_read`, `question_open` and `result_publish`, with no shell (R8″); grants for the first controlled-proof H2 are composition-issued under O1: the composer writes `lifecycle-service` `GrantConfig` under the research-lifecycle root's O1 authority, recorded exactly (bytes and SHA-256 retained with the run), and every claim says "composition-issued under O1"; task-host issuance (R8′) follows with the production path and is not claimed for this run (operator ruling, option (a), operator 2026-11-18 (root session), board 5919).
- Holders (R4, 5906): the operator is research owner, decision owner, and accepts the estimand and the audit result. The research-lifecycle root verifies, and the O1 channel is bound to that root.
- Account identifier verified absent: the revised L1 removes the need for email-retention consent (5904); the closed metadata allow-list is the 5911 ruling; Claude Code's observed metadata is exactly that shape (5912). `device_id` and `session_id` remain in the retained bodies.
- Budget: the root's $200 window covers live H2 inference, paid from the Claude subscription quota.

## What changed from r1, and why

| Artifact | Change | Why |
| --- | --- | --- |
| gate trace / loci | Re-verified at `5fb03526`: 97 loci, up from 47; six r1 lines moved. G1–G7 restated as passes / not reached for this composition. G2 is now the Docker profile and G7 is `claude-sdk` via the Pi Host. Added P1–P5: realization identity, runner capture check, eligibility and settlement, account-identifier check, the `verify_delivery` Pi Host arm. D5 kept as Codex history. | New realization and H2 class |
| lc7-protocol | Model, thinking and tier settled. Identity rule: what must be equal across arms, what differs by construction, and a post-run check. Pi Host tool surface and bounds. Root as verifier. | R6, R8″, R4, 5886(6) |
| advisory-input | Rewritten for the Pi Host capture: the advisory snapshot is one member, the A4 material. States the exact projection rules and the recommendation codec, and that only a controlled card exists. | `research_delivery.rs` Pi Host arm |
| authority-skeletons | Holders filled (R4) with acts kept as slots. Pi Host identities replace the Codex build. Grant issuer: composition-issued under O1 (operator ruling, option (a)). S marked unused in the first H2. | R4, R8′, 5909 |
| audit-request, snapshot, rubric, completeness | Register and members re-pinned at `5fb03526` (blobs identical). Member IDs and the A4 member added. Source result bound of 8,192 bytes. Root verifier and blinding limit. New `settled` state. | Pi Host bounds; R4 |
| check.py | Adds occurrence counts, gate rows, holder-needs-slot (keys holder/owner/verifier/identity), Pi Host constants against source, `gpt-6-sol` only as history, index self-digests (r1 review 6c). | Packet item 8; review F6 |
| review repair (`017-s2-r2-review`, F1–F7) | Closed LC-7 post-run comparison; fourth-tool discrepancy rewritten to its two bounded outcomes; treatment ledger range and byte budget as a precondition; advisory read plan pinned at 32 KiB; needs item 1 reworded; env inheritance and credentials identity recorded; citations fixed; grant issuer stated as awaiting operator ruling (since settled: option (a), see below). 108 loci. | `O/016-s2-r2/REPAIR.md` |

## Gate summary (detail: `gate-trace.md`)

| ID | Status in the first H2 |
| --- | --- |
| G1 startup publication | not reached: no hold; the Pi Host runner refuses one (`pi_host.rs:595`) |
| G2 protection | passes as the Docker profile: observation re-verified, eligibility requires it (`pi_coding.rs:808–831`, `pi_host.rs:522`) |
| G3 install route | not reached: the predicate admits only the ChatGPT lease and refuses in `controlled-proof` builds; the card is a controlled card |
| G4 production prepare/adopt | not reached; the controlled mint is optional (recipe step 6) |
| G5 delivery consumption | passes in the controlled class only (`checked_unminted_basis`); no production `acquire` caller |
| G6 grants | passes by composition (`GrantConfig`); composition-issued under O1 (operator ruling, option (a)) |
| G7 provider | passes: `PiHostV1`, `claude-sdk`, `https:anthropic-oauth` with payer `claude`, billing guard, pre-dispatch check, credentials by env |
| P1–P5 | met offline at protocol `9a33912a` (q15/q16); **never run live** |

## Execution recipe (draft; nothing runs from this document)

Start from `rust/crates/lifecycle-service/tests/pi_host_join.rs` `pi_host_research_delivery` (`join(Some(a4))`) and the task host's `projection_tests::pi_host_q_delivery_verifies` (`research_delivery.rs:1597`, `controlled-proof`).

0. **Before any run (owner acts on frozen r2 bytes).** The operator's R4 acts: qualification, applicability, estimand acceptance, installer admission. Grant scope and budget filled. The root answers two things: whether the controlled mint is in scope (step 6) and the card route (step 2). **Grant issuer: settled, option (a)** (operator 2026-11-18 (root session), board 5919). The composer writes `lifecycle-service` `GrantConfig` under the research-lifecycle root's O1 authority (as `pi_host_join.rs` `config()` does); the grant is recorded exactly, its bytes and SHA-256 retained with the run, and every claim says "composition-issued under O1". Task-host issuance (R8′) follows with the production path and is not claimed for this run. Grant scope and budget values remain slots.
1. **Build** in a separate `CARGO_TARGET_DIR`: `work-engine-pi-host`; `lifecycle-service` tests with `--features controlled-proof`; `work-engine-task-host` tests with `--features controlled-proof`. Record binary SHA-256s as the run pins (`authority-skeletons.json` I.fixed_identities.run_pins).
2. **Controlled A4 card** for `context-boundary.h2-levels`. Publish a use bundle and the operator's two approval events in a repository at a pinned commit, then project through `synthetic-card-host` (`card_host.rs:376`). The outputs are `request.json`, `card.json`, `reconstruction.json` and `target.txt`. There is no tested path over a real repository (only `derive_full_a4_fixture` on a synthetic repository).
3. **Advisory run** (`research-lifecycle-h2-advice-1`). Same composition as `join(Some(a4))`, with these changes:
   - (a) no observer process;
   - (b) route table `[{"id":"subscription","payer":"claude","endpoint":"https:anthropic-oauth"}]`, `max_attempts_per_request` 1 (the API failover is untested for `claude-sdk` bodies), and the authority's `hosted_routes_sha256` recomputed;
   - (c) `WORK_ENGINE_HOSTED_CREDENTIALS` set in the test process. The Host inherits the composer's environment: the runner spawns it without `env_clear` (`pi_host.rs:609`; `native_stdio.rs:60` only redirects stdio). It names a `work-engine.pi-hosted-credentials.v1` file mapping payer `claude` to `{"kind":"claude-code-auth","path":<operator's Claude Code credentials file>}`. The HTTPS transport identity records `credentials_file_sha256` and the payer source kinds (`pi_coding_https.rs:209,210`); retain both so the step is reproducible;
   - (d) a real `SubmitTask`: objective and question, the target fact, the member ID and length, the read plan **pinned at 32 KiB** (offsets 0, 32,768, 65,536, …, last range the remainder; `advisory-input.md`), and the recommendation codec with the digest to copy;
   - (e) per-arm `GrantConfig`s; job `max_requests`, `budget_microusd`, `max_context_bytes`, `max_tool_calls` and `turn_deadline_secs` per the D9 slots;
   - (f) a live variant of the test without the scripted steps and without the exact read-count assertion. It keeps the eligibility, settlement, no-container and failure assertions, and writes `retained.json` and `mapping.json`.
4. **Verify delivery.** A live variant of `pi_host_q_delivery_verifies` over that output (`WE_PI_HOST_OUT`, `WE_A4_FIXTURE` = step 2 output), keeping the `spans == ⌈len/32768⌉` check as a record of whether the pinned plan was followed (the delivery outcome is `verify_delivery`'s own). Keep the four refusal controls and record `verify-delivery.json`. A refusal is the outcome: retained, not rerun silently.
5. **Baseline audit** (`research-lifecycle-h2-audit-baseline-1`, snapshot `snapshot:h2-baseline-r2`, 5 members). Run before treatment (LC-7 run order).
6. *(Root's choice)* **Controlled basis**: `checked_unminted_basis` → `mint` (`controlled_verify_joined_research_basis`). It exists but has not been exercised with the Pi Host, and it needs a test-only `BasisBundle` composition.
7. **Treatment audit** (`research-lifecycle-h2-audit-1`, snapshot `snapshot:h2-treatment-r2`: 13 members plus `research/a4-material`). Identical job parameters except the per-arm fields (`lc7-protocol.json#model_effort_tier.identity_rule`). **Preconditions:** `ledger_range` filled with lines 50, 295–374 and 431 (extract 5,753 bytes, sha256 `e3b452a8…`, instead of the 58,643-byte whole file), and the treatment byte budget met: member bytes ≤ 240,000 (126,096 with the range, about 238,593 with a fixture-size A4 member; `snapshot-manifest.json#size_bounds.treatment_byte_budget`). If the real A4 member breaks the budget, the run does not start and the root reports it. Every provider request refuses when the receiver's `contextBytes` exceeds `max_context_bytes` (`pi_coding_provider.rs:860`); such a refusal mid-run is retained as the LC-7 outcome (a failed treatment run), not rerun.
8. **After each run.** Exact `GrantConfig` bytes and SHA-256 retained with the run (composition-issued under O1). No container left behind. Token scan of the run files (as in live-c2). Billing headers retained. Capture and runner check retained. After both audits, the closed identity-rule comparison over every body (`lc7-protocol.json#model_effort_tier.identity_rule.check_after_run`: 9 keys by value, `tools` and `system` by digest, `metadata` and `messages` by construction).
9. **Verification** by the root per `rubric.md`; then the operator's acceptance act.

**What changes from the offline fixture upstream to the live `claude-sdk` route:**
- the scripted loopback observer is replaced by `api.anthropic.com` on the subscription;
- the billing guard and the pre-dispatch Claude Code check become active (they are inert on loopback);
- the credentials file is now required (without it the Host binds the loopback transport, and `https:` routes fail);
- thinking blocks appear: the offline observer sends none, so this is the first live relay test of the ledger's `{type,thinking,thinkingSignature}` shape and of exact thinking comparison across four or more requests;
- the model chooses its own reads, so the delivery's contiguity rule can refuse;
- the A4 material is a controlled card of the real decision, not the synthetic fixture;
- the audit arms have multi-member snapshots, which no Pi Host run has used yet;
- real bodies pass the account-identifier allow-list (observed live only on the analysis profile, live-c1/c2).

### Pi Host-side needs (for the root to send to dev-vertical for sizing)

1. **Confirm the live job shape** for the relay profile `work-engine.pi-lifecycle-hosted.v1`. The authority, job, serve-config and `pi-host.json` writers already exist in the root-owned `lifecycle-service/tests/pi_host_join.rs:128-229`; the live deltas are the route table (`https:anthropic-oauth`, payer `claude`), `max_attempts_per_request` 1, the credentials environment and the deadline. Every live Claude run so far (live-c1/c2) used the analysis profile; the relay profile has never run live.
2. **Route table and guards**: single route, payer `claude`, `max_attempts_per_request` 1. Confirm that the billing guard, the pre-dispatch check, `forwarderRefused == 0` and the retry-header rule apply to relay jobs.
3. **Runtime pins**: runtime manifest `52ace834…` (Claude Code binary `0dc4f4a9…`, Agent SDK 0.3.325) and catalog `e462beb2…`, currently under `~/.local/state/work-engine/dev-vertical-h2-route-20261117/runtime-claude-sdk`; also the receiver shim, stream adapter and pi-config manifests. Confirm this satisfies C1(vii) (mandatory Claude binary pin) and name the pinned location for the run.
4. **Image digest** `node@sha256:4b150b48…`: still current? Are the container limits (64 pids, 512 MB, 1 CPU, 64 MB noexec tmpfs) adequate for a turn of about 300 KB with thinking?
5. **Bounds for the treatment arm**: with the ledger range, about 126 KB of members plus the A4 material (budget 240,000 member bytes, step 7) against the per-request `max_context_bytes` ≤ 400,000 refusal, 8 MiB of bodies and 64 sends per turn, the 1,000,000-byte `thread/read` frame (relayed results included), `max_tool_calls` (16 in the fixture) and the turn deadline. Fit, or what must change?
6. **Host run record**: the runner spawns the Host with stderr discarded (`lifecycle-runtime` `NativeStdio::spawn`). Confirm that the state directory (`pi-state.json`, `relay.json`) keeps failures and billing headers, or retain stderr.
7. **Live test variants** (steps 3f and 4): test-only, in `lifecycle-service` and `work-engine-task-host`, which are the root's owner crates. Size them and ask the root before writing.
8. **Thinking on the relay path**: expected live behaviour of the ledger shape and of the canonical comparison (record P3-1).
9. **`DeferredToolPlaceholder`** in the pinned wire `tools` array (R8″ holds for the relayed surface, which is the three tools). Confirm the two bounded outcomes read from source: (a) a call Pi surfaces as a `toolCall` fails the turn, "tool call {id} was not relayed" (`pi_host_serve.rs:782`, unit test `:1503`); (b) a call absorbed inside Claude Code appears only as a non-projected `tool_use`/`tool_result` pair in later bodies, skipped by `verify_delivery` (`research_delivery.rs:474`).
10. **Credential freshness**: required margin of the Claude Code access token for runs of several minutes, and the token scan after each run.
11. *(Recommended in 5900)* F4, the per-request attempts array in capture metadata, before the first live run.

## Live inference cost estimate (catalog rates; paid from subscription quota)

Pi catalog `claude-haiku-5-5` (USD per million tokens): input 0.10, output 0.50, cache read 0.01, cache write 0.125. Above 100,000 input tokens per request: input 0.50, output 2.50, cache read 0.05. One-hour cache writes settle at twice the tier input rate (record W1–W4). Assumptions: 2.6 bytes per token (live-c2: about 159 KB of members was about 61.6k input tokens = 58,910 written + 2,716 read; `records/dev-vertical-h2-route-20261117/README.md:399-404` at `5fb03526`), +10% for JSON escaping, about 3k tokens of Claude Code overhead, thinking counted as output.

| Run | Context (tokens) | Estimate |
| --- | --- | --- |
| Advisory (A4 material 112.5 KB, fixture size) | about 51k | $0.01–0.03 |
| Baseline audit (45.8 KB) | about 22k | $0.01–0.02 |
| Treatment audit (126 KB with the ledger range + A4) | about 105k (above-100k tier) | $0.10–0.25 |
| Verification by the root (its own model; at `claude-opus-5-5` rates of 4/20/0.2/5) | root context + about 25k new | $0.5–1.5 |
| **Total** | | **about $0.6–1.8** |

Reference: live-c2 settled a 159 KB read job at 13,414 µUSD (351 + 703 + 12,360; same README lines). Job reservations (1,320,001 µUSD per request, held in advance) are not spend. Preparation, coordination, dev-vertical sizing and building the live variants are not estimated (unknown); LC-7 cost accounting includes them. The real card's size is unknown, and a hex-heavy card tokenizes worse; at 2 bytes per token the treatment arm costs about $0.25.

## Not established

- No build, test, native run or inference took place.
- The relay profile has never run live.
- A real-register card has never been produced.
- Treatment-arm fit within the bounds is unknown.
- Task-host grant issuance (R8′) is not built or claimed; the first H2's grants are composition-issued under O1 (operator ruling, option (a)).
- The root's verification model has not been named.
- Discrepancies are listed in `gate-trace.md` (last section).
