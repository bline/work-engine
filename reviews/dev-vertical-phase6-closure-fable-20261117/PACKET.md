# Closure review packet: review 3 remediation and the dev vertical's final record

## Reviewer and constraints

- Reviewer: Fable (`claude-sdk/claude-fable-5-1`, Claude subscription quota),
  fresh context, as a pi subagent. Same-provider review; a Claude Opus session
  wrote the code. The operator assigns Fable to complex reviews and wants this
  review for closure of the dev-vertical milestone.
- **Read-only.** Do not modify repository files, run builds, tests, Docker or
  network operations, or read credential files (`~/.pi/agent/auth.json`,
  `~/.codex/auth.json`, `~/.local/state/work-engine/credentials/`). Reading
  files and run directories under `~/.local/state/work-engine/dev-vertical-*`
  is fine.
- **No git tooling may be available to you.** The complete subject diff is in
  this directory: `subject.diff` = `git diff b50017ab 9b289171` restricted to
  `subject-manifest.txt` (only this vertical's commits `7c103ad2`, `272439ec`,
  `6519dbcc`, `caaa88cf`, `9b289171`; other sessions' commits in that range touch
  none of these paths). The working tree at the time of launch equals
  `9b289171` for these paths except where noted below.
- **Return your review as your final response** (it is saved to
  `REVIEW.md` here). Structure: disposition (can the milestone close, with what
  residuals); for each finding of review 3, closed / partly closed / open with
  evidence (`path:line`); new defects introduced by the remediation (severity,
  evidence, minimal change); whether the record's claims are supported; what you
  did not verify.

## Read first

1. `reviews/dev-vertical-phase6-transport-fable-20261117/REVIEW.md` (M1–M6,
   R-a..R-g) and `reviews/dev-vertical-phase6-retirement-opus-20261117/REVIEW.md`
   (F1–F8): the findings being closed.
2. `records/dev-vertical-hosted-qualification-20261117/README.md`, sections
   "Review 3", "Broad regression", "Not exercised", "Open".
3. `docs/dev-vertical-open-items.md`: the operator's decisions of 2026-11-17 on
   what is deferred or monitored rather than closed.

## Operator decisions after review 3 (2026-11-17)

- Subscription-route registration and permission checks: **deferred**, recorded
  in the open-items doc.
- API-route price parity (R-c): fine as is; tune later.
- Real quota refusal, genuinely revoked credential, live cancel or crash:
  **monitor and log** (`metrics/probes/hosted_live_observations.py`), revisit
  with evidence.
- Self-issued grants (K-c): acceptable for single-operator local use; revisit
  before jobs can be submitted by anyone other than the funder.
- `owner_pending` acceptance: unchanged.
- F2: `session_events.py` is digest-pinned by `session-query` `OWNER_SHAS`; the
  subscription switch was left and is a follow-up.

## Evidence after the remediation

- Offline matrix r8 on `20bb335c` (code `7c103ad2`):
  `~/.local/state/work-engine/dev-vertical-hosted-qual-20261117-r8`,
  `records/.../matrix.json` (19/19).
- Live cutover `c3` on `7c103ad2` through `coding-job-prepare` →
  `work-engine-server` → `work-engine-command`:
  `~/.local/state/work-engine/dev-vertical-cutover-20261117/c3` (`c3.json`,
  `result.json`, `state/pi-state.json`, `live-check.json`).
- Broad regression logs: `~/.local/state/work-engine/dev-vertical-regression-20261117/`
  (`r2.log`, `r2-rerun-tmpdir.log`, `baseline-4ceb14ff.log`).
