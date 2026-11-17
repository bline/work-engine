# Independent review packet: hosted transport after review 2, live routes and cutover evidence

## Reviewer authority and constraints

- Reviewer: `claude-fable-5-2`, effort high, fresh context, as a pi subagent.
  The operator assigns Fable to complex reviews. Same-provider review: a
  Claude Opus session wrote the code.
- **Read-only.** Do not modify repository files. Do not run builds, tests,
  Docker, network operations, and do not read credential files
  (`~/.pi/agent/auth.json`, `~/.codex/auth.json`,
  `~/.local/state/work-engine/credentials/`). Reading files, `git show`,
  `git diff`, `git log`, `rg`, and reading run directories under
  `~/.local/state/work-engine/dev-vertical-*-20261117*` are fine.
- **Subject:** `acb80187` (your previous review's subject) to `b50017ab`,
  restricted to the paths in `subject-manifest.txt`. A separate Opus review
  covers the retirement of the credential-mounted profiles (`8fa71188`) for
  general correctness; review it here only where it touches the transport,
  ledger, gate, or the live class's admission.
- **Write your review** to `reviews/dev-vertical-phase6-transport-fable-20261117/REVIEW.md`
  (the only file you may write), in the structure of your previous review:
  disposition; confirmed defects with evidence (`path:line`), why it matters,
  minimal change; risks and questions; previous findings table (N1–N6, Q-a,
  Q-c, Q-d, K-b..K-h, D4, and the harness weaknesses); what you did not
  verify. Prefer the smallest change that closes a real gap.

## Read first

1. `reviews/dev-vertical-phase4-piece-c-fable-20261117/REVIEW.md`: your
   previous review.
2. `records/dev-vertical-hosted-qualification-20261117/README.md`: the
   claimed remediation, offline matrix r7, live runs, cutover.
3. `planning/dev-vertical-completion-20261115.md` phases 4–6, and
   `planning/dev-vertical-hosted-gate-contract-20261116.md`.

## Operator decisions since your previous review

- N1–N6 and the harness fixes were to be implemented; N3 by incremental
  create-only retention (not narrowed).
- **Q-a:** the tail rule (only SSE comments or blank lines after the first
  `response.completed`) applies whether or not the framed end is observed.
  Phase 6 must check the live backend for a `data: [DONE]` sentinel first.
- **Live use authorized** (2026-11-17), including budget.
- **Retire the credential-mounted live paths** after qualification (decision
  of 2026-11-16, done in `8fa71188`).
- Binding contracts unchanged: the grant bounds routes, payers, attempt count
  and deadline, checked per attempt; per-attempt identity and reservation;
  only the ledger releases money; complete screened SSE retention; no network
  or access material in the inference container.

## Focus

1. **N1–N6/Q-a remediation** (`81c68b3b`, `96fe54de`): `pi_stream.rs` drain
   (screening, accumulation bound, held bytes), `terminal_is_last`, `usage`,
   `AttemptEvidence::from_relay` and `settle_attempt` in
   `pi_coding_provider.rs`; `hosted_emit`, stream file creation and
   registration, `hosted_close_uncertain`, reconcile in `pi_coding_hosted.rs`.
2. **HTTPS transport and per-payer access** (`38803cff`):
   `pi_coding_https.rs` (credential sources, kind/endpoint matching, every
   payer's secret screened, rustls with the platform verifier, no refresh),
   `Endpoint` and header allowlist, connect-before-gate (K-e) and the
   connect-failure path (settled `NotDispatched`), and the embedder binding in
   `coding-qualification.rs` (`WORK_ENGINE_HOSTED_CREDENTIALS`).
3. **Live class admission** (`8fa71188`): `ProviderPlan::validate` and the
   runtime admission in `pi_coding_provider.rs`; `verify_live_job` in
   `work-engine-server/src/job.rs`; `coding-job-prepare.rs` hosted inputs and
   the job-profile `env`.
4. **Evidence adequacy:** whether the offline matrix (r7), the live probe,
   the four live runs and the crash-cut tests support the record's claims; what
   must still be qualified before calling the vertical complete.
