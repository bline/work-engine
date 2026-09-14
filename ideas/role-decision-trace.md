# Role Decision Trace

## Status

Reconciled against current app-server implementation and prospective architecture in
`app-server/docs/role-decision-trace-reconciliation.md` — **accepted and its residue
authorized for design work 2026-09-14; see that document for the authorization record.**
This root idea file is retained as historical subject matter; the reconciliation document
is the living artifact. This idea does not survive as a
new claim-evidence domain profile or a new database: `claim-evidence-service.md` supplies
proven, reusable substrate patterns (revisioned identity, evidence/provenance fields,
immutable history, exact-revision reliance) without being the semantic owner of decisions
or judgments, and `proposal-decision-gated-implementation-compilation.md`'s sealed
"material decision set" is a real but narrow existing consumer covering only explicit
route selection, not interpretations, sufficiency judgments, assumptions, or placement
judgments. The idea's actual subject is broader than "decisions" in that narrow sense —
it is durable semantic judgment identity and ancestry, of which material decisions are
one subtype. What remains genuinely open: judgment identity, judgment class, judgment
lifecycle (including whether "contradicted" is even a lifecycle state), active
governing-judgment state (owned state vs. a projection over separately owned promote/
retire/supersede transitions — neither demonstrated), and all seven ancestry relations
(`PREMISE_FOR`/`SUPERSEDES`/`WEAKENS`/`CONTRADICTS`/`AFFECTS`/`REOPENED_BY`/
`CHANGED_BECAUSE_OF`) with endpoints still undefined. The open architectural question —
whether judgment history belongs on a general revisioned-state primitive or grows from
the sealed decision-set architecture — is preserved, not resolved. See the reconciliation
document for the full disposition.

Exploratory observability and recovery idea.

## Idea

Preserve an attributed semantic trace of **observable role judgments that materially shape execution**, without attempting to store hidden chain-of-thought or turning every thought into product state.

## Current evidence

Work Engine already preserves:

- proposal decisions;
- route revisions;
- receipts;
- state transitions;
- review findings;
- strategic handoffs.

Those artifacts capture outcomes, but they do not provide one general lineage for intermediate judgments such as assumptions, exclusions, sufficiency judgments, and decisions later invalidated by evidence.

## Required consequence

A decision record can identify:

- stable decision identity;
- role and logical actor;
- work/proposal/slice identity;
- decision class;
- conclusion;
- confidence as expressed by the role;
- evidence references and cutoff;
- assumptions and limitations;
- relations to earlier decisions;
- whether it remains active, stale, superseded, contradicted, or resolved;
- the later consequence it influenced.

Useful relations may include:

```text
PREMISE_FOR
SUPERSEDES
WEAKENS
CONTRADICTS
AFFECTS
REOPENED_BY
CHANGED_BECAUSE_OF
```

## Active decision set

The full decision trace and the smaller set of decisions currently relied upon are different.

A runtime role may consume an **active decision set** while the historical trace remains available for recovery and forensics.

## Boundaries

The decision trace does not replace:

- proposal meaning;
- workflow state;
- review artifacts;
- receipts;
- evidence claims;
- raw provider/session traces.

It references those owners.

It must not expose protected reasoning to roles whose independence depends on not receiving it.

## Does not own a Work Dossier

A future UI may aggregate proposals, decisions, state, receipts, and raw-trace references into a dossier-like view. That is a projection problem, not a second semantic owner and does not require a separate canonical data object here.

## Compact statement

> Preserve consequential observable judgments and their lineage so later roles can understand which premises governed action and which ones became stale, without storing hidden reasoning.
