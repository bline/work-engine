# E1a-Terra result: Terra 6.1 medium first artifacts on the E1a protocol (2026-10-27)

Pre-registration: [`e1a-terra-prereg.md`](e1a-terra-prereg.md). Protocol and scoring as in
[`e1a-result-20261027.md`](e1a-result-20261027.md), with a Codex harness. Scored by F3.1
(`battery_score.py` sha256 `51ef39399c3d…`, `--interface e1a-v1`). The coordinator read
only the scorer's count fields.

**Result class: spread obtained (n = 3).** All three arms produced a committed,
interface-conformant artifact, and all three are scoreable. This is a strategy
comparison: model and harness change together.

## Per arm

Counts are over 34 cases.

| Arm | Confirmed failures | Families | Pass | Nonblocking | Indeterminate | Conformant docs |
|---|---:|---:|---:|---:|---:|---:|
| T1 | 7 | 5 | 27 | 0 | 0 | 33/33 |
| T2 | 7 | 5 | 27 | 0 | 0 | 33/33 |
| T3 | 11 | 6 | 23 | 0 | 0 | 33/33 |

| Arm | Cost (rollout, frozen rates) | Calls | Wall | Diff | Tests added | Own tests / fmt / clippy | Model guard |
|---|---:|---:|---:|---|---:|---|---|
| T1 | $0.1253 | 17 | 98 s | 3 files, +416 −1 | 2 | pass (59) / pass / pass | `verified_exact` |
| T2 | $0.1517 | 23 | 113 s | 4 files, +459 −1 | 3 | pass (60) / pass / pass | `verified_exact` |
| T3 | $0.1422 | 18 | 120 s | 4 files, +466 −1 | 4 | pass (61) / pass / pass | `verified_exact` |

- All three reported `complete` with no unfinished obligations, and committed once
  on branch `e1a` with a clean tree. There were no stops and no rate-limit text.
- **Arms total: $0.42.**
- **Quota:** the reading was 99.0% before T1 and 99.0% after T3. Readings are whole
  percents, so the observed movement is under 1 point. Modelled consumption is about
  0.07 points at $6 per point (unverified: Terra 6.1 is not in the quota fit).

## Against E1a's Claude arms (same packet, interface and scorer; different harness)

| Configuration | Runs | Confirmed failures | Families | Cost per run | Wall per run | Tests added |
|---|---:|---|---|---|---|---|
| Terra 6.1 medium (Codex CLI) | 3 | 7, 7, 11 | 5, 5, 6 | $0.13–0.15 | 1.6–2.0 min | 2–4 |
| Opus 5.5 medium (Claude Code) | 1 | 9 | 5 | $2.35 | 7.6 min | 10 |
| Sonnet 5.5 medium (Claude Code) | 3 | 15, 11, 13 | 7, 5, 6 | $0.67–0.73 | 3.1–3.8 min | 10–14 |
| Haiku 5.5 high (Claude Code) | 1 | 14 | 7 | $0.78 | 11.2 min | 11 |

**Reading** (descriptive, as pre-registered):
- **Terra's range is 7–11** (a spread of 4, like Sonnet's 11–15). Two of three Terra
  runs are below Opus's single run of 9, and one is above it.
- **The two ranges touch only at 11.** Terra's best runs are 4 failures below
  Sonnet's best.
- **Cost per run:** Sonnet costs 4.4–5.8 times as much as Terra, and Opus 15.5–18.8
  times as much.
- **Terra writes smaller changes with fewer tests:** +416 to +466 lines against +783
  to +1,417, and 2–4 new tests against 10–14. The battery scores behaviour, not test
  coverage, so this difference is not in the failure counts. Fewer builder tests may
  move cost into review.

No model ranking follows. It is n = 3 against n = 3 and n = 1, on one task, with the
harness confounded with the model.

## Limitations

- Strategy comparison: Codex CLI 0.164.0 with a frozen minimal Codex home, against
  Claude Code 2.1.311 with E1a's frozen Claude home.
- Arms ran sequentially, about 4.5 hours after E1a's parallel batch.
- No context checks inside the arms. Each arm finished in under 2.5 minutes and
  23 calls.
- One task, the easier end of our work: one crate, a reviewed interface.
- The other E1a limitations apply: network not confined, prior exposure, and an
  interface that carries review-derived structure.

## Artifacts

- Arms, evidence and the runner: `~/cbm-work/e1a-terra/`. The runner is
  `tools/codex_arm_run.py`; its sha256 is recorded in each arm's `arm.json`.
- Scores (count fields read): `~/cbm-work/e1a-terra/scores/T{1,2,3}.json`.
- Sandbox smoke test, with no model call: arm `sandbox-smoke` (login status,
  isolation, offline build).
