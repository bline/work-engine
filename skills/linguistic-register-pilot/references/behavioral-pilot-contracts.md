# Behavioral-pilot contracts

Use this reference only after an expanded treatment has passed its frozen
treatment-validation gate and a human has authorized behavioral-pilot
construction.

The current `behavioral-pilot-contract-v2` artifact family separates three authorities:

1. The construct ledger classifies what the canonical role already requires
   before tasks are authored.
2. The sealed contract owns tasks, evidence, keys, subject configuration,
   schedule, scoring, and stopping rules.
3. A later group-3 gate may bind rendered artifacts and authorize only the
   eight-trial budget-calibration stage. It does not rewrite the contract.

Validate the current contract with:

```bash
python3 skills/linguistic-register-pilot/scripts/behavioral_pilot_contract.py \
  --repository . \
  --contract skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/behavioral-pilot-contract-v2 \
  validate
```

The schedule is deterministic and must recompute byte-for-byte:

```bash
python3 skills/linguistic-register-pilot/scripts/behavioral_pilot_schedule.py validate \
  --task-manifest skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/behavioral-pilot-contract-v2/tasks/manifest.json \
  skills/linguistic-register-pilot/pilot/behavioral-pilot-construction/behavioral-pilot-contract-v2/execution-schedule.json
```

V2 is a prospective presentation-contract repair. Each task has a sealed
`tasks/presentations/<task-id>.json` artifact containing exactly two sequential
user messages. Turn 2 must follow the retained turn-1 response, must contain the
frozen `second_turn_record.text`, and must expose only observations whose
`available_at` value is `turn_2`. The task, objective key, task manifest,
execution schedule, and contract seal bind the presentation digest. A trial
that does not deliver those exact bytes is invalid. The v1 seal and its schedule
are historical and non-executable.

Before any model-visible behavioral task, call `check-launch --stage
calibration`. It refuses execution until a separately sealed
`group-3-preoutcome-gate.json` binds the contract seal, all six rendering
digests, semantic and salience reviews, actual-artifact manipulation checks,
cue-concentration checks, and exact subject verification. After T008, call
`check-launch --stage full`; it additionally requires the bound weekly-budget
result to pass. The launcher must treat a refusal as terminal for that stage.

Do not expose objective keys, schedule condition fields, or rendering identity
to the subject. Do not rerun an invalid trial. Preserve raw attempts and
receipts; derived scoring may be recomputed with
`scripts/behavioral_pilot_scoring.py`.

Schemas:

- `behavioral-pilot-contract.schema.json` covers task, evidence, key, and
  schedule shapes.
- `behavioral-pilot-preoutcome-gate.schema.json` defines the later rendering
  and launch gate.
- `behavioral-pilot-run.schema.json` defines one retained trial record.
- `behavioral-pilot-score.schema.json` defines derived primary scores.

The contract deliberately records unavailable snapshot and backend fields as
unexposed. Group 3 must fill only identifiers actually returned by the launch
surface and must not promote a fixed designation into an immutable snapshot
claim.

## Retained group-3 stop

The first group-3 attempt, against behavioral contract v1, generated and bound six opaque full-role artifacts,
then stopped before T001. Its final `group-3-preoutcome-gate.json` is a valid
consumer artifact with `overall_gate: false`. The launch check must reject it.

The retained result passed six-artifact binding, canonical coverage, advisory
speech-act equivalence, and local subject-configuration checks. It failed
semantic equivalence, salience control, both-perspective manipulation,
single-scalar cue concentration, and the frozen profile-mediated quotation
threshold. Direct material quotation and near-verbatim checks were clean.

Review transport and publication failures are preserved separately from the
final clean-packet judgments. Do not rewrite the failed gate or substitute an
earlier packet. A future treatment rendering for v2 must use a fresh
preregistration, fresh artifacts, and fresh review evidence; the v2 behavioral
tasks, presentations, and keys remain preoutcome and must not be exposed during
that rendering work.
