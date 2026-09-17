# Idea Status Grammar

> **Question:** How does a pending idea document state what it still is — a live architecture source, a drained-but-historical origin, an open question ledger, an operative plan — without collapsing all of that into one ambiguous "pending"?

## 1. Purpose

`app-server/docs/architecture/status-grammar.md` answers this question for the 20 canonical views, using four axes: design, reconciliation, authorization, implementation. Those axes describe a claim's position *inside settled architecture*.

Documents under `app-server/ideas/pending/` are not inside settled architecture — they are its source material, in every state from raw capture to fully-drained provenance. The canonical grammar's axes do not fit them: a pending idea has no "design: accepted" of its own (acceptance belongs to whatever it feeds), and "reconciliation: reconciled" turned out, on inspection, to mean something dangerously weaker than most readers would assume — it means *the canonical view's content was checked against the idea*, not that *the idea has nothing left to say*. Reusing that grammar here would either understate real, still-open content or overstate how settled an idea is merely because something was built from it.

This document defines a separate grammar for exactly this corpus: **three independent status axes**, an **`audit_scope_completeness` flag** (explicitly a claim about coverage of a declared scope, not about exhaustive document understanding — §5), and **provenance**, which is metadata rather than a lifecycle status.

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
    No dedicated staged-plan section exists (or every one present is
    COMPLETED, SUPERSEDED, or MOOT — §4.3), and every BACKLOG-kind
    ledger item (if any) is ANSWERED or MOOT.

present
    A dedicated staged-plan section exists with disposition OPEN or
    PARTIAL (§4.3), OR at least one BACKLOG-kind ledger item is
    confirmed OPEN.

active
    present, and a live campaign or worktree is currently executing
    it (staged-plan disposition ACTIVE specifically — not PARTIAL,
    which by definition has nothing currently advancing it). Distinguish
    "someone should do this," "someone did some of this," and "someone
    is doing this" as three different claims.

unknown
    No staged-plan section with disposition OPEN/PARTIAL/ACTIVE and no
    confirmed-open BACKLOG-kind ledger item, but at least one staged-
    plan section or BACKLOG-kind ledger item is UNCHECKED.
```

**A staged-plan section's mere existence never sets this axis by itself — its own disposition (§4.3) does.** A `Stage 0-6` or `§A-I` block sitting in the document proves nothing about whether that plan is still operative: six months on it could be executed, superseded by a different approach, or abandoned, while the text itself is unchanged. Checking a staged-plan section's disposition is exactly as mandatory as checking a ledger item's — mere presence of content is never itself evidence of current liveness, for either.

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

### 4.3 Staged-Plan Section Disposition

A dedicated staged-plan section (migration steps, pilot stages) is not a ledger of independently decidable items — it is normally one sequenced whole — so it carries one disposition tag for the section, not one per stage, unless there is actual evidence that individual stages have diverged (some executed, others not):

```text
[PLAN: OPEN — no stage has started]
[PLAN: PARTIAL — some stages have real evidence of execution (fixtures,
    campaigns, landed code, a recorded pilot result), others do not, and
    no live campaign is currently advancing it]
