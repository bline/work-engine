# Idea Status Grammar

> **Question:** How does a pending idea document state what it still is — a live architecture source, a drained-but-historical origin, an open question ledger, an operative plan — without collapsing all of that into one ambiguous "pending"?

## 1. Purpose

`app-server/docs/architecture/status-grammar.md` answers this question for the 20 canonical views, using four axes: design, reconciliation, authorization, implementation. Those axes describe a claim's position *inside settled architecture*.

Documents under `app-server/ideas/pending/` are not inside settled architecture — they are its source material, in every state from raw capture to fully-drained provenance. The canonical grammar's axes do not fit them: a pending idea has no "design: accepted" of its own (acceptance belongs to whatever it feeds), and "reconciliation: reconciled" turned out, on inspection, to mean something dangerously weaker than most readers would assume — it means *the canonical view's content was checked against the idea*, not that *the idea has nothing left to say*. Reusing that grammar here would either understate real, still-open content or overstate how settled an idea is merely because something was built from it.

This document defines a separate grammar for exactly this corpus: **three independent status axes**, plus **provenance**, which is metadata rather than a lifecycle status.

## 2. The Governing Distinction

> **A canonical view citing an idea as its `owner` is not the same claim as the idea having nothing left to say.**

This session found five idea documents cited as the literal `owner:` field of a canonical view, all marked `reconciliation: reconciled` there. A first pass treated that as evidence the ideas were retirement-ready. A closer check — auditing each idea's own open-question ledger against the *current* text of what it produced — found that **none of the five retire cleanly as whole documents**. Every one has a real split: a core that a canonical view now states better than the idea does, and a remainder that exists nowhere else and would be lost if the file were discarded or waved off as "done."

A second distinction, found while applying the grammar rather than anticipated in its first draft: **that remainder is not one kind of thing.** An open question can be *architectural residue* — a real gap in the 13 dimensions, 5 mechanisms, or 2 substrates, something no owner has decided — or it can be *operative backlog* — a proving vertical, a schema-completion task, an investigation of an existing document's own text, something whose architecture is already settled and only needs doing or checking. A bare `[OPEN]` tag collapses these into one bucket. That is a real loss: a reader deciding what to do next needs to know whether the next step is "someone must decide an architecture question" or "someone must go build or verify something" — those have different owners and different urgency. §5 below requires every live ledger item to carry that distinction.

The grammar below exists to make both splits explicit instead of forcing a single verdict, or a single kind of open-ness, onto a document that contains more than one.

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

> Does this document contain **architectural** open questions or design content that no dimension, mechanism, or substrate owns?

```text
none
    Every architectural-kind question this document ever posed is now
    answered elsewhere or no longer applies. Checked directly, not
    assumed from architectural_supersession — a document can be
    `architectural_supersession: full` and still carry `residue: open`
    if its own question ledger was never re-swept against what got
    built.

mixed
    The document's ledger contains at least one item tagged
    residue-kind (§5) whose disposition is answered, and at least one
    tagged residue-kind whose disposition is open or unchecked.
    Requires the itemized ledger — see §5.

open
    At least one residue-kind item remains open or unchecked, and none
    of the ledger's residue-kind items are answered — or the document's
    own content is itself an active, undecided architectural question
    surface (a raw capture, an unevaluated hypothesis) rather than a
    settled design with a separate open-questions appendix.
```

**`residue` is derived only from ledger items tagged residue-kind (§5.1).** Backlog-kind items never move this axis, regardless of their own disposition — a document with twelve open backlog-kind questions and zero open residue-kind questions is `residue: none`, not `residue: mixed`.

### 3.3 Backlog

> Does this document contain operative, non-architectural content — a staged plan, a proving vertical, a schema-completion task, an investigation of another document's text — that no canonical view or other document duplicates?

```text
none
    No operative content of this kind, staged or itemized.

present
    Operative content exists (name it: a staged-plan section, or
    ledger items tagged backlog-kind with an open or unchecked
    disposition) with no other home. Canonical views describe what a
    dimension owns, never a rollout sequence, a proving vertical, or a
    verification task — this content is categorically different from
    architecture and is lost if the idea is discarded.

active
    Present, and a live campaign or worktree is currently executing it.
    Distinguish "someone should do this" from "someone is doing this."
```

