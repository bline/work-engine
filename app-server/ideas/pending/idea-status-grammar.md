# Idea Status Grammar

> **Question:** How does a pending idea document state what it still is — a live architecture source, a drained-but-historical origin, an open question ledger, an operative plan — without collapsing all four into one ambiguous "pending"?

## 1. Purpose

`app-server/docs/architecture/status-grammar.md` answers this question for the 20 canonical views, using four axes: design, reconciliation, authorization, implementation. Those axes describe a claim's position *inside settled architecture*.

Documents under `app-server/ideas/pending/` are not inside settled architecture — they are its source material, in every state from raw capture to fully-drained provenance. The canonical grammar's axes do not fit them: a pending idea has no "design: accepted" of its own (acceptance belongs to whatever it feeds), and "reconciliation: reconciled" turned out, on inspection, to mean something dangerously weaker than most readers would assume — it means *the canonical view's content was checked against the idea*, not that *the idea has nothing left to say*. Reusing that grammar here would either understate real, still-open content or overstate how settled an idea is merely because something was built from it.

This document defines a separate, four-axis grammar for exactly this corpus. The axes are independent of each other and of the canonical grammar.

## 2. The Governing Distinction

> **A canonical view citing an idea as its `owner` is not the same claim as the idea having nothing left to say.**

This session found five idea documents cited as the literal `owner:` field of a canonical view, all marked `reconciliation: reconciled` there. A first pass treated that as evidence the ideas were retirement-ready. A closer check — auditing each idea's own open-question ledger and backlog content against the *current* text of what it produced — found that **none of the five retire cleanly as whole documents**. Every one has a real split: a core that a canonical view now states better than the idea does, and a remainder — unanswered questions, or an operative plan — that exists nowhere else and would be lost if the file were discarded or waved off as "done."

The grammar below exists to make that split explicit instead of forcing a single verdict onto a document that contains more than one kind of content.

## 3. The Four Axes

### 3.1 Supersession

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

> Does this document contain open questions or design content that no other document has answered or owns?

```text
none
    Every open question this document ever posed is now answered
    elsewhere or no longer applies. Checked directly, not assumed from
    supersession — a document can be `supersession: full` and still
    carry `residue: open` if its own question ledger was never re-swept
    against what got built.

mixed
    The document contains an open-question ledger where items have
    different dispositions: some answered elsewhere (cited), some
    genuinely still open (no owner found), some moot (the question no
    longer applies given current architecture). Requires an itemized
    ledger — see §4.

open
    The document's own content is itself an active, undecided question
    surface (a raw capture, an unevaluated hypothesis) rather than a
    settled design with a separate open-questions appendix.
```

`residue` is checked independently of `supersession`. A document can be architecturally drained (`supersession: full`) while its own open-questions section is stale — some items secretly answered by the very views it produced, others genuinely still open — which is exactly `residue: mixed`, not `residue: none`.

### 3.3 Backlog

> Does this document contain a staged, operative plan — migration steps, pilot stages, a proving vertical — that no canonical view or other document duplicates?

```text
none
    No staged plan content. Purely conceptual or architectural.

present
    Contains real staged plan content (name the sections) with no other
    home. Canonical views describe what a dimension owns, never a
    rollout sequence to get there — this content is categorically
    different from architecture and is lost if the idea is discarded.

active
    Present, and a live campaign or worktree is currently executing it.
    Distinguish "someone should do this" from "someone is doing this."
```

### 3.4 Provenance

> Where did this document come from, and what does it supersede or get superseded by?

Not a maturity scale — a set of pointers, recorded because they answer questions architecture archaeology keeps re-deriving from scratch otherwise:

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
    reconciliation record beyond what supersession already names)
```

## 4. Residue Ledgers: Per-Item Disposition

When `residue: mixed`, the document's existing open-question list gets per-item tags, prepended to each item, using the same bracket convention this session already used for the persistent-provider-runtime-managers idea's 30 spike questions:

```text
[ANSWERED — <canonical view §, one line of why>]
[OPEN — no owner found]
[MOOT — <why the question no longer applies>]
```

An item may carry a fourth tag when the honest answer is genuinely uncertain rather than checked and open:

```text
[UNCHECKED — not verified this pass]
```

`[UNCHECKED]` must not be used as a substitute for doing the check when time permits; it exists so a partial audit is honestly labeled partial rather than silently rounding unchecked items into `[OPEN]`.

## 5. Format

Every idea document under `app-server/ideas/pending/` that has been through this audit declares one block in its own `Status` section:

```yaml
idea_status:
  supersession: partial
  superseded_by:
    - view: app-server/docs/architecture/organizational-compilation.md
      scope: "Parts 1-6, 8 (core thesis)"
    - view: app-server/docs/architecture/authority-and-ownership.md
      scope: "Parts 1-6, 8 (core thesis)"
  residue: mixed
  residue_ledger: "Part 12 (27 items) and Part 13 SS22-27 — see inline tags"
  backlog: none
  provenance:
    origin: synthesized_from_raw_material
    raw_material: app-server/ideas/history/2026-09-15-pre-synthesis/
  status_as_of: 2026-09-16
```

A document not yet audited under this grammar carries no `idea_status` block — absence means "not yet run through this process," never "clean." This mirrors the canonical grammar's own discipline: nothing gets an implicit status by omission.

## 6. What This Grammar Does Not Decide

- It does not decide whether a document's *architecture* is correct — that is exactly what `supersession` points to, and the canonical view remains the authority on its own content, per `status-grammar.md`.
- It does not authorize implementation of anything in `backlog: present` — that remains whatever authorization the document's own Authority section already states, unchanged by this grammar.
- It does not retire a document, delete it, or move it out of `app-server/ideas/pending/`. `supersession: full` plus `residue: none` plus `backlog: none` is the condition under which a document is a pure historical/provenance artifact going forward — a judgment call for whoever reads that combination, not an automatic file operation this grammar triggers.
