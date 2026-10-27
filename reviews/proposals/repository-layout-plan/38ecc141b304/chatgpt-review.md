# External review of the repository layout plan (ChatGPT)

- Subject: [`subject.md`](subject.md), revision 1 of
  `planning/repository-layout-plan.md`, SHA-256
  `38ecc141b3041a73d4db744d3e464299f60a1e4226f9cd54fafd6a25ac6abb40`.
- Reviewer: ChatGPT, reading the GitHub branch `rescue-detached-20260911` at
  `ea65c2d0` (the push of 2026-10-27 15:40 -0600). It could not see the local
  working tree, uncommitted files, or `415ceda2`, which reached the remote in
  the 16:31 push.
- Pasted into Claude session `4b0947de-6afa-41b4-aade-862183380127` by the user
  on 2026-10-27. Reproduced verbatim below the rule; tracking parameters in
  links are left as received.
- Disposition: [`disposition.md`](disposition.md).
- `subject.md` is byte-identical to the reviewed revision, so its relative
  links are written from `planning/` and resolve from there, not from this
  directory.

---

# Work Engine repository cleanup — review

Conditional approval — revisions needed

I'm reviewing the plan against [bline/work-engine · rescue-detached-20260911](https://github.com/bline/work-engine/tree/rescue-detached-20260911?utm_source=chatgpt.com).

The overall direction is sound. Organizing by authority and document lifetime rather than subject is the right architectural choice, particularly for a repository where agents must distinguish binding rules, current design, historical evidence, and implementation state.

The phased approach is also sensible: establish placement rules before relocating documents, preserve digest-bound evidence, and avoid disturbing other sessions' uncommitted work.

## 1. Verification limitation: GitHub is not at the plan's exact snapshot

The remote rescue branch currently points to `ea65c2d0`, while the plan describes measurements at `415ceda2`. GitHub cannot resolve that latter commit.

More importantly, the remote branch does not yet contain `skills/skill-release/` or `docs/rust-development.md`, both of which the plan treats as existing. This suggests the planning session has local changes not represented in the pushed branch.

The plan's 56 tracked `docs/rust-*` records are consistent with what GitHub shows. The additional eight untracked records, along with the other local working-tree state, cannot be independently inspected through GitHub.

Consequence: I can validate the structural design and many code dependencies against the rescue branch, but cannot certify the exact move set or all claimed preconditions until the local and remote snapshots are reconciled. I would not reject the plan merely because those unpushed changes are absent.

## 2. P1 — Phase 4 undercounts broken links

I found a concrete defect in the proposed 53-file move.

The plan evaluates relative links largely as though all Rust records move together, but it explicitly leaves nine owner-held records and two code-dependent records in `docs/`. That creates additional broken links between documents moved now and documents retained temporarily.

Verified examples:

| Source                                 | Line   | Target                                 | What happens               |
| -------------------------------------- | ------ | -------------------------------------- | -------------------------- |
| `rust-compiler-c3-plan.md`             | 11     | `rust-compiler-implementation-plan.md` | Source moves; target waits |
| `rust-compiler-c1-result.md`           | 3      | `rust-compiler-implementation-plan.md` | Source moves; target waits |
| `rust-compiler-implementation-plan.md` | 9, 333 | `rust-migration-plan.md`               | Source waits; target moves |

These are four additional broken link occurrences beyond the three acknowledged for immutable documents, if the files move in the proposed groups.

