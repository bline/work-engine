# Knowledge Oracle S0: outcomes, safety and cost per correct resolution

**Status:** analysis requested by the user on 2026-11-08 (second decision,
item 6). Committed on 2026-11-08 with the user's approval, as a
retrospective.

**Basis:**

- the accepted record `metrics/probes/knowledge-oracle-s0-result-20261108.md`
  (commit `daad4f2b`);
- the frozen `results/analysis.json` (sha256 `95afe8f9…`), which holds the
  primary scores;
- the user's B/M5 adjudication `private/scores/adjudication-user-20261108.json`
  (sha256 `d7d2e040…`);
- the round-3 routing-harness re-review
  `~/cbm-work/routing-harness-v2-20261108/qualification/report-round3.md`
  (sha256 `22e9fe35…`).

**Sample size:** n = 12 items per arm, and n = 1 run per item and arm. A
difference of one item is within noise throughout.

## 1. Outcomes, kept separate

| Arm | Fully correct | Partial | Incorrect | False confident reuse (of 6 items) | False invalidation (of 6 items) |
|---|---|---|---|---|---|
| A, unassisted | 8 | 2 | 2 | 1 (M4) | 0 |
| B, existing claims readers | 8 | 3 | 1 | 1 (M5, as adjudicated) | 0 |
| C, curated projections | 12 | 0 | 0 | 0 | 0 |

**Denominators:**

- **False confident reuse** can occur only where the key's conclusion is not
  plain support. That is 6 items: E4, E5, E6, E7, M4 and M5.
- **False invalidation** can occur only where the conclusion is supported.
  That is the other 6: E1, E2, E3, E8, M1 and M6.

**By item type:**

| Arm | Explicit questions, fully correct (of 8) | Embedded needs, fully correct (of 4) | False confident reuse |
|---|---|---|---|
| A | 6 | 2 | 1, embedded |
| B | 7 | 1 | 1, embedded |
| C | 8 | 4 | 0 |

Both false-confident-reuse occurrences are on embedded tasks, and both involve
the same corrected evidence (the K5 handoff figures). This is n = 4 embedded
tasks per arm.

**Scoring disagreements, all preserved:**

- **B/M5:**
  - the primary scorer recorded it partial, without false confident reuse;
  - the independent reviewer scored it incorrect, with false confident reuse;
  - the user adopted the reviewer's score.

  Both scores are kept.
- **Second scorer (9 random runs):**
  - C/M1 rated partial (the primary scorer said correct);
  - B/M6 rated correct (the primary scorer said partial).

  The primary scores stand.
- **False-confident-reuse agreement:** 9 of 9, but none of the 9 runs
  contained an occurrence, so agreement on that judgment is untested.
- **Who scored:** all scoring was done by Claude models.

## 2. Total inference cost per correct resolution

**Definition:** all model inference spent producing an arm's answers. That is
the run costs, plus, for C, the curator who prepared the projections.

- **Excluded:** the key, scoring and review (evaluation costs), and the
  routing benchmark (it was not part of any arm).
- **Run costs:** API-equivalent USD at the usage ledger's standard pricing.
- **Curator cost:** the CLI's list-price estimate. Whether the two share a
  pricing basis is `not established`.

| Arm | Runs | Preparation | Total | Fully correct | Per correct | If the second scorer's reading is applied |
|---|---|---|---|---|---|---|
| A | 1.504 | none | 1.504 | 8 | **0.188** | unchanged (no A run was in dispute) |
| B | 1.393 | none | 1.393 | 8 | **0.174** | 0.155 (9 correct, with B/M6 correct) |
| C | 0.802 | 2.164 | 2.966 | 12 | **0.247** | 0.270 (11 correct, with C/M1 partial) |

- **C without preparation:** 0.067 per correct. That figure is reported for
  information only; it is not the pre-registered measure.
- **Per projection:** the curator averaged 0.180. The per-projection split is
  `not established`.
- **Embedded tasks:**
  - A's runs cost 0.524 for 2 correct;
  - B's cost 0.485 for 1 correct;
  - C's cost 0.170 for 4 correct, plus about 0.72 of curator cost at the
    average.

  Delivered evidence made the runs themselves cheaper, but preparation
  dominated C's total.
- **B against A on cost:** B is 7% lower per correct (0.174 against 0.188).
  One more correct A judgment, or one fewer correct B judgment, reverses it.

## 3. Arm C is an upper bound, not an automated oracle

**What arm C had:**

