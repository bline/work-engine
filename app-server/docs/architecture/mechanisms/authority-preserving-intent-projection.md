# Authority-Preserving Intent Projection

> **Question:** How does an already-authorized human or operator intent become a bounded, admissible candidate action — discovered, rendered, submitted, and tracked through to its authoritative disposition — without the projection surface ever deciding domain meaning or manufacturing authority it wasn't given?

## Purpose

This is a **mechanism view**, not a dimension view. It describes a reusable intent-to-candidate translation discipline, confirmed by independent convergence across two unrelated consumers, not a class of architectural truth this page owns itself.

> **The projection may constrain and encode authority; it may never enlarge it. It never decides domain meaning, and it never mutates directly merely because intent exists.**

**Named by architectural synthesis** (2026-09-16), inverting the usual test: this concept was assumed architectural until disproven, then pressure-tested for category rather than existence. Four candidate homes were checked and ruled out one by one: `authority-and-ownership.md` owns the authority *model*, never the *mechanism* that turns intent into a bounded candidate; `role-and-contract-structure.md` explicitly disclaims this territory in its own text ("does not extend to how new contracts get authored, derived, or admitted"); `runtime-realization.md` §11's operator-policy-overlay is a real, independently-arrived-at *instance* of this exact invariant, not its home. What remained was a missing mechanism, confirmed — not assumed — by a genuine multi-consumer test: `work-engine-studio-reconciliation.md`'s own text unifies two originally-separate Studio needs (Control view, Contract/design authoring) into "not two parallel UI-mutation systems... but one authority-preserving interactive command/edit projection," and `runtime-realization.md` §11's operator-policy-overlay was written with **zero cross-reference to Studio or its reconciliation**, yet follows the identical shape. Independent convergence, the same bar `mechanisms/resource-lease-and-fencing.md` and `mechanisms/candidate-resolution-and-admission.md` were each confirmed against.

---

## The Reusable Shape

Verified directly against `work-engine-studio-reconciliation.md`'s own diagram and disposition text:

```text
canonical owner / owning domain
    owns: what operations currently exist, subject + revision binding,
          authority requirement, admission/refusal, the resulting
          authoritative transition

THIS MECHANISM
    owns: discovering and rendering available operations, collecting
          bounded operator/human intent, submitting it without
          manufacturing authority, and showing proposed / pending /
          admitted / refused / completed / stale lifecycle feedback
          with attributable reason
```

```text
owned state
        ↓
discoverable projections + admitted operations
        ↓
display / edit / request
        ↓
identity + revision + authority-bound intent
        ↓
owning transition (domain-owned admission)
        ↓
authoritative result
        ↓
lifecycle feedback
    (proposed -> pending -> admitted -> refused -> completed -> stale,
     with attributable reason)
```

The generic invariant this mechanism exists to enforce, independent of which domain's intent is being projected:

> **The projection may constrain and encode authority; it may never enlarge it. The projection never decides domain meaning, and it never mutates directly merely because intent exists.**

---

## What This Mechanism Does Not Decide

- **it does not decide what operations exist or their authorization** — that remains the canonical control plane's, `OperatorProjection`'s, or whichever domain owns each operation's own content;
- **it does not decide domain meaning** — the owning transition and its authoritative result remain entirely the owning dimension's own truth, never this mechanism's;
- **it does not resolve a candidate set to a selection** — that is `mechanisms/candidate-resolution-and-admission.md`'s own territory (§ Relationship, below); this mechanism only produces the bounded candidate that resolution mechanism might later act on;
- **it does not mint authority merely because an intent was collected, rendered, or displayed** — the source's own words, quoted directly because they are the entire reason this mechanism exists: "The UI does not decide what is authorized. It projects the existing authority model";
- **it does not define the domain-specific meaning of `refused`, `stale`, or any other lifecycle state for a specific domain** — the shared vocabulary is this mechanism's own; what each state means for a given operation belongs to the owning domain.

