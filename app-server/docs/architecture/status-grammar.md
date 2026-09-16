# Status Grammar

> **Question:** How does an architecture view state a claim's maturity without either hiding it (omission) or overstating it (one binary badge)?

## 1. Purpose

The architecture views under `app-server/docs/architecture/` deliberately include material at every maturity level — accepted design, reconciled-but-unaccepted candidates, and open exploratory hypotheses — because excluding anything provisional makes the atlas *less* useful, not more careful.

That only works if maturity is never ambiguous. The rule this grammar exists to enforce:

> **Nothing may appear in an architecture view without its status being explicit.**

This document defines the vocabulary once. Every other page in `app-server/docs/architecture/` references it rather than re-explaining it — the same discipline that keeps a relation vocabulary from getting reinvented per file.

---

## 2. Unit of Status: the Smallest Independently Decidable Architectural Claim

Status does not belong to a *page*. It belongs to a *claim*.

A page mixes maturity levels routinely — `organizational-compilation.md` contains an accepted admission mechanism, a reconciled-but-unaccepted compiler shape, and a genuinely open cross-cutting-realization boundary, all in one file. Assigning one status to the whole page would either overstate the weakest claim or understate the strongest one.

The test for "one claim": could an owning authority accept, reconcile, authorize, or implement this *without necessarily doing the same for the surrounding material*? If yes, it needs its own status when that status diverges from the page default (§3).

---

## 3. Page Defaults and Local Overrides

Restating full status on every paragraph is its own failure mode — it drowns the content and invites drift between copies. Instead:

**Every architecture page declares one default** in its `Source and Status` section:

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: partial
  owner: <path to the authoritative owning document>
  status_as_of: 2026-09-15
```

**Only sections whose status diverges from that default carry their own block**, overriding exactly the fields that differ:

```yaml
status_override:
  design: exploratory
  reconciliation: not_applicable
```

Any field not listed in a `status_override` inherits the page default. A section with no `status_override` at all is fully described by the page default.

This is what makes the grammar usable: most of a page shares one maturity level, and the override mechanism exists precisely for the parts that don't.

---

## 4. The Four Axes

Four axes, each independent. A claim's position on one axis constrains none of the others directly — constraint only flows through the inheritance rule in §7.

### 4.1 Design status

> Do we believe this is part of the intended architecture?

```text
accepted
    Explicitly adopted by its owning authority.

proposed
    A concrete architectural candidate with enough definition and
    reconciliation to evaluate, but not explicitly accepted.

exploratory
    A hypothesis, an unresolved direction, or architecture whose shape
    itself remains open.

superseded
    Previously meaningful architecture that is no longer current.
```

**Reconciliation maturity does not by itself produce `accepted`, and thin reconciliation does not by itself demote something to `exploratory`.** A claim that has been extensively checked against every neighboring document but never put to an explicit acceptance decision is `proposed`, not `exploratory` — its shape is settled; only the decision is missing. A claim whose *shape itself* is still genuinely open is `exploratory` regardless of how much reconciliation surrounds it. Confusing "not yet decided" with "not yet understood" is exactly the distinction this axis exists to preserve.

### 4.2 Reconciliation status

> Has this been checked against the architecture it touches?

```text
reconciled
    All presently known material seams within the claim's declared scope
    have been checked and their dispositions recorded.

partial
    Some material seams have been checked; named reconciliation work
    remains.

unreconciled
    Material neighboring architecture is known to exist but has not been
    checked against this claim.

not_applicable
    No meaningful reconciliation dependency exists for this claim (it
    doesn't yet touch other owned architecture in a way reconciliation
    could resolve).
```

**`reconciled` is scope-relative, not universal.** It means reconciled against the relationship set that was known and declared at the status's own `status_as_of` date — not "provably compatible with every architecture Work Engine will ever contain." A new document appearing later does not retroactively falsify an existing `reconciled` marker; it creates new reconciliation work, tracked as its own claim.

### 4.3 Authorization

> What work are we presently allowed to do from this design?

```text
exploration_only
    No standing authorization beyond continued exploration and
    reconciliation. A deliberate ceiling: this is the confirmed bound,
    not an absence of information about the bound.

design_work_authorized
    Further design work, profile formation, or proposal refinement is
    explicitly authorized. Implementation is not.

implementation_authorized
    Building the thing itself is explicitly authorized.

unrecorded
    No explicit authorization decision has been found for this claim.
    Implementation may have proceeded through informal, piecemeal, or
    untracked approval. This is a traceability gap, not an assertion
    that authorization was absent.
