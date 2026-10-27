# Disposition of the external review of the repository layout plan

- Subject: [`subject.md`](subject.md), revision 1 of
  `planning/repository-layout-plan.md`, SHA-256
  `38ecc141b3041a73d4db744d3e464299f60a1e4226f9cd54fafd6a25ac6abb40`.
- Review: [`chatgpt-review.md`](chatgpt-review.md), read against the GitHub
  branch at `ea65c2d0`.
- Disposition by Claude session `4b0947de-6afa-41b4-aade-862183380127`,
  2026-10-27. Each finding was checked against the local tree at `df288c3d`
  with its working tree, before this disposition was written. The user asked
  for the revision; the result is revision 2 of
  [the plan](../../../../planning/repository-layout-plan.md).
- This disposition accepts no phase of the plan.

## Findings

| # | Finding | Disposition | Local verification |
| --- | --- | --- | --- |
| 1 | Remote not at the plan's snapshot | **Partly out of date** | `df288c3d`, pushed 2026-10-27 16:31 -0600, contains `415ceda2` and `skills/skill-release/`; the review saw the 15:40 push. `docs/rust-development.md` and the 8 untracked records are still local only, so that part stands. |
| 2 (P1) | Partial phase-4 move breaks more links than stated | **Accepted; the defect is larger than reported** | Simulating the exact 53-file set gives 39 newly broken links: 6 in digest-locked records, 13 in rewritable moved records, 12 in owner-held deferred records, 8 in other files. The review's examples are confirmed line by line. |
| 3 (P1) | Digest identity is not path resolution | **Accepted** | No code consumes `design-sources.json`; only documents mention it. It declares `baseline.repository_head` `7583e209`, so its paths resolve at that commit. Revision 1's done-check ("cited digests verify at their new paths") conflated the two. |
| 4 (P2) | A dated note was listed for rewriting | **Accepted** | `notes/rust-context-lifecycle-slice-state-20261014.md` states it was written at HEAD `cfafee4` and carries no authority. The inbound set also held a dated assessment (`DESIGN_EVIDENCE.md`) and an owner-held untracked plan (`BOUNDED_INFERENCE_IMPLEMENTATION_PLAN.md`). |
| 5 (P2) | Phase 5 needs an authority and provenance boundary | **Accepted** | `status-grammar.md` says the views deliberately mix maturity levels. The views hold 26 `owner:`/`source:` lines in 10 files, including `owner:` paths into `app-server/ideas/pending/`. |
| 6 (P2) | Catalog statuses mix dimensions; release validation | **Accepted** | Revision 1's status list mixed activity, kind and validity. Of the requested validation, declared-against-actual (`status`, `not_linked`) and orphan handling (`plan` and `deploy`) already exist in `skill_release.py`, which the review could not see; idempotency is not asserted by any test. Release state at the time: `415ceda2` current, no undeployed changes, all 38 links resolving. |
| 7 | Phase-4 outcome overstated; phase-1 done-check | **Accepted** | Phase 1 creates 3 files and updates 2; revision 1 said "four files". |

## Errors in revision 1, stated as corrections

- **Phase-4 link count.**
  - *Wrong claim:* a byte-identical move of the phase-4 set leaves 3 broken
    links.
  - *Error class:* the simulation moved all 64 records, not the 53 the phase
    actually moved.
  - *Consequence:* the phase would have broken 39 links, including 12 inside
    files the plan forbids editing, while its done-check expected 3.
  - *Check that catches it:* simulate the phase's exact move set. This is now
    invariant 7.
- **Inbound rewrite set.**
  - *Wrong claim:* the six files linking into the move set were live files to
    rewrite.
  - *Error class:* files were selected by whether they link into the move set,
    not classified by their own status text.
  - *Consequence:* a dated observation note would have been rewritten, against
    the plan's own invariant 3.
  - *Check that catches it:* classify each referencing file from its status
    text first. This is now the reference-classes section.
- **Digest done-check.**
  - *Wrong claim:* matching digests at new paths verifies the cited files.
  - *Error class:* content identity was treated as path resolution.
  - *Consequence:* a path-bound consumer resolving at HEAD would have failed
    unseen.
  - *Check that catches it:* establish each consumer's resolution basis. This
    is now the three reference classes.

## Changes in revision 2

1. Phase 4 runs as one wave of 62, or 64 with the `lifecycle-proof` change, after
   the owners commit. Simulated: 25 or 32 newly broken links, of which only 3
   are unrepairable. Those 3 are named exceptions inside digest-locked records.
   The 53-file move is withdrawn.
2. Three reference classes (live path, historical, content digest), each with
   its resolution and treatment. Every phase produces a before/after
   resolution report over links, path mentions and structured path fields.
3. Inbound files are classified one by one. Records are left as written, and
   `records/README.md` maps old to new paths.
4. "Placement is not acceptance" is a principle. Phase 5 is direction only, with
   a reviewed inventory, including structured path fields, required before
   execution.
5. The catalog separates `kind` and `state`, adds four install rules with
   reasons (including `not_installed_reason` and `install_reason`), and lists
   the release validation: existing checks, plus an idempotency test and rule
   checks to add.
6. The phase-4 outcome is narrowed to the Rust records, and the phase-1
   done-check covers the three new files and both entry-point updates.
