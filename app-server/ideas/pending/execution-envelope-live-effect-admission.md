# ExecutionEnvelope Live Effect Admission

## Status and authority

Exploratory architectural idea, lowered from an eight-round reconciliation
chain (2026-09-27 – 2026-09-28) that traced the Branch Plan / Proposal /
Implementation Contract Compilation / slice-campaign seam down to one
concrete mechanism question: how does `ExecutionEnvelope` authority stay
live at effect boundaries instead of being frozen into a stale attempt-local
snapshot.

This document does not accept a design, amend canonical architecture, change
the roadmap, authorize implementation, or establish that a new architectural
dimension exists. It records a candidate composition — a design specification
only — that has not yet been formally reconciled against every relevant
owner or adversarially reviewed. No canonical architecture page under
`app-server/docs/architecture/` has been edited to produce this document.

```yaml
idea_provenance:
  origin: direct_capture
  captured_on: 2026-09-28
```

**Explicit non-authorization, restated because this design touches authority
semantics directly:** nothing in this document authorizes building
`ExecutionEnvelope`'s schema/store, `admitEffect`, `ExecutionAttemptAdmission`,
or any slice-campaign schema change. `ExecutionEnvelope` itself remains
`design: accepted`, `implementation: none` per `organizational-compilation.md`
§7 and `organizational-execution-envelopes-reconciliation.md` — this document
does not change that status. The reconciliation chain that produced this
document settled an architectural *shape*; it did not open an implementation
gate for any dimension that does not already have one.

### Revision note — fresh-context adversarial review, 2026-09-28

A fresh-context, same-model reviewer with no inherited builder context
reviewed revision
`a8aaea1857e55bddfb74e6ed87dd6da0111e8283de8abbdf74bba91af4690239` of this
document and returned **`BLOCKING`**. **[Rev. 2026-09-28, EEA-006, corrected]**
The review's own metadata declares `independence_claimed: false` — this
note originally described the reviewer as "independent," which contradicts
that declared metadata; corrected here to avoid asserting a property the
review itself does not claim (evidence class `accepted_same_model_review`:
same-model, fresh-context, not independent).

**[Rev. 2026-09-28, third review pass — EEA-006, unsubstantiated
explanation removed.]** The prior revision asserted that `AIR-001` had been
"folded into the `EEA-001` remediation text in the condensed response
rather than listed as its own item." That explanation was not substantiated
by anything this document can read, and is withdrawn. Only the following is
stated, because only the following is verifiable from material available
here:

- the review text delivered to this document enumerated **seven** findings:
  `EEA-001`, `EEA-002`, `EEA-003`, `EEA-004`, `EEA-005`, `AIR-002`,
  `AIR-003`;
- a later review pass reported that the durable review record contains
  **eight** finding IDs, including `AIR-001`;
- this document has no access to the durable review record, cannot verify
  which is correct, and **does not explain the discrepancy**.

`AIR-001` is tracked below on the strength of the later pass's own
statement of its content, not on any reconstruction of how it came to be
absent from the delivered text.

All findings are verified directly against cited source below — none
accepted on the review's word alone. Verification and resolution, one line
each:

- **EEA-001 (Critical — admission not atomic with commit).** Verified: the
  cited `workspace-coordination` implementation executes the mutation
  synchronously *inside* the same transaction as the authority/lease check
  (`service.mjs:112`'s `admitMutation`, `sqlite-store.mjs:57`'s
  `withAdmission`) — the design's receipt-then-execute-later shape did not
  match this. **Resolved**: §5 and §6 redesigned around a linearizable
  commit-boundary split (host-transactional effects vs. externally-committed
  effects with an explicit fence-through-commit obligation) — see §6.
- **EEA-002 (Critical — replay can duplicate irreversible effects).**
  Verified: `resource-lease-and-fencing.md` Key Invariant 4 states replay of
  an operation id is "rejected outright, never silently treated as a
  retry," and `sqlite-store.mjs:59` throws on a reused `operation_id` rather
  than returning a cached result. The design's "operation_replay = safe
  idempotent no-op" claim directly contradicted this. **Resolved**: §5/§12
  redesigned around adapter-owned idempotency (durable
  prepared/started/committed/unknown state) and a separate inspection
  lookup, with reconciliation — not automatic replay — for adapters that
  cannot provide their own idempotency guarantee.
- **EEA-003 (High — receipt cannot unambiguously identify role/subject).**
  Verified: the original `ExecutionAttemptAdmission` schema (§4) had no
  explicit role-instance, role-contract-projection, or subject field, yet
  §5 said "derive trusted role/subject from the receipt." **Resolved**: §4
  now binds `role_instance`, `role_contract_projection`, and
  `organizational_subject` explicitly; §5's derivation step now cites these
  fields directly instead of an unspecified derivation.
- **EEA-004 (High — currentness wrongly called receipt-local).** Verified:
  `slice-campaign/sqlite-store.mjs`'s `slice_campaign_admission` table is
  live, mutable state that `supersede` (line 43) atomically transfers to a
  successor identity key — the immutable receipt itself has no such state.
  **Resolved**: §5 step 2 redefined as a live lookup against the
  authoritative attempt/campaign admission table, not a property of the
  receipt; §6 fences this check through to the commit boundary.
