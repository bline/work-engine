# Idea Status Grammar

> **Question:** How does a pending idea document state what it still is — a live architecture source, a drained-but-historical origin, an open question ledger, an operative plan — without collapsing all of that into one ambiguous "pending"?

## 1. Purpose

`app-server/docs/architecture/status-grammar.md` answers this question for the 20 canonical views, using four axes: design, reconciliation, authorization, implementation. Those axes describe a claim's position *inside settled architecture*.

Documents under `app-server/ideas/pending/` are not inside settled architecture — they are its source material, in every state from raw capture to fully-drained provenance. The canonical grammar's axes do not fit them: a pending idea has no "design: accepted" of its own (acceptance belongs to whatever it feeds), and "reconciliation: reconciled" turned out, on inspection, to mean something dangerously weaker than most readers would assume — it means *the canonical view's content was checked against the idea*, not that *the idea has nothing left to say*. Reusing that grammar here would either understate real, still-open content or overstate how settled an idea is merely because something was built from it.

This document defines a separate grammar for exactly this corpus: **three independent status axes**, an **audit-completeness flag**, and **provenance**, which is metadata rather than a lifecycle status.

## 2. The Governing Distinctions

> **A canonical view citing an idea as its `owner` is not the same claim as the idea having nothing left to say.**

This session found five idea documents cited as the literal `owner:` field of a canonical view, all marked `reconciliation: reconciled` there. A first pass treated that as evidence the ideas were retirement-ready. A closer check — auditing each idea's own open-question ledger against the *current* text of what it produced — found that **none of the five retire cleanly as whole documents**. Every one has a real split: a core that a canonical view now states better than the idea does, and a remainder that exists nowhere else and would be lost if the file were discarded or waved off as "done."

> **An open question can be architectural residue or operative backlog, and a bare `[OPEN]` tag hides which.**

Architectural residue is a real gap in the 13 dimensions, 5 mechanisms, or 2 substrates — something no owner has decided. Operative backlog is a proving vertical, a schema-completion task, an investigation of an existing document's own text — something whose architecture is already settled and only needs doing or checking. These have different owners and different urgency; collapsing them into one `[OPEN]` bucket loses that.

> **`UNCHECKED` proves incomplete audit coverage. It proves nothing about whether residue or backlog exists.**

An item that was never re-verified against current architecture is not evidence that its question is still live — it is evidence that nobody looked. Treating unchecked items as if they prove presence (`residue: present` or `backlog: present` merely because unchecked items exist) is the same failure mode this whole grammar exists to catch elsewhere: state getting inflated by absence of a check rather than by a check's actual result. §4 makes this the derivation rule, not a stylistic preference.

> **Kind describes a question's current unresolved-consequence class, not a permanent property of its wording.**

The same textual question can start as an architectural question ("which object should own X?") and, once an authoritative decision answers it, reduce to an implementation or schema question ("how do we represent X in the object that now owns it?") — with no change to the question's own text. Kind tracks what is *currently* unresolved about the question, and must be re-examined, not assumed frozen, at each audit pass. Reclassification is real work, not drift: it requires a cited reason (§4.1).

The grammar below exists to make all three distinctions explicit instead of forcing one verdict, one flavor of open-ness, and one immutable classification onto content that has none of those properties.

## 3. The Three Status Axes

### 3.1 Architectural supersession

> Does a canonical architecture view now state this document's architectural claims better than the document itself?

```text
none
    No canonical view derives from this document. It remains the sole
    source for whatever architecture it describes.

partial
    Named sections are superseded by named canonical view(s); other
    named sections remain the sole source for their own content.

full
    Every architectural claim in this document is now represented in
    canonical view(s). The document is provenance for architecture, not
    a live architecture source.
```

`partial` and `full` must name the canonical view(s) and, for `partial`, exactly which sections are covered — a bare "superseded by X" with no scope is not enough to prevent a reader from either over- or under-trusting the rest of the document. Superseded sections should carry a short note pointing to the canonical view rather than being deleted — the idea document is not being rewritten, only labeled.

### 3.2 Residue

> Is there *confirmed* architectural residue — an item asking what a dimension, mechanism, or substrate should own or decide, currently open?

