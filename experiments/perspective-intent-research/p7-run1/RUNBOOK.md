# P7 run 1: execution runbook

The deterministic procedure for executing the frozen protocol ([PROTOCOL.md](PROTOCOL.md)). The **executor** is the coordinating Pi session. It launches agents through role_host's Claude subscription container lane, and gives `pipeline.py` each launch's host state root and session id. The pipeline reads the receipt and the output from the host records. The **scorer** is a fresh `p7-scorer` agent per output. The **adjudicator** is the operator. Nothing here runs until the operator gives the separate execution go-ahead (PROTOCOL §10).

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

## 1. Agent prompts

`$P/agents/p7-arm.md` and `$P/agents/p7-scorer.md` each hold one agent's exact system prompt and nothing else. Their SHA-256 is in `asset_digests` in `runs/plan.json`. Nothing is installed: the prompt is passed on each launch (§2), and the receipt compares the launched prompt with the file.

## 2. Launch contract

Every smoke, calibration, arm, fallback and scorer launch is one fresh, single-turn Pi session on the Claude subscription container lane of `skills/manual-orchestration/scripts/role_host.py` ([Pi host reference](../../../skills/manual-orchestration/references/pi-host.md)). It bills the Claude subscription, never an API key; the lane refuses rather than falling back to extra usage, and has no fallback model.

```
H=~/cbm-work/p7-run1-host       # role_host state root for this run only (host-only records)
S=~/cbm-work/p7-run1-scratch    # empty scratch cwd, outside the repository; the same for every launch
A=p7-arm                        # p7-scorer for scoring
python3 skills/manual-orchestration/scripts/role_host.py --state $H launch pi \
  --cwd $S --name <label> --prompt-file <task file> \
  --auth subscription --container --max-requests 1 \
  --host-arg=--model --host-arg=claude-sdk/claude-opus-5-5 --host-arg=--thinking --host-arg=high \
  --host-arg=--no-tools --host-arg=--no-skills --host-arg=--no-context-files \
  --host-arg=--system-prompt "--host-arg=$(cat $P/agents/$A.md)"
```

- The task file is passed unchanged. For the smoke test it is a file holding `Reply with the single word READY.`
- `<label>` is the run, task or score id, for the operator only.
- The launch prints the session id. Wait for `role_host.py --state $H status pi <sid>` to leave `running` (a background watch, not a foreground loop), then `role_host.py --state $H retire pi <sid>`, then record it with the pipeline command for its stage.
- The session is never delivered a second message.

What this lane removes: no subagent extension, supervisor bridge, acceptance contract or intercom runs. Pi loads only the pinned Claude provider extension (`--no-extensions`), runs in a networkless container with a placeholder credential, and reaches Anthropic only through the host egress proxy, which injects the real credential.

**Receipt.** `pipeline.py` reads the receipt from the role_host records, outside the model. It runs `role_host.py --state $H status pi <sid>` (a local, read-only view) and reads the files it names.

**Host-only evidence**, written by role_host and its egress proxy outside the container:
- the turn spec, with Pi's exact argv;
- the durable turn intent;
- the proxy log, with one admission row and one disposition row per request.

It establishes:
- **launch options:** exactly `--model`, `--thinking high`, `--no-tools`, `--no-skills`, `--no-context-files` and `--system-prompt` equal to the agent's prompt file, with a request cap of 1;
- **tool availability:** the request to Anthropic carried an empty tool list;
- **requests:** exactly one request was admitted, and it was settled;
- **context:** the request's system-prompt digest, effort and message count;
- **served model:** the first `message_start` model, verified;
- **billing:** the subscription verdict;
- **usage:** the provider-reported tokens and the latency.

**Pi's events file** is written by Pi inside the container. The model has no tools to alter it. It supplies:
- the session header;
- the system sections (exactly `preamble` and `cwd`, with the preamble equal to the agent's prompt);
- the single user message carrying the task;
- tool executions;
- the provider, model and effort of each assistant message;
- the final assistant text.

The pipeline writes that final text to the record as the output. The executor supplies no output text.

Findings are sorted into three classes:

- **Violation** (`invalid_receipt`, stop):
  - any launch option other than the contract, or another prompt;
  - a request cap other than 1;
  - more than one request, turn, user message or assistant message;
  - a request declaring tools, or any tool execution;
  - another model or effort, requested or served;
  - a billing verdict of `refused`, or a lane refusal;
  - system sections other than `preamble` and `cwd`;
  - a system digest, message count or user wrapper different from the pinned expectation;
  - a user message not carrying the planned task text;
  - a pending turn intent role_host has no record of.
- **Provider failure** (`provider_failure`):
  - the request did not complete (an HTTP or stream error);
  - no request reached the provider;
  - the turn failed;
  - no assistant message, an error stop, or empty final text.
- **Unestablished** (`receipt_unestablished`, stop):
  - required evidence is missing: an unreadable status, spec, intent, proxy log or events file;
  - a request admitted but not settled;
  - a billing verdict that is not established, or usage not reported;
  - the container's removal not recorded;
  - the session still running;
  - the expectations in `receipt_expectations.json` not pinned.

  Missing evidence is never read as zero.

Each record also stores:
- the host state root and session id;
- the digests of the status, spec, proxy log and events file;
- the environment fingerprint;
- the request's system digest and message count;
- the user wrapper digest;
- the per-request summaries;
- usage.

A session id can be bound to only one record. Raw private reasoning is not archived. The host state root stays outside the repository and is kept until the run's report is written.

## 3. Smoke test (first step after the go-ahead)

Launch `p7-arm` once and `p7-scorer` once, under the contract, each with the task `Reply with the single word READY.` Record each one:

```
python3 -I -B $P/pipeline.py record-smoke $H <arm-sid>
python3 -I -B $P/pipeline.py record-smoke $H <scorer-sid> p7-scorer
```

Each must exit 0. This requires a clean receipt in calibration mode (unpinned digests allowed) and a final text of exactly `READY`. The scorer smoke is the scorer's calibration observation; calibration tasks (§4) launch only `p7-arm`.

`runs/smoke/1.json` is the failed smoke of the superseded pi-subagents route (supervisor bridge active, acceptance contract injected, no tool manifest). It is kept as a disclosed paid call and is not a smoke of this route.

## 4. Calibration batch (disclosed, not counted)

Run before any counted launch, after the rubric and inputs are frozen. Launch one run of each of the 15 tasks under the contract, and record each one:

```
python3 -I -B $P/pipeline.py record-calibration <task_id> $H <sid>
```

Records go to `runs/calibration/`, separate from the run ledger. They hold:
- receipt observations, including the request's system digest and message count and the user wrapper digest;
- validity under the strict quote policy, with every quote failure listed;
- leakage alerts;
- output size.

From these records the operator decides, and records in PROTOCOL §5 and `receipt_expectations.json` (`"pinned": true`):
- **Expected digests:** the system digest per agent (the arm's from calibration, the scorer's from its smoke), the request message count and the user wrapper digest. Each must be one value across all observations; a spread stops the run for diagnosis.
- **Quote policy:** `comparator.QUOTE_POLICY`.

Then re-freeze (new commit and digests), and re-run `build_tasks.py --check` and the tests. Calibration outputs are never scored or counted, and are reported as calibration.

## 5. Arm runs

Launch runs in `execution_order` from `runs/plan.json`, a seeded interleaving. For each run:

```
python3 -I -B $P/pipeline.py record-attempt <run_id> $H <sid>
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
python3 -I -B $P/pipeline.py record-fallback <run_id> <index> $H <sid>
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
python3 -I -B $P/pipeline.py record-score <score_id> $H <sid>
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
