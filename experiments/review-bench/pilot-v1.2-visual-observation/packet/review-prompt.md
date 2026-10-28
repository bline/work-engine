# Review case: Normalize Visual Evidence authored observations

Act as a read-only independent verification reviewer. Treat material claims as hypotheses, verify them from the supplied snapshot, and check the most plausible contradictory path. Report acceptance-critical uncertainty as blocked_unverified instead of assuming it away. Stop once material claims reach high confidence. Do not treat builder rationale or passing tests alone as proof. Before assigning blocking status, trace the relevant producer, validation or ownership boundary, supported caller, and consumer. Separate supported production reachability from malformed or contract-excluded direct invocation. A bounded practical consequence does not make an explicit acceptance-criterion violation nonblocking; report severity and blocking as separate judgments. Return result schema v2. Put only defects or material risks in findings. Put evidence-backed acceptance conclusions in verified_claims and identify the acceptance criteria they support. Put real non-defect improvements or out-of-contract notes in observations. Verified claims and observations have no severity or blocking label. Classify each claim once.

## Exact case identity

- Bench: `site2json-visual-observation-defective-20260820`
- Case: `visual-evidence-authored-observation-pre-review`
- Snapshot: `refs/work-engine/review-bench/visual-observation/pre-review`
- Snapshot digest: `7ea148539e201994d52aae122e04891890809cfc59f786b63e3356f31c336459`
- Review class: `implementation`
- Route class: `runtime_cross_boundary`
- Semantic domains: `runtime, schema, ui_visual, integrity`
- Topologies: `cross_module`
- Novelty: `new_ownership`
- Evidence types: `static_source, deterministic_test, runtime_inspection, historical_reconstruction`
- Consequences: `workflow_blocking, state_corrupting`
- Risk: `high`

## Contract

Normalize Detection and Graph selections into one validated, session-only Visual Evidence observation that preserves truthful status, action, interpretation, alternatives, and provenance without entering persisted or inferred project state.

Acceptance criteria:

- Detection and Graph projects expose canonical alias-free semantic targets derived from their authoritative models.
- A rendered selection, or an explicit unresolved result, produces one contract-valid session observation with truthful status, action, interpretation, and provenance.
- Ambiguous Graph paths remain unresolved, preserve their alternatives, and retain recorded click evidence without inventing semantic certainty.
- The session-only observation contains no raw live objects or copied page text and is not consumed by persistence, inference, or runtime execution.

Excluded scope:

- Persisting Visual Evidence observations into project or output schemas
- Changing extraction, inference, or runtime execution semantics
- Adding OCR, image recognition, or automatic semantic disambiguation
- Redesigning the Detection or Composition Graph interfaces

## Available artifacts

- `docs/roadmap.md` — task objective and scope
- `docs/visual-evidence-architecture.md` — session-only observation architecture
- `extension/content/page-picker.js` — page selection and click provenance producer
- `extension/editor/sidebar-composition-graph.js` — Graph semantic target adapter
- `extension/editor/sidebar-detection-first.js` — Detection semantic target adapter
- `extension/editor/sidebar-visual-evidence.js` — UI observation orchestration
- `extension/editor/sidebar.html` — Visual Evidence authoring surface
- `extension/editor/sidebar.mjs` — sidebar integration boundary
- `extension/ir/visual-evidence-contract.js` — observation schema and validator
- `extension/ir/visual-evidence-observation.js` — authored observation normalizer
- `package.json` — test and module configuration
- `tests/sidebar-visual-evidence.test.js` — sidebar observation integration tests
- `tests/visual-evidence-contract.test.js` — contract boundary tests

## Deterministic evidence

- The reconstructed pre-review snapshot passes all 10 focused Visual Evidence tests that existed at its review boundary.
- All 13 manifest hashes match the immutable pre-review ref and its archived tree.
- The independently reconstructed post-review checkpoint differs from this tree in only the observation normalizer and its focused integration test.

Return one `review_bench_result_v2` JSON object with `findings`, `verified_claims`, and `observations` arrays. Do not include ground-truth identifiers.
Put only claimed defects or material risks in `findings`; each may carry severity and blocking status.
Put acceptance-critical conclusions established by evidence in `verified_claims`; identify the supported acceptance criteria and do not attach severity or blocking status.
Put real non-defect improvements or out-of-contract notes in `observations`; do not attach severity or blocking status.
Finding fields are `finding_id`, `severity`, `blocking`, `category`, `claim`, `evidence`, and `confidence`.
Verified-claim fields are `claim_id`, `claim`, `evidence`, `confidence`, and `acceptance_criteria`; quote supported criteria verbatim from this case.
Observation fields are `observation_id`, `category`, `claim`, `evidence`, and `confidence`.
Verified claims require high confidence. An accepted verdict must verify every listed acceptance criterion and contain no blocking finding; otherwise use the evidence-supported non-acceptance verdict.
Do not duplicate one claim across arrays. Use only the configured verdict, severity, and confidence vocabularies.
Every item must cite concrete evidence from the supplied snapshot.