```

Deliberately named `design_work_authorized` rather than `design_authorized` — the latter reads as "the design has been accepted," which is already what `design_status: accepted` means. This axis answers a permission question, not a belief question. `design_status: accepted` and `authorization: design_work_authorized` can and do coexist on the same real artifact (§10.2) — being accepted as architecture does not automatically authorize building it.

**`unrecorded` is deliberately distinct from `exploration_only`, added 2026-09-16 after `context-lifecycle.md` needed it and the difference between the two was checked, not assumed.** `exploration_only` is a confirmed, deliberate ceiling — checked, and nothing above it is authorized. `unrecorded` is silence — no authorization decision was found, which is not the same fact as confirming none exists. **Implementation having occurred is never sufficient by itself to justify `implementation_authorized`.** It proves implementation happened; it does not prove the architecture as currently scoped received an authorization decision at that level — code can predate an architecture's own current shape, be authorized piecemeal across many smaller decisions, or simply exist without any single recorded architecture-level authorization. Treating "implemented" as proof of "implementation was authorized" would make authorization an inference from implementation evidence, exactly what this axis exists to prevent (§8). Use `unrecorded`, not a guessed value, whenever no specific authorization source can be named.

### 4.4 Implementation status

> How much of the described architecture is realized in the implementation?

**Corrected 2026-09-16** — the original question ("how much of the *authorized* design actually exists?") quietly presumed authorization was established before implementation could be measured. Adding `unrecorded` to §4.3 exposed the dependency: `authorization: unrecorded` + `implementation: partial` is not a contradiction (`context-lifecycle.md`'s own real status), so this axis cannot be phrased as depending on the other. The two axes answer genuinely independent questions — what work was permitted, and what actually exists — and neither may be inferred from the other.

```text
none
planned
partial
implemented
```

`implemented` describes the claim it's attached to, not the whole page or dimension. A dimension page with several claims may show some `implemented`, some `planned`, and some `none` at once — that is the expected shape, not an inconsistency to resolve.

---

## 5. Owner and Source Provenance

`owner` (page default) or `status_source` (local override) is not a status value — it is the field that makes every status value checkable. It names the specific document that is the authoritative record for this status: the reconciliation document that accepted it, the idea document proposing it, or the design document that authorized it.

A status without a named owner cannot be `accepted`, `implementation_authorized`, or anything above `exploratory` — those stronger claims exist because *something* decided them, and that something must be nameable.

---

## 6. `status_as_of`

Every page default (and any override that changes a status) carries the date that status was last true. This is not decoration — §4.2 depends on it (`reconciled` is relative to what was known on that date), and it gives a future reader a mechanical way to notice staleness: if a cited owning document has moved since `status_as_of`, the status needs re-checking, not blind trust.

---

## 7. No-Upward-Inheritance: Synthesis Does Not Promote Status

> **A derived architectural claim inherits no status stronger, on any axis, than its weakest required premise — unless the authority owning the derived claim explicitly assigns a stronger status through its own decision.**

Stated per axis, because each behaves independently:

```text
accepted premise + proposed premise
    → a claim derived from both is at most proposed, absent a new decision

reconciled inputs
    → does not by itself make the derived claim reconciled;
      the derived claim is its own reconciliation surface

implementation-authorized input
    → does not grant implementation authorization to what consumes it

an implemented mechanism
    → does not make a newly proposed use of that mechanism
      "implemented architecture" — the mechanism being built does not
      build the new thing that reuses it
```

That last case matters more than the others in practice: Work Engine reuses existing mechanisms constantly (revision-binding, CAS, the observe/nominate/decide/admit/execute vocabulary), and a *use* of a proven mechanism for a new architectural purpose is a new claim with its own status, not an automatic inheritance of the mechanism's own maturity.

This is the property that makes it safe to include provisional material in an architecture view at all: assembling several claims into one page or one diagram can never quietly launder the weakest one into looking as settled as the strongest.

---

## 8. Status Is Declarative, Not Inferred

No architecture view, and no synthesis document built from these views (including the planned-architecture capstone), may *calculate* that something has become accepted because the surrounding evidence seems to point that way.

> Evidence supporting acceptance is not acceptance. Explicit owner decisions establish `accepted` design status and authorization changes. Reconciliation work may recommend or justify such a change; it cannot perform it.

This is the discipline already in force throughout this session's own reconciliation work — Sol's own recommendation to accept a candidate was recorded as a recommendation, and the candidate stayed `proposed` until the user's own explicit decision. This section makes that discipline a stated grammar rule rather than an unwritten convention a future session could plausibly not know about.

---

## 9. Compact Display Shorthand

The full four-axis block is source metadata. A human-facing architecture view may render it compactly:

```text
[A]  design: accepted
[P]  design: proposed
[E]  design: exploratory
[S]  design: superseded

R✓   reconciliation: reconciled
R~   reconciliation: partial
R✗   reconciliation: unreconciled
R–   reconciliation: not_applicable

