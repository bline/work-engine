# P7 run 1: execution runbook

The deterministic procedure for executing the frozen protocol ([PROTOCOL.md](PROTOCOL.md)). The **executor** is the coordinating Pi session. It launches agents and feeds their outputs and native run directories to `pipeline.py`. The **scorer** is a fresh `p7-scorer` agent per output. The **adjudicator** is the operator. Nothing here runs until the operator gives the separate execution go-ahead (PROTOCOL §10).

All commands run from the repository root. `P=experiments/perspective-intent-research/p7-run1`.

`pipeline.py` exit codes:
- `0`: ok.
- `2`: usage error.
- `21`: refused or error; nothing was written.
- `22`: a stop condition holds. Every recording command refuses to write while one holds.

## 0. Preconditions

1. The protocol is frozen at a recorded commit, and these pass:

   ```
   python3 -I -B $P/build_tasks.py --check
   python3 -I -B -m unittest discover -s $P -p 'test_*.py'
   ```
2. The board has been read, and no claim conflicts with this run.
3. The operator has given the execution go-ahead.

## 1. Install the agent definitions

Copy `$P/agents/p7-arm.md` and `$P/agents/p7-scorer.md` to `~/.pi/agent/agents/`, and confirm that their SHA-256 equals `asset_digests` in `runs/plan.json`. Each definition fixes:
- an empty `tools` allowlist;
- `systemPromptMode: replace`;
- no project or global context and no skills;
- only the Claude provider extension;
- fresh context.

## 2. Launch contract and the supervisor bridge

Every smoke, calibration, arm, fallback and scorer launch uses:
- the `subagent` tool with agent `p7-arm` (or `p7-scorer` for scoring);
- `model: "claude-sdk/claude-opus-5-5:high"`, with no fallback model;
- `options: {context: "fresh", agentScope: "user"}`;
- `cwd` set to the empty scratch directory `~/cbm-work/p7-run1-scratch/`, outside the repository;
- the task text exactly as in the task file, passed unchanged;
- `async: true`, collected on native completion.

The installed `subagent` tool (pi-subagents 0.89.0) has no per-launch `intercomBridge` override. That override exists only on the extension delegation API, which this executor does not call. No global configuration is edited.

The bridge is therefore excluded by positive receipt evidence, not by marker words:
- **Tools:** an active bridge adds `contact_supervisor` to the child's declared tools, and the receipt requires every system message to declare no tools.
- **Prompt:** an active bridge appends its instruction to the system prompt, and the receipt requires the system content to equal the digest pinned from calibration (§4).

If calibration shows bridge content or a bridge tool, the run stops before any counted launch, and the operator decides on an isolated configuration.

**Output file.** After each launch, the executor writes the run's final assistant text, as reported natively, to a file. That file is the `OUTPUT_FILE` given to the pipeline, and the receipt checks it against the final assistant text in `events.jsonl`.

**Receipt.** `pipeline.py` reads the receipt from the native async run directory (`status.json`, `events.jsonl`), outside the model. It sorts findings into three classes:

- **Violation** (`invalid_receipt`, stop):
  - wrong agent;
  - wrong step model or thinking;
  - any assistant call on a different provider, model or effort;
  - any declared tool at any system message;
  - any tool execution;
  - system sections other than `preamble` and `cwd`;
  - system content or user wrapper different from the pinned expectation;
  - more than one step or user message;
  - a user message not carrying the planned task text;
  - an output file different from the final assistant text.
- **Provider failure** (`provider_failure`):
  - the run did not complete;
  - non-zero exit;
  - no assistant message;
  - an error stop;
  - empty final text.
- **Unestablished** (`receipt_unestablished`, stop):
  - required evidence is missing: no run id, no system message, no `toolsAdded` record, no recorded context or skills, malformed event lines;
  - the expectations in `receipt_expectations.json` are not pinned.

  Missing evidence is never read as zero.

