# F3 result: 34-case battery scorer (2026-10-27): accepted

Plan: [`metrics/experiment-plan-20261026.md`](../experiment-plan-20261026.md) §4 F3.

**Status: built, independently verified, and accepted by the owner on 2026-10-27.**
The record is committed; the scorer and its sealed files stay outside the repository.

**Who saw what.** An independent Fable 5.1 agent built the scorer; it read the
battery. The coordinating session, which designs experiment arms, did not read the
cases, the sealed rules or the sealed notes. It re-ran the reproduction and looked
only at counts.

## Deliverables

All deliverables live outside the repository, at `~/cbm-work/f3-battery-scorer/`.

| File | Readable by arm designers | sha256 |
|---|---|---|
| `battery_score.py` (driver, no case content) | yes | `10a0ad303dd8c9e5d6d4b17ec8b80539ea024627572236aa8dd21160d3db5958` |
| `README.md` | yes | `58d7804aef7fdd5a9acd340fbb656613f6251893c90bd17633a2f232822d1203` |
| `rules_sealed.py` (verbatim port of the four per-arm adjudicators and the historical `adjudicate.py`) | **no** | `4ec616651969d1d66f584ef991d6ce6f501db60887447ff0f0a6b538672d5916` |
| `NOTES-SEALED.md` | **no** | `430dc156a58e1c9f0f7b43222f79b78533337795de26a9eb528861c2ca7a4b13` |

Case manifest sha256: `43bf8861a993b62262fcc7185a948ccd5c99c10654bb0ac1f1bbd08d9d300a46`.

**The sealed files stay out of the repository and out of arms' reach.** An arm's
clone holds only its baseline commit, so anything committed now would be readable to
any arm whose baseline is a later commit. Each
experiment's allowlist (plan §2 rule 5) must deny read access to
`~/cbm-work/f3-battery-scorer/` and `~/.local/state/work-engine/`.

```bash
cd ~/cbm-work/f3-battery-scorer
python3 -I battery_score.py score --repo PATH --commit SHA --out FILE.json [--detail-out SEALED.json] [--rules final|initial]
python3 -I battery_score.py reproduce --out FILE.json [--mode observations|binary|rebuild|all]
```

## Reproduction (verified by the coordinator, counts only)

Confirmed failures in each of the three modes:
- **observations:** re-adjudicate the frozen observation files;
- **binary:** re-run the frozen binary;
- **rebuild:** fresh clone and offline build of the recorded commit.

| Builder | Phase | Published | Observations | Binary | Rebuild |
|---|---|---:|---:|---:|---:|
| Terra | initial | 3 | 3 | 3 | n/a: initial snapshot matches no commit; frozen binary only |
| Terra | final | 0 | 0 | 0 | 0 |
| Sol | initial | not established | n/a | n/a | n/a |
| Sol | final | 0 | 0 | 0 | 0 |
| Sonnet | initial | 9 | 9 (+2 nonblocking) | 9 (+2) | 9 (+2) |
| Sonnet | final | 0 (+2 nonblocking) | 0 (+2) | 0 (+2) | 0 (+2) |
| Opus | initial | 8 | 8 (+2 nonblocking) | 8 (+2) | 8 (+2) |
| Opus | final | 0 (+2 nonblocking) | 0 (+2) | 0 (+2) | 0 (+2) |

- Case-level mismatches: 0 across all builders, phases and modes; 0 indeterminate.
- Each scoring takes 2–3 seconds; `reproduce --mode all` takes about 20 seconds.
- Rebuilt binaries are not byte-identical to the frozen ones, because of build
  paths. They are equivalent on the identical status of all 34 cases.

Cost of the build: 22 Fable calls, 0.626M weighted tokens.

| Meter | Before | After |
|---|---:|---:|
| Fable weekly (`seven_day_overage_included`) | 0% | 2% |
| Main weekly (`seven_day`) | 7% | 8%, mixed with the coordinator's own use |
| `five_hour` | 21% | 31% |

## Findings (no case content)

1. **The battery is interface- and schema-specific.**
   - Each builder chose its own CLI flag (`--export-usage`, or `--export-calls` for
     Opus) and its own output schema.
   - Frozen rules exist for six schemas: Terra, Sol-hybrid, Sonnet, Opus,
     incremental and end-only.
   - The baseline `1e21e71c` scores 34 of 34 indeterminate. A new implementation
     that invents a new interface or schema will do the same until someone writes
     projection rules for it, which is a new judgment.
   - This is the plan's P1-2 concern in concrete form: as frozen, the battery does
     not accept every contract-compliant implementation automatically.
2. **Rule tiers.** Sonnet and Opus each have an initial-phase and a stricter
   final-phase adjudicator. `final` is the default. Under the initial-phase rules,
   Sonnet's final commit shows 2 confirmed failures that the builder agent describes
   as a field rename. Headline tier: `final` (accepted with this record).
3. **One flagged mapping.** The historical adjudicator's `unresolved_check_failure`
   disposition maps to confirmed failure, and the raw disposition is kept. No
   published artifact exercises it.
4. **No contract-versus-regression split.** The battery does not record whether a
   case comes from the contract or from a past review finding, so the category is
   `not established`. Counts by fixture-provenance directory are given instead.

