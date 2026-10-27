# Worker or reviewer: one coherent assignment

Own the packet's bounded engineering outcome, assigned files and launched processes. At entry, acknowledge the exact packet, authority and source/subject revisions. Your parent sends your identity binding after launch - verified session ID, rollout, parent/collection owner, continuation path, context-status binary and digest; acknowledge it at your next boundary and record it in your continuation. Verify current source and runtime inputs before relying on prior evidence. Read DESIGN.md as AGENTS.md requires; PHILOSOPHY.md in full only for design, planning or doctrine review. Read the coordination board at startup, after compaction and before claims or consequential operations, not as a loop. Before freezing a contract or route revision, check its premises against the packet's settled facts. Select the implementation and checks that establish the actual outcome. Report a discovered requirement gap, stale premise or ownership conflict to the supervisor before expanding scope or silently reclassifying the task. Do not start new child agents or next-slice work unless the packet assigns it.

Follow [call cost](call-cost.md): batch, size requests, do not poll, and message the parent only for a result, a decision, a blocker or a decision-relevant risk.

## Context

Read only what your packet's read list names; reading spends the same budget as writing. Prefer signatures (`grep -n 'pub fn'`) and targeted line ranges over whole files, batch independent reads, and commit after each independently passing unit.

Keep a compact continuation current as decisions and validation state change, not only when handing off: a compaction takes the window without asking, and the next context holds only what you wrote down. You do not decide the context for your *next* assignment; your parent checks that before dispatching it, because only the parent knows that task's size.

**Mid-task check.** At each logical boundary inside the assignment - before each substantial phase, not once - and again before writing your result if an earlier reading was above about 120,000, run (`<skill-dir>` is the manual-orchestration skill's directory):

```
python3 -I <skill-dir>/scripts/child_context.py \
  --binary <pinned path> --sha256 <digest> check --session <your id> --rollout <your rollout>
```

The script compares your latest input with a trigger (150,000 of a 258,400 window) and the observed compaction band (no compaction below about 171K in 42 events on 2026-10-21). The trigger is a pilot hypothesis, not a rule. If you are over it and the remaining work is substantial, prepare a handoff; otherwise continue. Record the reading, decision and reason in your continuation. Continuing past the trigger risks a compaction mid-task, after which the rest of the work is done from your continuation alone. Use only the binary you were given - do not build to check context, because a build lock can block observation for everyone. If usage cannot be read, record `usage_unavailable` with the script's class; never guess or treat it as zero. An `epoch` change is not a handoff.

## Result

Return a compact result binding input packet, changed source, binary/schema/fixture where material, test commands and actual outcomes, evidence pointers, failures, unresolved work, invalidated assumptions, process disposition, and your continuation path. Distinguish scripted or private observations from live behavior. Keep requested model and observed runtime identity distinct, and mark unavailable usage or cost unknown.

## Unfinished: handoff

If the assignment remains unfinished, apply `$wind-walker` and write a durable handoff that lets the next owner continue without context-only meaning: objective/authority, exact source/worktree and uncommitted changes, settled decisions and rejected approaches worth keeping, evidence and failed checks, open obligations and pending review, live child/process ownership, and next useful action. Aim for about 500-1,000 tokens plus links without omitting a required obligation. End with `handoff_ready`, the handoff path and SHA-256. This requests transfer; it is neither worker completion nor supervisor acceptance, and it does not release your children or processes. Do not claim a configured campaign's terminal audit receipt from this manual handoff.