[PLAN: ACTIVE — <campaign/worktree citation>, currently advancing it]
[PLAN: COMPLETED — every stage has evidence of execution]
[PLAN: SUPERSEDED — <what replaced it>]
[PLAN: MOOT — <why the plan's goal no longer applies>]
[PLAN: UNCHECKED — not verified this pass]
```

`ACTIVE` means current, ongoing execution — it must not be stretched to cover a plan with real historical partial progress but nothing presently advancing it; that is `PARTIAL`, a materially different claim ("some real work happened, then stopped or paused") from either `OPEN` ("nothing has happened") or `ACTIVE` ("something is happening now").

**The evidence for any of these dispositions must be about the plan's own named stages, not merely the implementation status of the architecture the plan targets.** A target view reporting `implementation: none` does not by itself establish `PLAN: OPEN` — the target being unbuilt is consistent with an early stage (a baseline measurement, a drafted contract schema, a shadow-mode pilot) having been attempted or even completed without ever changing the target view's own status. Check for the plan's own deliverables directly: real code, fixtures, campaign or worktree records, a recorded pilot result, a drafted schema — for each stage the plan names. Only cite the target's implementation status as corroborating context, never as the disposition's sole evidence.

Only `OPEN`, `PARTIAL`, and `ACTIVE` set `backlog: present`/`active` (§3.3) — `PARTIAL` sets `present`, the same as `OPEN`, since real remaining work exists but nothing is currently advancing it. `COMPLETED`, `SUPERSEDED`, and `MOOT` establish that this particular source does *not* currently contribute to backlog — the document could still be `backlog: present` from a ledger's own BACKLOG-kind items, checked independently. `UNCHECKED` contributes only to audit incompleteness (§5), exactly like an unchecked ledger item.

### 4.4 Combined tag format

Prepended to each ledger item:

```text
[KIND: RESIDUE] [OPEN — no owner found]
[KIND: BACKLOG] [UNCHECKED — not verified this pass]
[KIND: RESIDUE] [ANSWERED — authority-and-ownership.md SS9, delegation modes]
[KIND: BACKLOG, reclassified 2026-09-16 — <cited reason>] [OPEN — ...]
```

## 5. Audit Scope and Completeness

**"No formal ledger" is not evidence of nothing left to check — it is silence, and silence must not read as clean.** A document with no "Open Questions" heading can still carry `TBD`s, hedged "future work" asides, or an unresolved closing line in ordinary prose. Treating the absence of a ledger as automatic `audit_scope_completeness: complete` recreates, at the document level, exactly the omission failure `UNCHECKED` exists to prevent at the item level.

### 5.1 Audit scope (declared, not assumed)

Every audited document declares what was actually examined:

```yaml
audit_scope:
  - open-question-ledger        # a formal, numbered ledger, if one exists
  - staged-plan-section          # a dedicated plan/pilot/migration section, if one exists
  - keyword-scan: full_document  # a pattern search (TBD, unresolved, remains
                                  # open, undecided, etc.) across the entire
                                  # text, catching prose residue outside any
                                  # formal ledger
  - close-read: <section list>   # sections read in full, beyond a keyword scan
```

A document with neither a ledger nor a staged-plan section still requires at least a `keyword-scan: full_document` entry before `residue`/`backlog: none` can be claimed — a bare "no ledger found" is not itself a completed scope.

### 5.2 Completeness

**`audit_scope_completeness` is a claim about coverage of the declared `audit_scope`, never about the scope's own depth and never about exhaustive understanding of the document.** A document audited only by `keyword-scan: full_document` can be legitimately `complete` — every item that scan could find got a disposition — while still missing residue only a full close-read would surface. That is not a defect in the `complete` value; it is exactly why `audit_scope` is declared as its own field (§5.1) rather than folded into one number. Read the two fields together: `audit_scope_completeness` says whether the declared method was actually finished; `audit_scope` says how strong a method it was.

```text
complete
    Every source named in the declared audit_scope has a checked
    disposition — every ledger item and every staged-plan section is
    ANSWERED/OPEN/PARTIAL/MOOT/COMPLETED/SUPERSEDED/ACTIVE (never
    UNCHECKED), and any declared keyword-scan or close-read was
    actually carried out (not merely asserted).

partial
    At least one source within the declared audit_scope remains
    UNCHECKED. (A narrow or shallow audit_scope is not, by itself, a
    reason for `partial` — see above.)
```

`audit_scope_completeness: partial` is not a defect to be hidden — it is the honest state of a bounded pass, and `residue`/`backlog` values of `unknown` are exactly what should follow from it rather than a guessed `present` or `none`.

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
  audit_scope:
    - open-question-ledger
    - keyword-scan: full_document
  audit_scope_completeness: partial
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
- It does not require every `[UNCHECKED]` item to be resolved before a document can be audited under this grammar. A partial pass, honestly labeled `audit_scope_completeness: partial` with `residue`/`backlog: unknown` where that is the honest derivation, is a valid outcome — not a reason to guess dispositions merely to leave no `[UNCHECKED]` tags behind.
- It does not treat a kind assignment as permanent. A later pass may find that an architectural decision collapsed a RESIDUE item into BACKLOG (or, in principle, the reverse, if a prior decision is itself reopened) — provided the reclassification cites what changed.
