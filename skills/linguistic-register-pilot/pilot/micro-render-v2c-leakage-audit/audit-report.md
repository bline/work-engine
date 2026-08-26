# Micro-render v2c read-only leakage audit

## Result

No direct forbidden leakage or matcher-visible transport cue was found in the retained v2c subject. Exact corpus-quotation coverage remains unresolved because the bound checkpoint contains profile digests and screening claims but not the source-corpus bytes.

The audit therefore assigns `unresolved_exact_corpus_quotation_coverage`, not `clean` and not `contaminated`. The same-family profile-traceability result is retained with that explicit qualification.

## Bound subject

- Checkpoint: `62cc7f7039cfc6238de8cc7d16a030ebe1892bf2`
- Tree: `3b4a3eb525636dbe19eaafaffaece8bb4c6153bf`
- Inventory: 893 files; every exported file path and Git blob object matched the frozen inventory.
- Audit preregistration checkpoint: `7fcbcfc501d2647771cbc295538bcb219b945ecc`

Evidence was read only from an exact checkpoint export. No v2c artifact was changed, regenerated, retried, normalized, adjudicated, or rematched.

## Matcher packet reconstruction

All three retained matcher packets reconstructed exactly and matched their key, launch-manifest, attempt-marker, receipt, and result digests:

- pass 1: `fa6b9d9a71e848745852e9b3d5384fd850357e35a8e2bff18580bb2d5d4bb49a`
- pass 2: `e444ed3b8ab438a431507a15e2dd8618a0aacdd060d243f913af76cdb93064c1`
- pass 3: `1e7002cc51d5409d86662dd500a0192cbfd57c53e231f37632cc91d209ab7e3b`

Each anonymous text contained exactly `text_id` and normalized `text`; each reference contained exactly `reference_id` and `licensed_style_features`. No packet contained wrapper disposition, normalization, transport metadata, condition identifiers, answer mappings, or original `X…` sample IDs. The event streams contained no matching key or condition material.

This mechanically falsifies wrapper disposition as a direct cue available in the retained matcher packets.

## Wrapper and incidental-feature audit

Wrapper distribution recomputed exactly:

| Condition | Direct | Unwrapped once |
|---|---:|---:|
| Gelman | 3 | 1 |
| Leveson | 0 | 4 |
| Neutral | 2 | 2 |
| Shaw | 3 | 1 |

The Leveson association is strong transport-compliance metadata, but the information did not survive into matcher-visible bytes.

No author name, profile/condition label, frozen source-domain phrase, or condition identifier appeared in any normalized render. No single scalar metric among word count, sentence count, paragraph count, headings, list markers, or punctuation had pairwise-disjoint observed values across all four conditions.

Question-mark count did distinguish Shaw one-versus-rest: every Shaw render had 3–5 question marks and every other render had zero. This is an intended manipulation signal, not leakage, because the frozen Shaw feature explicitly licenses a parallel series of guiding questions. The audit does not treat licensed syntax, rhythm, or punctuation as incidental contamination.

No candidate render shared a normalized sequence of three or more words with its full retained profile feature descriptions. Six of sixteen renders shared such a sequence with the shorter licensed matcher style card: three Leveson and three neutral renders. These are intended profile/style-card traceability cues; none contained an author or source-domain clue.

## Global skill exposure

All sixteen render event streams were inspected.

Two invocations recorded an external skill read:

| Sample | Condition | Skill | Event-recovered SHA-256 |
|---|---|---|---|
| `X1832FB59B14B` | Shaw | `/home/bline/.codex/skills/codebase-memory/SKILL.md` | `8efe4aa10efa92b52148cce5840abac552b92e5dc3af567e5d167a5e33624db1` |
| `X9C9FA0A475B3` | Neutral | `/home/bline/.codex/skills/repo-search/SKILL.md` | `cf72505d1850e757f6cc6ef3331d0f66ee4c5bf51b52a527c27fe8b78c17a949` |

The complete historical contents recovered from the events are retained in `audit-result.json`. Neither skill contains author names, profile labels, candidate-domain language, or condition material. Both contain generic repository-evidence instructions and generic epistemic words such as “evidence” or “claim.” Exposure was unequal but condition-neutral and occurred in only two different conditions, so it cannot supply a common four-way candidate discriminator.

The event stream directly establishes that the command completed and records the returned bytes. It does not establish mechanistic attention or use; that effect remains unresolved.

## All-rationale audit

All 48 assignments were coded under the frozen multi-label taxonomy:

| Category | Assignments |
|---|---:|
| Intended register features | 48 |
| Exact 3+ word overlap with supplied profile/style-card language | 33 |
| Length or formatting observations | 22 |
| Domain or author clues | 0 |
| Structural artifacts | 0 |
| Unsupported confidence | 0 |

Every rationale identifies an observable textual cue. None cites wrapper state, metadata, IDs, ordering, author, domain, or condition. This supports the intended-signal interpretation but does not prove that incidental cues were mechanistically unused.

Confidence was uniformly 5.0. It is retained as an uncalibrated, failed secondary metric with no useful variance—not as leakage evidence.

## Unresolved boundary

The checkpoint contains candidate profiles whose screening metadata says names, copied language, and domain terms were removed. It does not contain the digest-bound source corpus bytes. Therefore the audit cannot independently search those sources for exact five-word quotation recurrence. Absence of direct forbidden literals and source-domain clues lowers the concern but does not resolve that exact-quotation claim.

Resolving this boundary would require a separately bound, read-only source-corpus audit against the exact corpus artifacts; it does not require rerendering or changing v2c.
