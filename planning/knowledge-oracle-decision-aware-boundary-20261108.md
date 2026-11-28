# Knowledge Oracle and the decision-aware path: proposed boundary

**Status:** a proposal from the Knowledge Oracle side, 2026-11-08, for
reconciliation with the decision-aware investigation before either path is
implemented. It decides nothing and changes no file the other session pinned.
The oracle design at `14031bc2` is not edited here. The changes to it that this
note implies are listed in section 6, to be applied once the boundary is
agreed.

**The two sides:**

- **Knowledge Oracle:**
  [`design/ideas/pending/knowledge-oracle-question-resolution-adapter.md`](../design/ideas/pending/knowledge-oracle-question-resolution-adapter.md)
  (`14031bc2`, exploratory) and the
  [S0 packet](knowledge-oracle-s0-experiment-20261108.md) (revision 2).
- **Decision-aware path:** session `a6defb59`'s *Evidence-to-decision
  integration: investigation proposal v1*, off-repo at
  `~/cbm-work/evidence-claims-integration-20261108/proposal-v1.md`, awaiting
  approval. It asks how Work Engine recognizes when established knowledge could
  improve a real decision, supplies it economically, and leaves applicability to
  the agent's judgment. It takes the oracle design and the S0 packet as inputs,
  tests whether a smaller path (register decisions → task preparation →
  existing consumer API) suffices, and aligns its qualification plan with S0.

## 1. Responsibilities

The user's division:

| | Decision-aware path | Knowledge Oracle |
|---|---|---|
| Owns | recognizing when an agent's work creates a knowledge need, and supplying knowledge economically at that point | resolving an expressed question, or exact refs, to attributable evidence |
| Does not | resolve free-form questions to claims with its own router; record applicability; maintain claims | detect needs; push knowledge unasked; decide when a decision needs knowledge; record applicability; maintain claims |

Neither takes over the other's responsibility. Owners both paths preserve:

- **Rust claims.** The `claim-evidence` crate and claims host own identity,
  revisions, trusted publication and reliance.
- **Research register.** The register owner owns studies, findings, events,
  judgments, decisions and applicability, through `register.py` and its rules.
- **Claim maintenance.** Change, refresh and reopening belong to
  `claim-maintenance-and-reliance-propagation` and evidence-anchor once
  implemented, and to the `reassess` worklist and register events today.

## 2. Decision-identity contract (proposed)

Both paths need to name a decision. This proposes one identity and one owner.

- **Identity.** A decision identity is a register decision record, named by its
  `decision_id` and its exact `revision`, for example
  `model-selection.builder-default` at `git:41d47209`. The register currently
  holds 11 such records, each with `document`, `lines`, `anchor`, `owner` and
  the `findings` it rests on (`metrics/register/decisions.json`).
- **Owner.** The register owns the records. Each record's named owner owns the
  decision itself. Neither path mints, edits or infers decision identities.
- **Unknown decisions.** When a need bears on a decision with no register
  record, the identity is `not established`. The decision-aware path may
  propose a new record to the register owner. The oracle accepts an
  unregistered decision label only as unverified context, and never routes on
  it.
- **Not decision identities.** Claims `decision_scope` (for example
  `repository-research`) is an authority and read scope fixed by a claims
  startup. `decision-readiness` assessments and `portfolio-decision` records
  are separate crates outside the root workspace, with their own owners. None
  of these is adopted as the shared identity unless its owner decides so.
- **Revisions.** A need recorded against `decision_id@revision` stays bound to
  that revision. When the decision document changes, the register's own rule
  applies: applicability must cite the decision's current revision. Neither
  path carries a decision forward to a newer revision silently.

This would settle the oracle's OD-5 in proposed form, pending the
decision-aware investigation's findings and the register owner's agreement.

## 3. Request and response boundary

```text
agent work
   |
   v
decision-aware path: detects a knowledge need at a decision point
   |  (owns: when to look, what the need is, whether a lookup is worth its cost)
   |
   |-- exact refs known (for example findings a register decision already lists,
   |   once materialized as claims) --> existing consumer API, or oracle.check_refs
   |
   '-- expressed question --> oracle.ask
                                 |  (owns: routing, retrieval, projection,
                                 |   provisional composition, referral records)
                                 v
                           response: resolution state, route, projections with
                           owner standing, provisional text, ids
   |
   v
agent (or the decision-aware path's delivery) decides how to use it.
Applicability stays with the agent's judgment and the decision owner.
```

**Crossing the boundary.** The `oracle.ask` request is the message the
decision-aware path sends. It carries:

- the question text, or exact refs;
- the decision identity (section 2), when established;
- task identity and role kind;
- intended use;
- structured context;
- whether a provisional composition is wanted;
- whether an investigation request may be prepared.

The response is the oracle's resolution state, route, projections, optional
composition, referral ids and optional investigation request record. Nothing
in the response is an applicability judgment or an instruction to act.

