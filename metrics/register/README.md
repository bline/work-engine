# Research register (Stage 0)

A small, reproducible record of what studies observed and what they support. It keeps three
things apart: what a study observed (`studies/`), how that is assessed now (`judgments.jsonl`,
`events.jsonl`), and whether it bears on a particular decision (`applicability.jsonl`). None of
it changes a decision. Decision owners decide.

## Rules

1. Studies record what the sources say. They carry no judgment fields.
2. Events, judgments and applicability records are append-only. A correction is a new event
   that links the corrected finding; nothing is rewritten.
3. The current-knowledge view is generated, never edited. `register.py build` regenerates it
   byte for byte.
4. `records/` is never edited. A correction to a record is a new record elsewhere.
5. In-repo sources are pinned `{path, commit, blob, lines?}`, valid when
   `git rev-parse <commit>:<path>` equals `blob`. Off-repo sources are
   `{location, sha256 | null, custody: "off_repo", status: verified | unverified | unavailable}`
   with an optional `note`. When an off-repo primary exists it is recorded as well as any
   in-repo summary. Sealed files are cited by path and digest only.
6. An unknown is never entered as a value. Unassessed axes carry no judgment record and the view
   prints `not established`. A decision whose owner no source names says so in its `owner` text.
7. No record carries a global usefulness or obsolescence field; such keys are rejected.
8. Nomination reports; it never invalidates, reopens, retires or edits anything.

## Files

| File | Content |
|---|---|
| `questions.json` | the 20 enduring questions (text from pass 1 section 1; state is computed, not stored) |
| `studies/<id>.json` | 8 studies, one per pass 1 finding K1 to K8, with findings, lifecycle, sources and `aliases` carrying the K number |
| `events.jsonl` | correction, challenge and status_discrepancy events |
| `judgments.jsonl` | claim kind, evidence strength, standing and currency per finding, with assessor, date and basis |
| `decisions.json` | tracked decisions at exact git revisions, with owners and the findings they rest on |
| `applicability.jsonl` | per (finding, decision@revision): relation, rationale, evidence, proposer, authorization |
| `materialized.jsonl` | register finding to claim revision; empty until live publication is approved |

Closed key sets are enforced by `validate`. Identifiers are kebab-case; finding ids are
`<study-id>#<slug>`.

## Tool

`python3 -I metrics/scripts/register.py <command> [--register DIR] [--repo DIR]`. Standard library
only, no network, no model calls; it writes only to `--out`.

- `validate [--check-offrepo]`: closed key sets, unique ids, resolving references, repo pins by
  `git rev-parse`, off-repo shape. `--check-offrepo` also hashes existing off-repo files and
  reports `MATCH`, `MISMATCH`, `UNHASHED` or `ABSENT`; absence is reported and is not a violation,
  a digest mismatch is. Exit 0 on pass, 1 on violations.
- `build --out DIR`: `current-knowledge.json` and `current-knowledge.md`. Per finding: study,
  lifecycle, latest judgment per axis, standing derived from events, events, and per-decision
  applicability with authorization status. Per question: linked findings. Deterministic.
- `nominate --out FILE [--claims-host PATH --startup PATH --startup-sha256 HEX]`: per event, the
  decisions to review, applicability records, and claim revisions. Without the claims options,
  claim revisions are reported as `not_queried`. Disposition is always `review_nominated`.

## Conventions this register adopts

**Standing from events.** A correction event on a finding gives `corrected`; a challenge gives
`challenged`; both give `corrected_and_challenged`. A status_discrepancy sets a flag and leaves
standing alone. The judged standing (`standing | qualified | corrected | withdrawn`) is shown
beside it, not replaced by it.

**Which decisions `nominate` reaches.** For each event it lists every decision whose `findings`
include the event's `subject_finding` **or its `by_finding`**, and says which (`via`:
`subject_finding`, `by_finding` or `both`). The chosen rule is to follow `by_finding`. So a
challenge to K8 by K7 reaches the Pi coding profile acceptance contract because that decision
lists K7, not because it lists K8; the roadmap route is reached because it lists K8.

**Qualification events.** The event vocabulary has no `qualification` kind. The two K3 events
(F1.1, 2026-10-28, and the 10-31 audit) are recorded as `challenge`, with the basis saying so.

**Historical decision revisions.** `model-selection.claude-host-builder-rows-at-df288c3d` pins a
superseded revision of the Claude-host builder rows, the one that cited full-battery counts
before the traceability correction reached the document. It exists so a correction reaches the
revision that relied on the corrected reading, not only the current one.

**Decision revisions.** Taken from the nominations file where it states one (`1a1cb443` for the
quota row, `d80dd242` for the dogfood plan status line and the Pi acceptance code), otherwise the
last commit that changed the anchored lines (`git log -L`). The decision's own `revision` is the
only revision applicability may cite (`decision-revision-mismatch` otherwise).

**Relation vocabulary.** `supports | constrains | challenges | does_not_apply_directly |
not_established`. Pass 1 says the M22 replay "informs" the context-boundary levels; there is no
`informs`, so that record is `not_established` and says why.

**Judgments.** Values come from pass 1 section 2 only. Axes pass 1 does not state for a finding
have no record. Assessor: `claude-opus-5-5 (evidence-map pass 1, 2026-11-06)`.

**Applicability starts pending.** Every record is proposed by the register builder with
`authorization.status: pending` and the decision owner named. Authorizing or declining is the
owner's act: they set `authorization.status` and `at` on the record, and git history keeps the
prior state.

## Not in this stage

Live claims, CBM captures, changes to the claims host, plans, docs, skills or records, and the
other inventoried studies.
