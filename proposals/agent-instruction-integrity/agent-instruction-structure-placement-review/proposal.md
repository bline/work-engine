# Proposal: Agent-Instruction Structure and Placement Review

## Identity and state

- Proposal ID: `work-engine.agent-instruction-structure-placement-review`
- Family ID: `work-engine.agent-instruction-integrity`
- Source revision: `dee25c3`
- State: formed; placement probable; not reviewed, decided, or authorized for
  implementation
- Decision owner: user or future explicitly authorized portfolio owner

The canonical lifecycle, placement, uncertainty, relationship, and authority
metadata is in [`packet.json`](packet.json). This narrative owns the proposal's
current meaning. Supporting artifacts describe its [placement](placement.md),
[relationships](relationships.md), and a possible
[implementation shape](implementation-plan.md).

## Candidate and consequence

Add a specialized reviewer for materially normative agent-facing text. It
diagnoses whether an instruction preserves a distinction required for a valid,
safe, authorized, or observable outcome; whether correctness requires the exact
route it prescribes; and whether that text lives at the correct semantic owner,
authority scope, audience, and context-loading layer.

Implementation review can challenge actual skills, role prompts, workflow
instructions, `AGENTS.md`, templates, adapter guidance, and instruction-loading
contracts without turning project philosophy into a style preference or
requiring every agent-readable document to pass a universal gate.

## Proposed product change

Create a distinct specialist review capability, probably as a repository-local
skill. For material instructions it can bind the exact subject and inspect the
artifact owner, intended consumer, authority, scope, loading behavior, upstream
outcome, downstream mechanism, and any stronger, narrower, or duplicated owner.

Its compact diagnostic remains:

1. Does this instruction define something that must be true for the outcome to
   remain valid, safe, authorized, or observable?
2. If the same valid outcome could be reached by a different route, sequence,
   trigger, or mechanism, would this instruction still need to hold?

The shorthand is:

> What distinction does this instruction preserve?
>
> Does correctness require this exact route?

These are reasoning affordances, not a mandatory sentence-by-sentence form.
The reviewer concentrates on materially normative text and investigates only
the placement context needed to judge it.

## Required properties

### Structure and mechanism remain distinguishable

Desired outcome: an instruction is retained as structural only when its
violation loses a named validity, safety, authority, or observability
distinction. A preferred route remains a mechanism, default, or example unless
correctness causally requires that route.

Invalid outcome: the reviewer equates imperative wording with architecture,
or deletes necessary exact procedure merely because alternative routes are
usually desirable.

### Placement conditions the judgment

Desired outcome: the reviewer evaluates normative text relative to the
artifact's semantic owner, audience, authority, scope, precedence, and loading
behavior. It can distinguish a correctly local mechanism, a misplaced
invariant, an over-broad instruction, and duplicated competing owners.

Invalid outcome: the same sentence receives one context-free classification
regardless of whether it appears in project doctrine, `AGENTS.md`, a skill
entrypoint, a conditional reference, an adapter contract, or a nonbinding
example.

### Applicability follows actual instruction consequence

Desired outcome: implementation review selects this specialist when changed
artifacts contain materially normative agent-facing text. Proposal review
selects it only when the proposal itself contains or changes an actual agent
instruction contract, loading boundary, precedence rule, or exact route claimed
as required.

Invalid outcome: every proposal about a future skill receives speculative
instruction review, every documentation edit becomes a review gate, or an
actual normative instruction escapes review because it is stored outside a
`SKILL.md` file.

### Findings preserve evidence and consequence

Desired outcome: a finding identifies the exact instruction and revision, its
placement and owner, the protected distinction or failure to identify one,
route necessity, consequence, evidence, confidence, limitations, and a
recommended outcome such as retain, restate, split, move, demote, or remove.

Invalid outcome: the reviewer emits vague philosophical-alignment criticism,
mechanically lints style, or treats a preferred rewrite as proof that the
current text is invalid.

### Review and authoring remain separate

Desired outcome: the specialist diagnoses and advises. Skill authors,
architectural owners, proposal formers, implementation builders, and named
decision authorities retain their existing mutations and decisions.

Invalid outcome: the reviewer rewrites the subject, amends doctrine, chooses
architecture, selects itself for every panel, grants itself blocking authority,
or treats persistence of its own judgment as evidence.

### The capability does not certify itself

Desired outcome: initial implementation is reviewed by an independent
specialist applying the candidate contract without inheriting the builder or
the implemented skill's own self-assessment. Later changes preserve truthful
review provenance and ordinary remediation continuation.

Invalid outcome: successful execution of the new skill is presented as proof
that its own instructions are structurally aligned.

## Boundary and consumers

The probable semantic owner is a distinct instruction-structure and placement
review capability. Its primary consumer is implementation review of actual
agent-facing surfaces. Adaptive proposal review is a conditional consumer only
when the proposal's present content is itself normative agent text.

This specialist does not own general system placement, doctrine, seam
coherence, skill creation, review coordination, proposal formation, or
acceptance. A subject may justify selecting several of those perspectives, but
their findings and authority remain separate.

## Scope

In scope:

- materially normative text consumed by agents;
- instruction owner, audience, authority, scope, precedence, and loading reach;
- invariant-versus-mechanism diagnosis and bundled-text separation;
- exact-route causality, hidden objectives, and authority boundaries;
- duplicated or misplaced instruction ownership;
- proposal review only for actual instruction contracts; and
- implementation review and retained remediation of instruction findings.

Out of scope:

- general prose quality, spelling, or documentation style;
- broad code or component placement;
- mechanical philosophical-alignment linting;
- authoring or automatically rewriting skills and prompts;
- universal review selection based only on file extension or directory;
- proposal or implementation acceptance; and
- self-certification of the reviewer skill.

## Evidence needs and uncertainty

- Dogfood one causally necessary exact route and one unnecessary concentrated
  route so the reviewer proves it can distinguish them.
- Compare findings from this specialist with architecture, doctrine-authority,
  and seam review on the same bounded subject.
- Determine the smallest neighboring context needed to judge placement without
  loading every repository instruction.
- Exercise a proposal that contains normative agent text and one that merely
  anticipates a future skill to test conditional selection.
- Exercise an instruction outside `SKILL.md` so storage location is not mistaken
  for applicability.
- Determine whether proposal and implementation review can share one specialist
  contract and retained-state profile after actual use.

## Authority

Formation of this candidate does not review any instruction, accept its probable
placement, add a specialist to a production panel, change project doctrine,
authorize skill implementation, or decide proposal or implementation
readiness. Those consequences remain with their named owners.

## Acceptance consequence

If later accepted and implemented, a fresh implementation or proposal decision
owner can see whether materially normative agent text preserves a named
distinction, whether correctness requires its prescribed route, whether it
lives at the right instructional layer, and what remains uncertain—without
turning philosophy into a fixed procedure or the reviewer into an authority.

