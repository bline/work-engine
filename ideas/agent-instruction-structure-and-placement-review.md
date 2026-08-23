# Agent-Instruction Structure and Placement Review

## Status

Exploratory specialized-review capability. This idea does not create a review
gate, accept its own placement, or authorize changes to agent-facing text.

## Problem

Agent-facing text is an executable control surface. Instructions in
`AGENTS.md`, skills, role definitions, prompts, workflow contracts, references,
templates, and adapter guidance influence what an agent treats as required,
permitted, preferred, or out of scope.

The same sentence can be a necessary invariant at one layer, an appropriate
implementation mechanism at another, and unjustified path concentration when
placed in a broader document. Reviewing wording without its owner, audience,
authority, scope, and loading behavior cannot distinguish those cases.

General architectural placement review is not an exact substitute. It asks
where system responsibility, state, or capability ownership belongs. This
specialist asks where normative agent text belongs so the correct agents receive
it with the correct authority and scope.

## Core diagnostic

The review reduces philosophical structure to two questions:

1. Does this instruction define something that must be true for the outcome to
   remain valid, safe, authorized, or observable?
2. If the same valid outcome could be reached by a different route, sequence,
   trigger, or mechanism, would this instruction still need to hold?

Interpretation:

| Answers | Likely consequence |
| --- | --- |
| Yes / Yes | The instruction is probably structural. |
| No / No | It is probably procedure and should usually be removed from the governing layer or demoted to a current mechanism, default, or example. |
| Yes / No | It may bundle a real invariant with one preferred realization and should be split. |
| No / Yes | It may conceal an objective, authority boundary, or protected distinction that should be stated directly. |

The compact form is:

> What distinction does this instruction preserve?
>
> Does correctness require this exact route?

If no protected distinction can be named, the instruction is probably
unnecessary at that layer. If correctness does not require the exact route,
prescribing it as architecture is probably path concentration.

These questions are a diagnostic lens, not a mandatory sentence-by-sentence
questionnaire. The reviewer concentrates on materially normative text and may
investigate neighboring contracts when placement changes the answer.

## Placement context

Placement is not a third philosophical test. It supplies the context needed to
answer the two tests correctly. For each material instruction, the reviewer may
bind:

- the exact text, section, and immutable subject revision;
- the artifact's semantic owner and intended consumers;
- the authority and scope the artifact actually has;
- whether and when the text is loaded into an agent's context;
- the upstream outcome or distinction it protects;
- the downstream mechanism it constrains; and
- any stronger owner, narrower adapter, or duplicated instruction that already
  governs the same meaning.

This can distinguish:

- structural text correctly placed in a governing contract;
- a real invariant buried below the layer that needs it;
- an implementation mechanism correctly placed near its adapter;
- a mechanism improperly elevated into a general skill or repository-wide
  instruction;
- one sentence bundling an invariant with a preferred mechanism;
- a hidden objective or authority boundary;
- duplicated normative text with competing owners; and
- useful nonbinding guidance that is presented truthfully as a default,
  example, or current implementation detail.

## Required consequence

A useful finding identifies:

- the exact instruction and placement under review;
- the protected distinction, or the inability to identify one;
- whether correctness requires the prescribed route;
- the instruction owner's actual authority and audience;
- the consequence of retaining the current wording and placement;
- confidence, evidence, and limitations; and
- a recommended outcome such as retain, restate as an outcome, split invariant
  from mechanism, move to a stronger or narrower owner, demote to nonbinding
  guidance, or remove.

The review should not treat all procedure as invalid. Exact routes can be
causal requirements for atomicity, safety, authorization, observability,
provider independence, or another named distinction. Operational steps can also
remain useful near their implementation owner without becoming architecture.

## Adaptive applicability

This specialist is a candidate member of the open adaptive-review registry. It
has credible decision value when a proposal or implementation creates or
changes materially agent-facing normative text.

Typical selection consequences:

- a local code or component ownership change may need architectural placement
  review without this specialist;
- an agent-facing instruction change may need this specialist without broad
  architectural review;
- a new skill, role, workflow, or instruction-precedence boundary may warrant
  both perspectives; and
- a spelling or presentation-only edit may warrant neither.

Selection remains consequence-based. The existence of agent-readable prose
does not make this a universal gate.

## Relationship to neighboring capabilities

- **Architectural diagnostic review** challenges the system model, semantic
  owner, decomposition, or capability placement. This specialist consumes that
  context when needed but does not replace it.
- **Cross-cutting seam review** asks whether independently valid contracts
  correspond at a boundary. This specialist focuses specifically on normative
  instruction structure, reach, and placement.
- **Adaptive review-panel coordination** may select this specialist and preserve
  its findings, omissions, and limitations. It does not absorb the specialist's
  judgment.
- **Skill authoring** creates or revises a skill. This reviewer diagnoses the
  resulting instruction surface but does not own the edit.
- **Doctrine and authority review** checks binding conflicts and authority
  inflation more broadly. This specialist supplies a narrower instruction-level
  diagnosis and may expose a conflict that warrants that broader review.

## Authority and independence

The reviewer may diagnose path concentration, hidden objectives, misplaced
invariants, duplicated ownership, and scope or authority mismatch. It may
recommend a different instructional placement or representation.

It does not:

- define or amend project philosophy;
- decide the underlying architecture;
- rewrite the reviewed text;
- accept a proposal or implementation;
- grant itself blocking authority; or
- treat its own durable judgment or instruction set as evidence of correctness.

The capability cannot establish its own alignment by applying its rubric to
itself. Its instruction contract requires independent review when that contract
is a material subject.

## Candidate placement and first evidence

Probable placement is a distinct specialist reviewer available to proposal and
implementation review through adaptive panel selection. Its instruction-text
placement lens should not be folded into the general architectural reviewer,
whose system-placement context, evidence needs, and likely applicability differ.
Permanent skill, profile, and review-artifact placement remain undecided.

A bounded first dogfood subject could be the recent retained-Claude workflow
instructions in `skills/claude-recon-implementation/SKILL.md`,
`skills/work-engine-mcp/SKILL.md`, and
`reviews/proposals/bootstrap-review-procedure.md`. That subject contains real
invariants, mechanisms, provider boundaries, loading choices, and authority
language. Selection as dogfood would not presume those instructions are wrong.

## Open evidence needs

- Test whether the two-question diagnostic produces materially distinct
  findings from general architecture, doctrine, and seam review.
- Exercise one necessary exact route and one over-concentrated route so the
  reviewer proves it can preserve causal procedure instead of deleting it by
  style preference.
- Determine the smallest placement context a fresh reviewer needs without
  loading every neighboring skill or repository instruction.
- Determine whether review artifacts need a distinct classification vocabulary
  or can reuse the open specialized-review finding profile.
- Observe whether proposal and implementation subjects genuinely share one
  reviewer capability before accepting shared placement.

## Compact statement

> Architectural placement asks where system responsibility belongs.
> Instruction placement asks where governing text belongs so it preserves the
> necessary distinction for the correct agents without prescribing an
> unnecessary route.
