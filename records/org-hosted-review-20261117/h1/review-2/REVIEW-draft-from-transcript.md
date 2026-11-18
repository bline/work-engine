<!-- Root-extracted: reviewer a35990e4's denied Write of REVIEW.md at ? (permission rule refused the path); content verbatim below. -->
# H1 recheck review 2: `work-engine-review-hosted` (remediations 1–5) — PROVISIONAL

Reviewer: Fable 5.2, read-only, fresh context, for Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. Subject: worktree
`wt-org-hosted-review-20261117`, uncommitted on `700f45e3`. Paths are under `rust/crates/work-engine-review-hosted/`
unless rooted; Host paths are `/home/bline/code/work-engine/rust/crates/…` at `5af34cbf`; `e2e/` and `h1/` are under
`/home/bline/cbm-work/org-hosted-review-20261117/`.

**Verdict so far: `accept_with_changes` (PROVISIONAL).** No P1 and no P2 found in what was read. Six P3. The items under
"Not yet verified" could still change the verdict.

Nothing was built or executed by me except `sha256sum`, `diff`, `jq`, `wc`, `ls`. Regex behaviour is assessed by reading.

## Findings (all P3)

**P3-1. The Host suffix is length-checked only.** `src/check.rs:270-283` accepts any 202 bytes after the task as "the
Host suffix". The suffix is a fixed sentence with no variable part (`pi-pressure-context-host/src/analysis.rs:42-47`),
and the container, not the Host, authors the body. Consequence: a body whose first text block is the task plus 202
bytes of other instruction passes `delivery`. Fix: add `HOST_SUFFIX_SHA256` in `src/task.rs` and require
`sha256(text[task.len()..]) == HOST_SUFFIX_SHA256` in `task_with_suffix` (no suffix text copied; both wires).

**P3-2. A deleted file with a sensitive name is not refused.** `src/subject.rs:227` skips status `D` before any path
rule; `secrets::sensitive_path` runs only on members (`src/secrets.rs:196`). The deleted content still leaves in
`.review/patch.diff`, where only the line rules see it (a deleted `.env` holding `DATABASE_URL=…` passes). Not reachable
today: `scope.digest` pins one subject (`src/subject.rs:189-194`), whose patch is one modified Rust test file. Fix: in
`materialize`, apply `secrets::sensitive_path` to every raw-diff path before `"D" => continue` and refuse with
`sensitive-member-path`.

**P3-3. `CLAIMS.md:94-96` is stale.** It says the shape of a real Claude Code body is `not established`. The root now
holds three real bodies (`e2e/org-step3-offline-1/state/coding-hosted-{1,2,3}.body.json`). Fix: state what is
established (n = 1 offline run, 3 requests, canned observer answers) and what is not (a live model's bodies: thinking
blocks in assistant turns, several tool results in one user turn, any other Claude Code insertion).

**P3-4. Insertion digests are of a re-serialization, and the label overstates their source.** `value_sha256`
(`src/check.rs:286-288`) hashes `serde_json::to_vec(&Value)`, not the body's bytes; the result depends on whether
`serde_json/preserve_order` is enabled in the build (`not yet verified`). The field doc (`:380-384`) says "`serde_json`
serialization" without the key order. They sit under `host_reported`, but they are container-authored bytes the Host
retained, not facts the Host reported. Fix: document "compact, sorted keys" (the Host's own convention,
`work-engine-manual-host/src/pi_coding_analysis.rs:31`) and pin it with a test on a two-key object; name the group
`retained_request_insertions` or say so in the doc.

**P3-5. Standing of r1 after decision D is not stated as the operator's.** `h1/remediation-2/PACKET.md:7` describes
decision D (2026-11-18) as superseding "the earlier D2 removal of both constraints"; `fixtures/…-r2/DERIVATION.md:9-10`
narrows that to "for this revision", and `--provider openai-codex` still emits job inputs on r1
(`src/revision.rs:111-114`, `CLAIMS.md:10`, `:13-16`). Whether r1 remains authorized for a live run is the operator's
call and is `not established` from the quoted text. Fix: one sentence in `CLAIMS.md` quoting the operator on r1's
standing; until then the root should not run the `openai-codex` profile live.

**P3-6. Cosmetic.** `frozen_world_unmet` detail prints Rust `Some(0)` (`src/check.rs:728-733`; visible in
`e2e/org-step3-offline-1.envelope-r5.json:152`). Print the number or `not established`.

## Priority 1: first-review findings

| Finding | State | Where |
|---|---|---|
| P2-1 (a) assignment forms | closed as specified | `src/secrets.rs:109-127` |
| P2-1 (b) token prefixes | closed | `src/secrets.rs:88-105` |
| P2-1 (c) member-path rule | closed for members; deleted files: P3-2 above | `src/secrets.rs:183-191`, `:196` |
| P2-1 (d) private paths | closed | `src/secrets.rs:130-132` |
| P2-1 (e) binary patch | closed | `src/secrets.rs:140` |
| P2-2 run binding (4 checks) | closed; passes on the real run | `src/check.rs:575-624`; envelope-r5 `:155-173` |
| P2-3 model and tools vs bodies | closed, both wires | `src/check.rs:357-364`, `:417-431` |
| P3-1 host-reported fields | closed | `src/check.rs:146-157`, `:854-860` |
| P3-2 unknowns as null | closed | `src/check.rs:129`, `:137`, `:176`, `:694-698` |
| P3-3 git environment | closed, no test (carried truthfully, `h1/remediation-1/RESULT.md:81`) | `src/subject.rs:65-79` |
| P3-4 member path components | closed | `src/subject.rs:285-293` |
| P3-5 no output on failure | closed | `src/inputs.rs:217-251` |
| P3-6 read budget | closed | `src/subject.rs:334-348`, `src/inputs.rs:108` |
| P3-7 suffix length | closed as asked; content unpinned: P3-1 above | `src/check.rs:276-281` |
| P3-8 test depth | partly closed, remainder carried truthfully (`h1/remediation-1/RESULT.md:69-71`) | — |
| P3-9 CLAIMS wording | closed | `CLAIMS.md:60-65`, `:46-47` |

