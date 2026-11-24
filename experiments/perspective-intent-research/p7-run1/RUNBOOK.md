# P7 run 1: execution runbook

The deterministic procedure for executing the frozen protocol ([PROTOCOL.md](PROTOCOL.md)). The **executor** is the coordinating Pi session. It launches agents and feeds their outputs to `pipeline.py`. The **scorer** is a fresh `p7-scorer` agent per output. The **adjudicator** is the operator. Nothing here runs until the operator gives the separate execution go-ahead (PROTOCOL §10).

All commands run from the repository root. `P=experiments/perspective-intent-research/p7-run1`.

## 0. Preconditions

1. The protocol is frozen at a recorded commit. `python3 -I -B $P/build_tasks.py --check` passes, and all tests pass:

   ```
   python3 -I -B -m unittest discover -s $P -p 'test_*.py'
   ```
2. The board has been read, and no claim conflicts with this run.
3. The operator has given the execution go-ahead, including the bridge decision in step 2.

## 1. Install the agent definitions

Copy `$P/agents/p7-arm.md` and `$P/agents/p7-scorer.md` to `~/.pi/agent/agents/`. Confirm their SHA-256 equals `asset_digests` in `runs/plan.json`. Every launch passes `agentScope: "user"`, so a same-named project agent cannot win. Each definition fixes:
- an empty `tools` allowlist;
- `systemPromptMode: replace`;
- no project or global context and no skills;
- only the Claude provider extension;
- fresh context.

## 2. Supervisor bridge

pi-subagents' supervisor bridge defaults to `always`. When active, it appends coordination instructions to a child's system prompt, after the definition digest is taken.

- **Preferred:** the operator approves setting `"intercomBridge": {"mode": "off"}` in `~/.pi/agent/extensions/subagent/config.json` for the duration of the run, with a backup and a restore at step 12. This affects every session's subagents while it is off, so check the board for active coordinators first.
- **If the operator declines:** the run proceeds only while every receipt shows no bridge text. The receipt check in step 3 enforces this, and a failure stops the run.

## 3. Launch contract and receipts

Every arm, fallback and scorer launch uses:

- agent `p7-arm` (or `p7-scorer` for scoring);
- `model: "claude-sdk/claude-opus-5-5:high"`, with no fallback model (Pi subagents resolve one model per launch);
- `context: "fresh"` and `options.agentScope: "user"`;
- `cwd` an empty scratch directory outside the repository (`~/cbm-work/p7-run1-scratch/`), so the recorded working directory reveals nothing;
- the task text exactly as rendered in `$P/tasks/<task_id>.md`, or in the fallback and scorer task files, passed unchanged;
- `async: true`, collected on native completion.

After each launch, the async run directory (`/tmp/pi-subagents-uid-1000/async-subagent-runs/<run-id>`) holds `status.json` and `events.jsonl`. `pipeline.py` reads the **receipt** from them, outside the model. The receipt passes only with:
- agent name, model and thinking level as expected;
- `toolCount` 0 and no tool executions;
- provider `claude-sdk` on every assistant call;
- no skills, fresh context;
- system prompt sections limited to `preamble` and `cwd`, with no bridge markers.

The receipt and its digests are stored with the attempt. The run directory is retention-managed, so it is read immediately after completion.

## 4. Smoke test (first step after the go-ahead)

Launch `p7-arm` once, under the contract above, with the task `Reply with the single word READY.` Then run:

```
python3 -I -B $P/pipeline.py receipt p7-arm <run-dir>
```

It must exit 0. Any receipt problem stops the run before any arm is launched.

## 5. Arm runs

Launch runs in `execution_order` from `runs/plan.json`. That order is a seeded interleaving, so no cell runs as a block. For each run:

```
python3 -I -B $P/pipeline.py record-attempt <run_id> <output-file> <run-dir>
```

**Attempt statuses:**

