# P7 run 1 pre-registration: independent review (Astra)

- Subject: `subject-PROTOCOL.md`, byte-identical copy of `experiments/perspective-intent-research/p7-run1/PROTOCOL.md` at commit `29d843864667a4c5cdf9650750238ffbd67d4750` (SHA-256 `a4de2ebc87d7d9ce7f69312e32da1ba64ea97294924613d37002038b400bc6e4`), together with the built prerequisites in that directory at the same commit (fixtures, schema, prompts, comparator and tests, agent definitions, build script, tasks, run plan), which are not copied here; git holds them at that commit.
- Review: `review.md`. Fresh Pi subagent `reviewer`, run `d8af0eb0-8a22-4423-bf8f-ba20b238046a`, model `openai-codex/gpt-6.2-astra:xhigh` (ChatGPT/Codex subscription via OAuth; no model fallback). Read-only apart from two local checks it ran (`build_tasks.py --check`, unit tests). Operator-authorized 2026-11-24.
- Verdict: `revise_and_resubmit`; freeze blocked. Eleven findings: two blockers (typed procedure confounds citation form, support and authority; model-visible answer cues), seven high, two medium.
- Parent spot checks, confirmed by reading: every fixture announces "constructed negative control for P7 run 1"; the comparator returns `supported` for a widened actor set regardless of `replaces_parent`; the pi-subagents intercom bridge, when active, appends instructions to the system prompt after the definition digest is taken.

## Re-review of revision 2

- Subject: `subject-PROTOCOL-r2.md` (byte-identical, commit `233821e11db00ad7db1510506b6fda006b6fa11f`, SHA-256 `9ee81b51cfdad527e10dd5192d345c065dea31a5c26d48bb427929f210b35031`) with RUNBOOK.md and instruments at that commit.
- Review: `review-r2.md`. Same reviewer resumed (run `1562e9c3-9fed-4ec9-815a-d3c12c79b51a`), `openai-codex/gpt-6.2-astra:xhigh`.
- Verdict: `revise_and_resubmit`; freeze blocked. Original findings 2, 3, 10 closed; others partially closed. New findings A–G (two blockers: receipt does not establish tool availability or bind output to task; comparator can still report `supported` for unassessed obligations).
- Parent spot checks: system message records `toolsAdded` (22 tools in run d8af0eb0), ignored by `extract_receipt`; pi-subagents documents a per-launch `intercomBridge: {mode: "off"}`; a partial inherited entry with only an ambiguous field returns standing `supported` with no flags.

## Third review (revision 3, after the route change and calibration)

- Subject: `subject-PROTOCOL-r3.md` (byte-identical, commit `da012df4a1f9b204eea4a4ed9bff09f1e0f19dee`, SHA-256 `c7856f94d505ff7f6d3e6a23549a11d0370d4300aa8df47011289ef390d089e4`) with RUNBOOK.md, instruments, and the smoke and calibration records at that commit.
- Review: `review-r3.md`. Fresh Pi subagent `reviewer`, run `4e42c25d-a6fe-458c-b678-998067fda65d`, `openai-codex/gpt-6.2-astra:xhigh` (ChatGPT/Codex subscription; operator-approved 2026-11-25). Read-only; ran `build_tasks.py --check` and the unit tests (75 passing).
- Verdict: `revise_and_resubmit`; freeze blocked. Ten findings:
  - three blockers: positive plan controls have no typed record to earn credit; no pre-dispatch assignment of launches to rows; census and weakening counted despite failed or duplicated judgments;
  - five high: missing linkage reported as a change; valid-but-unusable or crashing extractions; launch claims beyond what the receipt checks (two extensions); failure classification; report joins;
  - two medium: lifecycle gates; calibration manifest and denominators.
- The reviewer independently verified all 24 calibration-era launches against their host records and found no substitution, repair or counting of calibration outputs.
- Parent spot checks, confirmed:
  - a derived `ambiguous` field without `value` passes `validate` and raises `KeyError` in `report`;
  - every turn spec carries two `-e` extensions (the Claude provider and `pi_service_tier.ts`);
  - on an upstream non-200 the proxy writes an `upstream_error` disposition and a `refused` row, which the receipt classifies as a violation rather than a provider failure.
