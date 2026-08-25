# Semantic Context Manager Intake Assessment — Revision 2

## Subject

This assessment binds revision 2 of the complete `Semantic Context Manager`
raw idea at its immutable source checkpoint. Revision 1 remains in
`assessment.md` as the historical interpretation of the earlier hooks-first
source.

## Changed evidence and authority

The user adopted a parallel Codex App Server scaffold after a bounded live
experiment showed one App Server thread surviving a transition between two
distinct model context windows. The experiment also showed that an emitted
`contextCompaction` item did not, by itself, describe the semantic transition.

This falsifies the revision-1 implementation premise that hooks should be the
first prototype and App Server only a later realization. It does not falsify
the surviving Context Manager candidate, its semantic-safety boundary, or the
need for provider-neutral runtime ownership.

## Assessment

One independently useful claim still has downstream decision value:

> Allocate centralized context observation, semantic-difference inspection,
> preservation coordination, transition-readiness judgment, transition
> classification, and rehydration verification to a durable Wind Walker role,
> while target roles retain semantic ownership and perform only bounded
> correction or veto.

The revised candidate now distinguishes three identities:

```text
logical role instance
        ↕ current runtime binding
App Server thread
        ↓ contains over time
successive model context windows
```

The thread can preserve a reasoning runtime across context replacement without
becoming the canonical owner of workflow state. Wind Walker's own thread also
does not own the lifecycle result; an external ledger records the inspected
revision, readiness, transition, classification, and reconciliation.

Pressure and clearing remain separate judgments. Rising pressure triggers
preservation because loss risk is increasing. Clearing occurs when semantic
safety is established and a fresh context is preferable to retention.

## Disposition

The revised claim remains ready for proposal formation. This means only that a
fresh proposal former may decide whether and how to form it. It does not accept
the idea, authorize implementation, establish permanent placement, or change
roadmap priority.

The candidate boundary includes:

- one durable Wind Walker observer role for non-human-facing role threads;
- host-owned App Server lifecycle and context-window observation through a
  provider-neutral adapter;
- immutable context-revision binding and a stale-decision write fence;
- semantic compilation rather than transcript summarization;
- pressure-triggered preservation and separately judged clearing;
- target-role bounded correction or veto;
- native fresh-context replacement after continuation readiness;
- multi-signal transition classification rather than event-name inference;
- post-transition rehydration and reconciliation;
- an external lifecycle ledger; and
- total lifecycle-cost and continuation-correctness evidence.

It excludes:

- ownership of semantic-durability doctrine, role state, authority, domain
  state, or App Server protocol mechanics;
- opaque summarizing compaction as a continuation owner;
- general task-agent responsibility for continuous context management;
- a claim that thread history or notes replace canonical state;
- a claim that `contextCompaction` proves one transition kind;
- fixed pressure thresholds or mandatory boundary clearing;
- recursive context management of Wind Walker;
- automatic migration between runtime actors;
- broad human-facing replacement; and
- proposal acceptance, implementation, cleanup, or roadmap authority.

## Neighboring ownership and conflicts

The App Server scaffold and role-port direction is a separate raw idea because
it owns provider execution structure, role-thread binding, skill injection, and
role migration order. It is an enabling neighbor, not a second claim inside
this intake.

The formed semantic-durability, context-observability, role-owned-state, and
operational-history proposals are compatible under their stated ownership
boundaries. They become conflicts only if a future implementation lets Wind
Walker or the runtime scaffold absorb their semantic state, authority, or
canonical history. The revision-1 hooks-first route is explicitly superseded;
the historical assessment remains evidence of that route revision rather than
a competing current disposition.

## Material uncertainty

- Manual `thread/compact/start` has not been tested and may still perform
  summarizing compaction.
- It is unresolved whether the host can initiate native `new_context` or only
  configure and observe it.
- The exact intervention and fencing mechanism before automatic exhaustion is
  unresolved.
- A stable provider source for context-window identity has not been selected.
- The available runtime evidence comes from an alpha build and lacks the raw
  protocol transcript and complete window identifiers in this intake.
- Thread-local skill isolation and dynamic-tool behavior require versioned
  integration tests.
- The lifecycle ledger's exact owner and schema remain unresolved.
- Specialist inspection and target validation may cost more than they save.
- Human-facing unresolved meaning may require a different boundary.

## Handoff

A proposal former should receive this one revised candidate and the named
evidence. It should retain the stronger semantic-durability, context
observability, role-state, authority, and runtime-adapter boundaries.

The first implementation campaign, if separately authorized, should pair a
read-only `strategic-planner` role vertical with Wind Walker and exercise one
fenced observe → preserve → replace → rehydrate → reconcile lifecycle. It
should not treat automatic budget exhaustion as the normal policy or invoke
manual compaction until that operation is independently characterized.