| Condition | Arm C in S0 | An automated oracle |
|---|---|---|
| Invocation | the experiment attached a projection to every task | the agent must ask, or something must trigger delivery. Recognition was not measured: B's agents looked for the need on 4 of 4 embedded tasks, but those tasks were written to be discoverable |
| Question | the curator saw the item author's exact text | the oracle sees how the agent phrases the need |
| Selection | the curator (Opus 5.5 high) searched the claims readers and register view freely, per item, without the key. The accepted record does not measure how complete its selection was; the draft results (`results/s0-results.md:199`) called it near-perfect | a router chooses among candidates (measured below) |
| Preparation | one projection per item, used once; 0.180 on average | per query, unless projections are cached and reused |
| Delivery | at task start | at query time |
| Scoring | blinding imperfect: answers could mention the attached material | — |

So C's 12 of 12 shows what correct, curated evidence delivered at the right
moment did for this model on these items. It does not show what an automated
oracle would achieve.

**What S0 measured about the automated parts** (routing benchmark: 12 items,
no agents, model router `claude-sonnet-5-5` medium, 21 candidates, about 0.021
per item by CLI estimate). Tallies are correct / false / missed / correct
abstention / router error:

| Reading | Model router | Substring router |
|---|---|---|
| S0 as scored (official) | 8 / 0 / 3 / 1 / — | 4 / 0 / 7 / 1 / — |
| Revision 3, reading A (the pre-registered rule, plus the settled E8 correction) | 8 / 0 / 2 / 2 / 0 | 4 / 0 / 6 / 2 / 0 |
| Revision 3, reading B (E4, E6 and M4 treated as abstain items with decoys; the key does not settle this) | 6 / 2 / 1 / 3 / 0 | 2 / 2 / 5 / 3 / 0 |
| Reading A, with E5's null-id strings mapped to the findings they quote (the reviewer's inference) | 7 / 1 / 2 / 2 / 0 | 4 / 0 / 6 / 2 / 0 |

**Limits of these routing figures:**

- **Register findings were never routed.** E3 and E4's targets are register
  findings, and all 12 register candidates had null ids. Routing to register
  findings was not measured at all.
- **Router failures:**
  - the model router had none, on 12 of 12 replies, checked against its
    transcripts;
  - for the substring router a total failure is ruled out, but a partial
    failure is `not established`.
- **The router saw the author's text,** as the curator did, not an agent's
  phrasing.

**A modelled composition.** The basis is modelled; the combined effect was
not measured. Its assumptions:

- an item the model router routed correctly (S0 as scored: E1, E2, E5, E6, M1,
  M4, M5, M6) gets arm C's outcome and run cost;
- the other 4 items get the fallback arm's outcome and run cost;
- every item pays the router's 0.021;
- each routed item pays the curator's average 0.180, divided by k, the number
  of uses each projection serves.

| Fallback | Fully correct | False confident reuse | Per correct, k = 1 | k = 2 | k = 4 | k at which it equals B's 0.174 |
|---|---|---|---|---|---|---|
| B (readers still available) | 12 | 0 | 0.218 | 0.158 | 0.128 | about 1.6 |
| A (no readers) | 10 | 0 | 0.270 | 0.198 | 0.162 | about 3.0 |

**How to read this.** Under these assumptions, an oracle with single-use
projections costs more per correct resolution than existing readers. It
matches them only if each projection serves about 2 to 3 uses. The composition
leaves out:

- routing on an agent's phrasing;
- routing to register findings (unmeasured);
- whether a router-selected projection produces arm C's outcomes;
- recognition;
- the reading-B and mapped-E5 routing results.

It shows which parameters decide the case (routing accuracy, projection reuse
and fallback quality). It is not an estimate of oracle performance.

## 4. What this supports, and what it does not

- **Supported, as a feasibility signal:** at n = 12, curated delivery gave the
  most fully correct answers (12, against 8 and 8). The shortfalls of the
  unassisted and readers-only arms concentrated on embedded tasks.
- **Not established:**
  - a safety difference: false confident reuse was 1, 1 and 0, differences of
    one occurrence;
  - a cost advantage for existing readers (7%, reversible by one judgment);
  - an advantage for an automated oracle;
  - reuse of projections across agents, which decides the oracle's cost case.

## 5. A correction to the accepted S0 record

- **The wrong claim:** "The S2 default is met under every reading"
  (record, section 6).
- **Error class:** an over-general claim. "Every reading" covered only the two
  readings the record tabulates, official and routable-only. It did not cover
  the truth designations the key leaves open (E4, E6 and M4), or E5's null-id
  strings.
- **Consequence:**
  - The default holds under the pre-registered rule with the E8 correction:
    0 false routes, and 8 correct routes against 4.
  - It does not hold under reading B (2 false routes).
  - With E5's strings mapped, it holds with zero margin: 1 false route, where
    the pre-registered limit is 1 of 12 or fewer.
  - Arm outcomes, the S1 decision rule and the register study are unaffected.
    The study states no S2 conclusion.
  - Any S2 proposal must fix the truth designations before it runs.
- **The check that would have caught it:** list the designations the key
  leaves open and score each one, before writing "every".

The user approved a dated correction. It is section 14 of the record, which
keeps the original statement and marks it superseded.
