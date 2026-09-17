# Idea: Remediation Delta for Native Review

**Status:** Architecture idea; not accepted, prioritized, or authorized for
implementation. Depends on [Candidate Trajectory — Durable Foundation](candidate-trajectory-durable-foundation.md).

**Scope:** What a retained/continuing native reviewer receives when a
remediation candidate replaces the candidate it previously reviewed.

**Authority:** Exploratory only. This document does not amend the migration
roadmap, admit an implementation, or authorize spending.

```yaml
idea_status:
  architectural_supersession: not_applicable
  architectural_supersession_note: "Checked 2026-09-17, per review: no plausible canonical owner exists to compare against, so this needs no additional semantic-coverage check to earn none rather than unknown -- this is App Server slice-campaign/native-review implementation detail consuming the (also-none) Candidate Trajectory primitive, not the kind of content any of the 13 dimensions, 5 mechanisms, or 2 substrates tracks."
  residue: none
  backlog: present
  backlog_ledger: "SS4 ('Evaluation before defaulting') names a two-stage plan (1: three-way controlled historical replay A/B/C; 2: shadow production comparison). Section disposition below."
  audit_scope:
    - keyword-scan: full_document
    - close-read: "SS1-6"
  audit_scope_completeness: complete
  status_as_of: 2026-09-17
```

```yaml
idea_provenance:
  origin: direct_capture
  related_reconciliations:
    - candidate-trajectory-durable-foundation.md (upstream dependency, itself architectural_supersession: none)
```

## Summary

Today, when remediation produces a new candidate C2 after a reviewer found
issues in C1, the reviewer boundary App Server constructs is always
**baseline → current candidate**, never **previous candidate → current
candidate**. The retained reviewer is expected to use conversational
continuity to work out what actually changed since its last review. This idea
proposes an additional host-owned, deterministic section — an exact
remediation delta — so the reviewer spends its judgment on *whether the
remediation resolved the finding and avoided introducing a new one*, instead
of reconstructing what changed.

Because this changes what a model reads and is expected to reduce judgment
work, it should not be defaulted into production without comparison evidence,
even though the underlying diff computation is itself fully deterministic.

## 1. Current behavior, confirmed in code

`createReviewBoundary` always diffs the original baseline against the current
candidate:

```js
export function createReviewBoundary({workspaceRoot, campaign, subject}) {
  const candidate = campaign.candidate;
  const baselineCommit = execFileSync("git", ["-C", workspaceRoot, "rev-parse",
    `${candidate.baseline_commit_oid}^{commit}`], {encoding: "utf8"}).trim();
  ...
  const patch = execFileSync("git", ["-C", workspaceRoot, "diff-tree", ...],
    ...`${baselineCommit}^{tree}`, candidateTree...);
```

(`app-server/src/services/slice-campaign/native-review-host.mjs:125-198`)

There is no branch anywhere in this function, or in `executeRemediation`
(`app-server/src/services/slice-campaign/native-review-host.mjs:482-495`), that
computes a boundary between the *previous* candidate and the current one.
`campaign.candidate` is always the single current candidate
(`app-server/src/services/slice-campaign/service.mjs:99`, overwritten on
replacement at `service.mjs:255-278`) — there is no prior candidate reference
available to diff against even if the code wanted to.

This matches the state described in
[deterministic-refactor-pressure-from-work-engine-evidence.md](deterministic-refactor-pressure-from-work-engine-evidence.md):
the review boundary contract gives the reviewer "exact baseline → candidate
diff," full stop. What "the exact delta and prior findings" means in practice
for a remediation turn is that the *prior findings* are exact and durable
(claim evidence), but the *delta since the previous candidate* is not — the
reviewer either re-derives it from the full baseline diff or leans on its own
conversational memory of the previous turn.

## 2. Proposed addition

Once [Candidate Trajectory](candidate-trajectory-durable-foundation.md) exists,
`executeRemediation` has a durable predecessor profile and delta available for
free at the moment C2 is bound. The retained reviewer's context can gain one
additional host-owned section, alongside the existing baseline→candidate
boundary (which is not removed):

```text
EXACT REMEDIATION DELTA

previous candidate: C1
current candidate:  C2

structural delta (from Candidate Trajectory):
  paths_added / paths_removed / paths_retained_and_modified
  modules_added / modules_removed / modules_retained
  symbols_modified_again
  generated_dependency_delta / validation_dependency_delta
  coverage / limitations

exact C1 -> C2 diff:
  (derived the same way createReviewBoundary derives baseline -> candidate,
   using the previous candidate's commit/tree instead of the baseline's)
```

The exact C1→C2 diff should be derived from Git the same way
`createReviewBoundary` already derives baseline→candidate — same tooling, same
`--unified` context, same evidence-catalog construction — just against a
different pair of trees. The structural summary above it comes entirely from
Candidate Trajectory's already-computed delta; no new analysis is introduced
here.

