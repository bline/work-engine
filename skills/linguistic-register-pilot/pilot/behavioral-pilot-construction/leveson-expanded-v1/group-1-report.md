# Expanded Leveson treatment — implementation group 1

## Outcome

The pre-outcome construction completed, but the treatment is **stopped before
role rendering**. The eight-source corpus, abstract profile, semantic ledger,
quotation audit, profile-card recognizability check, and shallow cue baseline
are all retained. Four of eight `realization_only` features cite two source
documents rather than the frozen minimum of three, so the overall retention
gate does not pass. The profile card is also uniquely identifiable by total
description length among the equal-feature-count comparators.

No behavioral task, scoring key, role rendering, or outcome was generated or
shown to either profile-construction model process.

## Corpus

The exact corpus contains eight sole-authored works and 73,794 normalized
tokens. It is balanced 4/4 between `formal_analysis` and
`practitioner_public`, with six genre classes:

| Source | Genre | Tokens | Exact public source |
| --- | --- | ---: | --- |
| Systems-theoretic software-intensive systems | Journal article | 17,116 | [MIT OCW PDF](https://ocw.mit.edu/courses/16-355j-software-engineering-concepts-fall-2005/afc60de2c951a9c44ef67ca8d37390c8_tdsc_final.pdf) |
| Role of software in spacecraft accidents | Journal article | 15,475 | [MIT OCW PDF](https://ocw.mit.edu/courses/16-355j-software-engineering-concepts-fall-2005/364e4d0c757e93e44abbc89758f2e69c_jsr.pdf) |
| Limitations of safety assurance | Technical white paper | 7,366 | [MIT PSASS PDF](https://psas.scripts.mit.edu/home/wp-content/uploads/2020/07/White-Paper-on-Safety-Assurance.pdf) |
| Safety-III in healthcare | Technical white paper | 15,831 | [MIT PSASS PDF](https://psas.scripts.mit.edu/home/papers/2026_Safety-III_A_Systems_Approach_to_Safety_.pdf) |
| Improving the risk matrix | Practitioner presentation | 1,953 | [MIT PSASS PDF](https://psas.scripts.mit.edu/home/wp-content/uploads/2019/04/WedMorning_Leveson_Improving-the-Risk-Matrix.pdf) |
| Workshop FAQ | Responsive presentation | 971 | [MIT PSASS PDF](https://psas.scripts.mit.edu/home/wp-content/uploads/2024/2024-06-05-1540__FAQ_Nancy.pdf) |
| Oil and gas testimony | Public testimony | 4,516 | [MIT Energy Initiative](https://energy.mit.edu/news/risk-management-in-the-oil-and-gas-industry/) |
| Software system safety notes | Teaching notes | 10,566 | [MIT OCW PDF](https://ocw.mit.edu/courses/16-355j-software-engineering-concepts-fall-2005/ccea41d41a1473d2341559dce5b01e4e_cnotes11.pdf) |

Every artifact matched the digest in `source-selection.yaml`. Public
readability and MIT hosting do not establish redistribution permission. Exact
source bytes, the source-bearing extraction packet, and source-bearing event
stream are retained outside Git in the machine-local content-addressed cache
`/home/bline/.cache/work-engine/linguistic-register/leveson-expanded-v1/`.

## Fresh gates

The two isolated Sol processes extracted 16 features and then classified them
against the frozen role: 8 `realization_only`, 7 `semantic_addition`, and 1
`uncertain`. Retained weight is 44.07%; seven retained features have evidence
across both genre families.

| Gate | Result |
| --- | --- |
| At least 5 retained features | Pass — 8 |
| At least 35% weighted retention | Pass — 44.07% |
| At least 4 cross-family retained features | Pass — 7 |
| At least 3 evidence documents for every retained feature | **Fail — minimum 2** |
| No material quotation survival | Pass |

Across all eight source comparisons, the longest exact normalized overlap with
any committed feature description, evidence observation, or classification
rationale was four tokens. There were zero exact matches of eight or more
tokens and zero near-verbatim matches under the frozen 12--30 token, 0.90
positional-identity rule.

The fresh profile-card classifier correctly assigned expanded Leveson and the
neutral card, but swapped Gelman and Shaw: 2/4 overall. Expanded Leveson thus
passes the narrowly preregistered same-family card-traceability check. All four
confidence values were 1 because the packet did not define a confidence scale;
confidence is unusable.

The shallow baseline found total card-description length sufficient to identify
expanded Leveson across all three order replicas (`word_count == 86`, or an
equivalent threshold). This is incidental card-construction evidence, not a
claim that length caused the model assignment.

## Group 2 dependency

Do not use this profile to author C0/C1/C2 roles yet. Group 2 needs a fresh,
pre-outcome decision on one of two bounded repairs:

1. obtain third-document evidence for the four already-frozen retained
   features without changing their wording, weight, or disposition; and
2. length-match the anonymous profile cards, then repeat the fresh card check
   and shallow baseline under a new packet digest.

If either repair is not authorized, Leveson remains provisional and Gelman
should be evaluated as the reserve. The current stopped record must remain
unchanged rather than being overwritten.