- **EEA-005 (High — current envelope head doesn't prove upstream currency).**
  Verified: `revision-cas-and-publication.md` explicitly disclaims deciding
  "when a revision should be superseded... never a mechanical consequence
  of CAS succeeding," and no source found in `authority-and-ownership.md`
  or `organizational-compilation.md` establishes an automatic obligation
  linking parent-grant/policy contraction to `ExecutionEnvelope`
  supersession. **Resolved as a named open question**, not picked
  arbitrarily: §2/§5 now state both candidate designs (admission-time
  dependency revalidation vs. an atomic upstream-contraction obligation),
  recommend the former as the safer default absent evidence, and flag the
  question for the owning dimensions per §17.
- **AIR-002 (High — mechanical comparison lacks an owned grammar).**
  Verified: `role-and-contract-structure.md` §5 states the
  primitive-to-AEG mapping is itself "Not Yet Mapped," an open question in
  its own canonical source. **Resolved**: §5/§7 no longer claim step 6 is
  a self-contained mechanical comparison; the closed, enforceable
  effect_boundary/subject_scope grammar is named as a blocking prerequisite
  owned by `role-and-contract-structure.md`, not invented here (§16 slice
  0).
- **AIR-003 (Medium — provider-inference exclusion overbroad).** Verified:
  `runtime-realization.md` names "authority grants... for cost, provider
  use, or evidence custody" as a distinct policy-overlay category, and §9
  Safe Execution Boundaries explicitly includes "a provider turn" among
  admitted operations requiring boundary discipline. **Resolved**: §7
  narrowed to exclude only harmless local reasoning, not provider turns
  with external consequence (budget spend, data transmission, custody
  authority).
- **AIR-001 (High — tracked from the second review pass's statement of
  it; see the EEA-006 note above regarding its absence from the delivered
  first review text, which this document does not explain).** Content as
  restated by that pass: "the
  prescribed routes still do not causally guarantee 'no post-revocation
  commit.'" **Not resolved by this first-pass revision** — this document's
  first revision closed the *issuance-time* gap (a token issued and never
  re-checked) but left the *confirmation-time* gap (§6's `confirmFenced`
  step) dependent on adapter compliance with no mechanical guarantee. See
  the second revision note below for the actual fix attempt.

Eight findings were tracked in total; none failed verification in this
first pass. See inline `[Rev. 2026-09-28]` markers at each changed section.
**This first-pass revision did not fully close EEA-001 or AIR-001 — see the
second revision note immediately below, added after a second review pass
correctly identified the remaining gap.**

### Revision note 2 — second fresh-context adversarial review pass, 2026-09-28

Reviewed revision
`c4cc0598437d2eaffacd1249c9bf966f7e758d980754d7163fb7c0331ac1eee8` (the
first revision) and returned **`BLOCKING`** again. This pass confirmed
EEA-002, EEA-003, AIR-002, and AIR-003 as resolved and did not reopen them.
Six items remained: EEA-001 (still Critical), an unflagged `.transactional`
atomicity overclaim, EEA-004 (still High), EEA-005 (still High), AIR-001
(still High), and EEA-006 (the finding-count bug fixed above). The review's
own stated remediation: *"Closing it requires a host-enforced commit
mediator or remote conditional-commit/CAS protocol bound to the exact
operation and commit attempt — not merely an instruction to call
confirmFenced."*

**Investigation before redesign, not another wording patch.** Direct code
read of `workspace-coordination/service.mjs` and `sqlite-store.mjs`
confirmed the review's core claim precisely: `withAdmission`'s `mutate()`
callback executes *inside* the SQLite transaction, but SQLite's own
`BEGIN IMMEDIATE` / `COMMIT` / `ROLLBACK` atomicity covers only
SQLite-observable state — it cannot roll back a side effect `mutate()`
performs against a different store (Git, the filesystem, a network call).
The document's blanket `.transactional` classification of "repository
mutation" and "publication" (§7) as safe-by-construction was therefore
genuinely wrong, not merely imprecisely worded, exactly as the review
states citing lines 477–487 against 1123–1132.

**But investigation also found something the review did not have visible
to it: this codebase already implements exactly the "remote
conditional-commit/CAS protocol" the review's remediation calls for, for
the one effect class this document most needs it for.**
`git-publisher.mjs`'s `promote()` (line 163) calls
`git.run(sealed.repository, ["update-ref", sealed.branchRef, sealed.commit,
sealed.expectedParent])` — git's own three-argument `update-ref`, a
genuine compare-and-swap at the Git ref-storage layer: it durably succeeds
only if the ref currently equals `expectedParent`, and fails atomically
(at Git's own layer, independent of SQLite) otherwise. This is composed
*inside* the same `admitMutation` SQL transaction as the authority check,
exactly mirroring Path 1's shape for the check-and-act ordering — closing
that half of EEA-001/AIR-001 already, by construction, without inventing
anything new. And `git-publisher.mjs`'s own `promote()` `catch` block and
standalone `reconcile()` function (lines 165–181) already implement the
*other* half — the crash-recovery reconciliation the first-pass revision
called "the genuinely hard case" and declined to solve: on a failure after
`admitMutation` is called, both functions **re-observe the actual current
Git ref state** and recover cleanly — `observed === sealed.commit` means
the external effect already happened (recover or report the admission via
`inspectAdmission`, never re-execute); `observed !== expectedParent` and
`!== sealed.commit` is reported as `ambiguous` for the owning workflow to
resolve, never silently retried. This is a real, already-working, tested
pattern this document was not reusing.

Verified and resolved per finding:

- **EEA-001 (Critical) + the `.transactional` overclaim — genuinely fixed
  for the effect classes with a conditional external-write primitive; honestly
  still residual for those without one.** §5/§6/§7/§8/§13/§15 redesigned
  around a third admission shape, `admitEffect.conditionalExternal`,
  reusing `git-publisher.mjs`'s own compose-CAS-inside-the-transaction +
  observe-and-reconcile-on-failure pattern verbatim rather than inventing a
  mediator. `.transactional` is narrowed to genuinely host-local,
  single-store effects only (per the review's own remediation: "reclassify
  anything touching Git/filesystem/an external system"). `.fenced` remains,
  narrowed further to effects with no conditional-write primitive at all —
  the one path that is honestly still an open residual, not claimed closed.
- **EEA-004 (High).** `confirmFenced` (§6, `.fenced` path) and the
  reconciliation step of `.conditionalExternal` (§6) both now re-validate
  the **full** admission conjunction — attempt currentness (live campaign
  lookup), RoleRealization validity, resource fencing, token expiry, and EV
  currency — not EV alone, per the review's explicit list of what was
  missing.
- **EEA-005 (High).** The review is correct that naming a question is not
  resolving it. §2/§18 corrected to state explicitly that this document
  considers admission into implementation planning **blocked** on
  `organizational-compilation.md` and/or `authority-and-ownership.md`
  (whichever is later determined to own it) making this decision, and that
  the "admission-time revalidation" default is a **provisional
  placeholder, not adopted**, used only so the rest of this design has
  something concrete to reference — not a resolution this document is
  entitled to claim.
- **AIR-001 (High).** Resolves as a direct consequence of the EEA-001
  redesign, not separate work: causally guaranteed for `.transactional`
  and `.conditionalExternal` (by construction, per above); honestly
  residual for `.fenced`, for exactly the same reason EEA-001's own
  residual remains — no source, and no code found in this codebase, gives
  a conditional-commit primitive for effects with no such primitive at the
  external system.
- **EEA-006 (Medium).** Fixed above, in revision note 1.

**Honest assessment, stated directly rather than implied:** this revision
closes the critical findings for the effect classes that have a real
conditional-write primitive to compose with (confirmed to exist for Git
ref publication; not confirmed for any other external system this document
lists in §7). It does **not** claim to have solved atomicity in general —
`.fenced` effects (network effects with no CAS primitive, human-facing
actions) remain a named, scoped, adapter-compliance-dependent residual,
identical in kind to the one the first-pass revision left, just narrower in
scope now that Git-backed effects have moved to a real fix. Whether that
residual is acceptable, or whether it represents a genuine architecture gap
needing its own investigation before those specific effect classes can be
authorized, is not resolved here — see §18.

**[Superseded by revision note 3: the third review pass found this
revision's central `.conditionalExternal` claim — that the authority check
and the external write share one SQL transaction — to be false. It is
retracted below.]**

### Revision note 3 — third fresh-context adversarial review pass, 2026-09-28

Reviewed subject: `a3c4968bfb9428797b9f434f473140e72acdce445b2c0f4480e57ad92db31d3a`.
Verdict: **`BLOCKING`**. Evidence class `accepted_same_model_review`,
`independence_claimed: false`.

This pass is predominantly a **retraction**, not an extension. Every
finding was verified against source before acting; none was accepted on the
review's word, and none failed verification.

| Finding | Verified? | Disposition |
|---|---|---|
| EEA-001 (Critical) — no common serialization owner; "host-local" proves nothing | Yes — `slice-campaign.sqlite3` and `coordination.sqlite` are distinct files on distinct connections (construction sites read directly) | **Scope removed.** Both admission shapes are now explicitly *gated* on a serialization precondition that no current effect class satisfies (§5). |
| EEA-002 (Critical, reopened) — ABA on `expectedParent` observation | Yes — `reconcile()` returns `eligible` on the parent branch without consulting `inspectAdmission` | **Still open, named.** Operation-bound external evidence or a no-ABA generation is stated as a requirement (§5, §15); not designed here. |
| Crash recovery overstated — `published_unconfirmed`, not "recovered" | Yes — `promote()` and `reconcile()` both return `unconfirmedPublication(...)` when no admission record exists | **Fixed (factual correction).** §15 now quotes the real code and describes both branches correctly. |
| EEA-004 (High) — currentness not fenced through commit | Yes | **Largely dissolved by scope removal**: `.fenced` withdrawn, so its TOCTOU window no longer exists to defend. |
| EEA-005 (High) — upstream-authority prerequisite | Yes (dispositioned correctly already) | **Still open, named**, retained as an explicit planning blocker. |
| AIR-001 (High) — routes do not causally guarantee the invariant | Yes | **Conceded outright.** §6 now states the invariant is a requirement and that *no* current route guarantees it. |
| EEA-006 (Medium) — unsubstantiated seven-vs-eight explanation | Yes — this document cannot read the durable record | **Fixed.** Explanation deleted; only verifiable statements retained. |

**Net effect of this pass:** `.fenced` is withdrawn and fail-closed;
`.conditionalExternal`'s "same transaction" claim is retracted as false;
`.transactional` is gated on a precondition nothing satisfies. The document
no longer specifies any admittable effect class. What it now contributes is
a verified statement of *what must hold* and *which two pieces of
infrastructure are missing* — a host-enforced commit mediator, and
operation-bound external evidence binding authority generations into the
external commit check.

This is a smaller claim than the prior two revisions made. It is the
largest claim the verified evidence supports.

---

## Summary of the settled architectural placement (inherited, not reopened)

The eight prior reconciliation rounds established the following chain, none
of which this document revisits unless explicitly flagged:

```text
BranchPlan authority/mutation boundary (ceiling, not grant)
    ↓
Organizational Compilation §7 (Candidate Resolution and Admission instance):
    available ∩ authorized ∩ satisfies(required) → organizational
    authority ADMITS — the first place actual execution authority exists
    ↓
ExecutionEnvelope (immutable, revisioned — records the admitted selection,
    never itself the authority that selects it)
    ↓
Role/Contract Structure §4 (Part 2.2, proposed): derives the FINAL role
    contract, folding in "current ExecutionEnvelope" — distinct from
    Organizational Compilation's own preliminary, requirements-only role
    contract at §5 (the two-stage resolution of the corpus's earlier
    apparent three-way ordering contradiction)
    ↓
Runtime Realization: consumes the final Role/Contract Structure output only
    — never ExecutionEnvelope directly (confirmed: `RR_CONSUMES_
    PROJECTED_ROLE_CONTRACT_ONLY`)
    ↓
ExecutionAttemptAdmission (immutable receipt, Attempt grain — not Slice,
    not Operation)
    ↓
live effect admission  ← this document's own scope
    ↓
tool/provider effect, fenced by mechanisms/resource-lease-and-fencing.md
```

No part of this diagram is reopened here. This document lowers exactly one
box — "live effect admission" — into a design specification.

---

## 1. Purpose and exact owned/non-owned boundary

**Purpose.** Specify the smallest mechanism by which a later effect request,
issued during an already-admitted execution attempt, can prove that the
specific operation it wants to perform is still permitted under the
*current* authoritative organizational grant — without freezing that grant
into a stale attempt-local copy, without making the fencing mechanism or
Runtime Realization the authority owner, and without requiring every domain
adapter to reinterpret `ExecutionEnvelope` semantics independently.

**Owned by this document (proposed, not accepted):**

- the minimum durable schema `ExecutionEnvelope` needs to support live
  effect admission (§2);
- how `ExecutionEnvelope` revisions participate in the existing
  `revision-cas-and-publication` mechanism (§3);
- the `ExecutionAttemptAdmission` receipt shape (§4);
- the `admitEffect` host operation contract (§5);
- the atomic race-closure protocol between authority currency and effect
  commit (§6);
- effect-class applicability (§7);
- revocation/narrowing behavior at each lifecycle point (§8);
- the minimum integration surface with slice-campaign (§11);
- a bounded failure vocabulary and durable receipt shape (§12–§13).

**Explicitly not owned here, and not redefined here:**

- what organizational authority is, how it is admitted, or its
  available/authorized/required/selected decision procedure — owned by
  `organizational-compilation.md` §7 and
  `organizational-execution-envelopes-reconciliation.md`;
- what a role contract's semantic content is, or how it is derived —
  owned by `role-and-contract-structure.md`;
- concrete provider/model/harness realization or `RoleRealization`'s own
  generation/invalidation model — owned by `runtime-realization.md`;
- resource exclusivity/fencing mechanics — owned by
  `mechanisms/resource-lease-and-fencing.md`, reused here exactly, not
  reinvented;
- revision succession mechanics — owned by
  `mechanisms/revision-cas-and-publication.md`, reused here exactly;
- who may publish an `ExecutionEnvelope` revision, or delegate authority —
  owned by `authority-and-ownership.md`;
- slice-campaign's own workflow semantics (phases, review, terminalization,
  production-path claim consumption) — owned by its own existing code,
  touched here only at the minimum integration point (§11).

---

## 2. ExecutionEnvelope minimal schema

No `ExecutionEnvelope` schema exists in production today — confirmed by
direct grep of `app-server/src`, zero hits for `ExecutionEnvelope` or
`AuthorityGrant` as a stored artifact. The schema below is derived from two
sources only: the minimum truthful proposition already established
(`Admits(EV@e, role R, authority-ceiling A ⊆ delegable(parent), subject S,
effect-boundary X)`) and `organizational-execution-envelopes-reconciliation.md`'s
own residue definition ("instantiated multi-role organization + provenance +
role-scoped organizational projections").

```yaml
ExecutionEnvelope:
  # --- owned EV truth: identity and revision machinery ---
  envelope_identity:        # stable across revisions; names "this problem's
                             # organization," not one instant of it
  revision_id:               # content digest of this revision record minus
                             # its own id field (revision-cas-and-publication.md)
  predecessor_revision:      # exact prior revision this succeeds; absent
                             # only for the first revision of an envelope_identity

  # --- owned EV truth: the admitted selection itself ---
  admitted_organization:
    - role_identity:         # a logical role/vantage identity admitted
                              # into this envelope
      authority_ceiling:     # A, bounded per authority-and-ownership.md
                              # §8-9's child_authority ⊆ delegable(parent_authority)
      effect_boundary:       # X — the set/class of effects this role may
                              # exercise within this envelope
      subject_scope:         # S — the bounded problem/branch this role's
                              # participation covers

  # --- referenced upstream truth: never copied, cited by exact revision ---
  organizational_subject_ref:  # exact BranchPlan revision (or equivalent
                                # upstream semantic-planning artifact) this
                                # envelope was admitted for — a stable
                                # reference, not inlined branch-plan content
  authority_source_ref:        # exact parent authority grant this envelope's
                                # ceilings were delegated from
                                # (authority-and-ownership.md's own model)
  applicable_policy_refs: []   # references to organizational
                                # policy/invariant documents that bounded
                                # admission — cited, not restated
                                #
                                # [Rev. 2026-09-28, EEA-005 — STRENGTHENED
                                # on second review: naming this question is
                                # not the same as resolving it, and this
                                # document does not have authority to
                                # resolve it.] Storing these as exact
                                # references does NOT by itself guarantee a
                                # current EV head reflects current upstream
                                # authority — verified directly:
                                # revision-cas-and-publication.md explicitly
                                # disclaims deciding "when a revision should
                                # be superseded... never a mechanical
                                # consequence of CAS succeeding," and no
                                # source found in authority-and-ownership.md
                                # or organizational-compilation.md
                                # establishes an automatic obligation
                                # linking parent-grant/policy contraction to
                                # EV supersession.
                                #
                                # THIS DOCUMENT CONSIDERS ADMISSION INTO
                                # IMPLEMENTATION PLANNING BLOCKED ON THIS
                                # DECISION, owned by organizational-
                                # compilation.md and/or authority-and-
                                # ownership.md (exact owner undetermined —
                                # that determination is itself part of what
                                # is blocked). The candidate design below is
                                # a PROVISIONAL PLACEHOLDER only, adopted by
                                # nothing, used solely so the rest of this
                                # design has a concrete reference:
                                #   (a) admission-time dependency
                                #       revalidation — admitEffect's own
                                #       admission step re-checks that
                                #       authority_source_ref and
                                #       applicable_policy_refs are still
                                #       current before granting (recommended
                                #       default, since it requires no new
                                #       obligation on upstream authority-
                                #       owning workflows — but re-verify
                                #       "current" here means what it means
                                #       for the parent grant/policy's OWN
                                #       revision mechanism, not EV's);
                                #   (b) an atomic upstream-contraction
                                #       obligation — whichever act narrows a
                                #       parent grant/policy must, as part of
                                #       that same act, also supersede every
                                #       EV delegated under it.
                                # Neither is adopted here; §17 names the
                                # owning pages this belongs to.

  # --- operational metadata ---
  producer:                  # the organizational authority act/actor that
                              # admitted this selection (Candidate Resolution
                              # and Admission's own AVAILABLE/AUTHORIZED/
                              # REQUIRED/SELECTED procedure, per
                              # organizational-execution-envelopes-reconciliation.md)
  admitted_at:
  integrity_digest:          # covers the full record; distinct from
                              # revision_id, which is itself already a
                              # content digest — kept separate in case a
                              # future storage layer needs an
                              # independently-verifiable seal
```

**Explicitly not stored as EV fields (derived projections, not owned
truth):**

- `delegation_mode` (non-transferable / delegable / nomination-only /
  advisory) — this is a *classification* `authority-and-ownership.md` §9
  applies when authority is projected, not a literal field an admitted
  selection carries as its own content. **Unresolved as a schema field** —
  flagged, not invented.
- `capability_requirement` — an upstream *input* to admission (the
  `required` side of Organizational Compilation's own CRA intersection,
  per `organizational-execution-envelopes-reconciliation.md`'s corrected
  available/authorized/required/selected mapping), not something the
  admitted envelope itself needs to restate as its own content once
  selection has occurred.
- the final Role Contract — a downstream *projection* of this envelope
  (§9, below), never inlined here.

**Lifecycle/current-head state** is not a stored field either — per
`revision-cas-and-publication.md`'s own discipline, "the current head of a
revision chain is whichever revision has no successor yet — computed, not
separately stored as a mutable pointer." `ExecutionEnvelope` follows this
exactly (§3).

---

## 3. ExecutionEnvelope revision/store contract

Reuses `mechanisms/revision-cas-and-publication.md`'s vocabulary exactly —
this section does not invent an EV-specific revision theory, per that
mechanism's own Key Invariant 6 ("reusing this mechanism for a new domain
does not inherit whatever authority or scope governed its prior use — each
new instance is its own contract") and per `organizational-compilation.md`
§7's own text, already naming this exact instance ("organizational
authority admits a selection... as a new revision" of `ExecutionEnvelope`).

| Concept | Applied to ExecutionEnvelope |
|---|---|
| Stable identity vs. revision identity | `envelope_identity` (stable) vs. `revision_id` (content digest per revision) — exactly the claim-id/revision-id split already implemented in Evidence/Claims |
| Predecessor chain | `predecessor_revision`, explicit, never inferred |
| Create-only vs. successor publication | first revision for a new `envelope_identity` is create-only; every later revision for the same `envelope_identity` is a successor publication |
| Expected-head / CAS | a publish names the exact head `envelope_identity`'s chain is expected to be at; if the actual head moved, reject as conflict — never silently applied |
| Current-head lookup | computed as the revision with no successor, same as every other confirmed instance — never a separately tracked mutable pointer |
| Historical revision lookup | by `revision_id`, always available — required for `admitEffect`'s own provenance needs (§5) and for `ExecutionAttemptAdmission`'s audit trail (§4) |
| Supersession | a new revision under the same `envelope_identity`; narrowing or revoking prior grants is a domain judgment organizational authority makes, never a mechanical consequence of CAS succeeding (per the mechanism's own "What This Mechanism Does Not Decide") |
| Branching | **not permitted for this instance** — one `envelope_identity` names one bounded problem's organization; two concurrent valid heads for the same problem would mean two competing organizations exist for the same subject, which is exactly the "organizational gap" case `organizational-execution-envelopes-reconciliation.md`'s own decision procedure resolves through re-admission, not through parallel heads |
| Integrity validation | `integrity_digest`, checked independently of `revision_id`'s own content-digest identity |
| Idempotency identity | admission is a Candidate Resolution and Admission act with its own `operation_id`-style idempotency, orthogonal to the CAS conflict check — a retry of the same admission is a no-op; a competing admission against a stale head is a hard conflict, exactly Evidence/Claims' own confirmed split between these two concerns |
| Publication atomicity | the new `ExecutionEnvelope` revision and anything causally produced with it (e.g., any derived claim materialization, if one is ever wired) become visible in one indivisible step |
| Crash recovery between write and head movement | inherited directly from the mechanism's own atomic-visibility invariant — a crash between writing a revision record and it becoming the computed head cannot occur, because "head" is never a separately-set pointer to begin with; either the revision record exists (and is therefore already the head, having no successor) or it does not |

---

## 4. ExecutionAttemptAdmission schema

Re-derived from Round 5/6's candidate, testing necessity of each field
against what `admitEffect` (§5) and the audit/reconstructability discipline
(`implementation-contract-compilation.md` Key Invariant 6;
`runtime-realization.md` §10) actually require.

**[Rev. 2026-09-28, EEA-003]** The prior revision let `admitEffect` "derive
trusted role/subject scope from the receipt" without the receipt containing
any field that names a role or subject. That is unsafe for a multi-role
envelope — nothing forced a *unique* derivation. Three fields are added
below (`role_instance`, `role_contract_projection`, `organizational_subject`)
so the binding is explicit, not inferred:

```yaml
ExecutionAttemptAdmission:
  attempt_identity:
    run_id:
    slice_number:
    attempt_id:
    plan_version:            # necessary: this is the real code identity
                              # slice-campaign already uses
                              # (contract.mjs's identityKey shape)

  implementation_contract:
    identity:
    revision:                 # necessary: this is what "contract_applicable"
                              # (§5) is checked against

  # --- [Rev. 2026-09-28, EEA-003] explicit acting-role/subject binding ---
  role_instance:
    identity:                 # the exact logical role/vantage instance
                              # this attempt executes as — necessary:
                              # without this, "trusted role" in §5 step 6
                              # has no unambiguous referent when one
                              # ExecutionEnvelope admits multiple roles
    role_contract_projection:
      identity:
      revision:                # the exact Role/Contract Structure §9
                              # projection this attempt was admitted
                              # under — necessary for the model-facing
                              # causal-closure guarantee §9 (below) depends
                              # on, and for reconstructing what the model
                              # was told it could do, independent of what
                              # the live EV currently permits
  organizational_subject:
    identity:                 # the exact bounded problem/branch this
                              # attempt's role participation covers —
                              # necessary: `subject_out_of_scope` (§12)
                              # is checked against this, not inferred from
                              # implementation_contract content

  execution_envelope:
    identity:                 # necessary: scopes which envelope's lineage
                              # admitEffect must re-read
    admitted_revision:        # necessary, but PROVENANCE ONLY — the
                              # revision that was current at admission time.
                              # Never re-consulted by admitEffect as the
                              # live authority source (§5, §6). Recording
                              # it is what makes the receipt auditable;
                              # treating it as authoritative would silently
                              # recreate the frozen-snapshot failure mode
                              # this entire document exists to avoid.

  accepted_routing:
    identity:
    revision:                 # necessary: routing_accepted (§5) checks
                              # this reference

  role_realization:
    identity:
    revision:                 # necessary: runtime_realization_valid (§5)
                              # checks this reference

  admitted_at:
  admitted_by:                # the admission act/authority, for audit
  predecessor_attempt:         # necessary for the retry/successor case —
                              # slice-campaign's real `supersede` mechanic
                              # already creates a new identity key per
                              # retry; this field lets a new attempt's
                              # receipt name which prior attempt it
                              # succeeds, without slice-campaign itself
                              # needing to interpret admission content
  integrity_digest:
```

Note that `role_instance` and `organizational_subject` are still exact
*references* into `ExecutionEnvelope`'s own `admitted_organization` entries
(§2) at admission time — never a copy of the ceiling/boundary content
itself. `admitEffect` (§5) still re-reads the *current* EV head to check
those entries; this schema addition only removes the ambiguity of *which*
entry a given attempt is claiming to act as, it does not change what
governs whether that claim is still valid.

**Every field above is necessary; none is dropped.** Two candidate
additions from the task's own list were tested and rejected:

- **copied effect grants** — rejected outright (Candidate A, §6, falsified
  in round 7): a frozen copy structurally cannot reflect later contraction.
- **workstream_runnable snapshot** — not included. Round 6's Falsifier 1
  established `workstream_runnable` is a strictly earlier planning-vantage
  predicate (Branch Planner instantiation eligibility), resolved before a
  Branch Plan is even accepted, let alone before an implementation contract
  compiles against it. It has no reason to appear in an attempt-admission
  receipt for an already-compiled contract.

**Branch/workstream identity does not belong in this receipt.** Per Round 5's
grain table, Branch/workstream identity is a **Slice**-grain binding (stable
across an attempt's retries), while every field in this receipt is
**Attempt**-grain (freshly admitted per attempt). Placing it here would
duplicate a fact slice-campaign's own Slice-level state already owns once
that state is extended to carry it (§11) — this receipt cites the Slice
identity transitively, through `attempt_identity.run_id` +
`attempt_identity.slice_number`, rather than repeating Branch/workstream
identity as a second, potentially divergent copy.

---

## 5. `admitEffect` operation contract

**[Rev. 2026-09-28, twice-revised]** First restructured in response to
EEA-001, EEA-002, EEA-003, and EEA-004 (first review pass); restructured
again in response to the second review pass, which correctly found the
first restructuring's `.transactional`/`.fenced` split still overclaimed
atomicity for effects crossing a store boundary (repository mutation,
publication) and left `.fenced`'s confirmation step checking only EV,
not the full admission conjunction. The corrected contract now has
**three** callable shapes, not two — the third reuses an existing,
already-implemented pattern (`git-publisher.mjs`'s compose-CAS-inside-
transaction plus observe-and-reconcile-on-failure shape) rather than
inventing a commit mediator.

### Precondition: a demonstrated common serialization owner

**[Rev. 2026-09-28, third review pass, EEA-001]** The third review found
that this document's remaining atomicity claims rested on an unstated and
false assumption: that being "on the same host" or "inside an SQLite
transaction" is sufficient to serialize an authority generation against a
mutation. It is not. A transaction serializes only the *one* database
connection it is opened on.

This document verified the reviewer's specific claim directly by reading
the construction sites of both stores:

- slice-campaign state opens
  `path.join(path.resolve(stateRoot), "slice-campaign.sqlite3")`
  (`app-server/src/services/slice-campaign/capability-host-runtime.mjs`,
  in `createSupervisorCampaignCapabilityHostRuntime`);
- workspace-coordination state opens
  `storePath ?? path.join(root, "coordination.sqlite")`
  (`app-server/src/services/workspace-coordination/runtime.mjs:37`).

These are **two distinct database files on two distinct connections**, each
wrapping its own writes in its own `BEGIN IMMEDIATE` (`#transaction` in
each store). A `BEGIN IMMEDIATE` on `coordination.sqlite` does not
serialize anything in `slice-campaign.sqlite3`, and vice versa. The
reviewer's finding is confirmed, not merely accepted.

`ExecutionEnvelope` compounds this: it has **no store at all** — zero
implementation anywhere in `app-server/src`, as this document has stated
since its first revision. There is therefore no database in which an EV
authority generation could today be read under the same transaction as
any mutation admission.

**The resulting requirement, stated as a precondition rather than a
solved problem:**

```text
For any effect class to be admittable with a causal
no-post-revocation-commit guarantee, EVERY generation the admission
decision depends on — EV revision, attempt currentness, RoleRealization
validity, resource fence generation — and the mutation admission record
itself MUST be serialized by a single, named, demonstrated owner:
one connection's transaction, or an external primitive whose own
conditional-commit check is bound to those same generations.

"Same host", "same process", and "inside an SQLite transaction" do not
satisfy this. The owner must be named per effect class and demonstrated,
not assumed.
```

**No effect class currently satisfies this precondition.** That is a
statement about the present state of the code, verified, not a pessimistic
framing. It is the reason both admission shapes below are described as
gated rather than available.

### Two admission shapes, both gated on unmet preconditions

```text
admitEffect.transactional(attempt_identity, effect_class,
    subject_or_resource_identity, operationId, mutate)

    — [Rev. 2026-09-28, narrowed again on third review] for effects whose
      ENTIRE mutation is host-local and SQL-observable — no Git,
      filesystem, or external-system write of any kind inside `mutate` —
      AND whose every authority generation is stored in THE SAME database
      connection as the mutation admission record, per the serialization
      precondition above.

      GATED, NOT AVAILABLE. No current effect class is known to satisfy
      the second condition, because `ExecutionEnvelope` has no store at
      all, so an EV generation cannot presently be read inside any
      mutation's transaction. This path becomes available only once EV
      state exists AND is demonstrably co-located with the mutation
      admission store it must be serialized against — a real constraint
      on where EV's store may live, recorded here as a design consequence
      rather than an implementation detail deferred to later.

      The second review's finding that SQLite cannot roll back a side
      effect in a store it does not own remains correct and is retained.
      The third review's additional finding — that "host-local" does not
      by itself establish a common serialization owner — is also correct,
      and is why this path is now gated rather than merely narrowed.

admitEffect.conditionalExternal(attempt_identity, effect_class,
    subject_or_resource_identity, operationId, conditionalMutate)
    → { status: "committed", result } | { status: "reconciliation_required" }

    — [Rev. 2026-09-28, added on second review, substantially corrected
      on third] for effects whose external system exposes its OWN
      conditional/compare-and-swap commit primitive. The one concretely
      confirmed instance: Git's three-argument
      `update-ref <ref> <new> <old>`, implemented in
      `git-publisher.mjs`'s `promote()` (line 163) —
      `git.run(repo, ["update-ref", branchRef, commit, expectedParent])`,
      which durably succeeds only if the ref currently equals
      `expectedParent`, and fails atomically at Git's own layer otherwise.

      GATED, NOT AVAILABLE — two verified defects, both found by the
      third review, neither closed by this document:

      (1) THE CAS IS NOT BOUND TO ANY AUTHORITY GENERATION. Git's
      conditional check compares only `expectedParent` — a ref value. It
      does not and cannot check an EV revision, attempt currentness,
      RoleRealization validity, or a fence generation. An authority
      revocation landing between this document's own step 6 (authority
      check) and the `update-ref` call is therefore entirely unprotected:
      the CAS will still succeed, because the ref has not moved. The
      second revision claimed `conditionalMutate` runs "inside the same
      SQL transaction as the authority check", which would have closed
      this — that claim is RETRACTED. It was not true: the authority
      generations are read from stores other than `coordination.sqlite`
      (and, for EV, from no store at all), so no single transaction spans
      them. Git CAS protects against a concurrent *ref* change. It does
      not protect against a concurrent *authority* change, and this
      document no longer claims it does.

      (2) NO OPERATION-BOUND EXTERNAL EVIDENCE EXISTS. This document
      checked whether the commit object itself carries anything tying it
      to a specific operation. It does not: `git-publisher.mjs:145`
      builds the commit via
      `["commit-tree", preparation.tree, "-p", preparation.expectedParent]`
      with input `subject` + optional `body` only. There is no
      `operationId` trailer and no operation-bound field. Whether a
      particular caller happens to put an operation id in the message
      body is a caller convention, not a guarantee of this mechanism, and
      this document does not assume one.

      Consequence (EEA-002, reopened and NOT closed here): observing
      `ref == expectedParent` after a failure does NOT prove the
      operation never committed. The sequence parent → our commit →
      external reset back to parent is indistinguishable from "nothing
      happened". This document verified that `reconcile()`
      (`git-publisher.mjs:173`) returns `status: "eligible"` on the
      `observed === sealed.expectedParent` branch WITHOUT consulting
      `coordination.inspectAdmission(operationId)` — even though the
      sibling branch (`observed === sealed.commit`, line 176) does
      consult it. A mitigating primitive therefore already exists in the
      codebase but is not applied on the path where ABA occurs.

      REQUIRED BEFORE THIS PATH IS ADMISSIBLE, stated as requirements
      this document does not implement:
        - an operation-bound external receipt (e.g. an `operationId`
          trailer written into the commit object, making "which operation
          produced this state" externally observable), or a monotonic
          no-ABA generation on the external resource; AND
        - a binding between the external conditional check and the
          authority generations, so that a revoked authority causes the
          external commit itself to fail — not merely an earlier
          host-side check to have passed.

admitEffect.fenced(...)  — REMOVED, 2026-09-28, third review pass.

    — This shape previously covered effects with no conditional-write
      primitive at the external system at all (arbitrary network effects,
      human-facing actions). It is withdrawn entirely rather than retained
      with caveats, per the third review's stated remediation ("exclude or
      fail-close `.fenced` effects until a host-enforced commit mechanism
      exists").

      Choice made and reason: EXCLUDED FROM SCOPE, and additionally
      specified as unconditionally fail-closed, so that the absence of a
      mechanism cannot be read as permission. An effect class with no
      external conditional-commit primitive is NOT ADMITTABLE by
      `admitEffect` under this design at any level of adapter diligence.

      Why "supported with a named residual" was the wrong disposition:
      the residual was adapter compliance with a `confirmFenced` call,
      and a guarantee that depends on the adapter choosing to invoke its
      own check is not a guarantee — a non-compliant or crashed adapter
      bypasses it silently, and the host cannot detect the bypass. The
      third review additionally established that even a fully compliant
      adapter retains a confirmation-to-commit TOCTOU window, because the
      confirmation and the external commit are separate steps with no
      common serialization owner between them. Both defects are inherent
      to the shape, not to its description, so narrowing the wording
      again would not have changed the guarantee.

      Admitting such effects requires a host-enforced commit mediator —
      a component that itself invokes the external commit primitive, so
      no gap exists between check and commit for an adapter to occupy.
      That mediator does not exist and is NOT designed by this document.
```

Everything beyond the four/five caller-supplied arguments above — role/
subject scope, which `ExecutionEnvelope` lineage applies, which
`RoleRealization` is current — is **derived host-side** from
`attempt_identity`, never accepted as caller input. This follows
`mechanisms/resource-lease-and-fencing.md`'s own precedent directly:
`holder` and `intentId` are opaque strings to that mechanism, with all
authorization judgment performed host-side against durable state, never
against caller-asserted content. Caller-supplied authority claims — an EV
revision string, a role identity, a grant description — must never become
authoritative merely by being present in the request. If a caller does
supply such a value, the host either ignores it or, if useful for early
fast-path rejection, treats it purely as a hint to be independently
re-verified, never trusted.

**[Rev. 2026-09-28, EEA-002] `operationId` semantics — corrected.**
`resource-lease-and-fencing.md` Key Invariant 4 is unambiguous: "admission
is single-use per operation id; replay of the same operation id is
rejected outright, never silently treated as a retry" — and
`sqlite-store.mjs:59` implements exactly that (throws
`"mutation operation id was already admitted"` on reuse, it does not
return a cached outcome). The prior revision's "identical operationId =
idempotent no-op" claim was wrong. Corrected contract:

- A repeated `operationId` — identical or conflicting request content —
  is **rejected by `admitEffect` itself**, exactly as the reused mechanism
  already behaves. `admitEffect` never decides on the caller's behalf
  whether a repeat is "the same, so safe to no-op" — that judgment
  requires knowing whether the underlying effect actually executed, which
  `admitEffect` (an authority-currency check) does not know and must not
  guess.
- A companion, read-only `inspectEffectAdmission(operationId)` lookup
  (mirroring `service.mjs:89`'s real `inspectAdmission(operationId)`) lets
  the calling adapter check, *before* generating a new `operationId*, or
  before treating a crash-recovery resume as safe, whether this exact
  operation was already admitted and what its recorded `outcome` (§12) and
  effect-adapter-owned idempotency state were.
- **Adapters, not `admitEffect`, own idempotency of the underlying effect —
  except for `.conditionalExternal`, where the external system's own CAS
  primitive supplies it.** Per the remediation this finding specified: an
  adapter that can express its own effect as idempotent (a durable
  `prepared → started → committed` state machine keyed by `operationId`,
  checked before re-attempting) may safely resume using
  `inspectEffectAdmission`'s result. An adapter whose external system
  cannot provide that guarantee must not resume automatically after an
  `unknown` outcome (§12, §15) — it must enter reconciliation, exactly as
  §8's "already-committed/irreversible effect" row already specifies,
  rather than this document claiming a safe replay it cannot actually
  guarantee. **[Rev. 2026-09-28, second pass]** `.conditionalExternal`
  effects are the one case where `admitEffect` itself can supply this
  guarantee rather than delegating it to the adapter, because the external
  system's own conditional-write primitive makes "was this already
  applied" mechanically observable (§5, §6) — this is why
  `.conditionalExternal` is worth the extra admission shape rather than
  folding it into `.fenced`.

### Validation order (applies to all three shapes)

```text
1. resolve ExecutionAttemptAdmission receipt from attempt_identity
   — belongs inside admitEffect (it is the entry point's own lookup)

2. [Rev. 2026-09-28, EEA-004] validate attempt currentness against the
   LIVE, authoritative attempt/campaign admission state — NOT a property
   of the immutable receipt itself. The receipt has no superseded/retired
   field; slice-campaign's own real `slice_campaign_admission` table
   (sqlite-store.mjs) is the actual live record of which attempt currently
   holds admission for a given workspace, and `supersede` (line 43)
   atomically transfers it. This check reads THAT table, keyed by the
   receipt's attempt_identity, and fails `attempt_not_current` if a
   successor now holds it — it is a live lookup, not a "cheap,
   receipt-local check" as the prior revision incorrectly called it.
   — belongs inside admitEffect, but is a call into slice-campaign's own
     authoritative admission state, not an admitEffect-owned fact

3. [Rev. 2026-09-28, EEA-003] resolve role_instance, role_contract_
   projection, and organizational_subject from the receipt's own explicit
   fields (§4) — no longer an ambiguous "derivation," a direct field read
   — belongs inside admitEffect

4. resolve EV stable identity from the receipt's execution_envelope.identity
   — belongs inside admitEffect

5. read CURRENT ExecutionEnvelope head for that identity
   — belongs inside admitEffect, via the existing revision-cas-and-
     publication current-head lookup (§3) — no new lookup mechanism

6. validate requested (role_instance, organizational_subject, effect_class)
   against the current envelope's admitted_organization entries (§2)
   — belongs inside admitEffect, but see AIR-002: this comparison is only
     mechanically decidable once role-and-contract-structure.md supplies
     a closed effect_boundary/subject_scope grammar (§16 slice 0); until
     then this step's own decidability is a named blocking prerequisite,
     not something this document may assume solved

7. resolve RoleRealization current/applicable state, IF this effect_class
   depends on runtime capability (§7, §10)
   — belongs ADJACENT, not inside: admitEffect calls into Runtime
     Realization's own validity check (already owned there) rather than
     re-implementing realization-validity logic itself

8. resolve resource fence/token, IF this effect_class touches a fenced
   resource (§6, §7)
   — belongs ADJACENT: admitEffect composes with
     mechanisms/resource-lease-and-fencing.md's own admitMutation, never
     reimplementing fencing

9. [Rev. 2026-09-28, EEA-001, twice-revised] admit and, for
   `.transactional` and `.conditionalExternal`, EXECUTE the effect
   (respectively: the plain synchronous mutation; the conditional
   external write) inside one linearizable SQL transaction boundary (§6)
   — not two separable steps for either path. For `.fenced`, issue the
   single-use admission token bound to the exact generations checked in
   steps 5/8 (§6).
   — belongs inside admitEffect: this is the operation's own
     race-closure step

10. return a durable admission outcome (§13) to the calling adapter —
    for `.transactional`, this includes the mutation's own result,
    admitted and executed together; for `.conditionalExternal`, this is
    `{status: "committed", result}` on success or
    `{status: "reconciliation_required"}` if the SQL commit failed after
    the external CAS already durably succeeded (§6); for `.fenced`, this
    is the token the adapter must present at external commit (§6)
    — belongs inside admitEffect: the operation's own durable output
```

`admitEffect` is deliberately **not** an omniscient function: steps 7 and 8
call into existing, independently-owned components (Runtime Realization's
own realization-validity judgment; Resource Lease and Fencing's own
`admitMutation`) rather than re-deriving their logic. `admitEffect` owns
only the authority-currency judgment (step 6) and the composition/atomicity
of the full check (step 9).

---

## 6. Atomic race-closure protocol

**[Rev. 2026-09-28, EEA-001, twice-revised.]** The first-pass revision
redesigned this section once, correctly for `.transactional` but
incompletely: it still classified Git/filesystem-touching effects as
`.transactional` (false — SQLite cannot roll back a side effect in a store
it does not own) and left `.fenced`'s `confirmFenced` re-checking EV only,
not the full conjunction. Both are corrected here, on a second review pass
that verified the finding directly against `workspace-coordination`'s real
code and, additionally, located an existing, working conditional-commit
primitive this document was not yet reusing (`git-publisher.mjs:163`'s
three-argument `update-ref`).

The race, formalized:

```text
t0: host checks current EV permits mutation M
t1: admitEffect begins
t2: EV supersedes; the successor revokes/narrows M's authorization
t3: mutation executes (possibly outside host control — an external commit)
```

**Path 1 — `.transactional` effects (§5): the gap is closed by construction,
not by a token — genuinely, now that this path is narrowed to effects with
no external-store component at all.** The authority check (steps 4–8 of
§5) and the mutation itself execute inside one transaction, exactly
mirroring `withAdmission`'s own shape: the transaction reads the current EV
head, validates the comparison, calls the caller's synchronous `mutate`,
and only then commits the admission record. A concurrent EV supersession
either lands before this transaction begins (t2 < t1: the transaction
reads the new, narrower state and fails closed at step 6) or after it has
already committed (t2 > t3: the effect already lawfully executed under
authority that was current at the time; §8's "already-committed" row
governs, not this section). There is no window in which stale authority
can be read and then acted upon later, because "later" does not exist for
this path — read and act are the same transaction, and — unlike the
reviewed revision's claim — nothing outside SQLite's own control is
touched, so SQLite's atomicity genuinely covers the whole operation.

**[Rev. 2026-09-28, third review pass — this holds only under an unmet
precondition.]** The paragraph above is written in the present tense but
describes a state that does not exist. It requires "the transaction reads
the current EV head" — which presumes the EV head is readable *from this
transaction's own connection*. `ExecutionEnvelope` has no store at all, and
the two stores that do exist (`slice-campaign.sqlite3`,
`coordination.sqlite`) are separate databases (§5). Path 1's guarantee is
therefore **conditional on Prerequisite A** (§16): EV state existing, and
being co-located in the same database connection as the mutation admission
it must be serialized against. Until then, Path 1 describes a design
target, not an available capability, and this document does not claim
otherwise.

**Path 2 — `.conditionalExternal` effects (§5): the race is NOT closed.
[Rev. 2026-09-28, third review pass — prior claim retracted.]** The second
revision asserted that "the authority check (steps 4–8) and the conditional
external write happen in the same SQL transaction." That assertion is
withdrawn as false. The authority generations are not all readable from one
connection: attempt currentness lives in `slice-campaign.sqlite3`, mutation
admission in `coordination.sqlite`, and EV in no store at all (§5's
serialization precondition, verified). No SQL transaction spans them, so
there is no transaction inside which the external write could be placed to
close the t0/t1/t2 race against an authority change.

What Git's CAS *does* close is a narrower race than the one this section is
about: it prevents a concurrent **ref** movement from being overwritten,
because `update-ref <ref> <new> <old>` fails at Git's own layer if the ref
no longer equals `<old>`. It does not observe authority at all. An EV
supersession occurring between step 6 and the `update-ref` call leaves the
ref untouched, so the CAS succeeds and a revoked-authority write commits.
This document does not claim otherwise.

Two further windows remain open on this path, both verified:

- **Crash between external CAS success and the admission record's durable
  write.** The external system committed; the host has no record. This is
  real and unavoidable without a mediator or an operation-bound receipt.
- **ABA on recovery (EEA-002, reopened).** Observing that the ref equals
  the pre-condition does not prove nothing happened, because the ref may
  have moved and been moved back. `reconcile()` returns `eligible` on that
  branch without consulting `inspectAdmission(operationId)`, so a retry can
  double-apply. See §5 for the verified line-level detail.

Closing Path 2 requires the two requirements named in §5 (operation-bound
external evidence or a no-ABA generation, AND a binding between the
external conditional check and the authority generations). Neither is
designed here.

**Path 3 — `.fenced` effects: WITHDRAWN (§5). [Rev. 2026-09-28, third
review pass.]** The token-plus-`confirmFenced` construction described in
the prior revision is removed, not narrowed. Two independent defects, both
inherent to the shape rather than to its wording, made it unfixable as
described:

- `confirmFenced` closes the gap only if the adapter calls it, at the
  correct moment, before committing. Nothing in this design or anywhere in
  this codebase can compel that, and a bypass is undetectable by the host.
- Even a fully compliant adapter retains a confirmation-to-commit TOCTOU
  window, because confirmation and external commit are separate steps with
  no common serialization owner between them (§5's precondition).

Effects with no external conditional-commit primitive are therefore
**not admittable** under this design, and are specified as fail-closed so
that the absence of a mechanism cannot be misread as permission. Admitting
them requires a host-enforced commit mediator that itself performs the
external commit; that mediator is named as required infrastructure and is
not designed by this document.

Candidate A (EV generation folded directly into the resource-mutation
admission compare condition) is rejected as insufficiently general — not
every effect touches a fenced resource (§7), so authority currency cannot
be piggybacked onto fencing's own compare step for effects that have no
resource-fencing check at all. Candidate C (one combined generation
parameter) is rejected for the same reason established earlier in this
chain: **EV revision and physical fencing generation must remain
independently validated, never collapsed into one generation** — stated as
a required invariant, not a preference, because
`resource-lease-and-fencing.md` itself states fencing "does not mint or
expand authority on acquisition," meaning a valid fence alone can never
substitute for a valid, current authority check.

**Required invariant, stated precisely. [Rev. 2026-09-28, AIR-001 — the
invariant is retained as a REQUIREMENT; this document explicitly does NOT
claim any currently-specified route causally guarantees it.]**

```text
No effect may commit if either:
    the semantic authority generation (current ExecutionEnvelope head, as
    it applies to this role/subject/effect), or
    the physical resource generation (current fencing generation, where
    the effect touches a fenced resource)
ceased to be authoritative before the effect crossed its mutation boundary.
```

**AIR-001, stated without hedging.** The invariant above is what the design
is *for*. No route specified in this document currently guarantees it:

- **Path 1 (`.transactional`)** — gated. Would guarantee the invariant for
  its narrow class, but only once EV state exists AND is co-located in the
  same database connection as the mutation admission. No effect class
  satisfies this today.
- **Path 2 (`.conditionalExternal`)** — gated. Git's CAS is bound to a ref
  value, not to any authority generation, so it does not enforce the
  invariant; and ABA on recovery is unresolved absent operation-bound
  external evidence.
- **Path 3 (`.fenced`)** — withdrawn and fail-closed. The invariant is not
  claimed for it because the path is not admittable.

The honest summary: this document specifies **what** must hold and **which
two pieces of infrastructure are missing** before anything can be admitted
under it. It does not currently deliver an admittable effect class. Stating
that plainly is preferable to a fourth restatement of a guarantee the
verified code does not support.

---

## 7. Effect classes / applicability

`admitEffect` applies to **authority-bearing effects only** — not to every
action an attempt performs. Per the task's own instruction not to impose
heavy mutation admission on harmless observation unless authority semantics
require it, and per `resource-lease-and-fencing.md`'s own precedent of
declaring resource types explicitly rather than universally (`RESOURCE_TYPES`
is a domain-declared enum, never every possible action):

```text
requires admitEffect.conditionalExternal — GATED, see §5 (Git-backed CAS
primitive confirmed to exist, but NOT bound to any authority generation,
and no operation-bound external evidence exists; not admittable until both
are supplied):
    - [Rev. 2026-09-28, second pass] Git-ref-backed repository mutation and
      publication specifically — anything routed through
      git-publisher.mjs's own promote()/publish() shape, which already
      uses update-ref's three-argument CAS form (§5, §6)

NOT ADMITTABLE — fail-closed [Rev. 2026-09-28, third pass; formerly
"requires admitEffect.fenced", now withdrawn per §5]:
    - repository/filesystem mutation NOT routed through a Git-ref CAS
      (e.g., a direct filesystem write with no compare-and-swap primitive
      at the target store)
    - network effects with external, organization-scoped consequence,
      where the external system offers no conditional-commit primitive
      this document has confirmed (§18 — not exhaustively surveyed)
    - human-facing actions taken on the organization's behalf
    - [Rev. 2026-09-28, AIR-003] provider turns that spend budget,
      transmit organization-scoped data, or exercise privacy/custody
      authority — see corrected boundary below

    These classes have no external primitive to condition against. Under
    this design they are refused, not admitted-with-a-caveat. Admitting
    them requires the host-enforced commit mediator named in §5, which
    this document does not design.

requires admitEffect.transactional — GATED, see §5 (host-local only, AND
requires EV state co-located in the mutation-admission database; no effect
class satisfies the second condition today because EV has no store):
    - [Rev. 2026-09-28, narrowed] any effect whose entire mutation stays
      inside SQL-observable state with no Git/filesystem/external-system
      component — e.g., a pure workspace-coordination-style admission
      record with no side effect of its own
    - any effect that exercises a fenced resource with no external-commit
      component (workspace-coordination's seven resource types, where the
      resource itself is host-local)

does not require admitEffect:
    - harmless local reasoning with no external consequence — a model
      "thinking" with no budget spend, no data transmission, and no
      custody/privacy exercise
    - filesystem reads
    - claim reads (evidence-and-claims.md's own record_reliance/read path
      is a separate, already-governed mechanism)
```

**[Rev. 2026-09-28, AIR-003] Corrected boundary — the prior revision's
blanket "provider inference itself... reasoning is not an effect" exclusion
was overbroad.** Verified directly: `runtime-realization.md` names
"authority grants" as one of four distinct operator-policy-overlay
categories, explicitly scoped to "cost, provider use, or evidence custody"
— i.e., a provider turn can be exactly the kind of thing organizational
authority already governs, not categorically outside it. §9 Safe Execution
Boundaries independently lists "a provider turn" among the admitted
operations requiring boundary discipline ("a provider turn, tool operation,
lifecycle transition, or other admitted operation executes under one
stable realization from start to finish"), treating it as a governed
operation, not an ungoverned one. The corrected rule: `admitEffect`
distinguishes a provider turn's *reasoning* (excluded — genuinely harmless,
no organization-scoped consequence by itself) from a provider turn's
*external consequences* (included — budget spend, data transmission,
privacy/custody exercise), and classification of any specific provider
interaction into "reasoning only" vs. "has external consequence" is left to
the owning runtime/effect contract (`runtime-realization.md`'s own policy
overlay and Role/Contract Structure's effect-boundary vocabulary), not
invented here as a bespoke rule.

This list is **delegate-first**: `admitEffect` does not invent a universal
effect taxonomy. Where an existing role/effect vocabulary already exists
(the role contract's own effect-boundary content, §1/§9 of
`role-and-contract-structure.md`), `effect_class` values should be drawn from
that vocabulary rather than a new one defined here. Per AIR-002 (§16 slice
0), that vocabulary itself is not yet closed — this document's own §7 list
is therefore provisional, not a substitute for the canonical mapping.

---

## 8. Revocation / narrowing behavior

Each case below uses only precedent already established in this
reconciliation chain or in the cited mechanism/dimension pages — no
invented interruption semantics.

| EV successor case | Effect on attempt |
|---|---|
| Unrelated topology successor | No effect. The current head's content for this attempt's role/subject/effect is unchanged. |
| Same grant preserved | Continue. No contraction occurred. |
| Authority narrowed before next operation | The next `admitEffect` call for the narrowed scope fails closed (`effect_not_authorized` or `subject_out_of_scope`, §12). Already-admitted, already-committed effects are not retroactively undone — `authority-and-ownership.md` §12: "negative evidence may contract what is currently valid... it may not create or expand who is authorized," which bounds *future* admission, not past, already-lawful acts. |
| Effect revoked while the model is reasoning (no effect request yet issued) | Reasoning may continue; the next affected `admitEffect` call fails closed. Reasoning itself is not an effect (§7). |
| Effect revoked after `admitEffect` admits but before the mutation commits | **[Rev. 2026-09-28, EEA-001, twice-revised]** Path-dependent, not a single answer. **Path 1 (`.transactional`):** cannot occur — admission and commit are the same transaction (§6), no window exists. **Path 2 (`.conditionalExternal`):** cannot occur in the read-then-act sense either — the authority check and the conditional external write are the same transaction, identically to Path 1; the only residual is the disjoint case where the external CAS already durably succeeded and the SQL commit separately fails, which is not a revocation race at all (the effect was lawful when it committed) but a bookkeeping gap closed by mandatory reconciliation (§6, §15), never by blind retry. **Path 3 (`.fenced`):** can occur, and is exactly what `confirmFenced` (§6) exists to catch — the adapter's presentation of the token at the external commit boundary re-checks the full admission conjunction (EV, attempt currentness, RoleRealization, resource fence, token expiry — §5, §6, corrected from the first-pass revision's EV-only check); if any term became invalid between token issuance and `confirmFenced`, the confirm call fails closed and the adapter must not proceed to the external commit. This remains honest about Path 3's residual trust boundary (§6, §18): the guarantee holds only if the adapter actually calls `confirmFenced` immediately before committing — this is the one row of this table that is still not fully closed, and this document says so directly. |
| Effect revoked during an already-committed/irreversible external effect | **Cannot be rolled back by this mechanism — only reconciliation, never retroactive undo, can be truthfully guaranteed.** This mirrors `runtime-realization.md` §9's own Safe Execution Boundaries language ("operation drains, terminates, or is reconciled") and `authority-and-ownership.md` §12's own generalized shape, applied here: authority currency at the moment of commitment is what `admitEffect` guarantees; it cannot retroactively make an already-externally-committed effect not have happened. The owning workflow (not this mechanism) determines what reconciliation means for that specific external effect. |
| Role removed/replaced | The current effect admission for that role fails (the role no longer exists in the current envelope's `admitted_organization`, so step 6 of §5 fails mechanically). Whether the whole attempt transitions to a blocked/reconciliation state, or only the affected effect fails while the attempt continues under its remaining valid role scope, is **not decided by `admitEffect` itself** — that is the attempt's own lifecycle owner's decision (slice-campaign's existing phase machinery, §11), consuming `admitEffect`'s failure outcome as an input, not inventing automatic cancellation here. |

---

## 9. Role Contract relationship

```text
ExecutionEnvelope
    the authoritative organizational grant — owned, revisioned, live-checked

Role Contract
    a bounded, role-facing, mechanically enforceable PROJECTION of EV's
    consequences for one role — not a second live-authority source
```

**Which EV facts must appear in Role Contract for model-facing causal
closure:** the role's own admitted `authority_ceiling`, `effect_boundary`,
and `subject_scope` (§2) — exactly the fields `role-and-contract-structure.md`
§4's Part 2.2 already lists as inputs ("semantic obligation + required
vantage + authority grant + independence constraints + available
capabilities + current ExecutionEnvelope + system invariants"). These need
to be visible to the model so it can reason about what it may do, without
the model needing to consult `ExecutionEnvelope` directly.

**Which facts need only be host-visible:** `envelope_identity`,
`revision_id`, `predecessor_revision`, and the operational metadata fields
(§2) — these support `admitEffect`'s own provenance and audit needs but have
no bearing on what the model should reason about.

**How Role Contract binds exact EV provenance:** Part 2.2's derivation
should record, as part of the derived role-contract content, the exact
`execution_envelope.identity` and `admitted_revision` it was derived from —
the same provenance-citation pattern this document's own
`ExecutionAttemptAdmission` uses (§4). This is not currently specified by
`role-and-contract-structure.md` itself; naming it here is a **proposed
schema addition** for that page (§17), not an assertion that it already
exists.

**Why Role Contract is not consulted as the live source of truth during
`admitEffect`:** `role-and-contract-structure.md` §7 states its own
compiler is "fully deterministic end to end... regenerating an unchanged
skill package produces no changes" — it describes a point-in-time
rendering pipeline with no automatic re-render-on-EV-change discipline. A
superseded EV does not imply Role Contract has already been, or ever will
be, automatically regenerated. Checking RC live would either (a) silently
assume freshness that isn't guaranteed, or (b) require RC regeneration to
become part of `admitEffect`'s own critical path, which would make a
projection mechanism into a second authority source — exactly what this
document's own primary question forbids. **Historical Role Contracts remain
reconstructable** by the same immutable-revision discipline every other
artifact in this chain already follows (§3): a superseded EV does not
rewrite a Role Contract already derived from an earlier EV revision: it
simply means that Role Contract is no longer the *current* projection, a
fact recorded by comparing its own cited EV provenance against EV's current
head, never by mutating the old Role Contract in place.

---

## 10. Runtime Realization relationship

```text
EV current + admitted_organization entry for (role, subject, effect)
    → effect is authorized                          [admitEffect, §5 step 6]

RoleRealization current/applicable
    → the chosen runtime can physically realize the effect
                                                      [Runtime Realization's
                                                       own §4/§7 validity]

resource fence current
    → this operation holds exclusive/live right to the affected resource
                                          [resource-lease-and-fencing.md]
```

Each is independently necessary where applicable, and `admitEffect` composes
with, rather than reimplements, the latter two (§5, steps 7–8).

**RoleRealization validity is checked both at attempt admission and at each
effect launch, depending on effect class:**

- **At attempt admission** (populating `role_realization.revision` in
  `ExecutionAttemptAdmission`, §4): a snapshot, sufficient for effects that
  don't independently re-verify capability per-call.
- **At each effect launch**, specifically for effects whose `effect_class`
  depends on runtime capability continuity (long-running or
  capability-sensitive effects): `admitEffect`'s step 7 re-resolves current
  RoleRealization validity rather than trusting the attempt-admission
  snapshot, because Round 6's own finding stands — RoleRealization can go
  stale independently mid-attempt without invalidating the attempt itself,
  and a *new* operation should not reuse a realization that has since gone
  stale even though the attempt's original admission snapshot still shows
  it as valid.

This never lets runtime capability become authority: RoleRealization
validity answers "can this be physically done," never "is this
organizationally permitted" — the latter is exclusively EV's question
(§2, §5 step 6).

---

## 11. slice-campaign integration

Minimum integration only, per this reconciliation's repeated finding that
slice-campaign remains `WORKFLOW_REALIZATION_ONLY` (round 5) and does not
acquire authority merely by storing references:

- **Slice** (the `(run_id, slice_number)` grain) gains a
  `branch_workstream_identity` reference field, if and when Branch Plan
  identity exists in code at all (it does not today — confirmed zero
  `branch_plan` hits in `app-server/src` across every round of this chain).
  This is named here as the correct *location* for that binding, not as
  something this document authorizes building before Branch Plan's own
  schema exists.
- **Attempt** (the `(run_id, slice_number, attempt_id)` grain) gains a
  reference to its `ExecutionAttemptAdmission` receipt — one reference
  field, not the receipt's content inlined.
- **Existing phases** (`accepted → implementing → gate_ready →
  review_ready → terminal`) remain unchanged. **No new phase is
  necessary**: admission is a precondition checked before/at the
  `implementing` phase begins, not a phase transition in its own right —
  the existing phase machinery already gates on external preconditions
  (e.g., `terminalize`'s existing requirement for production-path claim
  evidence) without each precondition owning its own phase.
- **Existing retry/supersede semantics** (`sqlite-store.mjs`'s `supersede`,
  creating a new identity key) already create a new attempt identity for a
  new attempt; per this document's own design, a new attempt identity
  requires a new `ExecutionAttemptAdmission` receipt (§4's
  `predecessor_attempt` field names the chain), reusing the existing
  supersession mechanic rather than inventing a second one.
- **Existing production-path claim consumption** (`terminalize`'s
  requirement, via `production-path-contract.mjs`) remains entirely
  distinct from `admitEffect` — one governs terminalization evidence, the
  other governs live effect authority; they are not merged.
- **slice-campaign does not acquire authority merely by storing these
  references** — it holds citations to `ExecutionAttemptAdmission` and
  (transitively) `ExecutionEnvelope`, never a copy of their content, and
  never itself performs the authority-currency judgment `admitEffect` owns.

---

## 12. Failure vocabulary

```text
admitted
    — the effect is authorized and admitted exactly once for this
      operationId

attempt_not_current
    — the ExecutionAttemptAdmission receipt is superseded or retired
      (a newer attempt exists for this slice, or this attempt's lifecycle
      has ended)

authority_superseded
    — the ExecutionEnvelope's current head is not the one this specific
      check can grant against for reasons other than a specific narrowing/
      revocation already classified below (a generic EV-currency failure,
      used when a more specific outcome doesn't apply)

effect_not_authorized
    — the current envelope's admitted_organization has no entry granting
      this (role, subject, effect_class) combination — includes both
      "never granted" and "narrowed since admission"

subject_out_of_scope
    — the requested subject/resource falls outside this role's admitted
      subject_scope

role_not_authorized
    — the role itself is no longer present in the current envelope's
      admitted_organization (the "role removed/replaced" case, §8)

runtime_realization_stale
    — RoleRealization's current/applicable state check (§10) failed;
      authority may be valid but the runtime cannot currently realize
      the effect

resource_fence_invalid
    — the composed resource-lease-and-fencing check (§5 step 8) failed;
      a superseded, expired, or absent fence for a resource this effect
      touches

operation_replay
    — [Rev. 2026-09-28, EEA-002, corrected] an operationId that already
      has a recorded admission — REJECTED by admitEffect itself, exactly
      as resource-lease-and-fencing.md Key Invariant 4 and
      sqlite-store.mjs:59 already behave ("rejected outright, never
      silently treated as a retry"). This outcome does NOT mean
      admitEffect performed a safe idempotent no-op — it means the caller
      must call inspectEffectAdmission(operationId) (§5) to retrieve the
      prior recorded outcome and decide, using its OWN adapter-owned
      idempotency state, whether resuming is safe. admitEffect cannot make
      that determination on the caller's behalf.

operation_identity_conflict
    — an operationId reused with different request content; rejected,
      never silently treated as an update

authority_state_reconciliation_required
    — [Rev. 2026-09-28, EEA-002, extended on second review] the recorded
      prior outcome for a repeated operationId is unknown/ambiguous (§15)
      and the adapter has not itself confirmed the underlying effect's
      actual state — this outcome exists specifically so no path silently
      claims "safe replay" when the adapter cannot prove it; the calling
      workflow must enter reconciliation (§8's "already-committed/
      irreversible effect" row), not automatic resumption. **Also the
      outcome `admitEffect.conditionalExternal` returns (§5, §6) when the
      external CAS already durably succeeded but the SQL transaction
      recording the admission subsequently failed to commit** — in that
      specific case, this outcome is a signal to run the observe-actual-
      external-state reconciliation step (§6, reusing git-publisher.mjs's
      promote()/reconcile() pattern), which is expected to usually resolve
      cleanly (recover or safely retry), unlike the fully adapter-owned,
      possibly-unresolvable `.fenced`-path case this outcome also covers.

authority_state_unavailable
    — the EV store could not be read (distinct from a definitive
      authority_superseded/effect_not_authorized denial — this is an
      infrastructure failure, and must fail closed exactly as a denial
      would, never fail open)

malformed_authority_state
    — the current EV head or its lineage failed integrity validation
      (§3's integrity_digest check); also fails closed

external_state_conflict
    — [Rev. 2026-09-28, new, second pass] `admitEffect.conditionalExternal`'s
      (§5) own conditional write was rejected by the external system's CAS
      primitive (e.g., Git's update-ref reporting a nonzero status because
      the ref no longer matches the expected prior value) — distinct from
      `authority_superseded`: this is external-state contention (something
      else changed the target ref/resource), not organizational-authority
      narrowing, and the caller's own retry-with-fresh-expected-state logic
      governs, not admitEffect
```

No generic `denied` — every outcome above is bounded and distinguishes
semantic authority refusal from stale state, unavailable state, physical
fencing failure, malformed state, and idempotent replay, per the task's own
requirement.

---

## 13. Durable effect-admission receipt

```yaml
EffectAdmissionReceipt:
  operation_id:
  attempt_identity:
  admission_path:                # [Rev. 2026-09-28, EEA-001, twice-revised]
                                # "transactional", "conditionalExternal", or
                                # "fenced" (§5/§6) — necessary: the three
                                # paths carry different guarantees (two
                                # closed by construction, one an honest
                                # residual), and an auditor must be able to
                                # tell which one governed this operation
                                # without inferring it from other fields
  execution_envelope:
    identity:
    current_revision:          # the revision actually checked against —
                                # NOT the attempt-admission-time revision;
                                # this field is what makes the receipt
                                # itself, not just ExecutionAttemptAdmission,
                                # independently auditable against a
                                # possibly-different EV revision than the
                                # one the attempt was originally admitted under
  role_instance:                 # [Rev. 2026-09-28, EEA-003] the exact
                                # role_instance (§4) checked, not a bare
                                # "role_identity" — matches the receipt's
                                # own unambiguous binding
  organizational_subject:        # [Rev. 2026-09-28, EEA-003] renamed from
                                # "subject" to match §4's explicit field
  effect_class:
  role_realization_revision:    # recorded if step 7 (§5) applied
  resource_fence:
    resource:
    generation:                 # recorded if step 8 (§5) applied
  fenced_confirmation:           # [Rev. 2026-09-28, EEA-001/EEA-004]
                                # present only for admission_path: fenced —
                                # records whether confirmFenced (§6) was
                                # called, the FULL set of generations it
                                # re-checked (EV, attempt currentness,
                                # RoleRealization, resource fence, token
                                # expiry — corrected from EV-only), and its
                                # own outcome; absent for transactional and
                                # conditionalExternal admissions, where
                                # confirmation is inherent to the single
                                # transaction
  external_reconciliation:       # [Rev. 2026-09-28, new] present only for
                                # admission_path: conditionalExternal —
                                # populated only if step 9 returned
                                # reconciliation_required; records the
                                # observed external state and the
                                # reconciliation outcome (recovered /
                                # safe_retry / ambiguous), per §6's reuse
                                # of git-publisher.mjs's own pattern
  admitted_at:
  outcome:                      # one of §12's vocabulary
  integrity_digest:
```

**`consumed_at` and the effect's own result do NOT belong on this receipt.**
Admission evidence (was this operation authorized) and effect-outcome
evidence (what actually happened when the tool/provider adapter executed)
are kept separate, matching the existing pattern in
`slice-campaign/completion-publication.mjs`, where the accepted-checkpoint
record cites `review.md`'s finding and `evidence-and-claims.md`'s
acceptance claim rather than asserting new content of its own —
composition by reference, not merged evidence. The tool/provider adapter
that actually performs the effect owns its own result receipt, which may
cite this `EffectAdmissionReceipt`'s `operation_id` but is a structurally
separate artifact.

---

## 14. Deterministic test matrix

| Test | Expected outcome |
|---|---|
| Caller supplies a forged EV revision | ignored; host derives the real current head independently (§5) |
| Caller supplies a broader effect class than the attempt/role permits | `effect_not_authorized` — the host's own admitted_organization comparison governs, never the caller's claim |
| EV supersedes between request and commit | closed by the atomic admission-and-consume step (§6); cannot slip through |
| EV successor preserves unrelated topology only | `admitted` — no effect |
| EV successor narrows subject | `subject_out_of_scope` on the next affected check |
| EV successor removes role | `role_not_authorized` |
| Stale RoleRealization | `runtime_realization_stale` |
| Stale physical fence | `resource_fence_invalid` |
| Valid EV + invalid fence | `resource_fence_invalid` (EV validity alone is insufficient — §6/§10, both independently required) |
| Invalid EV + valid fence | `authority_superseded`/`effect_not_authorized` (fence validity alone is insufficient — same invariant) |
| Repeated identical operationId | **[Rev. 2026-09-28, EEA-002, corrected]** `operation_replay` — REJECTED by admitEffect; caller must call `inspectEffectAdmission` and use its own adapter-owned idempotency state to decide whether resuming is safe. Not an automatic no-op. |
| Repeated conflicting operationId | `operation_identity_conflict` — rejected |
| Host restart after admission before effect executes (`.transactional`) | **[Rev. 2026-09-28]** Cannot occur as a distinct case — admission and execution are one transaction (§6); either both happened (receipt + mutation result both durable) or neither did (crash rolled the transaction back entirely). |
| Host restart after admission before effect executes (`.conditionalExternal`) | **[Rev. 2026-09-28, new]** admission and the conditional external write are one transaction, same as above, for the read-then-act half of the race. If the crash lands strictly before the external CAS call, nothing external happened and the transaction rolled back — a fresh attempt is safe. If the crash lands strictly after the external CAS durably succeeded but before the SQL COMMIT, `reconciliation_required` (§12) — observe actual external state, do not blindly retry (§6, §15). |
| External CAS itself fails at `.conditionalExternal`'s own commit step (unrelated concurrent change to the same external ref) | the conditional write's own rejection (e.g., Git's `update-ref` returning nonstatus 0) — `mutate`/`conditionalMutate` throws, the SQL transaction rolls back, `admitEffect` reports a distinct external-conflict outcome; not `authority_superseded` (this is external-state contention, not organizational-authority narrowing) |
| Host restart after admission (token issued) before external commit (`.fenced`) | **[Rev. 2026-09-28, third pass] Case no longer applicable — `.fenced` is withdrawn and fail-closed (§5, §7).** No token is issued, so there is no such resume path. Retained as a row only to record that the case was considered and removed by scope reduction, not left unaddressed. |
| Host restart after effect executes before result publication | **[Rev. 2026-09-28, EEA-002]** the adapter's own durable prepared/started/committed/unknown state (§5) governs recovery for `.fenced`. If the adapter's own state resolves to `committed`, resume by publishing the known result via `inspectEffectAdmission`. If the adapter's own state is genuinely `unknown` (the external system gives no idempotency signal), the outcome is `authority_state_reconciliation_required` (§12) — not an automatic replay, per the review's explicit remediation. For `.conditionalExternal`, this same window is closed by the external system's own conditional-write primitive rather than adapter-owned state (§6). |
| Unavailable EV store | `authority_state_unavailable` — fails closed |
| Corrupt EV lineage | `malformed_authority_state` — fails closed |
| Historical attempt replay (an old, superseded attempt's admitEffect call) | `attempt_not_current` |
| Successor attempt under the same Slice | a new `ExecutionAttemptAdmission` (§4, `predecessor_attempt` chain); the old attempt's own receipts remain historically valid records, never rewritten |

---

## 15. Crash/recovery semantics

- **EV revision publication crash:** covered entirely by
  `revision-cas-and-publication.md`'s own atomic-visibility invariant (§3
  above) — no partial-write state is observable, by construction of the
  reused mechanism.
- **`ExecutionAttemptAdmission` receipt crash:** the receipt is itself an
  immutable, content-addressed record (§4); a crash before it is fully
  written means it does not yet exist, and admission has not occurred — the
  attempt is not yet valid, and any `admitEffect` call against it fails
  `attempt_not_current`.
- **[Rev. 2026-09-28, EEA-001/EEA-002] `admitEffect.transactional`'s
  admission-and-execution crash (§5, §6):** covered entirely by the
  underlying transaction's own atomicity, genuinely, now that this path
  is narrowed to effects with no external-store component — a crash
  mid-transaction rolls back the whole thing (admission record and
  mutation both), exactly as `sqlite-store.mjs`'s `#transaction` already
  guarantees. Either the `EffectAdmissionReceipt` and the mutation's own
  result both exist, or neither does. A retry with the same `operationId`
  after a rolled-back crash is a genuinely fresh admission attempt, not a
  "replay" in the §12 sense, because nothing was ever durably admitted.
- **[Rev. 2026-09-28, new, second pass]
  `admitEffect.conditionalExternal`'s crash — the case the review
  correctly identified as unsolved in the first-pass revision, now
  addressed by reusing existing, working code rather than inventing a new
  mechanism.** Two sub-cases:
  - Crash strictly before the external CAS call (still inside the SQL
    transaction): the transaction rolls back, nothing external happened,
    identical to Path 1's guarantee.
  - Crash strictly after the external CAS durably succeeded but before
    the SQL `COMMIT`: the SQL side has no record, but the external system
    does. **[Rev. 2026-09-28, third review pass — prior description
    corrected.]** The previous revision said the admission record "is
    recovered". That misstated the existing code. Verified actual
    behaviour in `git-publisher.mjs`:

    ```js
    // promote(), line ~166
    const recoveredAdmission = coordination.inspectAdmission(operationId);
    return recoveredAdmission
      ? publicationReceipt(sealed, recoveredAdmission, "published_observed")
      : unconfirmedPublication(sealed, error.message);
    ```

    The mechanism does **not** recreate a missing admission. If the ref
    moved but no admission record exists, it returns
    **`published_unconfirmed`** — an explicit unresolved outcome handed
    onward, not a recovery. `reconcile()` (line ~176) behaves the same
    way, returning `unconfirmedPublication(sealed, "mutation admission is
    unavailable")` on that branch.

    Correct description of the two branches: if an admission record
    exists, the outcome is `published_observed`; if it does not, the
    outcome is `published_unconfirmed` and resolution belongs to the
    owning workflow. Neither branch re-executes the effect.

    This is therefore NOT a solved crash window. It is an observable,
    explicitly-flagged one — which is better than silent duplication, but
    is not the "genuine fix, not a residual" the prior revision claimed.
    That claim is withdrawn.

  - **ABA hazard on the `eligible` branch (EEA-002, open).** Verified:
    `reconcile()` returns `status: "eligible"` when the observed ref
    equals `expectedParent`, *without* consulting
    `inspectAdmission(operationId)` — although the sibling branch does
    consult it. If an admission record was lost (crash before `COMMIT`)
    and the ref was subsequently reset back to the parent, the
    observation is indistinguishable from "nothing happened", and a
    retry can double-apply an irreversible effect. Narrow in practice
    (it requires both admission loss and an external revert) but real,
    and not closed by this document. Note that `withAdmission` does
    reject reuse of an already-recorded `operationId`
    (`"mutation operation id was already admitted"`,
    `workspace-coordination/sqlite-store.mjs`), so the hazard exists
    specifically in the window where the admission record is absent.
- **`.fenced` crash cases — [Rev. 2026-09-28, third pass] no longer
  applicable.** Both the token-issuance-crash case and the
  crash-after-external-commit case described in the prior revision
  presupposed a `.fenced` path that is now withdrawn and fail-closed
  (§5, §7). They are removed rather than restated, because the effect
  classes they covered are not admittable under this design at all.
  The underlying hazard they described has not been solved — it has been
  placed outside the set of effects this design permits, pending the
  host-enforced commit mediator named in §5 and §16 (Prerequisite B).

None of the transactional-path or conditionalExternal-path recovery
requires a new theory — both are the same discipline
`revision-cas-and-publication.md`, `resource-lease-and-fencing.md`, and
(for the external-CAS half) `git-publisher.mjs`'s own working code already
provide, applied at this composition point. The fenced-path external-commit
gap is a genuine, narrower, still-named residual (§18).

---

## 16. Implementation decomposition into bounded slices

Named as candidate decomposition only — no implementation authorized by
this document (§ Status and authority).

**[Rev. 2026-09-28, third review pass] Two prerequisites now precede
everything below, and both are outside this document:**

- **Prerequisite A — a demonstrated common serialization owner (EEA-001).**
  Every authority generation an admission depends on, plus the mutation
  admission record, must be serialized by one named owner (§5). Verified
  today: `slice-campaign.sqlite3` and `coordination.sqlite` are separate
  databases, and EV has no store, so this holds for no effect class. Until
  it does, slice 4 cannot deliver its stated guarantee regardless of how
  well it is implemented. This is a constraint on **where EV's store may
  live**, not merely a sequencing note.
- **Prerequisite B — operation-bound external evidence, or a host-enforced
  commit mediator (EEA-002 / AIR-001).** For `.conditionalExternal`: an
  operation-bound receipt (e.g. an `operationId` trailer in the commit
  object) or a no-ABA generation, plus a binding between the external
  conditional check and the authority generations. For the effect classes
  formerly routed to `.fenced`: a mediator that itself performs the
  external commit. Neither is designed here.

0. **[Rev. 2026-09-28, AIR-002] BLOCKING PREREQUISITE, owned entirely
   outside this document: a closed, mechanically enforceable
   effect_boundary/subject_scope comparison grammar.**
   `role-and-contract-structure.md` §5 states, in its own words, that the
   mapping between its primitive vocabulary
   (`vantage + authority + obligation + continuity + information access +
   effect boundary + capability requirements + independence constraints +
   lifecycle`) and its AEG relation vocabulary
   (`bound_by / may_invoke / may_observe / may_mutate / owns / consumes /
   emits / mediated_transitions / forbidden_from`) is "Not Yet Mapped." §5
   step 6 of this document's own `admitEffect` (the mechanical
   `admitted_organization` comparison) cannot be built as claimed —
   "mechanically decidable" — until that mapping exists and is closed.
   This document does **not** invent that grammar; it names the gap and
   blocks slice 4 below on `role-and-contract-structure.md` resolving it.
1. **`ExecutionEnvelope` schema + store**, using `revision-cas-and-
   publication`'s existing generic revision machinery (likely the same
   underlying store shape as `claim-evidence/service.mjs`'s own
   `revisionHeads`/`publish_revision` pattern). Prerequisite for everything
   below.
2. **Organizational Compilation's own admission mechanism** actually
   publishing an `ExecutionEnvelope` revision (currently `design: accepted,
   implementation: none` — this slice is Organizational Compilation's own
   implementation, not this document's, but is a hard prerequisite).
3. **`ExecutionAttemptAdmission` schema + store**, referencing (2)'s output
   plus ICC, accepted-routing, and RoleRealization identities — each of
   those upstream artifacts' own implementation status gates this slice
   independently.
4. **`admitEffect.transactional` and `admitEffect.conditionalExternal` host
   functions** (§5, §6 — [Rev. 2026-09-28, third pass] two code paths;
   `.fenced` withdrawn), composing (1) and (3), plus calls into existing
   Runtime Realization validity checks,
   `resource-lease-and-fencing.md`'s `admitMutation`, and — for
   `.conditionalExternal` — `git-publisher.mjs`'s existing
   `promote()`/`reconcile()` pattern, reused rather than reimplemented.
   **Blocked on slice 0, Prerequisite A, and Prerequisite B.** Without A,
   neither path can serialize authority against its mutation; without B,
   `.conditionalExternal` cannot distinguish ABA from "nothing happened".
4a. **[Rev. 2026-09-28, EEA-002] `inspectEffectAdmission` read path and
   adapter-owned idempotency contract** — a companion slice to 4: the
   adapter-facing `prepared/started/committed/unknown` state machine (§5).
   Should also close the verified gap in which `reconcile()`'s `eligible`
   branch does not consult `inspectAdmission(operationId)` (§15).
5. **`EffectAdmissionReceipt` durable store** (now including
   `admission_path` and, for fenced admissions, `fenced_confirmation`,
   §13).
6. **slice-campaign integration** (§11) — the minimal reference fields on
   Slice and Attempt state, plus wiring `admitEffect` calls into whichever
   adapters currently perform authority-bearing effects (§7) without an
   admission check today. **[Rev. 2026-09-28, EEA-004]** must also expose
   (or reuse) a query surface over `slice_campaign_admission`'s live
   currentness state — `admitEffect` step 2 (§5) depends on this existing
   as a queryable fact, not merely as an internal implementation detail of
   `supersede`.
7. **Test matrix implementation** (§14).

Slices 1–3 are pure prerequisite plumbing already implied by each upstream
dimension's own accepted design; slice 0 is a prerequisite this document
cannot itself satisfy; slices 4/4a are this document's own novel surface
area; slices 5–7 are direct consequences of 4/4a.

---

## 17. Exact canonical documents requiring later amendment

Named only — none edited by this document:

- **`organizational-compilation.md`** §7–8 — name the live-check
  consequence explicitly (this document's §5/§6), and, per Round 5's
  finding, reconcile §8's own simplified diagram with
  `role-and-contract-structure.md`'s two-stage role-contract distinction
  (a pre-existing gap this document does not itself close).
- **`role-and-contract-structure.md`** — state Role Contract's
  projection-only enforcement role explicitly (§9 above), and specify
  that Part 2.2's derived content should cite exact EV provenance
  (`execution_envelope.identity` + `admitted_revision`).
  **[Rev. 2026-09-28, AIR-002] Blocking, not optional:** this page must
  also close its own §5 "Not Yet Mapped" gap between its primitive
  vocabulary and its AEG relation vocabulary before `admitEffect` step 6
  (§5, §16 slice 0) can be built as a mechanical comparison. This document
  does not attempt that mapping itself.
- **`mechanisms/resource-lease-and-fencing.md`** — cross-reference this
  document's composed, non-identical use of `admitMutation`'s discipline
  for authority-currency checking, per that page's own Key Invariant 3
  ("same fencing-token machinery does not mean same architectural
  invariant"). **[Rev. 2026-09-28, new, second pass]** This page (or a new
  sibling mechanism page) is also the more natural canonical home for
  `git-publisher.mjs`'s compose-external-CAS-inside-admitMutation +
  observe-and-reconcile-on-failure pattern than this document — that
  pattern is currently implicit in `git-publisher.mjs`'s own code, with no
  canonical architecture page describing it as a reusable composition,
  even though this document is now the second, independent consumer of
  its shape. Naming it as its own documented mechanism would let future
  `.conditionalExternal`-shaped designs reuse it without re-deriving it
  from source the way this document had to.
- **`mechanisms/revision-cas-and-publication.md`** — add `ExecutionEnvelope`
  revisions as a confirmed instance once (1) in §16 is built (currently
  listed there as accepted-design-only).
- **`authority-and-ownership.md`** — optionally cite this document's §8
  revocation table as another concrete instance of §12's generalized
  "invalidation never mints authority" invariant, alongside the existing
  four.
- **`implementation-contract-compilation.md`** — no change required by this
  document directly, but Round 5's still-open finding (a re-trigger for
  `blocked_by_evidence` against a sealed contract when a relied-upon claim
  refreshes) remains a separate, unresolved prerequisite this document
  does not itself close.
- **A canonical `slice-campaign` architecture document** — still does not
  exist (confirmed across every round of this chain); this document's §11
  names the minimum integration surface such a document would eventually
  need to describe, but does not substitute for writing it.
  **[Rev. 2026-09-28, EEA-004]** should also document
  `slice_campaign_admission` as the authoritative live-currentness record
  it already functions as in code — this document's §5 step 2 now depends
  on that fact being canonically named, not merely discoverable by reading
  `sqlite-store.mjs` directly.
- **`organizational-compilation.md` and/or `authority-and-ownership.md`**
  — **[Rev. 2026-09-28, EEA-005, new]** neither page currently states
  whether contraction of a parent authority grant or applicable policy
  obligates supersession of `ExecutionEnvelope` revisions delegated under
  it. This document names two candidate designs (§2, §5) and recommends
  admission-time dependency revalidation as the safer default absent
  evidence, but does not resolve which (if either) is correct — that
  determination belongs to these pages, not to this document.

---

## 18. Status/authority block and unverified assumptions

```yaml
architecture_status:
  design: proposed
  reconciliation: not_yet_reconciled
  authorization: design_work_only
  implementation: none
  owner: app-server/ideas/pending/execution-envelope-live-effect-admission.md
  status_as_of: 2026-09-28
```

This document:

- does **not** accept a design;
- does **not** amend any canonical architecture page;
- does **not** change the roadmap;
- does **not** authorize implementation of `ExecutionEnvelope`,
  `admitEffect`, `ExecutionAttemptAdmission`, or any slice-campaign schema
  change;
- does **not** establish that a new architectural dimension exists — every
  classification in this reconciliation chain concluded this is enforcement
  mechanics composed from existing mechanisms (`HOST_ADMISSION_EXTENSION`
  composed from `EXISTING_AUTHORITY_PROJECTION_COMPOSITION`), never a new
  Dimension, Substrate, or generic Mechanism;
- is offered for **adversarial review before any implementation planning**,
  per the task's own explicit instruction.

**Every assumption that remains unverified, stated explicitly:**

1. Whether `delegation_mode` (non-transferable/delegable/nomination-only/
   advisory) needs to be a literal `ExecutionEnvelope` field or remains a
   classification applied at projection time — left `UNRESOLVED` in §2,
   not decided here.
2. Whether Role Contract's Part 2.2 derivation, once built, will actually
   record exact EV provenance as proposed in §9 — this document proposes
   the addition but has not confirmed any commitment to build it that way.
3. **[Rev. 2026-09-28, narrowed twice — by EEA-001 then by the second
   review pass]** Whether `admitEffect.transactional`'s single-transaction
   shape (§6) is achievable in practice wherever `ExecutionEnvelope` and
   the affected store ultimately live relative to each other — narrowed
   further this pass: `.transactional` now excludes anything touching
   Git/filesystem/an external system by definition (moved to
   `.conditionalExternal` or `.fenced`, §7), so this residual applies only
   to genuinely host-local effects, which have not been exhaustively
   enumerated against `app-server/src`.
   **[Rev. 2026-09-28, new]** Whether `.conditionalExternal`'s reuse of
   `git-publisher.mjs`'s pattern generalizes beyond Git ref updates — this
   document confirmed exactly one conditional-write primitive exists in
   this codebase (Git's three-argument `update-ref`); whether any other
   external system this design will eventually cover (§7's `.fenced` list)
   also offers one has not been investigated, and this document does not
   assume one exists merely because Git happens to have one.
   **[Rev. 2026-09-28, third pass]** Note further that Git's primitive,
   even where it exists, conditions on a ref value and not on any
   authority generation — so its presence is necessary but not sufficient
   (§5, EEA-001).
4. Whether the effect-class list in §7 is complete — it is derived from
   this reconciliation chain's own findings about what needs authority
   checking, narrowed once (AIR-003) but not from an exhaustive audit of
   every effect-producing code path in `app-server/src`.
5. **[Rev. 2026-09-28, narrowed by EEA-002]** Whether every adapter this
   design will eventually cover can actually supply the
   prepared/started/committed/unknown idempotency contract §5 now requires
   of it — for external systems that genuinely cannot, `admitEffect`'s own
   honest answer is `authority_state_reconciliation_required` (§12), not a
   guarantee; whether that is operationally acceptable for every such
   adapter is not evaluated here.
6. Whether branching should genuinely never be permitted for
   `ExecutionEnvelope` (§3) under every future use case, or only for the
   fixed-topology first vertical `organizational-execution-envelopes-
   reconciliation.md` itself scopes to — this document assumes the
   narrower, currently-accepted scope and has not tested the branching
   question against any broader future use.
7. Whether `ExecutionAttemptAdmission`'s `admitted_by` field needs its own
   structured authority-reference shape or is sufficient as an opaque
   audit string — not resolved here, deferred as an implementation-time
   decision within slice 3 of §16.
8. **[Rev. 2026-09-28, third review pass — RESOLVED BY SCOPE REMOVAL, not
   by mechanism.]** The prior revision retained `.fenced` with
   `confirmFenced` as a named residual trust boundary. The third review
   established that this was not a defensible disposition: a guarantee
   contingent on the adapter choosing to invoke its own check is not a
   guarantee, and even a compliant adapter retains a
   confirmation-to-commit TOCTOU window. `.fenced` is therefore withdrawn
   and its effect classes are fail-closed (§5, §7). What remains open is
   not a residual inside this design but a **missing piece of
   infrastructure outside it**: a host-enforced commit mediator that
   itself performs the external commit. Named, not designed here.

8a. **[Rev. 2026-09-28, third review pass, OPEN — EEA-001.]** No effect
   class currently has a demonstrated common serialization owner across
   its authority generations and its mutation admission. Verified:
   `slice-campaign.sqlite3` and `coordination.sqlite` are distinct
   databases, and `ExecutionEnvelope` has no store at all. Until EV state
   exists and is co-located appropriately, neither remaining admission
   path can deliver its stated guarantee. This is a precondition on the
   design, not a caveat within it.

8b. **[Rev. 2026-09-28, third review pass, OPEN — EEA-002 reopened.]**
   `.conditionalExternal` cannot currently distinguish "the operation
   never committed" from "it committed and was reverted" (ABA). Verified:
   the commit object carries no operation-bound field
   (`git-publisher.mjs` builds it from subject/body only), and
   `reconcile()`'s `eligible` branch does not consult
   `inspectAdmission(operationId)`. Closing this requires operation-bound
   external evidence or a no-ABA generation. Not designed here.
9. **[Rev. 2026-09-28, EEA-005, STRENGTHENED — this document explicitly
   does not resolve this and states admission into implementation
   planning as blocked on it, §2]** Whether admission-time dependency
   revalidation and an atomic upstream-contraction obligation (§2) are
   genuinely the only two candidate designs, or whether
   `organizational-compilation.md`'s own admission mechanism, once built
   (§16 slice 2), will reveal a third shape this document could not
   anticipate without that implementation existing. Which of these (if
   either) is correct, and which page owns that decision, is left entirely
   to `organizational-compilation.md` and/or `authority-and-ownership.md`
   — this document takes no position beyond naming the question.
10. **[Rev. 2026-09-28, new, second pass]** Whether `git-publisher.mjs`'s
    `promote()`/`reconcile()` reconciliation pattern, reused here for
    `.conditionalExternal` (§5, §6, §15), has itself been exercised
    against every crash timing this document now relies on it for — this
    document confirmed the pattern exists and reads correctly for the
    cases described, but did not run or observe it under an actual
    induced crash; that verification is implementation-time work (§16),
    not something a design-only document can claim.

No file under `app-server/docs/architecture/` was read-and-then-modified by
this document's authoring process; only new content was written to this
single new file.
