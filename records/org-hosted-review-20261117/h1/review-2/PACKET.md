# H1 recheck packet (read-only, Fable): remediation 1 + retarget (remediations 2–5)

Role: read-only adversarial code reviewer (fresh context). Read `/home/bline/code/work-engine/docs/contracts/core.md` and `docs/contracts/evidence-and-claims.md` §3.9. Parent/collector: Pi root `01a1e56e-be12-7424-977b-c7c7983887d0`; no listeners, no children. Write only `/home/bline/cbm-work/org-hosted-review-20261117/h1/review-2/REVIEW.md`. **Write the review file early and update it as you go** — a cap stop must still leave a verdict.

Subject: worktree `/home/bline/cbm-work/wt-org-hosted-review-20261117` (uncommitted vs `700f45e3`: crate `rust/crates/work-engine-review-hosted`, `rust/Cargo.toml`, `rust/Cargo.lock`). Root verified the files equal `h1/remediation-5/final-hashes.txt` (`sha256sum -c` from the worktree root) and 60 tests pass. Paths below are under `/home/bline/cbm-work/org-hosted-review-20261117/`.

Contract: `h1/PACKET.md` (original outcomes, binding operator text); first review `h1/review/REVIEW.md` (changes_required); remediation-1 packet/result `h1/remediation-1/{MESSAGE.md,RESULT.md}`; retarget packet `h1/remediation-2/PACKET.md` (binding, includes operator decision D of 2026-11-18); `h1/remediation-{3,4,5}/{PACKET.md,RESULT.md}` (4 and 5 hold tests, mutants, hashes); `h1/MATRIX.md`.

Offline end-to-end evidence (root): `e2e/org-step3-offline-1-inputs/` (review-job-inputs --provider claude-sdk on subject 96808de3; snapshot = patch + one changed file + subject.json), `e2e/org-step3-offline-1/` (analyze run: real Claude Code in the container against the pinned offline Messages observer, Host/binaries pinned in `PINS.txt`; `state/coding-hosted-{1,2,3}.body.json` are REAL Claude Code request bodies), `e2e/org-step3-offline-1.envelope.json` (pre-remediation-5: delivery failed on `<total_tokens>` system messages) and `e2e/org-step3-offline-1.envelope-r5.json` (delivery passed; strict_json fails only because the observer's canned answer is not review JSON). Use the real bodies to judge priority 3.

Source of truth for the hosted route: main checkout `/home/bline/code/work-engine` at `5af34cbf` (`rust/crates/work-engine-manual-host`, `rust/crates/lifecycle-store/src/pi_analysis_proof.rs`, `records/dev-vertical-h2-route-20261117/{README.md,analyze.py}`).

Priorities (this crate gates a live provider run on real repository content):
1. Every first-review finding is closed or truthfully carried (cite file:line).
2. Operator decision D: r2 differs from the frozen requirements by exactly `execution_confinement` removed from both conditions, new run id, truthful propositions; frozen fixture untouched; claude-sdk realization against r2 yields no KnownIncompatibility; nothing becomes consumable.
3. Messages delivery check matches what the Host actually retains (body digest, model, exact tool set, task as first text block of `messages[0]` + Host suffix, observed Claude Code insertions tolerated only as recorded host_reported digests, thinking never treated as answer/evidence). Look for acceptance of bodies the Host would not produce, and for refusal of bodies it does produce.
4. Provider profiles are a closed set; no free-form provider/model; openai-codex path behaviour unchanged.
5. Tests and mutants are real (would fail on the regression they claim to catch); custody hashes match.
6. Secrets/private-path exposure in anything sent to the provider.

Verdict: `accept` / `accept_with_changes` (P2/P3 only) / `changes_required`, findings P1–P3 with file:line and a concrete fix. Do not edit code. Budget: cap $3.5 — be economical: targeted reads, no full-crate test reruns unless a finding depends on it.
