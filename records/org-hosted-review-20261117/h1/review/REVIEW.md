# H1 code review: `work-engine-review-hosted`

Reviewer: Fable 5.2, read-only, fresh context, for Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. Subject: worktree
`wt-org-hosted-review-20261117`, uncommitted on `700f45e3`. Paths are under `rust/crates/work-engine-review-hosted/` unless
rooted; `analyze.py` is `records/dev-vertical-h2-route-20261117/analyze.py`.

**Verdict: `changes_required`.** No structural defect and no P1. The operator conditions hold on the input side. Three
bounded changes belong before a live request: secret-rule gaps (P2-1), the checker does not bind the served snapshot, job
id or limits to the record (P2-2), and the realization claim is not compared with the retained bodies (P2-3).

Read: all 8 `src/` files and both bins in full; template, CLAIMS.md, DERIVATION.md; `tests/common/mod.rs`,
`tests/subject.rs:210-318`, `tests/check.rs:1-130`; a `diff` of the two requirements files; 2 of 9 `mutation.json` (M3,
M6); both custody summaries; the Host lines cited. Regex behaviour was assessed by reading the patterns; nothing was built
or executed, so each gap in P2-1 is `unverified` by execution.

## P2

**P2-1. Secret rule set `v1` misses common forms** (`src/secrets.rs`). Fix as `v2`, one positive test per alternative.
- (a) `secret-assignment` (`:82`) needs a quoted literal directly after the first `:`/`=`. It misses unquoted values
  (`.env`, `export X_TOKEN=…`, YAML `password: …`), typed declarations (`const API_KEY: &str = "…"`: the `:` is followed
  by a type; `tests/subject.rs:306-307` keeps typed forms quiet on purpose), values with whitespace, and call or header
  forms (`.header("x-api-key", "…")`, `Authorization: Bearer …`). Fix: allow an optional type before `=`; add an unquoted
  alternative for token-shaped values (≥ 16 of `[A-Za-z0-9+/=_.-]` with a letter and a digit, to end of line); add
  `Bearer\s+[A-Za-z0-9._~+/-]{20,}`.
- (b) `credential-token-prefix` (`:68-75`) lacks JWT (`eyJ[A-Za-z0-9_-]{10,}\.eyJ`), Stripe `[sr]k_live_`, AWS `ASIA`,
  `hf_`, Slack `xapp-`/`xoxe`, `AGE-SECRET-KEY-1`, `PuTTY-User-Key-File-`. Whether the Pi and Codex auth files hold JWTs
  is `not established` (excluded paths).
- (c) No member-path rule: `scan` (`:133-154`) uses `path` only in the report. Add `sensitive-member-path`: `.env*`,
  `*.pem`, `*.key`, `*.p12`, `id_rsa*`, `id_ed25519*`, `.netrc`, `.npmrc`, `auth.json`, `credentials*`.
- (d) `private-path` (`:89`): `/var/lib/work-engine` is absent though it is on this objective's never-read list; the
  trailing class misses `~/.codex-api`, `~/.claude.json` and `~/.pi.` at a sentence end (fix: end with
  `(?:[^A-Za-z0-9_]|$)`). Other `/home/<user>/…` paths pass by design; whether a local user name is private material is
  the operator's call.
- (e) A deleted binary file is not a member (`src/subject.rs:213`), but its old content is in `.review/patch.diff` as
  base85 that no rule can read. Fix: refuse a patch containing a `GIT binary patch` line.

**P2-2. The checker binds the task to the run, and nothing else.** `check` recomputes contract and task and requires the
task as the prefix of every retained body (`src/check.rs:280-340`). It does not show that the snapshot served was the one
this crate vetted, or that the job ran with that id and those limits: `tests/check.rs:38-49` reaches `checks_passed` with
a record whose manifest is one synthetic member beside the real frozen subject. Consequence: a wrong directory passed to
`analyze.py` sends content the secret rules never saw, and the envelope still reports `checks_passed` for the frozen
subject, with grounding (`:396`) counted against the record, not against what the Host served. Fix: the bin takes
`--run-inputs <base>/inputs` and `--job <base>/job.json` and adds four failing checks:
- `snapshot_served`: `snapshot-manifest.json` (`analyze.py:64`, pinned by the job at `:102`) = `record.snapshot_manifest`;
- `subject_bound_to_scope`: `subject.patch_identity` = the preparation's `scope_digest` = the manifest digest of
  `.review/patch.diff` (design review P3-4);
- `job_id_bound`: `record.job_id` = the Host job id. `analyze.py:88` derives it from the run directory name, so
  `--job-id` must be that string, else the contract's `attempt_id` names no real attempt;
- `limits_bound`: `record.limits` = `packet.json` (`analyze.py:96-97`; env defaults 8 and 16 at `:23-24`), and
  `authority.json` `task_sha256` (`:94`) = `record.task_sha256`.

**P2-3. The realization claim is not compared with the retained bodies** (design review P1-2(b), adopted by the packet).
`realization_claim` (`src/check.rs:275-279`) compares the record with the same crate constant, and `user_texts`
(`:139-157`) is the only reader of a body. Fix: in `delivery`, require each body's `model` = `REQUESTED_MODEL` and its tool
names = `HOSTED_TOOLS`. The Host enforces both (`work-engine-manual-host/src/pi_coding_hosted.rs:611`;
`pi_coding_analysis.rs:29-32`), so this adds evidence to the envelope, not a control.

## P3