| Status | Meaning | Action |
|---|---|---|
| `valid` | Receipt passes; M1 has its FLAGS section; M2/M3 output is one valid JSON extraction | None |
| `invalid_extraction` | M2/M3 output fails strict validation (JSON, shape, fields, spans quoted from the given documents, references) | Final for that run; counted as invalid. No repair or re-prompt |
| `incomplete` | M1 output lacks its FLAGS section | Final; counted |
| `provider_or_empty` | Non-zero exit or empty output | One more attempt on the same model; a second failure stops the run (§11) |
| `invalid_receipt` | The isolation or launch contract was not met | Stop the run (§11) |
| `suspected_leakage` | A leakage sentinel occurs in the output | Adjudicate (§6) before any scoring |

Every attempt is kept and numbered. Nothing is overwritten.

## 6. Leakage adjudication (before unblinding)

For each `suspected_leakage` attempt, the operator compares the sentinel's context with the run's recorded task text and receipt, before `score-tasks` is run:
- **Exposure plausible:** the attempt is void, one replacement attempt is made, and a second exposure stops the run.
- **Coincidental:** the attempt is reclassified `valid`, recorded with the reason.

Sentinels are alerts, not proof either way.

## 7. M3 fallbacks

For each valid M3 attempt:

```
python3 -I -B $P/pipeline.py fallback-tasks <run_id>
```

This fixes the list of ambiguous fields from the original extraction. Launch each fallback task once, under the arm contract, and record it:

```
python3 -I -B $P/pipeline.py record-fallback <run_id> <index> <output-file> <run-dir>
```

The comparator accepts only one of the original readings, or an abstention. Anything else is logged as an invention and rejected. An attempt with no ambiguous fields has no fallback opportunity; that is not a success.

## 8. Comparison

For every valid M2 and M3 attempt:

```
python3 -I -B $P/pipeline.py compare <run_id>
```

M3 reports both the pre-fallback and post-fallback comparison of the same extraction. Only the paired difference is attributed to the fallback.

## 9. Scoring

Run `python3 -I -B $P/pipeline.py score-tasks`. It writes one blinded scorer task per valid final attempt, in seeded order. Each task contains:
- the rubric for that input, with document ids translated to the neutral ids the output uses;
- the source documents;
- the output, plus the comparator report for typed outputs.

Run ids, methods and original document ids are absent; the key is stored separately in `runs/scores/key.json`.

Launch one fresh `p7-scorer` per task and record it:

```
python3 -I -B $P/pipeline.py record-score <score_id> <output-file> <run-dir>
```

Every quoted evidence span, and every flag in the census, is checked mechanically against the scored output. An invalid reply gets one retry, and the original is kept.

## 10. Human checks and adjudication

Run `python3 -I -B $P/pipeline.py human-sample`. It lists:
- one seeded output per input-set and method cell (22);
- every score with an `uncertain` answer, a quote failure or an invalid reply.

The operator checks each listed score against the source documents and records decisions in `runs/adjudication.json`: score id, item id, scorer answer, operator answer, reason. The scorer's original vote is always retained.
- **Disagreement:** any item where the operator's answer differs from the scorer's.
- **The operator's answer is final.** Unresolved items are reported as unresolved, not dropped.

## 11. Stopping rules

Stop and report on any of the following:
- an `invalid_receipt`;
- a provider failure that persists after one retry on the same model;
- confirmed leakage on a replacement attempt;
- a comparator or pipeline defect.

After a stop:
- prior attempt records are kept and labelled with the stop;
- a harness, comparator or pipeline fix requires a re-freeze (new commit and digests) and a note of which rows predate it;
- comparator outputs are recomputed deterministically, while arm outputs are never regenerated.

There is no quota stop (PROTOCOL §1).

## 12. Report and restore

1. Run `python3 -I -B $P/pipeline.py report`, then the analysis in PROTOCOL §9.
2. Restore `~/.pi/agent/extensions/subagent/config.json` from its backup if step 2 changed it.
3. Record total calls, tokens and latency, from the receipts, by stage: arm, fallback, scoring, smoke, retries. Values a receipt does not report are recorded as unknown.