**Calls that skip the oracle.** When the decision-aware path already holds
exact claim refs, it can resolve them through the existing consumer API, or
through `oracle.check_refs` once that exists. No routing is needed, and the
oracle adds nothing it should insist on. This keeps the decision-aware
investigation's "smaller path" open.

**Who initiates.** The oracle never initiates a lookup. A
"contract-designated preflight", meaning knowing that a decision needs a check
before acting, is need recognition, and belongs to the decision-aware path.
`oracle.check_refs` remains a resolution primitive that whichever side holds
the refs may call.

## 4. Guarding against duplicate mechanisms

| Mechanism | Single owner | Decision-aware path | Oracle |
|---|---|---|---|
| Decision records | register | reads; may propose new records | reads |
| Decision → finding links (`decisions.json` `findings`) | register | reads | reads, as identity-supplied L0 routes |
| Decision → question relations | **open (B-1 below)**. Proposed home: the register, owner-authorized like applicability | proposes, through I4 of its investigation, as non-authoritative proposals | reads, if adopted |
| Free-form question → target routing hints | oracle hint store | does not build one; calls `oracle.ask` | owns, with revalidation and challenge |
| Applicability | decision owners: register applicability records, and claim-maintenance reliance once implemented | does not record; leaves it to the agent's judgment | does not record |
| Need detection, delivery triggers, worth-it judgment | decision-aware path | owns | none |
| Change, refresh, reopening | claim-maintenance (not in Rust); `reassess`; register events | none | none. Referral records only route content challenges to owners. |
| Claim publication | trusted producer | none | none |
| Register records | register owner | proposes only | none |

**B-1. Decision ↔ question relations.** [OPEN] The decision-aware
investigation plans "candidate decision ↔ question relations with evidence, as
non-authoritative proposals" (its node I4). If both paths kept their own
version, the relation would exist twice: once as decision-aware state, and
once as oracle routing hints. The proposal is that any adopted relation lives
in the register under the register owner, with owner authorization like
applicability, and that both paths read it. Neither path keeps a private copy.

## 5. Measurements for an eventual end-to-end experiment

S0's split already matches the two responsibilities.

| S0 measure | What it isolates | Owner it informs |
|---|---|---|
| Arm C against A and B | value of receiving curated knowledge, with near-perfect recognition and routing (an upper bound) | both |
| r1, need recognized (arm B, embedded items; arm A baseline) | unaided need recognition | decision-aware path |
| r2, asked a knowledge source | the hand-off across the boundary | both |
| r3, retrieval returned the keyed target | resolution and routing | oracle |
| r4, used correctly | the agent's use, given correct material | neither; reported |
| False confident reuse, false invalidation, cost per correct item | end-to-end safety and economy | both |
| Routing benchmark | routing error, without agents | oracle |

**Proposed for the end-to-end experiment:** after S0, and only if both paths
reach a testable form.

- **Arm D:** decision-aware recognition and delivery, plus oracle resolution,
  real or emulated, compared with S0's A, B and C on the same items and frozen
  key format.
- **Shared items.** S0's embedded items are tied to register decision
  identities (S0 packet, section 3), so the same items and key can serve the
  decision-aware path's qualification and the end-to-end comparison.
- **Attribution.** Each run records:
  - the need-detection event: decision identity, detection point, and whether
    the lookup was judged worth its cost;
  - the oracle request and response ids;
  - the agent's action.

  A failure can then be attributed to recognition, the boundary, resolution or
  use.
- **The decision-aware path's own measures:**
  - recognized needs per decision point;
  - unneeded lookups and their cost, since a lookup should cost less than it
    returns;
  - missed needs.

  These sit beside the shared measures. They are not merged into the oracle's.
- **Key format.** One key format serves both paths. Evidence status and
  conclusion status are kept separate, and a byte change sets only evidence
  status (S0 packet, section 5).

## 6. Changes implied for the oracle design, applied once the boundary is agreed

These are not applied now, to avoid moving files the decision-aware
investigation pinned.

- §12.8: the interpretation "S1 then centres on `oracle.check_refs` and
  contract-designated delivery" becomes "a recognition gap: input to the
  decision-aware path". This change is already reflected in S0 packet
  revision 2.
- §6, `oracle.check_refs`: remove "the realization proposed for the first
  draft's contract-required preflight". The primitive stays; deciding when a
  preflight is needed belongs to the decision-aware path.
- §17 OD-5: replace it with the section 2 contract, citing this note.
- §2 vision: name the decision-aware path as owner of recognizing a need and
  delivering knowledge.
- §12: add the attribution fields from section 5, for the end-to-end form.

## 7. Open points for the decision-aware owner and the user

1. Confirm or replace register decision records as the shared decision
   identity (section 2).
2. Should the decision-aware path call `oracle.ask` for expressed questions,
   and the consumer API or `check_refs` for exact refs, or something else?
3. Where do decision ↔ question relations live (B-1)?
4. Will the decision-aware investigation's qualification plan reuse S0's items
   and key format? Its proposal says it aligns with S0.
5. Sequencing. S0 stage 1 does not depend on the investigation, and neither
   edits the other's files. The design changes in section 6 wait for agreement
   on this note.
