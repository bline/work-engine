# Round 2 join check on two round-1 records (2026-10-28)

Activation prerequisite 3 of round 2 (`metrics/dogfood-selection-plan-20261027.md`):
demonstrate the joins from profile to enrollment, assignments, attribution, ledger and
review receipts. Done in a scratch run directory (`~/cbm-work/round2-joins/run`) with the
round-2 tools at `57066b1a`; enrollments, reviews and closures there are **synthetic**
(reasons say so) and are not data.

- **Episodes:** `s10-config-readback-repair` (builder `gpt-6.1-sol` medium, Codex) and
  `rp3-private-git-abbrev-native-repair` (builder `claude-sonnet-5-5` high, Claude Code),
  each with its recorded Astra reviewer. Both draws gave `gpt-6.1-sol` (n = 2).
- **Ledger:** F1.1 build of 2026-10-28 15:00 to 16:30Z, Codex and Claude, 953 calls, 46 turns.
- **Script:** `~/cbm-work/round2-joins/join_check.py` (sha256 `63541e312664958a…`).

## Result: the chain resolves for both episodes

| Link | s10 config repair | rp3 native repair |
|---|---|---|
| enrollment → profile | found | found |
| assignment → attribution | `gpt-6.1-sol`/medium = attributed | `claude-sonnet-5-5`/high = attributed |
| ledger, builder (window) | 37 calls, tier `default`, $0.73, 923 s of turns | 20 calls, tier `standard`, $0.78, turn time unavailable |
| review receipt → reviewer | via `identity.session` → `01a1831e` | via `reviewer_session` → `01a18336` |
| reviewer → profile → ledger | 26 calls, Astra high, `default`, $3.85 | 12 calls, Astra high, `default`, $1.70 |

`status` reported no inconsistencies; `verify` exited 0 over episodes and profiles.

## Gaps found (to fix before activation)

- **J-1. The reviewer link depends on the receipt's format.** The two receipts name the
  reviewer in different fields. `review` should record the reviewer's session or thread ID.
- **J-2. No common verdict or severity scale.** The reviews return `rejected`/`accepted`
  and `ready_for_root_consideration…`, with severities `info`, `low` and `medium`.
  Mapping medium to P2 makes the rp3 first review a failure although its reviewer said
  ready. Reviewers of enrolled episodes need a standard verdict and P-scale severities, and
  the plan's first-review pass should rest on that verdict.
- **J-3. A first review can fail for evidence, not defects.** The s10 first review was
  `rejected` with status `blocked_on_source_evidence_scope`, and accepted after more
  evidence. `review` should record whether a failure is a defect or an evidence or scope
  problem, so packet quality does not read as builder quality.
- **J-4. No active model time for Claude builders:** F1.1's turns are Codex only. Round 2
  compares two Codex builders, so it is unaffected.
- **J-5. Both reviewers were given the builder's identity** (the s10 review has a
  `builder_identity` field). Round 2's guidance already forbids this for enrolled
  episodes; packets must actually leave it out.