## Owner decision on finding 1 (2026-10-27)

E1a fixes the export interface in its packet (option A), with Terra's final commit
`88b035f2` as the reference schema, so every run is scored by frozen rules and no
per-run rules are written. See the plan, §6 E1a.

## F3.1: fixed-interface mode for E1a (2026-10-27): accepted by the owner

Built by a second independent Fable agent; it read the sealed rules, and the
coordinator did not. Interface section the arms receive:
`~/cbm-work/e1a/packet/interface-v1.md`. It states Terra `88b035f2`'s structure,
and fixes reason codes only for conditions the contract names.

| File | Readable by arm designers | sha256 |
|---|---|---|
| `battery_score.py` (changed; existing modes untouched) | yes | `51ef39399c3ddab1704b28104fbf1e22ff459902bd4ccbace79d2d36543e5f7b` |
| `README.md` (changed) | yes | `5fe1c500a75274967c68acd3d89f445e6a4c2abe6cfe553ad2b8d727ef617a4a` |
| `interface_e1a_v1.py` (interface-v1 as checks) | yes | `ddcb424efe75a9fb4c702fc0706c71661cd126ec46325a42c4c49a712e986338` |
| `rules_e1a_sealed.py` | **no** | `804e8cf353e385a56fe1f27b9205ebd53ab386c15472404e8736e9cff70bb773` |
| `NOTES-E1A-SEALED.md` | **no** | `17f6b7b639f979b070f8a41c11105246f3a1767097ba18d23371fe9482017995` |

`rules_sealed.py` and `NOTES-SEALED.md` are unchanged.

```bash
python3 -I battery_score.py score --repo PATH --commit SHA --interface e1a-v1 --out FILE.json [--detail-out SEALED.json]
python3 -I battery_score.py vocab-test --obs-dir DIR --out FILE.json [--salt S] [--seed N]
```

- **Rule adaptations:** 3 checks in 3 cases. Each check depended on a reason code
  that interface-v1 leaves to the implementation. Each is replaced by the
  strictest check over stated elements. This is a one-time judgment made before
  any arm runs, recorded in the sealed notes.
- **Non-conformance:** a case whose output does not conform is `indeterminate`
  with disposition `interface_non_conformant`, and is never a pass or a failure.
  The run is flagged when any case is non-conformant, the `--json` combination is
  not rejected, or no invocation produces a conformant document.
- **Interface corrections needed:** none. Two statements are not exercised by the
  scorer: the `--session` form, and unchanged behaviour without `--export-usage`.
  The contract's existing regression tests cover the latter.

Validation, counts only:

| Subject | Conformant | pass / fail / nonblocking / indeterminate | Who ran it |
|---|---|---|---|
| Terra final `88b035f2`, rebuilt | yes, 33 documents + 1 refusal | 34 / 0 / 0 / 0 | agent and coordinator |
| Terra initial frozen binary | yes | 31 / 3 / 0 / 0 (published, original rules: 3) | agent |
| Baseline `1e21e71c` | no, 0 documents | 0 / 0 / 0 / 34 | agent and coordinator |
| Sonnet final, Opus final (other schemas) | no | 0 / 0 / 0 / 34 each | agent |
| Vocabulary test: every non-fixed code renamed, extra field added, key order shuffled, on Terra final's observations | n/a | counts unchanged, 0 mismatches; the unadapted rules give 3 failures on the same input, so the test is not vacuous | agent (2 seeds); coordinator (salt `coord-check`, seed 97: 33 documents perturbed, 0 mismatches) |
| `reproduce --mode all` (original modes) | n/a | 0 case-level mismatches in all 7 artifacts × 3 modes | agent and coordinator |

Cost: 26 Fable calls, 0.61M weighted tokens, $4.59 at list price. That is above
the 0.5M target; the agent's own report said "well under", which was wrong.

## Traceability of the 34 cases under `e1a-v1` (2026-10-27, after E1a)

An independent Sonnet 5.5 session checked each case, as scored under `e1a-v1`,
against the E1a packet (counts only to the coordinator). Per-case results:
`~/cbm-work/f3-battery-scorer/TRACE-ANCHOR-SEALED.md` (sealed, `a35ea21c…`).

| Class | Cases |
|---|---:|
| traces to a packet clause | 12 |
| over-constrains | 11 |
| rests on a requirement the packet does not state | 11 |
| untraceable | 0 |

- In 11 cases the scoring rule is stricter than the case's own expectation.
- One case has a branch gap in its scoring rule: an exit-0 document raises an
  assertion error instead of a verdict. No arm reached it.
- **Consequence.** Under the fixed interface, the battery's confirmed-failure
  count is not a measure of contract compliance. On the five E1a arms, 58 of 62
  failures fall on non-traced cases; see the
  [E1a result](e1a-result-20261027.md#battery-traceability-post-hoc-not-pre-registered).
- **This is finding 1 (P1-2) in measured form.** The headline-tier choice
  (`final`, the stricter adjudicator) contributes, through the stricter-rule
  cases.
- **Before the anchor is used again as a contract measure,** its cases need the
  same repair as the T0 battery: 22 cases to rewrite, remove, or report as a
  separate historical category.
