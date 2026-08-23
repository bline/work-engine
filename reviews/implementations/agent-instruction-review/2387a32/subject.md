# Agent Instruction Review Dogfood Subject

Status: immutable baseline-bound dogfood subject for the initial implementation
of `skills/agent-instruction-review/`. This record demonstrates the candidate's
behavior; it does not certify the candidate, accept implementation, or settle
permanent review-artifact or lifecycle placement.

## Identity

- Baseline commit: `2387a321f01b82d8c4916e211f2a17f3f1091dcf`
- Accepted plan SHA-256:
  `d0b3b845599f4b97f0b75141c34d1ca4a6a2326d10324c1ea8ee8715f141d98f`
- Candidate package: working-tree implementation produced by slice
  `agent-instruction-review-20260823/1/attempt-1`

## Immutable candidate package

| Path | SHA-256 |
| --- | --- |
| `skills/agent-instruction-review/SKILL.md` | `a9a0df0b2f7059d0f7dd7bc5f23aa1acafb875b1d429d8bef4c4af6245bb3d90` |
| `skills/agent-instruction-review/references/finding-contract.md` | `c64a4018f1cde074ea1fc54aedd9a1473bb3393fca708a36f0d4fcfae2d0c709` |
| `skills/agent-instruction-review/agents/openai.yaml` | `b1505aab3948e89ef90ea70cc3b2c7435baa3d5b16d4d4d8f44765584140f0d5` |

## Immutable real surfaces

| Path | Baseline blob | SHA-256 |
| --- | --- | --- |
| `skills/claude-recon-implementation/SKILL.md` | `b75b686a6086007b5269fd28cd46957382272fbe` | `d5e18de973f036caf6ed6695407154f3d028702a1a2eaeabe8632f60e8b1bc17` |
| `skills/work-engine-mcp/SKILL.md` | `ab785ed7d21f865d3373a2a627a96f5cdb6f87a9` | `06e8a8faaa2ebbb672d024a5bba40ddaabbc5310b9d6efaf087d8cba3fe015a7` |
| `reviews/proposals/bootstrap-review-procedure.md` | `ca62d1912b21caedabf66fd89827e3079d7f2bd9` | `f69260fe54a4109199f088f2b8bbed10f23bf309ab0df151a69928f76aa3be0d` |

## Applicability cases

The present-contract case is
`proposals/agent-instruction-integrity/agent-instruction-structure-placement-review/proposal.md`
at baseline blob `88a35dff533e242bbd1c6314f93c026731be8f6d` and SHA-256
`15ae064fee8d73065a07e82086eec4381142dfa669f1eb0ba834d8bca7e4c463`.
It contains actual reviewer instructions and authority boundaries.

The future-only case is
`reviews/proposals/agent-instruction-structure-placement-review/2b03e6a/subject.md`
at baseline blob `8d3e712b1e195d6d061aed44e290a620aeabdac8` and SHA-256
`573fa26050d86aee1e2e8a91ed9e9b514aebb103a9e852cc8975214b6fd0aaa0`.
Its selected-panel contract explicitly omits the then-unimplemented candidate.

## Candidate dogfood result

Applicability: `applicable` to the three real surfaces and present-contract
proposal; `omitted` for candidate review of the future-only subject because it
contains no implemented candidate instruction surface. Omission is not a pass.

| Case | Evidence | Distinction and route judgment | Advice |
| --- | --- | --- | --- |
| Causally necessary exact sequence | `skills/claude-recon-implementation/SKILL.md:197-205` requires authority and subject binding before provider entry, then reviewer publication and readback. | Reordering can cross the provider boundary without an authorized episode or return an unobserved transient result, losing authority and observability. The sequence is structural in this retained-review mode. | `retain` |
| Unnecessary concentrated route | `skills/claude-recon-implementation/SKILL.md:73-83` gives one preferred Claude command, while lines 36 and 51 define it as a high-assurance/default route selected by risk. | The exact CLI spelling and tool ordering are a correctly local example, not a universal correctness condition. Equivalent authorized mechanisms may preserve the outcome. | `retain` as an example; do not promote |
| Mixed invariant and mechanism | `skills/claude-recon-implementation/SKILL.md:36-51` combines a route diagram with hard read-only, authority, provenance, user-work, and validation boundaries. | The named boundaries are invariant; the diagram is a conditional operating shape. Their distinct effects are already stated rather than collapsed. | `retain` |
| Hidden authority boundary | `reviews/proposals/bootstrap-review-procedure.md:108-124` allows a coordinator to add a specialist only when changed consequences justify it and says metrics do not redefine acceptance. | Selection authority is bounded by consequence and does not transfer proposal decision authority. The qualification is causally necessary; unconditional panel expansion would broaden authority. | `retain` |
| Same concept at different layers | `skills/claude-recon-implementation/SKILL.md:188-205` commands provider-specific retained-review mechanics; `skills/work-engine-mcp/SKILL.md:30-43` describes the adapter's narrower projection and denial boundaries. | The workflow skill may direct its consumer; the MCP skill must constrain exposed capability. Similar retained-review language belongs at both layers for different owners and reach, without either becoming project-wide procedure. | `retain` |
| Normative text outside `SKILL.md` | `reviews/proposals/bootstrap-review-procedure.md:46-87` governs runtime bindings and retained remediation. | The Markdown path does not reduce applicability: agents consume it as an operating procedure with authority and lifecycle consequences. | `retain`; applicable |
| Present normative proposal | The proposal's required properties and scope define current instructions for the candidate reviewer. | Present instruction contract makes specialist review applicable even before implementation acceptance. | selected |
| Future-only proposal subject | The baseline subject states that no implemented candidate `SKILL.md`, prompt, role profile, or operational surface exists and omits the candidate to avoid self-validation. | Anticipating a future capability does not create a present review subject. | omitted |

## Consequence and limitations

The cases produce distinct route, placement, and applicability judgments rather
than labeling imperative wording. No candidate finding requires modification of
the three baseline surfaces before independent bootstrap review.

This candidate-authored dogfood is semantic proof of reach and discrimination,
not independent evidence of correctness. Fresh Claude review remains required.
The sample supports one shared semantic contract for proposal and implementation
subjects, but does not establish that their retained evidence lifecycles should
be unified. That lifecycle question remains open under the accepted reopening
conditions.