```text
none
    Every RESIDUE-kind item (§4) is ANSWERED or MOOT. No confirmed
    open architectural question remains, and none is unchecked.

present
    At least one RESIDUE-kind item is confirmed OPEN.

unknown
    No RESIDUE-kind item is confirmed OPEN, but at least one is
    UNCHECKED. The audit cannot currently rule residue in or out.
```

`present` and `unknown` are not the same claim. `present` means a check found a live architectural question. `unknown` means no check was completed — it must never be silently rounded up to `present` or down to `none`. See §4 for the exact derivation.

### 3.3 Backlog

> Is there *confirmed* operative backlog — a staged plan, a proving vertical, a schema-completion task, or a BACKLOG-kind ledger item, currently open?

```text
none
    No dedicated staged-plan section exists, and every BACKLOG-kind
    ledger item (if any) is ANSWERED or MOOT.

present
    A dedicated staged-plan section exists (migration steps, pilot
    stages — named), OR at least one BACKLOG-kind ledger item is
    confirmed OPEN.

active
    present, and a live campaign or worktree is currently executing
    it. Distinguish "someone should do this" from "someone is doing
    this."

unknown
    No dedicated staged-plan section and no confirmed-open BACKLOG-
    kind item, but at least one BACKLOG-kind item is UNCHECKED.
```

A dedicated staged-plan section (e.g. `hierarchical-planning-and-multi-supervisor-orchestration.md`'s §A-I, `proposal-decision-gated-implementation-compilation.md`'s Stage 0-6) sets `present` on its own, independent of any ledger — most documents with this kind of section have no formal ledger at all.

## 4. Ledgers: Per-Item Kind and Disposition

When a document has a formal open-question ledger, every item gets two independent tags.

### 4.1 Kind (reclassifiable)

```text
RESIDUE
    An architectural question: what should a dimension, mechanism, or
    substrate own or decide? Ownership, authoritative-source-object,
    placement-among-durable-states, and authority/evidence-boundary
    questions default to RESIDUE. Feeds the `residue` axis (§3.2).

BACKLOG
    An operative question: a proving vertical, a schema or
    representation still to be authored for an already-placed concept,
    a concrete component's implementation detail, or an investigation
    of what another document's own text already says. Its architecture
    is not in question; something needs to be built, chosen, or
    checked. Feeds the `backlog` axis (§3.3).
```

**Default to RESIDUE, not BACKLOG, whenever a question turns on ownership, an authoritative source object, placement among durable states, or an authority/evidence boundary — even when its surface wording sounds implementation-flavored.** "Which realization and invalidation evidence belongs in operation receipts, durable lifecycle state, and operator projections?" is not schema detail merely because the nouns are concrete artifacts — it is asking which of three distinct owners holds a given fact, unresolved until an authority places it. Reclassify only when a canonical view has actually collapsed the ownership question into an implementation detail — cite the view and section that did so.

**Kind is not frozen across audit passes.** Re-examine it every time, because an upstream architectural decision can change a question's unresolved-consequence class without the question's own wording changing at all:

```text
"Which object should own X?"          (RESIDUE)
        ↓ architecture decides: Runtime Realization owns X
"How do we represent X in
 Runtime Realization?"                 (now BACKLOG — same question,
                                         same wording is possible, but
                                         the ownership choice it once
                                         asked for no longer exists)
```

A reclassification must cite the specific decision, canonical-view section, or prior audit finding that changed the consequence class — `[KIND: BACKLOG, reclassified 2026-09-16 — runtime-realization.md SS4 now settles the ownership question this item originally asked]`. A kind carried forward unchanged needs no such citation; only a change does.

### 4.2 Disposition

```text
[ANSWERED — <canonical view §, one line of why>]
[OPEN — no owner found]
[MOOT — <why the question no longer applies>]
[UNCHECKED — not verified this pass]
```

`[UNCHECKED]` must not be used as a substitute for doing the check when time permits; it exists so a partial audit is honestly labeled partial rather than silently rounding unchecked items into `[OPEN]`. **`[UNCHECKED]` never by itself sets `residue: present` or `backlog: present`** — see §3.2/§3.3. An `[UNCHECKED]` item still gets a kind tag; classifying what *sort* of question it is does not require resolving whether it is currently answered, and doing so is not "resolving it for completeness."

