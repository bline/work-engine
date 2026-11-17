# Independent review packet: hosted Pi transport, review remediation and offline qualification (phase 4 pieces (b) remediation and (c))

## Reviewer authority and constraints

- Reviewer: `claude-fable-5-2` at `high` effort, fresh context. The operator
  gave Fable the independent review authority intended for the inherited
  Astra/xhigh review. This is a same-provider review; a Claude Opus session
  wrote the code.
- **Read-only.** Do not modify any file. Do not run builds, tests, Docker or
  network operations, and do not access credentials. Reading files, `git
  show`, `git diff`, `git log` and `rg` are fine.
- **Subject:** the changes from `c66f2baf` (the subject of your previous
  review) to `acb80187`, restricted to the dev-vertical paths listed in
  `subject-manifest.txt`. Other commits in that range belong to other
  sessions and are out of scope. Use `git show SUBJECT:<path>` wherever the
  working tree differs.
- **Scope:** check that your previous findings were actually fixed, review
  the new code, and judge whether the offline qualification supports what its
  record claims. Prefer the smallest change that closes a real gap.

## Read first

1. `reviews/dev-vertical-phase4-piece-b-fable-20261116/REVIEW.md`: your
   previous review (D1–D6, K-a..K-h).
2. `records/dev-vertical-hosted-review-fixes-20261116/README.md`: the
   claimed remediation.
3. `records/dev-vertical-hosted-qualification-20261117/README.md`: the
   piece (c) qualification, with `qualify.py`, `check.py` and `matrix.json`.
4. `planning/dev-vertical-hosted-gate-contract-20261116.md`: the lifecycle
   gate contract.

## Operator decisions since your previous review

- **K-a (2026-11-17):** a hosted attempt is Completed when a screened
  `response.completed` was relayed (status completed, no failure event) and
  only SSE comments or blank lines follow it, even if the HTTP framed end is
  not observed within the release drain. The attempt record then says
  `framed_end_observed: false`. A `data:` event after the terminal event, or
  anything ending before a terminal event, stays Uncertain. Phase 6 measures
  the live backend's close timing and may tighten this.
- **D4 (response bound)** is deferred to phase 6, before any live provider
  use.
- The binding contracts from your previous packet are unchanged: the grant
  bounds routes, payers, attempt count and deadline; per-attempt identity and
  reservation; only the ledger releases money; complete screened SSE
  retention; no network or access material in the inference container.

## Code (changes since `c66f2baf`)

- `rust/crates/work-engine-manual-host/src/pi_coding_hosted.rs`
- `rust/crates/work-engine-manual-host/src/pi_coding_provider.rs`
- `rust/crates/work-engine-manual-host/src/pi_stream.rs` (`Relay.unforwarded`,
  `terminal_is_last`)
- `rust/crates/work-engine-manual-host/src/pi_docker.rs`
- `rust/crates/pi-pressure-context-host/src/coding.rs` and
  `src/bin/coding-qualification.rs`
- `rust/crates/work-engine-manual-host/examples/pi_hosted_coding_observer.rs`
- Tests: `pi_coding_hosted.rs` (tests module), `pi_coding_provider.rs`
  (`attempt_tests`), `tests/pi_stream.rs`,
  `pi-pressure-context-host/tests/coding.rs`.

## Focus areas

1. **Remediation.** Is each of D1, D2, D3, D5 and D6 fixed as claimed, with no
   regression in financial correctness or grant coverage?
2. **K-a implementation.** Does `terminal_is_last` plus the ledger rule settle
   only when the operator decision allows? Check SSE edge cases: CR/LF
   variants, multi-line `data`, `event:`/`id:` fields, a terminal event split
   across chunks, held-back screening bytes, bytes read while draining, and a
   `response.completed` followed by a second one. Can a stream be both settled
   and not fully retained?
3. **Crash and reconcile.** `coding_hosted_reconcile_unsettled` and the
   `crash` scenario. Can anything be released, replayed or lost?
4. **Evidence integrity.** Retained `stream.sse`, `refusal.body`,
   `relay.json` and `unforwarded.bin`, journal artifact registration and
   receipt verification; `relay.json` for Host-closed streams.
5. **Qualification adequacy.** Do the 14 scenarios and `check.py`'s checks
   support the record's claims? Name important unexercised paths, and any
   check that passes for the wrong reason.

## Required output

Write your review as your final answer, in Markdown:

1. A **disposition**: `accept`, `accept with required changes`, or `reject`,
   for the offline hosted transport as the basis for phase 6.
2. **Findings.** For each: severity (P1/P2/P3), evidence (`path:line` at
   `acb80187`), why it matters, and the minimal change. Separate confirmed
   defects from risks and open questions.
3. The status of each previous finding (D1–D6, K-a..K-h).
4. Anything you could not verify, and why.

Be concise. Do not restate the design.