---

## Confirmed Instances

### Studio's command/edit projection — design work authorized, not implemented

`work-engine-studio-reconciliation.md`'s own finding: "the genuinely Studio-specific missing piece" — unifying the Control view (runtime command projection) and the Contract/design authoring path (structural edits, explicit in the idea's own uncompressed history as bidirectional: "edits should write back to the structured owners or produce reviewed change proposals"). Accepted 2026-09-14; the connective-layer schema itself (discovery/rendering/submission/lifecycle-feedback) is not yet specified, and implementation is explicitly deferred pending that.

### Runtime Realization's operator policy overlay — real, partial instance

`runtime-realization.md` §11, independently arrived at with no cross-reference to Studio: "A UI may project candidate states (`selected`, `preferred`, `admissible`, `requires_operator_approval`, `prohibited`, and similar) as a view over owned contracts, policy, and observations — it must not itself become the resolver, and it must not directly mutate the active adapter, provider thread, runtime session, or realization." This is the identical shape (expose bounded candidates, never resolve, never mutate directly) applied to one domain — evidence *for* this shared mechanism, not a competing definition of it.

### `OperatorProjection`'s control-packet model — a real precursor, not the mechanism itself

**Origin named 2026-09-16, not previously cited by this page**: `OperatorProjection` is defined in `app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md` §4 — one of that document's three named ports (`ProviderTurnPort`, `HarnessRuntimePort`, `OperatorProjection`), the other two being `runtime-realization.md`'s own domain detail, not this mechanism's. `control-plane-and-client-protocol-reconciliation.md` independently confirms `OperatorProjection` §4 "already claims most of [a separate, root-level control-plane idea's] 'Client protocol' and 'Control-plane ownership' sections almost verbatim" (accepted 2026-09-14), and that `operator-switchboard.mjs` implements part of this shape (discovery, runtime-binding lookup, bounded projection routing — `OPERATOR_COMMANDS`, `bindingView()`) but explicitly lacks "active-binding fencing or runtime-selection-policy-overlay machinery." The client-protocol contract's own control-packet schema (`action`/`subject`/`revision`/`authority`/`expected consequence`) is a real, adjacent precursor — but the *lifecycle-feedback* half (`proposed`/`pending`/`admitted`/`refused`/`completed`/`stale`) this mechanism requires appears nowhere in `operator-switchboard.mjs`'s own text, confirmed by direct read. A genuine, unbuilt mechanism, not an existing thing merely uncited. Active-binding fencing (`mechanisms/resource-lease-and-fencing.md`, `runtime-realization.md` §12) is not a piece of `OperatorProjection` split off — `control-plane-and-client-protocol-reconciliation.md` states this explicitly: "`OperatorProjection` projects lease/fence state for display and control but does not claim to mint it," and separately, "operator clients are orthogonal to this gap, not part of it." The two mechanisms address genuinely separate concerns that happened to surface in the same reconciliation, not one split responsibility.

---

## Key Invariants

1. **Projection may constrain and encode authority; it may never enlarge it.**
2. **Projection never decides domain meaning — the owning transition and its result remain the owning dimension's own truth.**
3. **Projection never mutates directly merely because intent exists — every effect passes through the owning domain's own admission transition.**
4. **Discovery and rendering of available operations is this mechanism's own territory; which operations exist and their authorization belongs to the canonical control plane and each owning domain.**
5. **Lifecycle feedback (`proposed`/`pending`/`admitted`/`refused`/`completed`/`stale`) with attributable reason is this mechanism's own shared vocabulary; the domain-specific meaning of each state belongs to the owning domain.**
6. **This mechanism composes with, but is never subsumed by or subsumes, Candidate Resolution and Admission — see Relationship, below.**

---

## What This View Does Not Show

This page does not define:

- which operations exist or their authorization (the canonical control plane, `OperatorProjection`, and whichever dimension owns each operation);
- domain meaning of an accepted intent, or the resulting authoritative transition (each owning dimension's own content);
- candidate-set reduction to zero/one/many (`mechanisms/candidate-resolution-and-admission.md`);
- concrete UI, transport, or rendering implementation details — this page describes the logical discipline, not a display or protocol implementation.

---

## Relationship to Authority and Ownership

A projected candidate exposes or encodes authority already granted; it never manufactures authority by existing. This is the same family as `authority-and-ownership.md` §12's invalidation-never-mints-authority invariant, applied here to *intent* rather than resource leases or candidate admission: collecting or displaying an intent never grants standing beyond what the owning domain already authorized.

## Relationship to Candidate Resolution and Admission

Composable, never subsuming or subsumed by the other — a real distinction, not a restatement:

```text
Authority-Preserving Intent Projection
    turns intent into a bounded candidate under the current
    authority/context envelope

Candidate Resolution and Admission
    decides whether an already-defined candidate set deterministically
    reduces to 0 / 1 / N and admits the selection
```

They compose in sequence where a domain uses both: `operator intent → projected bounded candidates → candidate resolution/admission → domain effect` — but neither one is the other's definition, and a domain may use this mechanism without ever invoking the other (a single unambiguous candidate needs no reduction).

## Relationship to Runtime Realization

The operator policy overlay (§11 there) is this mechanism's own real, partial instance — that dimension owns the concrete policy content being projected; this mechanism owns the discipline that keeps the projection from becoming a second resolver.

---

## Related Architecture Views

- **`authority-and-ownership.md`** — owns the authority model this mechanism's projections must respect; a further concrete instance of §12's invalidation-never-mints-authority invariant, applied to intent.
- **`runtime-realization.md`** — §11's operator policy overlay is this mechanism's own confirmed, partial instance.
- **`provider-turn-harness-runtime-and-operator-projection.md`** — origin of `OperatorProjection`, one of this mechanism's confirmed precursors (§ Confirmed Instances, above); its other two ports (`ProviderTurnPort`, `HarnessRuntimePort`) are `runtime-realization.md`'s own domain detail, not this mechanism's.
- **`mechanisms/candidate-resolution-and-admission.md`** — a distinct, composable mechanism; neither subsumes the other.
- **`role-and-contract-structure.md`** — explicitly disclaims the authoring/admission territory this mechanism partially serves; the owning dimension's own scope is unchanged by this mechanism's existence.

---

## Source and Status

```yaml
architecture_status:
  design: accepted
  reconciliation: reconciled
  authorization: design_work_authorized
  implementation: none
  owner: app-server/docs/work-engine-studio-reconciliation.md
  status_as_of: 2026-09-16
```

`design: accepted` — the reconciliation's own Acceptance section: "Accepted 2026-09-14 ... This document's findings and disposition are confirmed accurate." `reconciliation: reconciled` — this page's content traces directly to that document, read in full this session, plus `control-plane-and-client-protocol-reconciliation.md` and `runtime-realization.md`'s current text, cross-checked by a dedicated architectural-synthesis fork. `authorization: design_work_authorized` — the same Acceptance section states directly: "The stated residue — the authority-preserving interactive command/edit projection (discovery, rendering, bounded-intent submission, lifecycle feedback) — is authorized for design work only, not implementation yet, pending that connective-layer schema being specified." `implementation: none` — confirmed directly: no discovery/rendering/submission/lifecycle-feedback code exists for any domain; `operator-switchboard.mjs` is a real but partial precursor (§ Confirmed Instances), not an implementation of this mechanism itself.

**Citation completed 2026-09-16**: `OperatorProjection`'s own origin, `app-server/ideas/pending/provider-turn-harness-runtime-and-operator-projection.md` §4, was cited only transitively (through `control-plane-and-client-protocol-reconciliation.md`) until now. No disposition changes — the term was already used correctly; only its source was previously uncited by name.
