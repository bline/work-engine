# Sol adversarial review — initial result

## Verdict

`BLOCK`

- Immutable subject: checkpoint `d26be8da656a0be869a96648fef443a1d23d4da0`, tree `dc46019947cd45791129d416fe4ea728bc555f09`
- Task patch: `d97192a3ed8f1f645af132d75822a38ef01a22f4a4bfa6ba6b5045e89c4992fb`
- Report/gate digest: `aac0119fb8ceb9a2aaee9a71037a6a7f03b06c32f13938c139822650008911b0`
- Reviewer: OpenAI Codex, `gpt-5.6-sol`, `xhigh`, fresh process, `fork_context: false`, no inherited builder history
- Reviewer target: `01a03ace-e0ad-7ff2-90af-e08d3735f3e4` (“Huygens”), generation 1
- Same-model relationship: reviewer, renderers, and semantic reviewers are Sol-family
- `independence_claimed: false`
- Evidence class omitted because the result blocked
- Durable retained review-state publication was unavailable and is not claimed

## Verified evidence

The reviewer reconstructed all 16 render packets and their key from immutable
blobs; verified every packet, render, semantic packet, semantic result, and
report binding; validated 32 raw outputs; confirmed 32 distinct event threads;
and reconstructed 10 equivalent and 6 non-equivalent original judgments. It
confirmed that no matching artifacts or scores exist, matching preparation
fails closed at S02, the stopped and successful aggregate branches enforce
their run-count invariants, and the 206-path task patch contains zero
`app-server/` paths. Five focused immutable-checkpoint tests passed.

## Findings

### HIGH — MRV1-SEM-001

Semantic judgments were materially inconsistent. “Appropriately bounded” was
accepted at S12 while “appropriately narrow” was rejected at S13; “nearly
complete” framing was accepted at S09 and rejected at S06. The arithmetic and
stop remain valid, but the original 10/16 and per-condition pattern cannot
support the summary’s causal interpretation. Preserve original decisions,
adjudicate all 16 immutable pairs under one style-blind rubric, and qualify the
interpretation.

### HIGH — MRV1-RENDER-001

S02, S03, S07, S12, S13, S14, and S15 contain a serialized JSON object inside
the retained `text` string. The validator checked length, paragraphs, headings,
and bullets but not nested structured output. This violates the prose-only
contract and could contaminate matching in a future passing run. Reject or
exactly unwrap nested output and add the retained shape as a regression test.

### MEDIUM — MRV1-PREREG-001

The plan and harness first appear immutably in the same checkpoint as outcomes.
Packet and report digests prove consistent use, not pre-outcome publication.
Publish a plan digest in a prior checkpoint or timestamped receipt in the next
experiment.

### MEDIUM — MRV1-PROV-001

Provenance fields were accepted as arbitrary strings, while freshness and
packet-only execution were hard-coded attestations. Event logs support distinct
threads and exact outputs but do not mechanically attest model, effort,
launcher, or sandbox configuration. Bind future results to launcher receipts
and event digests and validate timestamps.

### MEDIUM — MRV1-TEST-001

Tests missed nested JSON, invalid provenance, both matching/run-count
fail-closed branches, successful aggregation boundaries, and retained-run
reconstruction. Add those dangerous paths.

## Agent-instruction-review result

`applicable` to `SKILL.md`, `references/artifact-contracts.md`, and the frozen
preregistration.

- `AIR-MRV1-001` (MEDIUM, `split` / `restate`): the artifact-contract document
  incorrectly assigned all field and arithmetic authority to
  `pilot_artifacts.py`; each artifact-family harness is its actual semantic
  owner.
- `AIR-MRV1-002` (LOW, `restate` / `demote`): “When abstract-card calibration
  fails, use…” promoted one human-authorized diagnostic into an automatic
  general trigger without causal necessity.

No additional structural instruction finding applied to the frozen plan’s
blindness, gates, or stopping rules. This bounded result did not evidence
pre-outcome publication.
