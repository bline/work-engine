# Semantic-licensed micro-render v2

## Disposition

`micro_render_v2_transport_contract_failed`

The experiment stopped at its transport gate. Eight of sixteen preregistered
render jobs were launched concurrently. Three returned plain prose and
normalized successfully; five returned an exact JSON object serialized inside
the outer `text` field and were rejected. The remaining eight samples have no
retained execution evidence after the all-samples prerequisite became
impossible. Absence cannot prove that an unretained or aborted invocation never
occurred.

No semantic adjudication or matching job ran. This result says nothing about
semantic equivalence, register recognizability, or candidate quality.

## Pre-outcome evidence

Before any model job, checkpoint
`d823f680ff55891ac2b11a927f93f80e0b39cbed` immutably retained the frozen plan,
licensed style cards, schemas, packet-only runner, harness, and tests. The plan
digest is `c6d47785a191630e42358f75121409580cd00e699e62058d300ce0dde8d71c3b`.
Every prepared render packet and the private key bind both values.

This supplies the pre-outcome publication evidence missing from v1.

## Observed transport result

| Status | Samples | Count |
| --- | --- | ---: |
| Plain prose, mechanically valid | S03, S04, S08 | 3 |
| Serialized inner `{ "text": "…" }`, rejected | S01, S02, S05, S06, S07 | 5 |
| No retained execution evidence | S09–S16 | 8 |

The five inner prose values each contained 90–94 words and appeared under the
same exact wrapper shape. They were not unwrapped because the frozen contract
required prose directly in the outer `text` field. No failed sample was retried
or substituted in the retained evidence; this does not exclude an unretained
attempt.

Each evidenced job retains a mechanically checked execution receipt binding its
packet, output schema, prompt, model and reasoning configuration, timestamps,
isolated visible inputs, raw output, and event stream. The machine-readable
`transport-stop-report-qualified.json` binds all eight execution evidence sets.
The receipts were first checkpointed together with the initial report, so Git
proves their final binding consistency but not that their bytes were immutable
before reporting work began.

## Interpretation

The strict transport gate worked as designed, but the selected output contract
was too brittle for the observed structured-output behavior. The failure is
upstream of the scientific manipulation: none of the valid or wrapped prose
entered semantic review, and no condition was unblinded or scored.

## Recommended next experiment

Freeze a v2b transport policy before any new model call:

1. accept direct prose in the outer `text` field;
2. otherwise accept only one exact serialized object with exactly one `text`
   string field;
3. unwrap that string exactly once;
4. reject arrays, quoted scalar strings, extra fields, recursive wrappers, and
   every other nested shape; and
5. apply the unchanged 90–130-word, one-paragraph prose gate after unwrapping.

Record whether unwrapping occurred so matching and review can test for a
condition-correlated transport artifact. Start a fresh experiment ID and
pre-outcome checkpoint; write a durable attempt marker before each invocation;
and checkpoint raw outputs, events, and receipts immediately after each launch
batch and before any reporting implementation or aggregation. Do not repair or
continue this run.

## Scope limits

All jobs used `gpt-5.6-sol`; no independent-family evidence exists. Licensed
cards remain refinements of provisional two-document profiles. This stopped
transport run has no corpus-selection, C0/C1/C2 manipulation-check, behavioral,
or production-integration authority.
