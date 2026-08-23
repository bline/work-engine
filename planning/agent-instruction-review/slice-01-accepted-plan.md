# Accepted Plan: Agent Instruction Review, Slice 1

- Run: `agent-instruction-review-20260823`
- Baseline: `2387a321f01b82d8c4916e211f2a17f3f1091dcf`
- Acceptance: `procedural_auto_approval`
- Route: `falsified-placement`
- Placement risk: medium
- Placement verdict: confirmed, provisionally at `skills/agent-instruction-review/`

## Bounded slice

Create one repository-local, discoverable `agent-instruction-review` skill using
`skill-creator`; demonstrate it on real normative instruction surfaces;
preserve immutable, evidence-bearing dogfood results; independently
bootstrap-review the implementation with Claude; repair only valid in-scope
findings; and validate the final bounded result.

The slice does not change proposal packet v1's
`implementation_authorized: false`, make the specialist universal or blocking,
grant it architecture, doctrine, seam, authoring, coordination, acceptance, or
mutation authority, settle proposal/implementation lifecycle reuse without
dogfood evidence, implement adaptive panel coordination or revision-bound
review artifacts, modify unrelated review-bench work, or authorize an ordinary
completion commit.

## Provisional semantic-path certificate

When implementation or proposal review encounters materially normative
agent-facing text, the invoking review or decision workflow produces an
immutable, revision-bound subject and invokes the `agent-instruction-review`
specialist.

The skill owns the instruction-diagnosis contract but no mutable product state.
The applicable implementation or proposal reviewer consumes the subject and
returns evidence-bearing advisory findings. The author or builder consumes
valid findings for remediation; the named decision owner consumes the result
for acceptance or placement decisions.

The lifecycle is conditional selection, immutable subject, fresh diagnostic
pass, retained remediation of the same findings when useful, and advisory
terminal consequence. Proposal and implementation adapters may share this
lifecycle only if dogfood supports it.

The observable consequence is a bounded review artifact that truthfully
distinguishes structural requirements from mechanisms and correct placement
from misplaced or over-broad instruction text, including truthful omission
where no present instruction contract exists.

Real review of `skills/claude-recon-implementation/SKILL.md`,
`skills/work-engine-mcp/SKILL.md`, and
`reviews/proposals/bootstrap-review-procedure.md`, plus applicability cases,
must prove necessary-route retention, unnecessary-route challenge, mixed-text
separation, applicability outside `SKILL.md`, selection of a present normative
proposal, and omission of a future-only proposal.

Successful packaging, candidate self-review, regex or lint checks, generic
philosophy commentary, or merely labeling imperative wording are insufficient
substitutes.

## Invariants

- Review is read-only, advisory, revision-bound, and evidence-bearing.
- The candidate neither self-certifies nor exercises mutation, architecture,
  proposal, implementation, panel-selection, or acceptance authority.
- Applicability follows semantic consequence rather than file type or path.
- Exact routes remain exact when causally required for validity, safety,
  authority, or observability; mechanisms, defaults, and examples do not become
  invariants merely because they are imperative.
- Placement judgment considers owner, audience, authority, scope, precedence,
  and loading; `DESIGN.md` and `PHILOSOPHY.md` retain distinct binding effects.
- Fresh, independently defined Claude review begins an authority-bound retained
  episode; ordinary remediation resumes that episode while useful.
- User work is preserved, proposal packet v1 is unchanged, and lifecycle
  uncertainty is resolved only by observed dogfood.
- New semantic ownership, integration, or permanent artifact/state scope
  requires a boundary-change request.

## Expected task boundary and overlaps

Expected additions are `skills/agent-instruction-review/SKILL.md`,
`skills/agent-instruction-review/agents/openai.yaml`, at most one focused
reference if dogfood justifies it, and compact immutable Git-bound dogfood and
independent-review records under a new review directory following repository
conventions.

Only verified dogfood findings may modify the three real instruction surfaces
named above. Shared independent-review-state or Work Engine MCP implementation
may change only after boundary reconsideration if evidence makes that necessary.

There is no accepted overlap with the pre-existing unrelated changes:

- `skills/review-bench/pilot-v1.2-role-state/README.md`
- `skills/review-bench/references/claims-aware-evaluation-roadmap.md`

## Smallest vertical semantic proof

One immutable table-driven dogfood record must cover a causally necessary exact
sequence, an unnecessary concentrated route, mixed invariant/mechanism text, a
hidden objective or authority boundary, materially similar text at different
instruction layers, normative text outside `SKILL.md`, selection of a proposal
with a present instruction contract, and omission of a future-only proposal.
Every material finding must identify evidence and authority basis, and the
reviewer must perform no mutation or acceptance action.

Passing requires findings materially distinct from generic architecture,
doctrine, or seam commentary while preserving causal exact routes. Failure
reopens placement rather than inviting a cosmetic rewrite.

## Validation mapping

- `semantic_proof`: deterministic artifact/schema checks plus inspection of the
  immutable real dogfood result against the vertical proof above.
- `risk_proportional_checks`: skill validation with
  `python3 /home/bline/.codex/skills/.system/skill-creator/scripts/quick_validate.py skills/agent-instruction-review`,
  focused checks for any shared runtime surface actually changed, and fresh
  independently defined Claude review after deterministic checks pass.
- `workspace_integrity`: exact task manifest against the baseline, exclusion of
  both unrelated dirty paths, and `git diff --check` before and after
  remediation.
- If independent-review-state code changes, run
  `python3 -m unittest skills.independent-review-state.tests.test_independent_review_state`.
  If Work Engine MCP code changes, run
  `node --test skills/work-engine-mcp/tests/server.test.mjs`.
- A broad suite is omitted while changes remain instruction and review
  artifacts; add it if implementation reaches shared runtime, schema, state, or
  broadly consumed instruction behavior.

## Deferred scope and reopening condition

Deferred: adaptive review coordination, a shared revision-bound artifact
product, universal panel or gate selection, mechanical instruction linting,
unrelated doctrine or architecture cleanup, permanent lifecycle unification,
proposal packet lifecycle rewriting, and the ordinary completion commit.

The remaining uncertainty is whether proposal-time and implementation-time
subjects can share one semantic contract and retained-review lifecycle. Reopen
placement or split adapters if dogfood shows indistinct findings, excessive
context requirements, incompatible evidence lifecycles, a stronger existing
owner, or consumers requiring a different artifact or state boundary. Preserve
the uncertainty if evidence remains insufficient.
