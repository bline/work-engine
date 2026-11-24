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
