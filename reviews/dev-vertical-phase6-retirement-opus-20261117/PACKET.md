# Code review packet: retirement of the credential-mounted live profiles and cutover

## Reviewer and constraints

- Reviewer: Claude Opus at xhigh effort, fresh context, as a pi subagent.
  The operator assigns Opus xhigh to normal code reviews. Same-model review: a
  Claude Opus session wrote the code.
- **Read-only.** Do not modify repository files, run builds, tests, Docker or
  network operations, or read credential files. `git show`, `git diff`,
  `git log`, `rg` and reading files are fine.
- **Subject:** `8fa71188` (retirement and cutover), plus the operating docs in
  `b50017ab`. Compare against `38803cff`. A separate Fable review covers the
  transport, ledger and evidence; focus here on correctness and completeness
  of the removal and the cutover wiring.
- **Write your review** to `reviews/dev-vertical-phase6-retirement-opus-20261117/REVIEW.md`
  (the only file you may write): disposition; findings with severity (P1 to
  P3), evidence (`path:line`), why it matters, minimal change; what you did
  not verify.

## Context

The operator decided (2026-11-16) to remove the credential-mounted live paths
once the hosted profile (`work-engine.pi-coding-hosted.v1`: Host-held access,
Host dispatch, network-none inference container) was qualified. It was
qualified live on 2026-11-17 (`records/dev-vertical-hosted-qualification-20261117/README.md`).
Retired: `work-engine.pi-coding-sdk.v1`, `-lifecycle-live.v1`,
`-lifecycle-sol-high.v1`, `-signed-subscription.v1`. The provider-disabled
fixture profiles (fixture, lifecycle fixture, claims bridge, signed fixture
with its fake provider) must keep working unchanged.

## Questions

1. Is anything that could still mount or project provider access into a
   container, or select a retired schema, left reachable? (Receiver
   `receiver/pi-coding.mjs`, `pi_coding.rs` mounts, `pi_docker.rs` admission,
   `pi_coding_provider.rs`, `examples/mcp_pi_coding.rs`.)
2. Did the removal break or weaken a provider-disabled profile, the signed
   fixture's `PiOwner` path, or the lifecycle gate (`pi-pressure-context-host`
   `coding.rs`, `sol_high` removal)?
3. Are `work-engine-server`'s `verify_live_job` and `coding-job-prepare`'s
   hosted inputs consistent with the Host's own admission (`ProviderPlan::validate`),
   and is the job profile's `env` (`WORK_ENGINE_HOSTED_CREDENTIALS`) safe under
   the profile's existing env rules?
4. Test changes: were deleted tests covering behaviour that still exists, and
   are the replacement assertions meaningful?
5. Docs: do the operating guide and preparation doc match the code?