The first example is visible in [rust-compiler-c3-plan.md](https://github.com/bline/work-engine/blob/rescue-detached-20260911/docs/rust-compiler-c3-plan.md?utm_source=chatgpt.com).

This also exposes a contradiction between invariant 1, which requires no new broken live relative links, and Phase 4's completion criterion, which allows known breakage.

Required change: Build the actual phase-specific dependency graph, including references in both directions between moving and deferred files. Either move interdependent files together when their owners release them, or establish explicitly justified historical-link exceptions. Don't certify Phase 4 from the eventual all-files-moved graph.

## 3. P1 — Digest preservation needs path-aware verification

The plan correctly protects documents with cited SHA-256 values, but byte identity is only part of the requirement.

I verified that [design-sources.json](https://github.com/bline/work-engine/blob/rescue-detached-20260911/experiments/context-lifecycle-rust/design-sources.json?utm_source=chatgpt.com) records both a repository-relative path and a SHA-256 for `docs/rust-generation-integration-plan.md`.

After moving the file to `records/`, its bytes can remain identical while the original path no longer resolves at HEAD.

This is not necessarily a historical-evidence failure if the manifest is resolved against its original commit. But that resolution behavior needs to be established, not assumed.

Required change: distinguish three kinds of references in the verification procedure: current-path references, historical references resolved at a specific commit, and content digests independent of location.

A successful SHA-256 comparison should not, by itself, certify all three.

## 4. P2 — Phase 4 proposes modifying a historical observation

The plan lists `notes/rust-context-lifecycle-slice-state-20261014.md` among the files whose references should be rewritten.

I inspected the [actual note](https://github.com/bline/work-engine/blob/rescue-detached-20260911/notes/rust-context-lifecycle-slice-state-20261014.md?utm_source=chatgpt.com). Its introduction explicitly describes an observation made at a particular repository HEAD, says it carries no authority, and warns that it will age.

That is precisely the kind of historical record invariant 3 says should remain unchanged.

Required change: remove this note from automatic reference rewriting. Its links should resolve in their historical context, or the cleanup should produce a separate navigation/index record. If it is deliberately reclassified as a living document, record that decision first.

The same classification check should apply to the dated experiment documents in the inbound rewrite set.

## 5. P2 — Phase 5 needs an explicit authority and provenance boundary

The current design corpus is more nuanced than simply current versus historical.

For example, [architecture/status-grammar.md](https://github.com/bline/work-engine/blob/rescue-detached-20260911/app-server/docs/architecture/status-grammar.md?utm_source=chatgpt.com) explicitly permits accepted design, proposed material, and unresolved hypotheses to coexist in the canonical architecture views, differentiated by status.

Moving those views into `design/` should not imply that every claim inside them is accepted.

I recommend two additions:

- `design/README.md` must say that placement identifies the design corpus, not the authorization or acceptance status of its contents. Existing per-claim status remains authoritative.
- Phase 5 needs an inventory of structured paths, not just Markdown links. I verified `owner:` and `source:` path fields in the architecture documents, including `authority-and-ownership.md`. Some are historical provenance; others describe current ownership. Blind rewriting would change their meaning.

There is a related distinction in the ideas merger. `ideas/README.md` describes an active reconciled projection, while `app-server/ideas/` contains pending, historical, and retired material. Those should not all become equivalent current-design documents merely because they share a destination directory.

I agree with the plan's decision to treat this reconciliation as a future design decision. Phase 5 is not ready for execution yet, but its direction is reasonable.

## 6. P2 — Skills catalog needs release-contract validation

I verified the remote branch contains 30 installable-skill directories with `SKILL.md`, plus `skills/planning/` without a `SKILL.md`. The latter contains the YAML stub described in the plan.

The proposed catalog is useful, particularly because it gives the release tool a single declared inventory.

However, `active`, `stable`, `research`, and `other-project` are not all the same kind of classification. Activity, intended scope, and installation eligibility can overlap.

You don't necessarily need a more complicated schema. You do need explicit rules for how status and install targets interact, including intentionally uninstalled active skills such as `coordination-channels`.

The release validation should check declared targets against actual installed artifacts, identify orphaned installations, and prove a repeated install is idempotent.

Because the remote rescue branch lacks the Phase 0 skill-release implementation, I couldn't independently review the proposed `skill_release.py` schema change.

## 7. Other corrections

Two smaller issues are worth fixing before accepting the final plan.

Phase 4 overstates its outcome. It says `docs/` will contain only durable references, but the out-of-scope section explicitly leaves other historical records there. A more accurate outcome is that the Rust campaign-record population has been migrated, with other cleanup deferred.

Phase 1's completion criterion is incomplete. It says four files must exist, but the described work creates three files and updates two existing files. Completion should verify both the new documents and the two entry-point changes.

## 8. What I verified positively

Several of the plan's safeguards are grounded in real repository behavior:

- The lifecycle-proof tests directly read `docs/rust-retained-task-host-contract.md` and `docs/rust-context-lifecycle-s8-implementation-plan.md`. Moving those with their code changes is the correct rule.
- Both Rust historical-evidence components contain the literal `docs/ai-workflow-metrics.jsonl` path at the lines identified in the plan. Keeping that path fixed is justified.
- The 56 tracked Rust campaign documents on the remote branch match the plan's tracked move/defer categories.
- The existing `ideas/` reconciliation map and separate app-server idea-status grammar confirm that merging the two idea corpora requires semantic reconciliation rather than a directory rename.

I have not run a full repository-wide link checker or the Rust tests. That requires the actual local tree and uncommitted files.

## Recommended phase decisions

| Phase               | Recommendation                                                      |
| ------------------- | ------------------------------------------------------------------- |
| 1 — Placement rules | Approve direction                                                   |
| 2 — Skills catalog  | Conditional — validate release behavior                             |
| 3 — Root clutter    | Approve direction — verify config consumers                         |
| 4 — Rust records    | Hold — correct partial-move links and evidence rules                |
| 5 — Current design  | Design approval only — separate classification and authority review |

### Final verdict

The repository layout strategy is worth keeping. I would not restart the planning exercise.

The central weakness is not the directory taxonomy. It's that the migration invariants are currently stronger than the proposed verification procedure, particularly when files move in different waves while other agents retain ownership.

I would make one focused revision: require an explicit per-phase move set, a before/after reference-resolution report that distinguishes live from historical references, and evidence that every digest- or path-bound consumer remains valid.

With that correction, Phase 4 becomes a verifiable migration rather than an inferred-safe cleanup. Phase 5 should then proceed under its own reviewed inventory.

That preserves the plan's central virtue: the cleanup should improve an agent's ability to establish what is authoritative, without changing the authority or historical meaning of the documents being organized.
