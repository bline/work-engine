# Provisional Profile-Separability Comparison

**Status:** Awaiting independent recognizability check and human profile audit  
**Role:** `agent-instruction-review`  
**Role artifact:** `1ca5b13fa66e53381f0980dcd87db8f8b8f831140aba441fd66de096f0265c94`

## Mechanical gate results

| Candidate | Retained features | Cross-genre retained | Weighted retention | Excluded semantic-addition weight | Mechanical disposition |
| --- | ---: | ---: | ---: | ---: | --- |
| Gelman model criticism | 6 of 9 | 6 | 58.82% | 14 of 34 | `candidate_viable` |
| Shaw engineering judgment | 5 of 9 | 5 | 48.57% | 18 of 35 | `candidate_viable` |
| Leveson system safety | 5 of 9 | 5 | 47.06% | 18 of 34 | `candidate_viable` |

All three candidates exceed the preregistered minimums of four retained
features, 35% salience-weighted retention, and three cross-genre retained
features. This means each has enough provisionally realization-only material to
enter a recognizability check. It does not establish that a classifier can
identify the register or that the semantic classifications are correct.

## Interpretation

The Gelman package currently has the strongest separability signal. Its retained
material spans symmetric contrast, attributed epistemic posture, pointwise
response structure, scoped agreement, informal cadence, and compact summary.
Its principal limitation is that the two works are one article-and-rejoinder
episode, so recurrence may reflect that discussion rather than a broader stable
register.

The Shaw package has the closest technical-evaluation vocabulary, but much of
its most distinctive organization is substantive: classifying questions,
results, validation, abstraction level, and maturity. Its retained profile is
more restrained and may be harder to distinguish from a well-written neutral
baseline.

The Leveson package has the strongest independent-review posture, but more than
half of its salience weight is substantive analytic method: boundary expansion,
causal tracing, counterexample selection, and category construction. Its
remaining direct and contrastive realization is still testable, but semantic
attenuation is material.

## Advisory next step

Advance all three retained-only feature cards into one blinded recognizability
check before selecting the full corpus. The check should compare each card with
a neutral decoy after removing candidate labels, source titles, domain nouns,
quotations, and excluded features. Use a separate classifier and preserve raw
scores and provenance.

If budget requires one candidate first, use Gelman for the recognizability
mechanics calibration because it has the largest retained signal. Do not treat
that provisional lead as final corpus selection until a second genre pair or an
independent classifier shows that the distinction is not episode-specific.

## Limitations

- Extraction, weighting, and semantic classification were performed by one
  Codex context and await human audit.
- No independent classifier was available in this slice.
- Two works per candidate test feasibility, not community-wide register
  stability.
- Public readability was verified, but no selected work states a general reuse
  license; source bytes are therefore not stored in the repository.