### 4.3 Combined tag format

Prepended to each ledger item:

```text
[KIND: RESIDUE] [OPEN — no owner found]
[KIND: BACKLOG] [UNCHECKED — not verified this pass]
[KIND: RESIDUE] [ANSWERED — authority-and-ownership.md SS9, delegation modes]
[KIND: BACKLOG, reclassified 2026-09-16 — <cited reason>] [OPEN — ...]
```

## 5. Audit Completeness

A document-level flag, independent of the three status axes, stating whether every ledger item has a checked disposition:

```text
complete
    No ledger, or every ledger item carries ANSWERED, OPEN, or MOOT —
    zero UNCHECKED items remain.

partial
    At least one ledger item is UNCHECKED.
```

`audit_completeness: partial` is not a defect to be hidden — it is the honest state of a bounded pass, and `residue`/`backlog` values of `unknown` are exactly what should follow from it rather than a guessed `present` or `none`.

## 6. Provenance (Metadata, Not a Status Axis)

> Where did this document come from, and what does it supersede or get superseded by?

Provenance answers factual questions about origin and lineage. It has no maturity ordering, no "better" or "worse" value, and does not participate in the derivation rules in §3-5 — it is recorded because architecture archaeology keeps re-deriving it from scratch otherwise, not because it describes where the document sits in a lifecycle.

```text
origin
    synthesized_from_raw_material | direct_capture | operator_authored
    | derived_from:<idea path>

raw_material
    path to preserved pre-synthesis material, if this document is a
    synthesized formulation rather than the original capture (the
    `ideas/history/` convention)

supersedes
    older idea document(s) this one replaces, if any

related_reconciliations
    docs/*-reconciliation.md documents that checked this idea against
    sibling architecture, if the reconciliation was NOT the mechanism
    that produced a canonical view's own citation (i.e., additional
    reconciliation record beyond what architectural_supersession
    already names)
```

## 7. Format

Every idea document under `app-server/ideas/pending/` that has been through this audit declares two blocks in its own `Status` section — status and provenance kept visibly separate, per §6:

```yaml
idea_status:
  architectural_supersession: partial
  superseded_by:
    - view: app-server/docs/architecture/organizational-compilation.md
      scope: "Parts 1-6, 8 (core thesis)"
    - view: app-server/docs/architecture/authority-and-ownership.md
      scope: "Parts 1-6, 8 (core thesis)"
  residue: present
  residue_ledger: "Part 12 (27 items) — see inline KIND/disposition tags"
  backlog: present
  backlog_ledger: "Part 12 items tagged KIND: BACKLOG — see inline tags"
  audit_completeness: partial
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: synthesized_from_raw_material
  raw_material: app-server/ideas/history/2026-09-15-pre-synthesis/
```

A document not yet audited under this grammar carries no `idea_status` block — absence means "not yet run through this process," never "clean." This mirrors the canonical grammar's own discipline: nothing gets an implicit status by omission.

## 8. What This Grammar Does Not Decide

- It does not decide whether a document's *architecture* is correct — that is exactly what `architectural_supersession` points to, and the canonical view remains the authority on its own content, per `status-grammar.md`.
- It does not authorize implementation of anything in `backlog: present` — that remains whatever authorization the document's own Authority section already states, unchanged by this grammar.
- It does not retire a document, delete it, or move it out of `app-server/ideas/pending/`. `architectural_supersession: full` plus `residue: none` plus `backlog: none` is the condition under which a document is a pure historical/provenance artifact going forward — a judgment call for whoever reads that combination, not an automatic file operation this grammar triggers.
- It does not require every `[UNCHECKED]` item to be resolved before a document can be audited under this grammar. A partial pass, honestly labeled `audit_completeness: partial` with `residue`/`backlog: unknown` where that is the honest derivation, is a valid outcome — not a reason to guess dispositions merely to leave no `[UNCHECKED]` tags behind.
- It does not treat a kind assignment as permanent. A later pass may find that an architectural decision collapsed a RESIDUE item into BACKLOG (or, in principle, the reverse, if a prior decision is itself reopened) — provided the reclassification cites what changed.