## 3. Fact authority is immediate; reliance policy is not

These are two separate questions, and this document keeps them separate:

```text
fact authority:
  the exact C1 -> C2 diff is authoritative evidence immediately.
  It is deterministic Git evidence, computed the same trusted way
  createReviewBoundary already computes baseline -> candidate.

reliance/substitution policy:
  whether the reviewer should be told it may rely on this section
  INSTEAD OF its existing continuity/reconstruction behavior
  requires comparison evidence (section 4).
```

The diff itself needs no pilot to be trustworthy — it is exactly as
deterministic as the baseline→candidate diff the reviewer already receives.
What needs evidence is a policy change: whether the review contract should
start telling the reviewer "you may treat this section as ground truth for
what changed since the last review" in place of reconstructing that itself.
Nothing prevents the section from being added and read by the reviewer before
that policy question is settled — the two are not gated on each other. Only
the *substitution* claim is.

Two concrete risks apply to the substitution question, not to the fact
itself:

- **Anchoring.** A reviewer handed "here is exactly what changed" might
  under-scrutinize the untouched majority of the candidate that the delta
  implies is unaffected, if the delta's coverage/limitations are misread as
  stronger than they are.
- **Redundant continuity.** If the existing baseline→candidate boundary plus
  claim evidence already gives the reviewer everything it needs, this section
  is pure token cost with no quality or inference benefit.

Sol's own framing of the closely related builder-side migration is the right
model here: "For a while, let both exist and compare them... Once equivalence
is demonstrated, retire the redundant inference requirement." The same
discipline should apply on the reviewer side before the exact remediation
delta becomes authoritative (i.e., before the reviewer is told it may rely on
the delta instead of re-deriving continuity itself).

## 4. Evaluation before defaulting

**[PLAN: OPEN — 2026-09-16. No durable execution evidence found within the surfaces checked: `app-server/src` and `planning/` (grepped for "controlled historical replay", "shadow production", "Candidate Trajectory" — zero hits outside this idea-document family), and Work Engine campaign/worktree state under `/home/bline/.local/state/work-engine` (no workstream named for this plan). Consistent with this document's own dependency on `candidate-trajectory-durable-foundation.md`, which is itself not accepted or authorized.]**

This does not need a registered controlled pilot in the sense of the
representation-effect studies under `app-server/ideas/pending/pilots/` (those
test subtle framing/preference effects on judgment; this is a mechanical
substitution of "what facts are present in context"). It does need evidence
before the delta section becomes something the reviewer is told is
authoritative:

1. **Controlled historical replay, three-way rather than merely with/without.**
   Re-run completed remediation review turns (real C1→C2 pairs with known
   accepted findings/outcomes) under three conditions, holding everything else
   fixed:

   ```text
   A: current behavior (baseline->candidate boundary + claim evidence only)
   B: A + the exact C1->C2 diff
   C: A + the exact C1->C2 diff + the structural trajectory summary
   ```

   Comparing A→B isolates whether value comes from not having to reconstruct
   the diff at all. Comparing B→C isolates whether the compact structural
   summary adds anything beyond the raw diff, or instead introduces the
   anchoring risk described above. Measure finding quality
   (missed/hallucinated regressions), reviewer turn count, and token cost
   across all three.
2. **Shadow production.** Compute and log the delta section for real
   remediation turns without changing what the reviewer is told to rely on,
   then compare its content against what the reviewer's own continuity-based
   account of "what changed" actually was, to check for disagreement before
   trusting either party's account by default.

Only after that evidence exists should the review contract wording change
from "examine the exact delta and prior findings" to something that
explicitly authorizes the reviewer to rely on the host-owned remediation delta
as ground truth for "what changed since the last review" — and, depending on
what the three-way comparison shows, that authorization might cover the exact
diff (condition B) without extending to the structural summary (condition C),
or vice versa.

## 5. Non-goals

- This does not change the baseline→candidate boundary or claim evidence
  contract; it adds one section, additively, for remediation turns only.
- This does not compute or expose any semantic judgment ("remediation
  resolved the finding") — that remains the reviewer's job, now with better
  evidence to do it.
- This does not touch the builder side; see
  [builder-side trajectory consumption](candidate-trajectory-builder-side-consumption.md)
  for that separate, larger-scope idea.

## 6. Relationships

| Direction | Relationship |
| --- | --- |
| [Candidate Trajectory — Durable Foundation](candidate-trajectory-durable-foundation.md) | Owns the predecessor binding and structural delta this idea consumes. |
| `native-review-host.mjs` (`createReviewBoundary`, `executeRemediation`) | Existing implementation this idea proposes to extend, additively. |
| [Builder-side trajectory consumption](candidate-trajectory-builder-side-consumption.md) | Sibling consumer idea; no shared implementation dependency beyond Candidate Trajectory itself. |

This document does not own the native-review contract, claim-evidence
semantics, or reviewer-selection policy.
