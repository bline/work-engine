# E3 pilot fixtures: supervisor decision replay (2026-10-27)

Plan: [`metrics/experiment-plan-20261026.md`](../experiment-plan-20261026.md) §6 E3
and §8 step 2.

**Status.** There are 10 fixtures, each with a correctness class. They were built
and sealed by a session that runs no arm and scores no arm. Arms have not run.

This record holds counts only. The coordinator did not read the key or the notes.

## Fixtures

The fixtures are in `~/cbm-work/e3-fixtures/fixtures/E3-01.json` to `E3-10.json`.
Each one is a past supervisor decision point, rebuilt as self-contained text.

- The text is a context list, a question and options; the arm chooses one option id.
- Arms run offline with no file access.
- Scrubbed: run names, session ids, timestamps, absolute paths and finding ids.
- Kept: project vocabulary and crate names. One fixture also keeps digests,
  because its question depends on them.

| Kind | Class 1 | Class 2 | Class 3 | Total |
|---|---:|---:|---:|---:|
| finding_disposition | 1 | 4 | 0 | 5 |
| plan_decision | 0 | 2 | 1 | 3 |
| handoff_timing | 0 | 0 | 1 | 1 |
| continue_stop_escalate | 0 | 0 | 1 | 1 |
| **Total** | 1 | 6 | 3 | 10 |

- Classes 1–2 are scored for accuracy, and there are 7 of them, against a target
  of at least 6.
- Class 3 is reported as agreement with the historical choice, not as accuracy.
- 6 fixtures are critical candidates; critical errors are counted separately.
- 2 fixtures are cases where the historical decision was wrong.
- Context sizes run from 308 to 6,396 tokens (characters ÷ 4), within the 30K cap.
- n = 10, about one fixture per cell. Figures support no inference by kind.

## Files

| File | sha256 | Access |
|---|---|---|
| `README.md` | `77bb5839827e47f901430d3d95db186d521589a861c37a289a2e4c959a7efd30` | readable by arms and coordinator |
| `score_e3.py` | `b2d0c54cda9ec0995cf001b18ab762ec6a951b2546fdf09b362186157c9c4faf` | readable; loads the sealed key |
| `fixtures/E3-01.json` … `E3-10.json` | `3fb164e7` `7f356215` `d6900fe0` `9954117f` `570577a8` `5e0f525b` `8dc2bd4a` `9f6a9c69` `c90d1b65` `1cb5ddf4` (first 8 hex characters, in order) | readable |
| `key-SEALED.json` | `8d6ef096009de168c557b36891108c0e15f0f53a77b71fe3917763de74432ecb` | sealed |
| `NOTES-SEALED.md` | `feaa6a12e244b17d6cf083ae12984432b8a5d696b6276ddfb73a181e314b3f15` | sealed |

`_build_SEALED/` and `_test_SEALED/` are sealed too.

## Verification

- **By the builder,** on synthetic answer files:
  - all-correct scored 7/7 on classes 1–2, with 0 critical errors and 3/3 class-3
    agreement;
  - a mixed file matched the builder's hand calculation.
- **By the coordinator** (2026-10-27): all 14 hashes match the builder's report.
  Two answer files were scored that cannot reveal the key:
  - an empty file gave 10 unanswered;
  - a file with 10 invalid choices gave 10 invalid, 10 unanswered.

  Both outputs reproduce the builder's totals: 7 fixtures in classes 1–2
  (5 finding_disposition and 2 plan_decision), 3 in class 3, and 6 critical
  candidates. The scorer prints counts only.
- **Not checked by the coordinator:** whether correct and incorrect answers are
  told apart. That needs the key, so it rests on the builder's synthetic tests.

## Limitations (from the builder, content-free)

- **Adjudicator.** A model adjudicated class 2 correctness from the run record and
  the repository source. It is not independent of the models that produced the
  record, and it is not a human. The criteria were written before adjudication.
- **One fixture failed its own pre-written criteria.** It was demoted to class 3,
  so the pilot has 7 class 1–2 fixtures rather than 8.
- **The historical-wrong pair is weak.**
  - In both, the historical record is a closure claim awaiting the same reviewer's
    verification, not a final supervisor acceptance.
  - No case was found where a supervisor overrode a reviewer and was later shown
    wrong.
  - One of the pair is arguable on its face.
- **Follow-the-reviewer.** 5 of the 7 class 1–2 fixtures include a reviewer
  finding, so an arm can often answer by deferring to it. Only 2 lack a reviewer
  verdict in context, and those discriminate better.
- **No builds or tests** were run for any correctness claim. Class 1 rests on
  string equality visible in the fixture and in the source it cites.
- **Supervisor view.** Some source excerpts were available to the supervisor, but
  the builder cannot show the supervisor read them. Each such fixture says so.
- **Prior exposure.** A model with prior exposure to this repository could
  recognise some material.
- **Excluded:** decisions on subject `7fdf392f` (S2).

## Cost

| Item | Model | Transcript cost |
|---|---|---:|
| First attempt (stopped; see plan §3 Fable note) | Fable 5.1 | $3.35 |
| Fixtures, key and scorer | Sonnet 5.5 | $2.35 |

The builder estimated "about 0.6M weighted" and did not measure it. Support
work (plan §3).