Each record also stores:
- the native run id, session id and session file;
- the digests of `status.json` and `events.jsonl`;
- the system content digest and the user wrapper digest;
- usage.

A native run id can be bound to only one record. Raw private reasoning is not archived. The run directory is retention-managed, so it is recorded promptly after completion.

## 3. Smoke test (first step after the go-ahead)

Launch `p7-arm` once, under the contract, with the task `Reply with the single word READY.` Then run:

```
python3 -I -B $P/pipeline.py record-smoke <output-file> <run-dir>
```

It must exit 0. This requires a clean receipt in calibration mode (unpinned digests allowed) and a final text of exactly `READY`.

## 4. Calibration batch (disclosed, not counted)

Run before any counted launch, after the rubric and inputs are frozen. Launch one run of each of the 15 tasks under the contract, and record each one:

```
python3 -I -B $P/pipeline.py record-calibration <task_id> <output-file> <run-dir>
```

Records go to `runs/calibration/`, separate from the run ledger. They hold:
- receipt observations, including the observed system content and user wrapper digests;
- validity under the strict quote policy, with every quote failure listed;
- leakage alerts;
- output size.

From these records the operator decides, and records in PROTOCOL §5 and `receipt_expectations.json` (`"pinned": true`):
- **Expected digests:** the system content digest per agent and the user wrapper digest.
- **Quote policy:** `comparator.QUOTE_POLICY`.
- **Bridge:** whether a bridge appeared.

Then re-freeze (new commit and digests), and re-run `build_tasks.py --check` and the tests. Calibration outputs are never scored or counted, and are reported as calibration.

## 5. Arm runs

Launch runs in `execution_order` from `runs/plan.json`, a seeded interleaving. For each run:

```
python3 -I -B $P/pipeline.py record-attempt <run_id> <output-file> <run-dir>
```

Status is decided by priority: a receipt status first, then `suspected_leakage`, then the output's own validity. Validity and leakage are always assessed independently and both are recorded.

| Status | Meaning | Action |
|---|---|---|
| `valid` | Clean receipt, no leakage alert, valid output | None |
| `invalid_extraction` | M2/M3 output fails strict validation (JSON, shape, quotes) | Final; counted invalid |
| `incomplete` | M1 output lacks a FLAGS section (or, for review tasks, a FLAGS entry for every derived document); A1 output lacks a numbered list | Final; counted invalid |
| `provider_failure` | The provider did not complete | One replacement attempt; a second failure stops (§11) |
| `receipt_unestablished` | Receipt evidence missing | Stop (§11) |
| `invalid_receipt` | Launch contract broken | Stop (§11) |
| `suspected_leakage` | A sentinel occurs in the output | Adjudicate (§6) before scoring |

**One authorized replacement.** A run's second attempt is accepted only when the first attempt's effective status is `provider_failure` or `void_leakage`. The pipeline refuses every other second attempt and every third attempt. The final attempt is the last one recorded, never the first success among retries.

## 6. Leakage adjudication (before scoring)

For each `suspected_leakage` record, the operator compares the sentinel's context with the recorded task text and receipt, then runs:

```
python3 -I -B $P/pipeline.py adjudicate-leakage <record> coincidental|exposure "<reason>"
```

`<record>` is the record file under `runs/`, for example `attempts/T1.M1.r2/1/attempt.json`. The decision is written once, next to the record.
- **Coincidental:** the effective status becomes the independently assessed status (`valid`, `invalid_extraction` or `incomplete`).
- **Exposure:** the effective status becomes `void_leakage`. One replacement attempt is authorized; a void replacement stops the run.

## 7. M3 fallbacks

For each M3 run whose final attempt is valid:

```
python3 -I -B $P/pipeline.py fallback-tasks <run_id>
```

This fixes the target list once, from the original extraction, bound to that attempt's output digest. Launch each task file once, under the arm contract, and record it:

```
python3 -I -B $P/pipeline.py record-fallback <run_id> <index> <output-file> <run-dir>
```