**`backlog` is derived from two independent sources, either of which is sufficient on its own:** a dedicated staged-plan section (migration steps, pilot stages — as in `hierarchical-planning-and-multi-supervisor-orchestration.md`'s §A-I or `proposal-decision-gated-implementation-compilation.md`'s Stage 0-6), **or** one or more ledger items tagged backlog-kind (§5.1) whose disposition is open or unchecked. A document can be `backlog: present` from ledger items alone, with no dedicated plan section at all — do not require a staged-plan section before setting this axis.

## 4. Provenance (Metadata, Not a Status Axis)

> Where did this document come from, and what does it supersede or get superseded by?

Provenance answers factual questions about origin and lineage. It has no maturity ordering, no "better" or "worse" value, and does not participate in the derivation rules in §3 — it is recorded because architecture archaeology keeps re-deriving it from scratch otherwise, not because it describes where the document sits in a lifecycle.

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

## 5. Ledgers: Per-Item Kind and Disposition

When a document has a formal open-question ledger, every item gets two independent tags: a **kind** (§5.1, permanent — what sort of question this is) and a **disposition** (§5.2, checked per audit pass — where that question currently stands).

### 5.1 Kind

```text
RESIDUE
    An architectural question: what should a dimension, mechanism, or
    substrate own or decide? Answering it changes or completes settled
    architecture. Feeds the `residue` axis (§3.2).

BACKLOG
    An operative question: a proving vertical, a schema or
    representation still to be authored, a concrete component's
    implementation detail, or an investigation of what another
    document's own text already says. Its architecture is not in
    question; something needs to be built, chosen, or checked. Feeds
    the `backlog` axis (§3.3).
```

Kind is a property of the *question itself*, not of its current disposition — a RESIDUE question does not become BACKLOG merely because it turns out to be answered, and an item's kind does not change between audit passes unless the question itself is later reworded.

### 5.2 Disposition

```text
[ANSWERED — <canonical view §, one line of why>]
[OPEN — no owner found]
[MOOT — <why the question no longer applies>]
[UNCHECKED — not verified this pass]
```

`[UNCHECKED]` must not be used as a substitute for doing the check when time permits; it exists so a partial audit is honestly labeled partial rather than silently rounding unchecked items into `[OPEN]`. **An `[UNCHECKED]` item still gets a kind tag** — classifying what *sort* of question it is does not require resolving whether it is currently answered, and doing so is not "resolving it for completeness."

### 5.3 Combined tag format

Prepended to each ledger item:

```text
[KIND: RESIDUE] [OPEN — no owner found]
[KIND: BACKLOG] [UNCHECKED — not verified this pass]
[KIND: RESIDUE] [ANSWERED — authority-and-ownership.md SS9, delegation modes]
```

## 6. Format

Every idea document under `app-server/ideas/pending/` that has been through this audit declares two blocks in its own `Status` section — status and provenance kept visibly separate, per §4:

```yaml
idea_status:
  architectural_supersession: partial
  superseded_by:
    - view: app-server/docs/architecture/organizational-compilation.md
      scope: "Parts 1-6, 8 (core thesis)"
    - view: app-server/docs/architecture/authority-and-ownership.md
      scope: "Parts 1-6, 8 (core thesis)"
  residue: mixed
  residue_ledger: "Part 12 (27 items) — see inline KIND/disposition tags"
  backlog: present
  backlog_ledger: "Part 12 items tagged KIND: BACKLOG — see inline tags"
  status_as_of: 2026-09-16
```

```yaml
idea_provenance:
  origin: synthesized_from_raw_material
  raw_material: app-server/ideas/history/2026-09-15-pre-synthesis/
```

A document not yet audited under this grammar carries no `idea_status` block — absence means "not yet run through this process," never "clean." This mirrors the canonical grammar's own discipline: nothing gets an implicit status by omission.

## 7. What This Grammar Does Not Decide

- It does not decide whether a document's *architecture* is correct — that is exactly what `architectural_supersession` points to, and the canonical view remains the authority on its own content, per `status-grammar.md`.
- It does not authorize implementation of anything in `backlog: present` — that remains whatever authorization the document's own Authority section already states, unchanged by this grammar.
- It does not retire a document, delete it, or move it out of `app-server/ideas/pending/`. `architectural_supersession: full` plus `residue: none` plus `backlog: none` is the condition under which a document is a pure historical/provenance artifact going forward — a judgment call for whoever reads that combination, not an automatic file operation this grammar triggers.
- It does not require every `[UNCHECKED]` item to be resolved before a document can be audited under this grammar. A partial pass, honestly labeled, is a valid outcome — not a reason to guess dispositions merely to leave no `[UNCHECKED]` tags behind.
