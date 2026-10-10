# Acceptance Evidence: Claim Maintenance and Reliance Propagation

This document elaborates the evidence needed to evaluate the candidate in
[`proposal.md`](proposal.md). It is not an acceptance record and grants no
implementation authority.

## End-to-end vertical

Acceptance requires a real lifecycle across both initial domain profiles. At
least one exact path proceeds from a real source event to candidate impact,
canonical nomination, authorized refresh, resulting claim revision, existing
exact-version reliance, domain-owned reopening obligation, delivery, semantic
disposition, and truthful completion without any intermediary acquiring
authority it does not own.

The proof includes one code-to-documentation and one documentation-to-code path
from exact checkpoint subjects, including planned-versus-actual scope
comparison.

## Structural and observation cases

The evidence matrix exercises:

- stable identity preservation across movement and rename;
- split, merge, deletion, addition, and ambiguous matching;
- one newly unmapped behavior or normative-documentation candidate;
- lossless recovery of the attributed change subject after terminal slice
  projection and model-context replacement;
- a stale or mixed-snapshot observation reported as incomplete through
  snapshot binding rather than compared;
- no claim-system record written into the observation backend, so neither
  parser-produced structure nor another owner's state can be overwritten;
- discovery results, such as indexed document references, never satisfying
  or rebinding a declared anchor;
- a full backend rebuild or concurrent canonical revision during observation
  producing a retry or explicit incompleteness, never a complete mixed result;
- unresolved anchor retention, with expected identity and cause, rather than
  silent loss;
- deterministic recomputation of derived correspondence after backend or
  cache loss;
- backend rebuild and indexer or schema migration;
- cycles, dangling or retracted references, partial watermarks, stale reads,
  exclusions, failed inputs, and truncated traversal; and
- proof that retrieved graph impact does not itself create a canonical
  nomination.

## Semantic lifecycle cases

The evidence matrix exercises:

- retained-unchanged and changed judgments;
- inapplicable, insufficient, contested, deferred, and superseded outcomes;
- concurrent and branching revisions, explicit conflict, identity forks, and
  canonical-support selection;
- duplicate and out-of-order nominations and deliveries;
- authority revocation and writer replacement;
- exact reliance retention, retirement, supersession, and selective reopening;
- inactive recipients and later authorized delivery; and
- context replacement before and after semantic publication.

## Failure recovery

Failures are injected on both sides of every durable publication boundary:

- nomination publication;
- refresh-episode opening;
- judgment publication;
- resulting claim revision;
- reliance update;
- obligation publication;
- cross-owner receipt;
- acknowledgement; and
- semantic completion.

Recovery demonstrates one semantic consequence, idempotent delivery, truthful
ambiguity where completion cannot be proven, and no replay of a possibly
completed non-idempotent transition merely to discover state.

## Evidence limitations entering evaluation

The formation dogfood positively represented the backbone record semantics but
did not exercise real consumers, adversarial concurrency, complete delivery
recovery, or outcome-independent falsification. Direct Codebase Memory
inspection established physical feasibility of a projection provider, not the
correctness of the proposed synchronization extension. Neither result is a
substitute for this vertical and adversarial matrix.

Later shadow experiments
([`evidence/codebase-memory-correspondence-20261010.md`](evidence/codebase-memory-correspondence-20261010.md))
replaced that synchronization extension with evidence-anchor observation in
the probable placement. They exercised declared-anchor observation only:
- strict resolution;
- absence versus uncertain coverage;
- non-binding successor candidates;
- concurrent canonical revision changes; and
- reconstruction.

They also showed that the supported observation interface cannot yet bind
observations to one coherent snapshot. The experiments are evidence for the
placement, not for this matrix.

**Verification gap: relationship-anchor evaluation.** The experiments did not
evaluate how typed authored relationships consume anchor outcomes. Open
questions include:
- whether realization, verification, and support relationships with
  different sensitivities nominate correctly from the same anchor outcome;
- how relationships spanning several anchors combine their outcomes; and
- whether relationship-profile traversal selects the intended related claims.

This gap is recorded for later verification. It is not an authorized design
expansion and does not change this acceptance bar.
