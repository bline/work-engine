# Pre-registered protocol: Claude S2 review-independence pilot, 2026-10-19

Frozen before execution. This is "Test R": a **ground-truthed** reviewer
independence test. Unlike the RP2 comprehension pilot, the reference here is not
my judgment - it is what a different model family actually found on this exact
tree, recorded at the time, with line-level evidence.

Written by the session that read Astra's findings, and therefore **disqualified
from being an arm**. Every arm runs in a cold `claude -p` process.

## The subject

| element | binding |
|---|---|
| candidate under review | `7fdf392f753e1bc270cff5834db8ab616cd4695c` - S2 candidate 2 |
| accepted predecessor baseline | `2bbc95c05d687de4320ad3ef216e4193ceafd8a0` - S3 accepted |
| contract | `s2-plan-v1.md`, mode 444, sha256 `98390bd9d1e48a387b7b9ca0f4e2af04f3476a690ce6fe37361c083bd681f002` |
| prompt | 62 lines, sha256 `27b666a40db97e857c4e3985484847cbf83c79fe3ef4c6f8ad5f3bb94dfc6da6` |
| declared boundary | 13 paths, stated in the prompt |
| subject size | 2,868 insertions over 13 files versus the baseline |
| original builder | `gpt-6-sol`/high, candidate attempt 2 of 5 |
| reference reviewer | `gpt-6-astra`/xhigh, `codex-adversarial-review`, generation 1 |

## Isolation, verified before freezing

Built by `git init` + `git fetch <repo> 7fdf392f` + `git checkout --detach
FETCH_HEAD`, so descendants are excluded by reachability. Verified **absent**
from the snapshot: `6bf17387` (C3), `709af7d9` (C4), `4071955` (C5 accepted),
`5beec3b1` (C5 candidate). Verified **present**: the baseline `2bbc95c0`, so
legitimate diffing works. `git log --all` returns 415 commits, all ancestors.

Leak checks, all clean: the contract contains zero mentions of findings; the C2
tree contains no `S2-C2-F*` string; `docs/ai-workflow-metrics.jsonl` at C2 holds
25 records, none of them this slice. The repair commits and the
`~/.local/state/work-engine` finding artifacts are outside the snapshot.

## Ground truth

Astra reported exactly **four** findings against this revision. Their substance,
recorded here so scoring cannot drift:

| id | sev | defect |
|---|---|---|
| F1 | high | Concurrent drains panic and poison the process registry. `close_and_drain` snapshots without serializing; the second caller unwraps a missing `get_mut`, panicking while holding the registry mutex. `process.rs:249-353` |
| F2 | high | Close timeliness classified by which timeout branch wins rather than a post-await deadline check, so a child exiting after the cutoff is labelled Resolved. Permits false proof-success evidence. `process.rs:284-299` |
| F3 | medium | Cancelling the task-drain waiter drops the only owned join handle, detaching the collector; a later drain then reports joined when no join occurred. `executor.rs:299-338` |
| F4 | medium | Tasks spawned at `:448`/`:479` but registered only at `:505`, violating the accepted registration-before-dispatch ownership boundary. `executor.rs` |

F5 and F6 from the campaign are **not** ground truth here: they were introduced
by the C3 and C4 repairs and do not exist at this revision.

## Scoring - fixed here, in the frozen text

Each finding an arm returns is classified by me, verified against source at the
cited lines:

- **MATCH** - the same defect and mechanism as one of F1-F4. Line agreement is
  not required; mechanism agreement is.
- **NOVEL-VERIFIED** - a real defect not in F1-F4 that I confirm in the source.
  Scored **positively**. It is evidence the reference reviewer was incomplete.
- **UNVERIFIED** - a claim I cannot confirm from the source. Recorded, not
  counted as a match, and not held against the arm beyond being reported.
- **MISS** - one of F1-F4 the arm did not report.

**Primary measure: recall against F1-F4.** Secondary: NOVEL-VERIFIED count,
UNVERIFIED count, and verdict.

**The verdict is scored.** The correct verdict at this revision is **BLOCK**;
the campaign's retained results record `blocked` at C3 and C4 and
`no_open_findings` only at C5. An arm returning ACCEPT has failed the primary
purpose of a review gate regardless of how many findings it lists, and that is
recorded as a gate failure rather than averaged into recall.

**Disqualifier:** a write to the snapshot, or a read outside the declared
boundary. Detected post-hoc by `git status --porcelain`, by scanning every
`tool_use` for `/home/bline/code/`, and from `permission_denials`.

## Honest limit on what recall means

Ground truth is **what Astra found**, not **all defects that exist**. Recall is
therefore measured against a reference reviewer, not against truth. This cuts
both ways and the protocol does not pretend otherwise: a miss may be a real
miss, and a NOVEL-VERIFIED finding is a result about the reference as much as
about the arm.

## Arms

- `claude-opus-5` - the reviewer-tier candidate, the role Astra/xhigh holds.
- `claude-sonnet-5-5` - the economy arm.

Invocation, with `--add-dir` for the contract (the defect that voided the first
RP2 attempt):

```
claude -p "$(cat ~/cbm-work/s2-review-pilot/prompt.txt)" \
  --model <arm> --permission-mode dontAsk \
  --add-dir /home/bline/cbm-work/s2-review-pilot/contract \
  --disallowedTools Write Edit NotebookEdit \
  --output-format stream-json --verbose
```

Working directory is the arm's own disposable copy of the snapshot. Ceilings per
arm: **$8.00** and **25 minutes**; exceeding either is recorded as exceeded.

## Confounds, recorded in advance

1. **Astra had more evidence than the arms get.** It operated inside the
   campaign's retained review state with an authority manifest, gate receipts
   and validation evidence. The arms get the plan, the boundary and the tree.
   This **disadvantages the arms**, and a miss must be read with that in mind.
2. **Effort is not comparable.** Astra ran at `xhigh`. The arms expose no
   equivalent knob through this invocation.
3. **Whether Astra could diff against the baseline is unknown** from the
   artifacts. The arms are told they may, which may advantage them.
4. **n = 1 per arm.** No variance estimate.
5. Unlike the RP2 contract, this subject has **no Claude-specific content**, so
   the self-referential asymmetry that favoured the arms in that pilot is absent
   here. This test is cleaner on that axis.

## Scope

This tests review capability on one frozen tree. It does not test implementation,
and it authorizes no Claude reviewer in any campaign, no change to S10, and no
alteration of the `codex-adversarial-review` evidence classes.
