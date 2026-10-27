# E1a-Terra pre-registration: Terra 6.1 medium first artifacts on the E1a protocol (2026-10-27)

Plan: [`metrics/experiment-plan-20261026.md`](../experiment-plan-20261026.md), §7 O3
(Terra builder-only arms), run early on the remaining Codex quota at the owner's
request. Protocol: [`e1a-prereg.md`](e1a-prereg.md), unchanged except where this
file says otherwise. Written before any arm was prepared or launched.

## Question

How many confirmed failures does a Terra 6.1 medium first artifact have on the
E1a task (same packet, fixed interface, scoring), and how do they compare with
E1a's Claude arms? Those arms had Sonnet 5.5 medium at 15, 11 and 13 failures of
34, and Opus 5.5 medium at 9.

This is a **strategy comparison**: model and harness change together, because Terra
runs only under Codex. n = 3 at most. It is descriptive only, and no model ranking
follows from it.

## Arms

| Arm | Model | Effort | Harness | Per-run cost limit | Wall limit |
|---|---|---|---|---|---|
| T1, T2, T3 | `gpt-6.1-terra` | medium | Codex CLI 0.164.0, `codex exec` | $1.50 | 3 h |

- **Arms run one at a time,** not in parallel as in E1a, because Codex quota is
  nearly exhausted. The quota reading is taken before each launch.
- **The limit** is about 2.7× the four-builder Terra build component ($0.56, which
  also covered design-review repairs).

## Fixed inputs, identical to E1a

| Input | Identity |
|---|---|
| Baseline | `1e21e71c5dea67d143ebd7696d7884789018ee69`, single-ref clone per arm |
| Contract | `~/cbm-work/e1a/packet/contract-e1a-v1.json` (sha256 `b2bd7207a8a2b9a8…`, checked at prepare) |
| Interface | `~/cbm-work/e1a/packet/interface-v1.md` (sha256 `7dd9525a160bc524…`, checked at prepare) |
| Worker instructions | E1a's frozen `repo-instructions` from `~/cbm-work/e1a/env-v1`, each file checked against its manifest |
| Scorer | F3.1, `battery_score.py` sha256 `51ef39399c3d…`, `score --interface e1a-v1`, counts-only output |

## What differs from E1a

- **Prompt.** `builder-prompt-codex-v1.txt` keeps the task text identical. It
  changes only the harness lines:
  - "native Codex builder" instead of "native Claude builder";
  - no Anthropic host mapping or Claude builder adapter, because Codex is the
    worker skill's default host;
  - no Claude transcript context check, because `child_context.py` cannot read a
    standalone `codex exec` session; the coordinator reads context from the
    rollout instead;
  - no session ID, which Codex assigns.
  
  Its sha256 is recorded at prepare.
- **Codex home.** A frozen minimal copy is mounted at `~/.codex`, containing:
  - `config.toml` with model, effort, `approval_policy = "never"`,
    `sandbox_mode = "danger-full-access"` (bubblewrap is the sandbox), the
    production `developer_instructions` and `personality`, the codebase-memory MCP
    server, and trust for the arm's clone only;
  - a copy of the global `AGENTS.md`;
  - `auth.json` bound, not copied.
  
  Production's plugins, hooks and other MCP servers are absent. The file hashes
  go in the arm record.
- **Sandbox.** The same bubblewrap layout as E1a, with Claude paths replaced by the
  Codex home. `~/.claude`, the main repository, other clones, the battery and
  scorer, and the main `~/.codex` are absent inside it.
- **Watchdog.** It reads the arm's own rollout every 20 s and stops the arm if:
  - its cost exceeds the limit (Terra rates: $1, $0.05 cached, $5 output per
    million tokens; ×2 input and ×1.5 output on calls whose input is over 272K);
  - the wall limit passes;
  - any turn runs a model other than `gpt-6.1-terra` (model guard);
  - the quota reading reaches 100%.

## Stop and quota rules

- **Before each launch:** take the latest Codex quota reading. Do not launch if it
  is 100%, or if the previous arm stopped on a rate-limit error.
- **Quota exhaustion mid-run** is an infrastructure failure: cost counted, arm
  invalid, no retry this window. The remaining arms wait for the reset
  (2026-11-02 06:24 UTC).
- **Readings are whole percents,** so the remainder before launch is somewhere
  between 0 and 1%. The planning estimate is about 0.1% per arm (unverified; Terra
  6.1 is not in the quota fit).

## Scoring and reading

- **Scoring** is as in E1a. F3.1 runs at the arm's final `HEAD` (uncommitted
  changes are committed by the coordinator as a separate, recorded commit). The
  coordinator reads only the counts-only output.
- **Also recorded:** cost, calls, wall time, model-guard outcome, tests added, and
  the coordinator's in-sandbox crate tests, `cargo fmt --check` and clippy.
- **Reading:**
  - Each Terra run's confirmed failures are listed against E1a's Sonnet range
    (11–15) and Opus single run (9).
  - With fewer than 2 scoreable Terra runs, the result class is *partial*: the
    counts are reported without a range.
- **Run classes:** as in E1a. Infrastructure failures are invalid (cost counted,
  at most one retry, and none this window if the cause is quota). Behavioural
  results are scored as they stand.

## Known limitations

- Model and harness change together.
- Arms run sequentially, at a different time of day from E1a.
- The minimal Codex home differs from production in the ways listed above.
- There are no context checks inside the arm.
- The other E1a limitations apply: network not confined, prior exposure, and an
  interface that carries review-derived structure.
- The coordinator (this session) has not read the 34-case battery or any
  adjudication detail. It has read the four-builder result summary, which gives
  counts and paths only.
