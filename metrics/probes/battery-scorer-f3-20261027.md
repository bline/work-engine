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