Residual in rule set v2, by reading: unquoted values under 16 characters or without a digit, and credentials inside a
URL, pass. Unreachable while one subject is pinned (see P3-2).

## Priority 2: decision D

- `diff` of the frozen fixture against r2: exactly `runId`, two `proposition` strings, two removed
  `execution_confinement` blocks; `provider_family: ["anthropic"]` kept in both conditions.
- Frozen fixture untouched: SHA-256 `7085318950ac…3b292` in the worktree, in the main checkout and in
  `h1/remediation-1/runs/main-1/hashes.stdout:22`; session-start `git status` listed 3 entries (2 workspace files, the
  new crate).
- r2 is the only revision that yields `claude-sdk` job inputs (`src/inputs.rs:93-99`, bin `:69-71`); the checker
  recomputes from the embedded revision, not the record (`src/check.rs:637`, `:662-663`).
- Real run: `known_incompatibilities: 0`, frozen world `prepared_conditions_unmet` with 2 unestablished confinement
  blockers, 18 undischarged blockers (envelope-r5 `:15-20`).
- Not consumable: literals at `src/check.rs:834-837`; no path writes a bare review (bin `:40-47`).

## Priority 3: Messages delivery

Compared with the three real bodies (`jq` structure dump): model `claude-haiku-5-5`; tools exactly
`mcp__pi__snapshot_list`, `DeferredToolPlaceholder`, `mcp__pi__snapshot_read`; `messages[0]` one text block of 29 374
bytes = `task.txt` 29 172 + 202; `messages[1]` role `system` (text-block list in request 1, string in 2 and 3); later
user turns one `tool_result` each; `<total_tokens>` system messages at `[4]` and `[7]`, directly after a tool-result
user turn. `messages_body` (`src/check.rs:409-515`) accepts exactly this, and envelope-r5 records `delivery: passed`.

Against the Host: model (`pi_coding_hosted.rs:732`), tool names with duplicates and unnamed entries refused
(`pi_coding_analysis.rs:130-143` vs `src/check.rs:420-431`). The Host does not check that `messages[0]` holds the task;
the checker is the only place that does.

Thinking: the checker never reads assistant turns (`src/check.rs:493`); the answer is `analysis-result.json` `answer`
only, which the Host builds from text blocks (`lifecycle-store/src/pi_analysis_proof.rs:61-78`).

Accepts what the Host would not produce: only P3-1. Refuses what the Host produces: none in n = 1 run. A live model's
bodies are `not established`; the checker runs after the job on retained state, so a refusal there costs a checker
rerun, not a second provider run.

## Priority 4: profiles

Closed enum of two (`src/revision.rs:72-86`), exact-name parse, bin refuses anything else (`src/bin/review-job-inputs.rs:51-54`);
record is `deny_unknown_fields` and its `provider`/`requested_model` strings are compared with the profile
(`src/check.rs:652-656`). Unchanged since remediation 1 by hash: r1 fixture, r1 DERIVATION, template v1, `secrets.rs`,
`subject.rs`, `lib.rs`, `review-result-check.rs`, `tests/common/mod.rs`, `tests/task.rs`.

## Priority 5: custody

`sha256sum -c h1/remediation-5/final-hashes.txt` from the worktree root: 24 of 24 `OK` (22 crate files, 2 workspace
files; expected 24).

## Priority 6: what reaches the provider

- Task, contract, requirements and snapshot of the real run (`e2e/org-step3-offline-1-inputs/`, all files, recursive):
  0 matches for `/home/|~/|@[A-Za-z]|/etc/|/var/lib|bline`.
- Claude Code's own insertions, read in body 3: container paths only (`/workspace`, `/pi/node_modules/…`), the host
  kernel version (`Linux 6.17.0-35-generic`), the date, a `device_id` digest and an empty `account_uuid`. None is a
  secret; the kernel version is host information the crate does not control and records by digest only.

## Not yet verified

1. `tests/check.rs:540-1215` (Claude tests): that the assertions are real and the synthetic bodies mirror the real ones.
2. Mutant records `h1/remediation-4/mutants/M1–M3` and R5-M1: faithfulness of the edit and the named killer.
3. `tests/revision.rs:237-448` (r2 derivation, no-KnownIncompatibility, frozen world writes nothing on `claude-sdk`).
4. Whether the `src/inputs.rs:93` guard has a test that would fail without it.
5. `serde_json/preserve_order` in the build (P3-4).
6. `job.json` provider and model against the profile (not compared by `run_binding`).
7. Host lines cited for the `anthropic` family claim (`src/revision.rs:139-145`).
8. `h1/MATRIX.md`, `h1/remediation-3/`, `h1/remediation-5/test-output.txt` (60 tests): not read.
