<!-- Root-saved verbatim from reviewer a35990e4's final message at 2026-11-18T17:14:46.838Z; the reviewer's own writes were refused by its permission rule (root configuration error). -->
# H1 recheck review 2: `work-engine-review-hosted` (remediations 1–5)

Reviewer: Fable 5.2, read-only, fresh context, for Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. Subject: worktree `wt-org-hosted-review-20261117`, uncommitted on `700f45e3`. Paths are under `rust/crates/work-engine-review-hosted/` unless rooted. Host paths are `/home/bline/code/work-engine/rust/crates/…` at `5af34cbf`. `e2e/` and `h1/` are under `/home/bline/cbm-work/org-hosted-review-20261117/`.

**Verdict: `accept_with_changes`.** No P1 or P2 in what was read. P3-1 and P3-2 are cheap and worth doing before the live run; none blocks it.

I ran only `sha256sum`, `diff`, `jq`, `wc`, `ls` and `head`; no build, no tests. Regex behaviour is assessed by reading the patterns.

## Findings (all P3)

**P3-1. The Host suffix is checked by length only.**
- `src/check.rs:270-283` accepts any 202 bytes after the task as the Host suffix.
- The suffix is one fixed sentence (`pi-pressure-context-host/src/analysis.rs:42-47`), and the container authors the body. The Host does not check that `messages[0]` holds the task (`work-engine-manual-host/src/pi_coding_hosted.rs:791-845`).
- Consequence: a body whose first text block is the task plus 202 bytes of other instruction passes `delivery`.
- Fix: add `HOST_SUFFIX_SHA256` in `src/task.rs` and require `sha256(text[task.len()..])` to equal it in `task_with_suffix`, on both wires. No suffix text is copied.

**P3-2. `job.json`'s provider and model are not compared with the profile.**
- `run_binding` (`src/check.rs:575-624`) reads `job_id` and `workspace.manifest` only.
- The real `e2e/org-step3-offline-1/job.json` carries `coding.provider.provider: "claude-sdk"` and `coding.provider.model: "claude-haiku-5-5"`.
- Fix: add a failing check `job_profile_bound` requiring both to equal `profile.provider()` and `profile.model()`.

**P3-3. A deleted file with a sensitive name is not refused.**
- `src/subject.rs:227` skips status `D` before any path rule; `sensitive_path` runs only on members (`src/secrets.rs:196`).
- The deleted content still leaves in `.review/patch.diff`, where only the line rules see it. A deleted `.env` holding `DATABASE_URL=…` passes.
- Not reachable today: `scope.digest` pins one subject (`src/subject.rs:189-194`), whose patch is one modified Rust test file.
- Fix: apply `secrets::sensitive_path` to every raw-diff path before `"D" => continue` and refuse with `sensitive-member-path`.

