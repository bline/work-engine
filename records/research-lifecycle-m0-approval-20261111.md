# Research lifecycle M0 — owner approval

Status: **M0 accepted; design gate closed**. Recorded from the user's “Approved”
reply in Codex session `01a1ccc5-012b-7111-9ec6-21fc5648f678` at
`2026-11-12T00:29:50.625Z` (2026-11-11 in America/Denver).
This accepts the exact candidate requested in the preceding approval question.

## Accepted subject

[Contract and H2 first-use packet, M0 candidate 2](../planning/research-lifecycle-m0-contract-and-first-use-20261111.md).
The dated packet is preserved byte-for-byte; this later record establishes its
accepted standing within the scope below. Its earlier pending labels describe
its standing before this approval.

| Binding | Value |
| --- | --- |
| Private checkpoint commit | `fbcb53e4fdb96f83830deb185f2f95aab3c3a50c` |
| Tree | `3da295491fcab8b57a58edc774bbc1fac23af2ed` |
| Task patch digest | `47593e327412cb85d618318211d34450afe98e724e6f3ee03c62363bdf90d352` |
| Primary file SHA-256 | `4450ee6448f3b855ffe73514aad48f7036b3f17c0eb8b0cfb97b0ef8d2387dda` |
| Plan / scope | `M0-candidate-2` / `M0-LC1-7-H2` |
| Validation receipt digest | `cddf90120996b7b5078ee07892c93a222767fbb6ea9bf4d2da69c6752715e8c0` |

## Scope of approval

The owner accepts LC-1 through LC-7, the selected H2 revalidation use and the
limited exploratory baseline. H2's first execution remains a bounded repository
analysis producing a proposed wording correction or supported no-change report.
The approval does not establish that H2 has already been revalidated.

The owner accepts the scoped A4 dispositions in the packet's §9:

- D-2 confirms the quoted pending notice and per-record R/X substitution.
- D-3 accepts the bounded Rust current-reading contract and joined research bundle.
- D-6 remains approved: invalid register produces no card. D-7 retains in-process
  anchors against the same pinned snapshot.
- D-8b/c/e/f adopt the proposed current-reading grouping, notices, historical
  marker and decision-listed finding semantics.
- D-8a/d/g/h/i remain outside this interface; historical requests are explicitly
  unsupported rather than approximated.
- Q-R2-2 is settled for this profile only: an unestablished/null required anchor
  yields no affected card and a typed gap. Null never establishes absence.
  Broader rename and partial-card semantics remain open.

The baseline is ordinary policy context without the added qualified research
bundle. Its existing research-derived text and the limited inference from one
pair remain explicit. This is acceptance of that comparison design, not evidence
of research benefit.

## Remaining boundaries

M1 is the next separately scoped implementation step. This approval creates no
implementation or live execution grant, paid-run budget, register publication,
research qualification or subject-specific applicability authorization. M3/M4
still need their packet inputs and prerequisite qualification. Q-B1, independent
R-2 acceptance and integration/distribution clearance remain unresolved. It does
not accept every field or behavior of the parent v0.2 proposal.

M0-ADV-01's three initial P2 findings were resolved in the retained review of
this exact subject, with no new actionable P1/P2 regression found in that delta.
This advisory review remains distinct from the owner's acceptance. No executable
code changed or runtime checks ran as part of recording this approval.

## Authority evidence and custody

The approval request is rollout line `973`,
event SHA-256 `9501f4654758f3b83796e2edefcd7e6124d0d383f25a7ada11be9e19ee9ba40b`.
The exact user reply is rollout line `980`,
event SHA-256 `49c8099e60d209a85ca7fe5cec26a7cc9f31ce5958d2144b2d9bf29b96582c3a`.
Both are in:
`/home/bline/.codex/sessions/2026/11/11/rollout-2026-11-11T15-40-25-01a1ccc5-012b-7111-9ec6-21fc5648f678.jsonl`.

Host-local exact events, assignment, validation, checkpoint receipts and review
reports are retained under
`/home/bline/cbm-work/research-lifecycle-m0-20261111/`, including
`owner-approval-binding.json`, `validation-final.json` and `review-turn-2.md`.
The hashes identify evidence; they do not replace the user's authority.
