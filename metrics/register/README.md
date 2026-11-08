# Research register (Stage 0)

A small, reproducible record of what studies observed and what they support. It keeps three
things apart: what a study observed (`studies/`), how that is assessed now (`judgments.jsonl`,
`events.jsonl`), and whether it bears on a particular decision (`applicability.jsonl`). None of
it changes a decision. Decision owners decide.

## Rules

1. Studies record what the sources say. They carry no judgment fields.
2. `events.jsonl` and `judgments.jsonl` are append-only: a correction is a new event that links
   the corrected finding, a changed assessment is a new judgment line (the latest per finding
   and axis wins), nothing is rewritten. `applicability.jsonl` is append-only except for one
   in-place edit: the decision owner sets `authorization.status` (and `at`) on a record. Every
   other change to an applicability record is a new record. `studies/`, `questions.json`,
   `decisions.json` and `materialized.jsonl` are edited in place and reviewed as diffs; the
   repository history is their only record of the prior state. Because `validate` requires an
   applicability record's `decision_revision` to equal the decision's current `revision`, a
   revision change to a decision means rewriting or re-adding its records; the historical
   decision record `...-at-df288c3d` below is the workaround for one case.
3. The current-knowledge view is generated, never edited. `register.py build` regenerates it
   byte for byte.
4. `records/` is never edited. A correction to a record is a new record elsewhere.
5. In-repo sources are pinned `{path, commit, blob, lines?}`, valid when
   `git rev-parse <commit>:<path>` equals `blob` and the object is a blob; `lines` (`N` or `N-M`,
   1-based) are lines of that file at that commit and must lie within it. Off-repo sources are
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
- `anchors --out DIR [--at COMMIT]`: reports whether each decision's anchored text still stands
  where the register says, and writes `anchors.json` and `anchors.md` to `--out` only. It changes
  nothing: nothing is re-anchored, no record is edited, no authorization is transferred, and the
  repository (including `.git`) is not written. `--at` (default `HEAD`) is resolved once to a full
  commit id. The register's `decisions.json` and `applicability.jsonl` are read from that same
  commit, not from the working tree, and their digests are recorded in the output; `--register`
  must therefore lie inside the repository. Every observation is repository source at the pinned
  commit; for a decision under `skills/` the installed skill release is not examined. Git is called
  with explicit diff options, so the result does not depend on git configuration. Each decision gets
  one class: `unchanged` (same lines, identical text), `moved` (no change to the anchored lines, they
  sit at other line numbers), `relocated_verbatim` (history changed the lines but the identical text
  occurs once in the document), `modified` (the range can be followed but its text differs),
  `ambiguous` (the exact text occurs more than once at the target, every occurrence is listed as a
  candidate and none is chosen) or `absent` (document gone, or the range deleted with the text
  nowhere), or an error entry (unresolvable revision, document missing at the revision, bad or
  out-of-range `lines`). A whole-document anchor is `unchanged`, `modified` or `absent` by blob. A
  rename is reported as `document_at_target`, not as a class. `relocated_verbatim`, and a `moved`
  anchor whose enclosing Markdown heading changed, are flagged `owner_review_required`; no other
  judgment of meaning is made. For every decision that is not `unchanged` the report gives the
  before state (revision, lines, anchored-text sha256), candidate or new lines, and each applicability
  record that cites it, copied verbatim, with an after-state of `carried_unchanged`,
  `owner_review_required` or `pending_judgment_required`; an `authorized` record that is not
  `carried_unchanged` is shown as `authorized_at_prior_revision`. Exit 0 when every decision is
  classified; 3 when outputs were written but a decision or register line has an error entry; 4
  when nothing could be written (target or register records unusable); 2 for an argument error or an
  `--out` inside the register or `.git`. Deterministic: two runs on the same inputs are byte-identical.

## Conventions this register adopts

**Standing from events.** A correction event on a finding gives `corrected`; a challenge gives
`challenged`; both give `corrected_and_challenged`. A status_discrepancy sets a flag and leaves
standing alone. The judged standing (`standing | qualified | corrected | withdrawn`) is shown
beside it, not replaced by it.

**Decision `lines`.** A decision's `lines` are lines of its `document` at its own `revision`,
not at any later commit. `validate` checks the format and that they lie inside that file; it
cannot check that they hold the anchored text.

**Which decisions `nominate` reaches.** For each event it lists every decision whose `findings`
include the event's `subject_finding` **or its `by_finding`**, and says which (`via`:
`subject_finding`, `by_finding` or `both`). The chosen rule is to follow `by_finding`. Two
cases depend on it. A challenge to K8 by K7 reaches the Pi coding profile acceptance contract
because that decision lists K7, not because it lists K8; the roadmap route is reached because
it lists K8. The K5 same-day correction (subject `...#rediscovery-free-reading`, by
`...#conditional-result`) reaches the Pi/Docker context replacement decision only through
`by_finding`: no decision lists the subject finding, so without the rule that correction would
nominate nothing. Each decision's `findings` follow pass 1: the context-boundary H2 levels list
only the K6 findings (pass 1: H2 rests on K6 and K5 only informs it), so K5 relates to H2
through its applicability record (`not_established`) and the K5 correction does not nominate H2.

**Qualification events.** The event vocabulary has no `qualification` kind. The K3 F1.1
qualification (2026-10-28) is recorded as a `challenge`, with the basis saying so. The 10-31
quota audit examined a later report (`quota-fit-20261031`), not the experiment-plan section 1
fit that K3 records, so it is not a challenge to K3's fitted values. It is recorded as a
`status_discrepancy` on K3 (the cost-table row gives a per-1% figure without the 10-29 quota
change the audit's report re-expresses); this departs from pass 1, which lists the audit as a
qualification of the fit. The event basis says so.

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
owner's act, and the one permitted in-place edit (rule 2): they set `authorization.status` and
`at` on the record, and the repository history keeps the prior state. `validate` rejects a
decided status without `at`.

## Not in this stage

Live claims, CBM captures, changes to the claims host, plans, docs, skills or records, and the
other inventoried studies.
