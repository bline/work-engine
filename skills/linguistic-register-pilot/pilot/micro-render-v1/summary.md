# Fixed-semantics micro-render probe

## Disposition

`micro_render_not_established`

The original semantic-equivalence pass accepted 10 of 16 renderings and judged
6 `not_equivalent`, so matching was not run. Adversarial review subsequently
found inconsistent semantic judgments and seven serialized JSON wrappers that
the original prose validator failed to reject. This is a stopped and
methodologically qualified experiment, not evidence that the registers are or
are not recognizable in rendered prose.

## What happened

The original reviews said every rendering preserved all four propositions and
the requested speech act. Their added-meaning decisions were not calibrated
consistently: similar evaluative phrases were accepted in some samples and
rejected in others. A post-review, style-blind adjudicator applying one rule to
all 16 immutable pairs accepted 5 semantically and rejected 11; it separately
marked 7 transport-invalid because the `text` value contained serialized JSON
rather than prose alone.

| Condition | Original accepted | Adjudicated accepted | Transport-valid | Required |
| --- | ---: | ---: | --- |
| Gelman model criticism | 2 | 0 | 3 | 4 |
| Leveson system safety | 2 | 0 | 3 | 4 |
| Shaw engineering judgment | 2 | 2 | 3 | 4 |
| Neutral editorial defaults | 4 | 3 | 0 | 4 |

These per-condition counts are diagnostic only. The disagreement and transport
failures make causal interpretation of the original 2/4, 2/4, 2/4, 4/4 pattern
unreliable. The result does not rank the candidate corpora.

## Integrity and stopping rule

- Sixteen fresh renderings were produced from two frozen briefs, four anonymous
  style cards, and two replicas per brief-condition pair.
- Sixteen fresh style-blind semantic reviews were retained with packet-bound
  provenance.
- The original decisions and report remain unchanged; post-review adjudication
  is retained separately and has no authority to reopen the frozen gate.
- The harness rejected matching preparation at S02, the first non-equivalent
  sample in deterministic order.
- No rendering was regenerated, substituted, or selectively excluded after
  outcomes were known.
- The aggregate report records `matching_status` as
  `not_run_semantic_gate_failed` and contains no matching scores.

The frozen result is in `micro-render-report.json`. Post-review qualification is
in `review-qualification.json`, with adjudication packet, raw output, normalized
result, and events retained beside it.

## Recommended next experiment

Freeze and checkpoint a second micro-render plan before launching any model
jobs. First reject or exactly unwrap nested structured output. Then separate
register realization from unsupported proposition by revising the style cards
into two explicit feature classes:

1. meaning-preserving surface/discourse features permitted in fixed-semantics
   rendering; and
2. stance-bearing or evaluative moves that require corresponding meaning in
   the semantic brief.

Use briefs that explicitly license a controlled evaluative proposition, and
apply one calibrated semantic rubric across the full set before matching. Bind
model outputs to launcher receipts and event-log digests. This is a proposed
new experiment, not a repair or continuation of the frozen run.

## Scope limits

This was an upstream selection probe using provisional two-document profiles.
Sol supplied the renderers, semantic reviewers, adjudicator, and adversarial
reviewer, so all are same-model-family checks with `independence_claimed:
false`. Recorded launcher provenance is not mechanically attested by the
retained event format. The preregistration digest was not published in an
immutable checkpoint before outcomes, so pre-outcome timing is asserted but
not independently evidenced. The run does not replace eventual subject-family
and different-family or human manipulation checks on actual C0/C1/C2 artifacts.