**P3-4. `CLAIMS.md:94-96` is stale.**
- It says the shape of a real Claude Code body is `not established`. Three real bodies now exist.
- Fix: say what is established (n = 1 offline run, 3 requests, canned observer answers) and what is not (a live model's bodies).

**P3-5. Insertion digests hash a re-serialization, under a label that overstates their source.**
- `value_sha256` (`src/check.rs:286-288`) hashes `serde_json::to_vec(&Value)`, not the body's bytes. The result depends on `serde_json/preserve_order`.
- No workspace manifest names `preserve_order` or `indexmap` (grep of `rust/Cargo.toml` and `rust/crates/*/Cargo.toml`, 0 matches). A transitive enabler is `not established`; I did not search `Cargo.lock` or third-party manifests.
- The digests sit under `host_reported`, but they are container-authored bytes the Host retained.
- Fix: document "compact, sorted keys" at `:380-384`, pin it with a test on a two-key object, and say in the doc that these are retained container bytes.

**P3-6. The standing of r1 after decision D is not stated in the operator's words.**
- `h1/remediation-2/PACKET.md:7` calls decision D a supersession of "the earlier D2 removal of both constraints". `fixtures/hosted-route-qualification-r2/DERIVATION.md:9-10` narrows that to "for this revision".
- `--provider openai-codex` still emits job inputs on r1 (`src/revision.rs:111-114`, `CLAIMS.md:10`, `:13-16`).
- Whether r1 stays authorized for a live run is `not established` from the quoted text; it is the operator's call.
- Fix: one sentence in `CLAIMS.md` quoting the operator on r1. Until then, do not run the `openai-codex` profile live.

**P3-7. Cosmetic.** The `frozen_world_unmet` detail prints Rust `Some(0)` (`src/check.rs:728-733`; `e2e/org-step3-offline-1.envelope-r5.json:152`). Print the number or `not established`.

## Priority 1: first-review findings

| Finding | State | Where |
|---|---|---|
| P2-1 (a) assignment forms | closed as specified | `src/secrets.rs:109-127` |
| P2-1 (b) token prefixes | closed | `src/secrets.rs:88-105` |
| P2-1 (c) member-path rule | closed for members; deleted files are P3-3 | `src/secrets.rs:183-191`, `:196` |
| P2-1 (d) private paths | closed | `src/secrets.rs:130-132` |
| P2-1 (e) binary patch | closed | `src/secrets.rs:140` |
| P2-2 run binding, four checks | closed; all four pass on the real run | `src/check.rs:575-624`; envelope-r5 `:155-173` |
| P2-3 model and tools against bodies | closed on both wires | `src/check.rs:357-364`, `:417-431` |
| P3-1 host-reported fields | closed | `src/check.rs:146-157`, `:854-860` |
| P3-2 unknowns as null | closed | `src/check.rs:129`, `:137`, `:176`, `:694-698` |
| P3-3 git environment | closed, no test; carried truthfully (`h1/remediation-1/RESULT.md:81`) | `src/subject.rs:65-79` |
| P3-4 member path components | closed | `src/subject.rs:285-293` |
| P3-5 no output on failure | closed | `src/inputs.rs:217-251` |
| P3-6 read budget | closed | `src/subject.rs:334-348`, `src/inputs.rs:108` |
| P3-7 suffix length | closed as asked; content is P3-1 above | `src/check.rs:276-281` |
| P3-8 test depth | partly closed; remainder carried truthfully (`h1/remediation-1/RESULT.md:69-71`) | — |
| P3-9 CLAIMS wording | closed | `CLAIMS.md:60-65`, `:46-47` |

Residual in rule set v2, by reading: unquoted values under 16 characters or without a digit, and credentials inside a URL, pass. This is unreachable while one subject is pinned.

## Priority 2: decision D

- **Diff is exact.** `diff` of the frozen fixture against r2 shows `runId`, two `proposition` strings and two removed `execution_confinement` blocks. `provider_family: ["anthropic"]` is kept in both conditions.
- **Tests pin it.** `tests/revision.rs:238-324` asserts the exact five differing paths and the kept constraint. `:327-392` asserts no `KnownIncompatibility` for the claude-sdk claim on r2.
- **Frozen fixture untouched.** Its SHA-256 is `7085318950ac…3b292` in the worktree, in the main checkout and in `h1/remediation-1/runs/main-1/hashes.stdout:22`. Session-start `git status` listed 3 entries: two workspace files and the new crate.
- **Only r2 yields claude-sdk job inputs** (`src/inputs.rs:93-99`). `tests/revision.rs:415-448` expects `Refusal::Requirements` for the frozen requirements, which by reading would fail without that guard. No mutant is recorded for it.
- **Real run.** 0 known incompatibilities; frozen world `prepared_conditions_unmet` with 2 unestablished confinement blockers; 18 undischarged blockers (envelope-r5 `:15-20`).
- **Nothing consumable.** The standing fields are literals (`src/check.rs:834-837`), and the bin writes only the envelope (`src/bin/review-result-check.rs:40-47`).

## Priority 3: Messages delivery

The three real bodies, by `jq` structure dump:

| Element | Observed |
|---|---|
| `model` | `claude-haiku-5-5` |
| tools | `mcp__pi__snapshot_list`, `DeferredToolPlaceholder`, `mcp__pi__snapshot_read` |
| `messages[0]` | one text block of 29,374 bytes = `task.txt` 29,172 + 202 |
| `messages[1]` | role `system`; a text-block list in request 1, a string in requests 2 and 3 |
| later user turns | one `tool_result` block each |
| `<total_tokens>` messages | role `system` at `[4]` and `[7]`, each directly after a tool-result user turn |

- **Accepts the real bodies.** `messages_body` (`src/check.rs:409-515`) accepts this shape, and envelope-r5 records `delivery: passed`.
- **Matches the Host on model and tools.** Model: `pi_coding_hosted.rs:732`. Tool names, with duplicates and unnamed entries refused: `pi_coding_analysis.rs:130-143` against `src/check.rs:420-431`.
- **Thinking is never answer or evidence.** The checker does not read assistant turns (`src/check.rs:493`). The answer is the `answer` field of `analysis-result.json`, which the Host builds from text blocks only (`lifecycle-store/src/pi_analysis_proof.rs:61-78`).
- **Accepts a body the Host would not produce:** only P3-1.
- **Refuses a body the Host produces:** none in n = 1 run. A live model's bodies are `not established` (thinking blocks, several tool results in one turn, other Claude Code insertions). The checker runs on retained state after the job, so a refusal there costs a checker rerun, not a second provider run.

## Priority 4: profiles

- The profile is a closed enum of two with exact-name parse (`src/revision.rs:72-86`).
- The bin refuses any other provider (`src/bin/review-job-inputs.rs:51-54`).
- The record is `deny_unknown_fields`, and its `provider` and `requested_model` strings are compared with the profile (`src/check.rs:652-656`).
- Unchanged since remediation 1, by hash: r1 fixture, r1 DERIVATION, template v1, `secrets.rs`, `subject.rs`, `lib.rs`, `review-result-check.rs`, `tests/common/mod.rs`, `tests/task.rs`.
- The two templates differ only in the template id and the four tool-name lines (`diff`).

## Priority 5: tests, mutants, custody

- **Custody.** `sha256sum -c h1/remediation-5/final-hashes.txt` from the worktree root: 24 of 24 `OK` (22 crate files, 2 workspace files; expected 24).
- **M3 (first text block).** The record's edit matches `src/check.rs:445`, and its killer case is at `tests/check.rs:793-796`.
- **M1 (model).** The killer test at `tests/check.rs:709-724` is real; I did not open the M1 record.
- **M2 and R5-M1.** The edits named in the RESULT files correspond to `src/check.rs:427` and `:468`, and tests of those names exist at `tests/check.rs:727` and `:1040`. I did not read those test bodies or the records.
- **Carried truthfully by the builders.** No custody `runs/` for remediations 2–5; no mutated-file hash for R5-M1; n = 1 mutant per property.

## Priority 6: what reaches the provider

- **Crate-authored material is clean for this subject.** The task, contract, requirements and snapshot of the real run (`e2e/org-step3-offline-1-inputs/`, recursive) have 0 matches for `/home/|~/|@[A-Za-z]|/etc/|/var/lib|bline`.
- **Claude Code's own insertions (body 3, parts read):** the third system block (read in body 1), the `messages[1]` environment message, `metadata`, and the tool-use and tool-result turns. They hold container paths (`/workspace`, `/pi/node_modules/…`), the host kernel version (`Linux 6.17.0-35-generic`), the date, a `device_id` digest and an empty `account_uuid`. None is a secret. The kernel version is host information the crate does not control; it is recorded by digest only.

## Not established

- `h1/MATRIX.md`, `h1/remediation-3/` and `h1/remediation-5/test-output.txt` (the 60-test count): not read.
- `tests/check.rs:816-1215`: names and positions only.
- The Host lines cited for the `anthropic` family claim (`src/revision.rs:139-145`): not opened.
- Anything about a live model's request bodies.
- Cost of this review: not observable by me.