# Bootstrap Review Subject

Status: provisional bootstrap evidence; not a canonical review-artifact schema,
proposal decision, placement decision, roadmap mutation, or implementation
authorization.

## Immutable subject

- Repository: `work-engine`
- Commit: `5367d9a12ed5ad72fd130e1ca918eadacd129eca`
- Tree: `5b8197b72a7ecfe6a431e2c1e2da3519bb7391fe`
- Proposal family: `work-engine.adaptive-specialized-review`
- Proposal IDs:
  - `work-engine.revision-bound-review-artifacts`
  - `work-engine.adaptive-review-panel-coordination`

Reviewers read an exported snapshot at that commit. Live working-tree content,
including concurrent work under `ideas/`, was outside the evidence boundary.
The snapshot had no Git metadata, so reviewers independently reproduced file
digests but treated the supplied commit/tree association as parent-provided
provenance.

## Bound content

| Path | SHA-256 |
| --- | --- |
| `proposals/adaptive-specialized-review/family.md` | `f217651ba3f1e2357729657c1fe57d174faedbb55632c3e08b6b534a76d72073` |
| `proposals/adaptive-specialized-review/revision-bound-review-artifacts/packet.json` | `0918ac876cc40094a4949801d44dd8cf08758fcef6ed0dcba7712a8e982bf646` |
| `proposals/adaptive-specialized-review/revision-bound-review-artifacts/proposal.md` | `0cea1829192a99e8bef82367eee3c6b6df6d0cccd1427cfdf431740109b7b2e9` |
| `proposals/adaptive-specialized-review/revision-bound-review-artifacts/placement.md` | `643704c1c544f8fd666e1c5b967122dfbf9a958e1b0f611a61cf9fa90ee4b581` |
| `proposals/adaptive-specialized-review/revision-bound-review-artifacts/relationships.md` | `c1e6c8466d7b136fd208f5efc71ba58c8b83bb75492c3aafe5b331e86a6fc1d9` |
| `proposals/adaptive-specialized-review/adaptive-review-panel-coordination/packet.json` | `0e9d75afe7a3edcc87b7ffc3f4b861bbffd24a3d54482cdb08802fbcfed1450b` |
| `proposals/adaptive-specialized-review/adaptive-review-panel-coordination/proposal.md` | `b61d7edb57db62a8aef968c2dc0b40e53888aafad1e2fc951318629325336824` |
| `proposals/adaptive-specialized-review/adaptive-review-panel-coordination/placement.md` | `d3e06c344c7682598a7284133a44979701518c7ac257bbf95b73d5b9e8f95487` |
| `proposals/adaptive-specialized-review/adaptive-review-panel-coordination/relationships.md` | `7d4a54d1fe6a36208f0065c53c7c175e66627ccaebdee9ff87d8693b46559946` |

## Panel and provenance

The coordinator selected architecture/placement, doctrine/authority, and
lifecycle/evidence perspectives because each could change the proposal
decision. UI, accessibility, security, and migration review were omitted because
the bound proposals create no current consequence in those dimensions.

Each specialist received the immutable subject and governing doctrine without
the other specialists' conclusions. All three were fresh delegated Codex
contexts; this establishes context separation, not provider or statistical
independence. The architecture reviewer additionally used one read-only Claude
Sonnet 5 challenge (one successful call, 99.7 seconds, reported cost
`$0.6898425`) as evidence, while retaining responsibility for its own judgment.

Repository evidence used Codebase Memory Tier 2 verification for project
`home-bline-code-work-engine`, generation `2026-08-22T19:33:45Z`, reported
complete at the subject commit with no recorded gaps for the relied proposal,
design, philosophy, and review scopes. Clean coverage is not proof of semantic
completeness. Mechanical proposal-packet validation succeeded for seven packets
and establishes only shape and reference validity.