Each reply is checked against the target at its own index: same entry and property, a legal status, string values, a non-blank reason. A reply naming another target, or one that is malformed, is recorded as `invalid_response` and resolves nothing. A provider failure gets one replacement.

## 8. Comparison

For every valid M2 and M3 final attempt:

```
python3 -I -B $P/pipeline.py compare <run_id>
```

An M3 comparison is refused until every target has a recorded reply and no reply awaits adjudication. Missing, invalid and out-of-list replies are logged per target and change nothing.

The output is written once, as `comparator-<digest>.json`, named by the comparator and schema digest. A recomputation must match it byte for byte; a mismatch is a comparator defect (§11). After a re-freeze, a new comparator digest gives a new file beside the old one.

## 9. Scoring

```
python3 -I -B $P/pipeline.py score-tasks
```

This writes batch `runs/scores/batch-<k>/` in seeded order, one task per valid final attempt.
- **Binding.** Each key entry is bound to the attempt's output digest and the comparator file's digest.
- **Re-running.** With an unchanged eligible set, the existing batch is returned and nothing is written. A changed set is refused unless `--new-batch` is given, which writes a new batch and keeps the old one.
- **Task contents:**
  - the rubric, with document ids translated to neutral ids;
  - the source documents;
  - one separately delimited material block per phase. Prose has a single `prose` phase. M2 has `pre`. M3 has `pre`, then `post` with its fallback log.
- **Blinding.** Run ids, methods and original document ids are absent. Phases are judged together and are not blinded from each other (PROTOCOL §8).

Launch one fresh `p7-scorer` per task, and record it:

```
python3 -I -B $P/pipeline.py record-score <score_id> <output-file> <run-dir>
```

A reply is **invalid** (`invalid_reply`, one replacement allowed) unless it has:
- the right input id;
- exactly the expected phases and rubric items, with legal answers;
- legal census classes;
- a census listing at least as many flags as each typed phase's report;
- a legal weakening answer.

Quotes are checked against that phase's material only. `NO SUPPORTING SPAN` is accepted for an answer that rests on absence. A failed quote does not invalidate the reply: the affected answer goes to human review and stays unresolved until adjudicated.

## 10. Human checks and adjudication

```
python3 -I -B $P/pipeline.py human-sample
```

This lists:
- one seeded score per input-set and method cell;
- every score with no valid reply, a quote failure or an `uncertain` answer;
- every M3 score whose pre and post answers differ;
- every M3 score with a fallback resolution, for a check of its textual warrant.

The operator records decisions in `runs/adjudication.json`, as a list of `{score_id, phase, item_id, scorer_answer, operator_answer, reason}`. The scorer's original reply is always retained.
- The operator's answer is final.
- Items without a valid answer or an adjudication are reported as unresolved, never dropped.

## 11. Stopping rules

`python3 -I -B $P/pipeline.py gate` exits 22 while any of these holds:
- an `invalid_receipt` or `receipt_unestablished` on any arm, fallback or scorer record;
- a `provider_failure` or `void_leakage` on a replacement arm attempt;
- a declared stop.

Declare a stop for a comparator or pipeline defect with `declare-stop "<reason>"`.

After a stop:
- prior records are kept;
- a harness, comparator or pipeline fix needs a re-freeze, with a note of which records predate it;
- comparator outputs are recomputed into new digest-named files;
- arm outputs are never regenerated.

There is no quota stop (PROTOCOL §1).

## 12. Report

`python3 -I -B $P/pipeline.py report` joins:
- every attempt (the ledger, with status, effective status and assessed status);
- every calibration, smoke, arm, fallback and scorer record, for calls, tokens and latency by stage, replacements included and unknowns counted;
- the latest score batch, with final replies and adjudications.

It reports, per cell, phase and item:
- answer counts;
- hit rates over valid runs and over attempted runs (undefined when the denominator is zero);
- the census by class;
- weakening.

For M3, per run, it reports item transitions, hits gained and lost, raw flag changes and fallback outcomes. PROTOCOL §9 defines how these are read.
