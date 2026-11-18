# H1 result: hosted review adapter crate (offline)

Role: builder (Opus 5.5, requested; observed runtime identity not recorded by me). Packet:
`h1/PACKET.md`. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`. No children, no
listeners. Status: **all six outcomes built and checked offline; nothing committed** (packet forbids
git mutation). Final status: `map_ready` (see end).

## Entry check

- Worktree `/home/bline/cbm-work/wt-org-hosted-review-20261117`, branch `org-hosted-review-20261117`,
  `git status --porcelain` empty, `HEAD` = `700f45e359e7b66ca7a811b0e19a3181688b79d7`.
- Board read (seq ≤ 5869): worktree claim `22a60c23…` (root, advisory) covers this crate; dev
  vertical 5866: the analysis profile is stable on `claude-realization-20261117`, pin a4 binary
  `150384ad…`; **live gpt-6-luna jobs will most likely be refused on both routes until 2026-11-21
  (subscription quota exhausted, no API credits)**; review jobs stay within the 300 s deadline.
- Context pressure: no identity binding or context-status binary was given to me, so no reading
  was taken: `usage_unavailable` (no binding). No compaction occurred during the task.

## Files (all new unless marked)

| File | Reason |
|---|---|
| `rust/Cargo.toml` (modified, +1) | workspace member |
| `rust/Cargo.lock` (modified, +16) | the new package entry only; no version change (`git diff` shows one added `[[package]]`) |
| `crates/work-engine-review-hosted/Cargo.toml` | lib + bins `review-job-inputs`, `review-result-check`; deps organization, review-preparation, review-delivery (default features, no `fixture`), review-episode-core, serde, serde_json, sha2, thiserror, `regex =1.13.1` (already in the lock); dev `tempfile =3.27.0` |
| `fixtures/hosted-route-qualification-r1/requirements.json`, `DERIVATION.md` | outcome 1: revision (D); quotes the operator decision and date |
| `templates/hosted-review-task-v1.txt` | outcome 3: template `work-engine.review-hosted-task.v1`, sha256 `7ecfbef9…0718` |
| `src/lib.rs` | crate doc (no-overclaim summary), `Refusal`, `sha256_hex` |
| `src/revision.rs` | revision/frozen bytes, `hosted_claim()`, `prepare_world` (prepare → refuse on `KnownIncompatibility` → freeze, transport `{job_id, 1}`), `frozen_world` |
| `src/task.rs` | `render` (digest check, Host limit bounds, placeholders, prompt bound incl. 202-byte Host suffix) |
| `src/subject.rs` | `materialize` (git objects only), `check_members` (form, bounds, secrets), `check_context_fit` |
| `src/secrets.rs` | rule set `work-engine.review-hosted-secret-rules.v1`, 5 named rules |
| `src/inputs.rs` | `build`/`write` job inputs + `JobInputs` record (`work-engine.review-hosted-job-inputs.v1`) |
| `src/check.rs` | checks + `Envelope` (`work-engine.review-hosted-envelope.v1`), `load_state` |
| `src/bin/*.rs` | thin CLIs over `inputs::write` and `check::check` |
| `tests/{common/mod.rs,revision.rs,task.rs,subject.rs,check.rs}` | 34 tests; synthetic git repos in temp dirs only |
| `CLAIMS.md` | outcome 6 |

No other crate edited. Diff stat vs `700f45e3` (tracked): `rust/Cargo.lock | 16 +`, `rust/Cargo.toml | 1 +`;
untracked: the 19 crate files (`runs/main-1/status.stdout`, 21 lines = 2 modified + 19 new; expected 19
crate files, listed by Glob before hashing). File hashes: `runs/main-1/hashes.stdout`.

## Runs (custody, `CARGO_TARGET_DIR=/home/bline/cbm-work/org-r2-core-20261117/target`)

- `h1/runs/main-1/` (plan `h1/plans/main.json`), summary sha256 `b2ba0845…f82b`, **9/9 passed**: fmt
  `--check`; clippy `--all-targets -D warnings`; `test-hosted` 34 passed (check 12, revision 6,
  subject 12, task 4); `test-preparation` 23 + 1 doctest; `test-delivery` 8 + 1 doctest (default
  features); `test-organization` 97 + 4 + 4 + 45 + 6; hashes; diffstat; status.
- `h1/runs/mutants-1/` (plan `h1/plans/mutants.json`), summary sha256 `c4fb979d…6a03`: 9 mutants, each
  `apply` exit 0, `test` exit 101 (killed by test failures, no compile errors), `restore` exit 0
  with SHA-256 verified; `restored.test` passes; `restored.hashes` equal `main-1/hashes` for the four
  mutated files. (The summary says "failed 20/29" because the 9 killed test steps and their
  `--keep-going` accounting count as failures; that is the expected outcome.)

## Matrix summary

`h1/MATRIX.md`: every outcome mapped to tests; 9/9 recorded mutants killed (known-incompatibility
refusal, patchIdentity check, scope-digest binding, size bound, PEM rule, strict parse, contract
recompute, delivery prefix, envelope `consumable: false`). n = 1 mutant per property.

## Measured facts

- Revision contract: 23 621 bytes (job id `job-7`); rendered task 26 919 bytes for job id `job-1`
  (+202 Host suffix = 27 121 ≤ 32 768; margin 5 647 bytes, varies by job-id length).
- Frozen world + hosted claim: refused, 2 known incompatibilities (both `provider_family`; confinement
  unstated → unestablished). Revision world: 18 blockers, all `unestablished`, none discharged.

## Source lines relied on (current text at `700f45e3`; corrections to the design's citations)

| Fact | Line(s) | Design cited |
|---|---|---|
| model must be `gpt-6-luna` | `work-engine-manual-host/src/pi_coding_provider.rs:459` | `:443` (shifted by G1) |
| provider `openai-codex`: catalog entry / request frame | `pi_coding_provider.rs:425`, `:650` | `:628` |
| analysis budget `1..=10_000_000`, requests `1..=32` | `pi_coding_provider.rs:460-463` | `:446` |
| reservation × max_requests ≤ budget | `pi_coding_provider.rs:435-444`; `full_reservation` `:568` | `:419-427`, `:546-555` |
| tools, reads, requests, context, page, snapshot bytes/files, answer | `pi_coding_analysis.rs:9`, `:11-12`, `:16`, `:22`, `:23-24`, `:28` | same, `:22` new |
| snapshot admission (regular files, path ≤ 256, bounds) | `pi_coding_analysis.rs:35-49` (path `:40`) | same |
| `snapshot_list` digest = manifest sha256 of member bytes | `pi_coding_analysis.rs:106-116`, read check `:144-148` | `:93-165` |
| prompt bound 32 768 / check / Host suffix | `pi-pressure-context-host/src/analysis.rs:26`, `:74-75`, `:42-47` (202 bytes incl. `\n\n`, counted with python) | `:74-81` |
| `analysis-result.json` members | `analysis.rs:293-302` | same |
| body retained create-only `coding-hosted-{n}.body.json` | `pi_coding_hosted.rs:521-539` | README:86-89 (per review) |
| body is plain JSON (content-encoding refused) | `pi_coding_hosted.rs:590-603` | — |
| `request_body_sha256` per attempt | `pi_coding_provider.rs:110-114` | — |
| ledger shape read by the checker | `records/dev-vertical-h2-route-20261117/analyze.py:139`, `:146` | — |
| analyze.py inputs: task sha `:94`, snapshot copy + manifest `:55-64`, BUDGET `:25`, request/read env `:23-24` | analyze.py | `:90-95`, `:25` |
| patchIdentity definition | `app-server/tests/supervisor-native-review-hosting-integration.test.mjs:255-257` | `:253-256` |
| realization: allowed check / mutation with tools / capabilities with tools | `work-engine-review-preparation/src/realization.rs:77-105`, `:134-138`, `:166-176` | `:83-97`, `:134-138`, `:171-175` |
| `BlockerClass`; `prepare` | `prepare.rs:202-207`; `:381` | `:611-627`, `:381` |
| `freeze`; accessors | `work-engine-review-delivery/src/lib.rs:144-169`; `:170-182` | same |
| `ReviewResult::parse`, `subject_digest`, evidence form | `review-episode-core/src/implementation_review_v1.rs:31-124`, `:126-128`, `:143-157` | same |
| `parse_json` (trailing whitespace ok, trailing bytes refused) | `review-episode-core/src/codec.rs:223-231` | `:223` |

## Deviations

1. **Delivery check covers every retained request body**, not only request 1 (each must have body
   digest = every attempt's digest, exactly one user text item, prefix = task). Stricter than P3-3.
2. **User text item** = an `input` item with `role: user` and no type or `type: message`; content as a
   string, or each `input_text` part. Inferred from the Responses body shape the Host forwards
   (`pi_native.rs:662-703`, `pi_coding_hosted.rs:31-45`); a real retained analysis body was not read
   (it lives under `~/.local/state/work-engine`, excluded). `not established` against a live body.
3. **`load_state` reads `pi-state.json` only as `analyze.py:139/146` does**; its full schema was not
   read. The root should confirm on the first offline run.
4. **The frozen-world refusal is retained in every successful output** (`frozen-world.json`, and in
   `job-inputs.json`) and recomputed by the checker, rather than through a separate binary mode.
5. **Deleted files** appear only in `.review/patch.diff` (no post-change content). Members under
   `.review/` in the subject commit are refused (layout collision).
6. **patchIdentity recompute mirrors the original command exactly**, with no `-c` overrides; user git
   config that changes `diff-tree` patch output (e.g. `diff.noprefix`) would change the bytes for
   both. Not observed; noted as a dependency.
7. **`secret-assignment` requires a quoted literal** (≥ 8 chars, ≥ 4 distinct, not a placeholder or
   env-var name); unquoted assignments are not matched. Rules have no allow-list: e.g. any e-mail
   address or `~/.claude` mention in the subject refuses the job.
8. Limits are passed in and bounded only by Host maxima (1..=32 requests, 1..=64 reads); computing N
   from the reservation (review P1-1) is the root's H0(d).

## Unfinished / not in scope

- Not run (root's, per packet): `analyze.py`, Docker, observer, any live call, H0 checks on the real
  subject `96808de3…` (existence, size, secret-rule hits, patchIdentity). Whether the real subject
  passes the secret rule set is `not established`; a hit refuses by design.
- DQ-1 (observer answer file in `work-engine-manual-host/examples`) not done: the packet forbids edits
  to other crates.
- Budget/request-count decisions (H0(d), OQ-3) not made.

## Unclaimed

Everything in `CLAIMS.md` "May not claim", and: no live behaviour is shown by this slice (all
evidence is offline unit/integration tests on synthetic inputs); envelope fields are produced by
this crate, not by any owner of admission; the task template's effect on gpt-6-luna's output form is
untested; cost of this session: not observable by me (unknown).

`map_ready` /home/bline/cbm-work/org-hosted-review-20261117/h1/RESULT.md
