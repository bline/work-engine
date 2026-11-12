# M3b — delivered legacy applicability representation

Date: 2026-11-12. Status: **root engineering disposition for the authorized M3b slice**, before fresh review and publication. This corrects one representational omission in proposed P3 of the [M3 implementation packet](research-lifecycle-m3-a4-implementation-packet-20261112.md); it does not change the [approved M0 semantics](../records/research-lifecycle-m0-approval-20261111.md) or authorize a new use.

The draft delivered finding `E` carries membership references but has no place for the current legacy applicability row's recorded relation, rationale, bounds and authorization attribution. Retaining that row only in a reconstruction source does not make it visible in the delivered card. Encoding the whole row in `membership.via` would obscure the distinction between a reference and its content.

Add one mandatory field to `E`:

```text
legacy_applicability:S[]
```

Each string is the canonical legacy applicability row JSON using `research_a4_json_v1`, preserving source array order and valid number spelling. Include every recorded authorized applicability row for the requested decision and revision to this delivered finding that is represented by `recorded_authorized_applicability` membership. Sort these row copies by their actual row ID. The field is an empty array when there are none. Keep `membership.via` as the actual applicability row ID. Preserve all competing rows; do not choose a winner by time or collapse attribution.

This field reports legacy register data. It does not authenticate the owner, establish scientific qualification, approve the new request or grant execution authority. The separate M1 record and trusted exact-use admission boundaries remain required. Pending, declined, other-revision and cross-decision disclosures retain their existing partition, scope and notices; they do not enter this field as current authorized support.

All other P3 shape, codec, ordering and outcome rules remain unchanged. This is an unreleased fixture contract without a published external consumer; the root selects this correction within the approved projection/codec objective. The original three byte vectors remain unchanged as historical generic-encoding examples. Their old delivered-entry examples do not establish conformance to the corrected semantic shape. Current semantic goldens must include the new mandatory field and test visible relation/rationale/recorded authorization, reference identity and proper exclusion/grouping of unauthorized or other-revision rows.

The builder reported this omission before freezing. At disposition its observed context was 189,063 tokens, epoch 0; the root retained it only for the narrow correction, affected qualification, freeze and result, estimating 9,000 additional tokens. Any larger remaining work requires a new capacity decision. The separate fresh review must cover this disposition together with the full M3b objective.
