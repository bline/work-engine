# Worker or reviewer: one coherent assignment

Own the packet's bounded engineering outcome, assigned files and launched processes. At entry, acknowledge the exact packet, authority and source/subject revisions. Your parent sends your identity binding after launch - verified session ID, rollout, parent/collection owner, continuation path, context-status binary and digest; acknowledge it at your next boundary and record it in your continuation. Verify current source and runtime inputs before relying on prior evidence. Read DESIGN.md as AGENTS.md requires; PHILOSOPHY.md in full only for design, planning or doctrine review. Read the coordination board at startup, after compaction and before claims or consequential operations, not as a loop. Before freezing a contract or route revision, check its premises against the packet's settled facts. Select the implementation and checks that establish the actual outcome. Report a discovered requirement gap, stale premise or ownership conflict to the supervisor before expanding scope or silently reclassifying the task. Do not start new child agents or next-slice work unless the packet assigns it.

Follow [call cost](call-cost.md): batch, size requests, do not poll, and message the parent only for a result, a decision, a blocker or a decision-relevant risk.

## Context

When assigned implementer discovery for an organization candidate, use [organization formation](organization-formation.md) to report bounded findings and whether you can own a coherent implementation outcome.

Read only what your packet's read list names; reading spends the same budget as writing. Prefer signatures (`grep -n 'pub fn'`) and targeted line ranges over whole files, batch independent reads, and commit after each independently passing unit.

Keep a compact continuation current as decisions and validation state change, not only when handing off: a compaction takes the window without asking, and the next context holds only what you wrote down. You do not decide the context for your *next* assignment; your parent checks that before dispatching it, because only the parent knows that task's size.

**Mid-task check.** At each logical boundary inside the assignment - before each substantial phase, not once - and again before writing your result if an earlier reading was above about 120,000, run (`<skill-dir>` is the manual-orchestration skill's directory):

```
python3 -I <skill-dir>/scripts/child_context.py \
  --binary <pinned path> --sha256 <digest> check --session <your id> --rollout <your rollout>
```

The script compares your latest input with a trigger (150,000 of a 258,400 window) and the observed compaction band (no compaction below about 171K in 42 events on 2026-10-21). The trigger is a pilot hypothesis, not a rule. If you are over it and the remaining work is substantial, prepare a handoff; otherwise continue. Record the reading, decision and reason in your continuation. Continuing past the trigger risks a compaction mid-task, after which the rest of the work is done from your continuation alone. If your parent bound a context listener to you, a current observation from it counts as the reading ([session control](context-control.md#what-to-do-with-an-observation)). Use only the binary you were given - do not build to check context, because a build lock can block observation for everyone. If usage cannot be read, record `usage_unavailable` with the script's class; never guess or treat it as zero. An `epoch` change is not a handoff.

## Builds, tests and frozen subjects

Run builds and tests through `custody.py run` (`<skill-dir>/scripts/custody.py`). It records argv, working directory, environment, wrapper and child PIDs, wait status, output and input digests, and any process still alive afterwards, and prints one line per step. A `--plan` runs a build and its tests in one call: `{"cwd", "env", "env_clear", "steps": [{"name", "argv", "cwd", "env", "env_clear", "digest", "cargo_json", "timeout", "kill_leftovers"}]}`, step values overriding plan values. With `cargo_json` set on a step named `build`, a later step's `{exe:build}` names the test binary it produced (`{exe:build:<target>}` when there are several), and its digest is recorded. Read the one-line results, and open logs only for a failure. Do not write your own runner, or copy PIDs, exit codes or tool-session output into separate records: the record is the custody. In the S10 session of 2026-10-28/29, children referenced 32 helper scripts of their own (13 named `run*`) and 88 tool calls named hand-kept `tool-session.json` records.

To freeze a subject, commit it when it lives in this repository ([slice checkpoint](../../slice-checkpoint/SKILL.md) writes a private ref without moving your branch). For a tree outside it, run `custody.py freeze`: one read of the tree gives a manifest, an archive built from the same bytes, and changes and a diff against a baseline. Task-specific assertions about the frozen subject belong in a check that reads the manifest, not in another freeze script.

<!-- cli-synopsis script=skills/manual-orchestration/scripts/custody.py display="python3 -I <skill-dir>/scripts/custody.py" -->
```text
python3 -I <skill-dir>/scripts/custody.py run --out OUT [--name NAME] [--plan PLAN] [--cwd CWD] [--env KEY=VALUE] [--env-clear] [--digest PATH] [--cargo-json] [--timeout SECONDS] [--kill-leftovers] [--keep-going] ...
python3 -I <skill-dir>/scripts/custody.py freeze --root ROOT --out OUT [--exclude GLOB] [--baseline-root BASELINE_ROOT] [--baseline-manifest BASELINE_MANIFEST] [--label LABEL]
```
<!-- /cli-synopsis -->

## Result

Return a compact result binding input packet, changed source, binary/schema/fixture where material, test commands and actual outcomes, evidence pointers, failures, unresolved work, invalidated assumptions, process disposition, and your continuation path. Distinguish scripted or private observations from live behavior. Keep requested model and observed runtime identity distinct, and mark unavailable usage or cost unknown.

## Unfinished: handoff

If the assignment remains unfinished, apply `$wind-walker` and write a durable handoff that lets the next owner continue without context-only meaning: objective/authority, exact source/worktree and uncommitted changes, settled decisions and rejected approaches worth keeping, evidence and failed checks, open obligations and pending review, live child/process ownership, and next useful action. Aim for about 500-1,000 tokens plus links without omitting a required obligation. End with `handoff_ready`, the handoff path and SHA-256. This requests transfer; it is neither worker completion nor supervisor acceptance, and it does not release your children or processes. Do not claim a configured campaign's terminal audit receipt from this manual handoff.
