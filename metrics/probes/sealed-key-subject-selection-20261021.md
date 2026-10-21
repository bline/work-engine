# Staged selection of a second sealed-key subject

**Status: procedure, 2026-10-21. Not a selection and not an authorization.** No
candidate's findings have been read by this session, and stage 0 below is designed
so that running it does not change that.

Every claim in the S2 line rests on one subject. A second independently keyed
subject is what converts those claims from "on this slice" to "replicated". The
scarce asset is not a sound plan - no plan in this family received a substantive
realizability review, so a candidate with no *known* plan defects is one nobody
audited - it is an **independently recorded sealed key**: findings written by a
different model family, at the time, before any Claude arm saw that revision.

## The constraint that sets the order

**Inspection is destructive of eligibility.** Reading a candidate's finding set
disqualifies the reader from being an assessment arm on that subject, and if the
same reader verifies the findings, the key's reference becomes partly their own
judgment - the thing Test R was built to avoid.

Three consequences for sequencing:

1. Run the cheapest non-contaminating screen across **all** candidates before any
   contaminating inspection.
2. Pay contaminating inspection on **one survivor at a time**, best first.
3. If more than one option must stay open, use a **different session per
   candidate**, since exclusion is per-subject rather than global.

## Stage 0 - metadata-only disqualification

Non-contaminating. Decided entirely from filenames, timestamps, integrity hashes
and git structure; no finding content is read.

1. An independent reviewer episode exists, from a **different model family** than
   the arms will use.
2. Its recorded timestamp **precedes** any Claude arm on that revision.
3. A candidate revision exists with an accepted predecessor baseline, so diffing
   against the baseline is a legitimate review route.
4. The candidate revision is **isolable by reachability**: descendants can be
   excluded from a snapshot, and the later accepted revision is verifiably absent.
5. The contract exists with a recorded integrity hash in the campaign record.
6. A **per-path attribution manifest** exists, so review packets can carry it. This
   is the E4 conflict-3 remedy: attribution was decidable from the receipt and the
   packet simply omitted it, which misattributed plan-mandated changes to builders
   across sixteen runs.
7. Leak checks at that revision: the contract mentions no findings, the tree
   contains no finding identifiers, and the metrics ledger at that revision excludes
   the slice.

Checks 3-7 were executed and recorded for S2 in
`claude-s2-review-independence-20261019-protocol.md` and are reusable verbatim.

**Candidates passing the existence precondition**, from a metadata scan of
`~/.local/state/work-engine`:

| campaign | review-named artifacts | candidate artifacts | plan |
|---|---|---|---|
| `rust-context-lifecycle-implementation-20261007` (S0) | 13 | 29 | `s0-plan-v1.md` |
| `rust-context-lifecycle-s1-20261007` | 14 | 47 | `s1-plan-v1.md` |
| `rust-context-lifecycle-s3-20261007` | 7 | 17 | `s3-plan-v1.md` |
| `rust-context-lifecycle-s67-20261008` | 45 | 44 | attribution plans v1-v5, no plan `.md` in listing |
| `rust-context-lifecycle-s23-20261007` | 2 | **0** | - |

S23 fails check 3 on its face: no candidate artifacts. The other four are
**not established** on checks 1-7 until the screen is actually run; the counts
above establish only that review and candidate artifacts exist under those names.

## Stage 1 - deep inspection

**Contaminating. Designate the reader before they start, and record it in the
trial protocol's §11 register.** One survivor at a time.

Judged on the finding set:

- **Substantive, not stylistic.** A key of naming and formatting findings scores
  nothing interesting.
- **Line-anchored**, with the decisive lines quoted.
- **Verifiable against source at that revision** - each finding re-established by
  reading, not accepted on the reviewer's authority.
- **Count**, and whether findings **span more than one mechanism class**. S2's four
  spanned concurrency, classification, ownership and registration order, which is
  why arms could lose one and keep others.
- **Verdict recorded**, so the verdict itself is scoreable as Test R scored it.

Output: suitable or unsuitable, plus - if suitable - the key written as **semantic
requirements, never artifact names**, per the trial protocol §5.

## Stage 2 - cross-vendor agreement

**Contaminating in both families.** Two independent readings of the same finding
set against source at that revision, one Claude and one Codex.

This **validates** that the findings are real and verifiable; it does not create
them, so it does not make the key either reader's judgment. Its purpose is to catch
a key containing an unverifiable or wrong entry before any arm is scored against it.

- Disagreement on an entry **downgrades it to secondary** and records both
  readings. It is not silently dropped, and it is not scored as primary recall.
- Agreement is recorded per finding, not as an aggregate.

## The cost of selection itself

| stage | cost | eligibility burned |
|---|---|---|
| 0 | cheap, mechanical | none |
| 1 | one reading of a finding set | one session, that subject |
| 2 | two readings | one session per family, that subject |

All of it belongs to the **experimental ledger**, not to either production workflow.

## Disqualifying on sight

No independent reviewer episode; findings recorded **after** a Claude arm saw the
revision; no isolable candidate revision; or a contract or tree that leaks the
findings at that revision.
