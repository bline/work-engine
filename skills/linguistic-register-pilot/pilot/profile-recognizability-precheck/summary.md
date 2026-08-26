# Profile-recognizability pre-check

The frozen pre-check passed on 2026-08-25. A fresh `gpt-5.6-sol` classifier,
running in an isolated temporary directory with only the blinded packet and
output schema, correctly identified the neutral card. All three retained
profiles passed the preregistered minimum scores for internal coherence,
non-neutral distinctiveness, and separation from the neutral decoy.

| Candidate | Coherence | Non-neutral distinctiveness | Separation from neutral | Result |
| --- | ---: | ---: | ---: | --- |
| Gelman model criticism | 5 | 5 | 5 | recognizable |
| Leveson system safety | 5 | 5 | 5 | recognizable |
| Shaw engineering judgment | 5 | 5 | 5 | recognizable |

This result does not rank or select the candidates. In particular, it does not
resolve the same-controversy dependence in the two-source Gelman mini-corpus.
The mini-corpora remain well below the pilot plan's preferred 6–12 documents or
50,000–200,000 source tokens. A second classifier perspective can test
cross-family agreement later, but neither profile-card pass substitutes for
the section 9.5 manipulation check on actual rendered C0/C1/C2 artifacts.

The raw classifier output, normalized result, execution event log, blinding
key, and digest-bound mechanical report are retained beside this summary.
The initial pass was superseded after adversarial review found that the scorer
did not reconstruct and verify the blinding key and that the packet's authority
label overstated the pre-check as a manipulation check. Its artifacts remain
under `superseded/pre-remediation-pass/`; the current pass used the corrected
packet in a new fresh classifier context.
