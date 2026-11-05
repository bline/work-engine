# Objective budget effectiveness pipeline, 2026-11-04

These are the scripts behind
[`../../budget-effectiveness-20261104.md`](../../budget-effectiveness-20261104.md)
and its [observations JSON](../../budget-effectiveness-20261104.json). Their
SHA-256 values are recorded in that JSON. They are copied exactly as run,
including the hard-coded private working directory
`~/cbm-work/budget-analysis-20261104`. `export.py` also accepts
`BUDGET_ANALYSIS_DIR`.

| Step | Script | Output |
| --- | --- | --- |
| 1 | `discover.py OUT ROOT...` | record files with a top-level `scope_budget` |
| 2 | `inventory.py`, `resolve.py` | records deduped by objective ID, with end time, end basis and status |
| 3 | `canon.py` | canonical objectives; views and copies set aside |
| 4 | `run_tree.sh ID RECORD SINCE UNTIL` | `metrics/scripts/usage_ledger.py tree` per objective |
| 5 | `table.py IN OUT` | per-objective spend, limits, root share and post-close candidates |
| 6 | `type-labels.json`, `classify.py` | hand type labels; `outcome()` maps status text to an outcome |
| 7 | `phases.py` | root setup and closeout around child work |
| 8 | `overhead.py`, `overhead_re.py`, `sample_hits.py` | root spend on budget-command calls, plus samples to validate the pattern |
| 9 | `export.py OUT` | the committed JSON, with every summary recomputed |

`classify.py` also holds `otype()`, a keyword type classifier that was
rejected: it agreed with the hand labels on 37 of 64. It is not used.