I✓   implementation: implemented
I~   implementation: partial
I○   implementation: planned
I✗   implementation: none
```

**Authorization is deliberately not glyph-compressed.** It is consequential enough that a terse marker risks being skimmed past. Render it as text:

```text
Build: not authorized
Build: design work only
Build: authorized
```

Worked display line, combining all four:

```text
Organizational-authority admission mechanism
[A] R✓  Build: design work only  I✗
owner: organizational-execution-envelopes-reconciliation.md · as of 2026-09-15
```

---

## 10. Worked Examples From Real Work Engine Artifacts

Chosen because every value below traces to something this session actually verified, not invented for illustration.

### 10.1 Organizational-authority admission mechanism

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: none
  owner: app-server/docs/organizational-execution-envelopes-reconciliation.md
  status_as_of: 2026-09-15
```

Traces to that document's own "Candidate answer for the organizational-authority layer" section: "Accepted 2026-09-15 (explicit user decision...). This acceptance is for the design, not for implementation." `design: accepted` and `authorization: design_work_authorized` coexisting on one real artifact is exactly the combination §4.3 exists to represent.

### 10.2 The impact-nomination / refresh-episode operation contract

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: none
  owner: proposals/evidence-lineage/claim-maintenance-and-reliance-propagation/operation-contract-surface.md
  status_as_of: 2026-09-15
```

Traces to that document's own Status section, sixth draft: "Architecture: PASS... Implementation authorization for a real end-to-end vertical: not quite yet — profile formation is now the remaining prerequisite." Passed design, explicitly *not* implementation-authorized — the second real instance of the §4.3 combination, from a completely different part of the architecture, which is why the axis earns its place rather than duplicating `design_status`.

### 10.3 Recursive organizational compilation (`auto-org`, core shape)

```yaml
architecture_status:
  design: proposed
  reconciliation: partial
  authorization: exploration_only
  implementation: none
  owner: app-server/ideas/pending/deterministic-authority-projection-and-adaptive-organizational-topology.md
  status_as_of: 2026-09-15
```

`reconciliation: partial`, not `reconciled` — that idea document's own Open Question 22 states directly: "Partially answered, not fully reconciled... whether this reframing survives a full, formal reconciliation pass against [`hierarchical-planning-and-multi-supervisor-orchestration.md`'s] complete text... [is] not checked." `design: proposed` rather than `exploratory`, per §4.1: the shape has survived two rounds of direct correction against that document's own §3/§7/§12 and is stable, even though no explicit acceptance decision has been made.

### 10.4 Cross-cutting organizational realizations (local override within the same page as 10.3)

```yaml
status_override:
  design: exploratory
  reconciliation: not_applicable
```

Inherits `authorization: exploration_only` and `implementation: none` from the page default (10.3), since neither of those has changed. Traces to the same idea document's Open Question 26: whether organizational-realization authority may ever lawfully cut across an obligation's own internal boundaries "is not yet placed," and "does not decide whether cross-cutting realizations should ever be permitted at all." This is genuinely open in shape, not merely unaccepted — the §4.1 test for `exploratory` rather than `proposed`. `reconciliation: not_applicable` because there is no declared neighboring document this specific question has been checked against; the question hasn't crystallized into a concrete proposal yet for reconciliation to apply to.

### 10.5 Context Lifecycle (page default, demonstrating `authorization: unrecorded`)

```yaml
architecture_status:
  design: proposed
  reconciliation: reconciled
  authorization: unrecorded
  implementation: partial
  owner: app-server/docs/semantic-context-lifecycle-manager.md
```

The occasion for adding `unrecorded` to this grammar (§4.3): substantial real, tested, merged implementation exists for this architecture (schema-migrated SQLite adapters, a gated live strategic-planner test), and an earlier draft of this page read that as `authorization: implementation_authorized` — implementation having happened, therefore implementation must have been authorized. Corrected: no specific, citable authorization decision names this architecture at its current scope, so the honest value is `unrecorded`, not a value inferred from the implementation evidence itself. `implementation: partial` and `design: proposed` remain fully supported directly from the source document's own text; only `authorization` needed the new value.

---

## Source and Status

This grammar is itself a design artifact of the architecture-views decomposition effort, settled by direct discussion rather than derived from any single prior document.

```yaml
architecture_status:
  design: accepted
  reconciliation: not_applicable
  authorization: design_work_authorized
  implementation: none
  owner: app-server/docs/architecture/status-grammar.md
  status_as_of: 2026-09-15
```

`design: accepted` because this grammar was explicitly settled by the user across several rounds of direct discussion, each refinement confirmed before the next was proposed — the same explicit-decision bar every other `accepted` value in this document is held to. `reconciliation: not_applicable` because this is a new convention, not a claim reconciled against pre-existing architecture. `authorization: design_work_authorized`, not `implementation_authorized` — nothing here is a runtime type or a service; it is a convention for how the architecture views themselves are written. Applying it to the two missing dimension pages (Evidence/Claims, Runtime Realization) is its first real test.