1. **Serving route unreported.** Live routes include a secondary payer `openai-api` at `https:openai-responses`
   (`analyze.py:73-76`). The envelope's `provider` is a constant (`src/check.rs:427`) and the proposition's "via provider
   openai-codex" is the request frame's provider id (`pi_coding_provider.rs:650`), not the serving endpoint. Carry
   `hosted_attempts`, `settlement`, `provider_requests`, `snapshot_deliveries`
   (`pi-pressure-context-host/src/analysis.rs:293-302`) into the envelope as reported; reword CLAIMS.md:18-20.
2. **Unknowns entered as values.** A failed recompute gives `undischarged_blockers: 0` and `run_id: ""`
   (`src/check.rs:310`) and `known_incompatibilities: 0` (`:320`). Use `Option` (null) for all three.
3. **`git` inherits the caller's environment** (`src/subject.rs:64-79`; `tests/common/mod.rs:20-34`). With `GIT_DIR` set
   (git sets it for hooks), production reads another repository than `--repo` and the tests' `init`/`add`/`commit` act on
   it, contrary to "No test touches a real repository". Fix: `env_remove` `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`,
   `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES` in both; set `GIT_NO_REPLACE_OBJECTS=1` in production.
4. **Member paths are unchecked** beyond UTF-8 and `.review/` (`src/subject.rs:202-233`) and `Snapshot::write` joins them
   (`:50`). Refuse `..`, `.git` and empty components.
5. **"Every refusal leaves no output behind"** (`src/lib.rs:44`) is false for an I/O failure after `create_dir(out)`
   (`src/inputs.rs:181-190`). Write to a sibling temporary directory and rename, or narrow the sentence.
6. **Read budget is not checked against the snapshot.** One read returns ≤ 24 576 bytes (`pi_coding_analysis.rs:22`) and
   exceeding the read limit fails the job with no answer (template `:11`). `inputs::build` should refuse when
   `ceil(len(.review/patch.diff) / 24576) > max_reads` and record the sum over all members; else the failure follows spend.
7. **Delivery prefix** (`src/check.rs:194`): also require `texts[0].len() == task.len() + HOST_SUFFIX_BYTES`, which copies
   no suffix text and stops extra user text after the task.
8. **Test depth.** One positive per secret rule (n = 1 per rule; 1 of 8 token alternatives exercised,
   `tests/subject.rs:214-241`). No recorded mutant covers the context-fit bound, prompt bound, tree mismatch, parent
   shape, answer digest, subject equality, attempt-digest equality, member count or four of the five secret rules.
9. **CLAIMS.md:24** "zero tool requests" sits beside two tools the template tells the reviewer to call; name the Host
   counter meant. CLAIMS.md:21-22 (grounding) is against the record's manifest until P2-2 is in.

## Checked and found as claimed

- **Revision.** `diff` shows exactly five changes: `runId`, two propositions, two removed `realizationConstraints` blocks
  (each `provider_family` + `execution_confinement`). The frozen file is absent from `git status` (3 entries) and is the
  22nd path in `main-1/hashes.stdout` (2 workspace + 19 crate + 1 frozen).
- **Subject.** Full lowercase ids, `^{commit}`, `^{tree}` equality, exactly one parent, patch recomputed with the defining
  command, `scope.digest` equality (`src/subject.rs:138-180`, fed from the preparation, `src/inputs.rs:84-88`). Changed
  files, removed lines (via the patch) and `.review/subject.json` are scanned; hits carry rule, path and line only.
- **Bounds equal the Host's.** 1 MiB, 256 members, 256-byte paths (`pi_coding_analysis.rs:23-24`, `:40`); 400 000 (`:16`);
  32 768 (`pi-pressure-context-host/src/analysis.rs:26`, `:75`); suffix 202 bytes (`:42-45`; my count, 200 + `\n\n`).
- **Claim and refusal.** Provider (`pi_coding_provider.rs:425`, `:650`), model (`:459`), tools (`pi_coding_analysis.rs:9`).
  Refusal precedes `freeze` (`src/revision.rs:95-98`); the frozen world must refuse (`:121-132`); the bin passes only the
  embedded revision (`src/bin/review-job-inputs.rs:60-61`).
- **Checker.** No trim or repair before `parse_json` (`src/check.rs:350`); `NotRun` fails the outcome (`:410-413`);
  standing fields are literals (`:417-420`). No path writes a bare review: the only writers in the 8 source files and 2
  bins are `inputs::write`, `Snapshot::write` and the envelope write (`src/bin/review-result-check.rs:30-35`).
- **Template** tool description matches the Host (`pi_coding_analysis.rs:114`, `:121-123`, `:158-159`).
- **Custody.** `main-1`: 9 of 9 steps exit 0, 34 tests. `mutants-1`: 9 of 9 `.test` steps exit 101, `restored.test` 0, no
  compile error in the nine `.test.stderr`. M3 and M6 are faithful as read; the other seven are from MATRIX.md only.

## Not established

- The shape of a real retained body against `user_texts` (builder deviation 2), and `pi-state.json` beyond the two members
  `analyze.py` reads. Run one offline `analyze.py` scenario through `review-result-check` before any live request.
- Whether the real subject `96808de3…` exists, fits the bounds, or holds material the rules miss (root H0). Until P2-1 is
  in, a manual read of that one commit's diff is the only cover for the forms listed there.
- `settlement` classes; whether `fresh_initial` is what the Host realizes.
- Not read: `d3/DESIGN.md`, `tests/task.rs`, the bodies of `tests/revision.rs` and `tests/check.rs:131-365` (names and
  assert lines only), seven `mutation.json`, the coordination board.
- Context reading: `usage_unavailable` (no binding given). Cost of this review: not observable by me.
