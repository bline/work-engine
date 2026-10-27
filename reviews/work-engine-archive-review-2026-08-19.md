I'll take a look at what's in the archive.Read through all of it — the four skills, both campaign configs, the scripts, the eight metrics records, the roadmap, and the prior review. Here's my honest take.

## What's genuinely good

The organizing rule — *information crosses a model boundary only when its expected lifetime justifies crossing* — is one of the better framings I've seen for agentic coding, and more importantly you actually applied it rather than stating it. Supervisor state is tiny, builder identity persists through a slice, recon is disposable. That's coherent.

The placement certificate is the best artifact in the system. `trigger → producer → owned state → consumer → consequence → proof → insufficient substitute` is compressed, falsifiable, and it does real work. The "insufficient substitute" clause in particular is doing something most review checklists never manage: it names the plausible-but-wrong thing in advance.

And the two-stage placement with a fresh falsifier has already paid for itself in your own data. Record 3 of `work-engine-roadmap.jsonl`: the shallow packet claimed `sidebar.js` was module-loaded, targeted evidence found `sidebar.html` using classic ordered scripts and turned up an unlisted consumer in `semantic-toolbox.js`. That's a caught architectural error, pre-implementation, from a process that assumed its own prior output was wrong. Keep that.

`run_gate.py` is clean — array commands, fail-fast, bounded excerpt, structured result, no shell interpolation. `resolve_provider.py` enforcing provider↔skill consistency and refusing `auto` rather than guessing is exactly right.

## The main problem: the elaborate part is the untested part

All eight records are `slice_number: 1`, each with a distinct `run_id`. **No campaign has ever executed a second slice.**

That means the `handoff_receipt` has never once been consumed by a downstream builder. Neither has the continuation logic, the limits machinery, the notification rules, or the outlier check ("more than twice a recent median" — against a median of one). The supervisor is your most heavily specified component and has essentially no execution history beyond "start one builder, record what happened."

Meanwhile 100% of your actual activity is concentrated in planning and placement, which is where six of eight runs terminated.

That inversion is worth sitting with. The 300–800 token handoff receipt is a good idea, but right now it's a good idea with zero evidence, specified at the same level of rigor as the parts you've run twenty times. I'd either run one genuine multi-slice campaign until it breaks, or cut the supervisor spec down to what actually executes and let it regrow from observed need.

## The measurement layer can't answer the questions the roadmap asks

Phase 0's exit criterion is a repeatable baseline. It isn't met, and Phases 5 and 6 shipped anyway.

Nearly every cost field is `null` across all eight records: `builder_input_tokens`, `builder_output_tokens`, `builder_wall_clock_seconds`, `builder_context_usage`, and provider cost. So your headline derived metric — provider output tokens per retrieved source line — is not computable from stored records. `decision-policy.md` instructs the builder to "prefer the lowest expected total cost across model tokens, context occupancy, latency…" with no data on any of those. Phases 8, 9, and 11 are explicitly A/B experiments requiring comparable measurements; they cannot be run against this JSONL.

`workflow_route` is absent from every record, though both the builder skill and the receipt doc require it. The validator only enforces it at `schema_version >= 4`, and no v4 record exists. So the central efficiency lever — whether `direct` is ever actually selected over `falsified-placement` — is unmeasured.

## Data integrity is worse than the drift the last review flagged

That review noted docs at v3 while records were v2. It's now docs at v4, records at v2 and v3, and the same class of problem has spread into fields that carry your truthfulness invariant:

- `engine_config.version` is `1` in all eight records, including runs sourced from `work-engine-roadmap.yaml`, which declares `version: 2`.
- `engine_config.validation.profile` is `engineering-full` in all eight. Both campaign files say `engineering-proportional`. Either every run was overridden (two records say so; six don't), or the recorded provenance is simply wrong.
- Two records name their source as `work-engin/campaigns/roadmap.yaml` — a path that doesn't exist. It validated fine.
- `producer_metrics` appears in three mutually incompatible shapes plus empty: `{}`, `{"model": null, "input_tokens": null, …}`, `{"supervisor_input_tokens": …}`, and `{"run_id", "planning_turns", "plan_acceptance", …}`.
- `repository_exploration_outside_evidence_packets` appears as `0`, as an array of strings, as `null`, and as an English sentence.

"Flexible namespaced metrics" is doing real damage here — the flexibility sits precisely where cross-run comparison is the entire point. I'd close the schema on a small typed set and confine flexibility to `additional_metrics`, then have `append_metrics.py` verify that `engine_config` matches the file it names rather than trusting the model's transcription of it. A `--min-schema-version` flag with an explicit `--allow-legacy` escape would turn "new runs must not emit v1–v3" from prose into something enforced.

## Prose is carrying load that code should carry

The recon skill says: on infrastructure failure, "inspect the execution conditions before retrying; do not repeatedly issue the same invocation." Record 2 of `work-engine-roadmap.jsonl` shows `evidence_recon_calls: 5` against a provider that returned no evidence at all, and the run died there. The instruction was clear, well-placed, and did not hold. Retry policy belongs in the adapter.

More broadly: ~45KB of normative SKILL.md against ~600 lines of Python, and the Python is what actually binds. The invariants/acceptance-conditions/routes/recovery-decisions taxonomy is stated four times — supervisor, builder, decision-policy, recon. Route selection criteria appear three times. Plan contents twice. The previous review flagged this; it grew instead.

A useful filter for every paragraph: *what observable field or check does this produce?* If nothing, it's prompt tax on every slice and a drift surface forever.

## Smaller things

Two of eight runs died to provider failure at the very first evidence call — that's your largest single failure category, ahead of genuine architectural stops, and it's an infrastructure problem being handled as a workflow problem.

Path conventions are inconsistent: the builder calls `python3 scripts/run_gate.py` (skill-relative), the supervisor calls `python3 work-engine/skills/slice-supervisor/scripts/append_metrics.py` (repo-root-relative). You're already running this engine against two different repositories, so that will bite.

`chrome-vision` is orphaned — full `src/`, tests, schemas, `package.json`, and no campaign, builder contract, or roadmap phase references it. Only `ideas/ui-review.md` does. Either wire it in as a selectable evidence capability or move it out of `skills/` until it has a caller.

## On the seam-review idea

It's the right generalization, and your own instinct against fifteen mandatory reviewers is correct — exposing seam capabilities and letting the decision policy invoke them is the right shape. But it's downstream work. Seam reviews are only affordable once the deterministic gate and the metrics are trustworthy, and right now you can't tell a cheap route from an expensive one. Building critics on top of an unmeasured base means you won't know whether they helped.

## What I'd do next, in order

1. Close Phase 0 for real. Capture tokens, cost, and wall-clock, or explicitly record why the provider can't supply them. Nothing downstream is meaningful without it.
2. Freeze the receipt schema at v4, enforce it in the validator, and validate `engine_config` against the file it cites.
3. Run one multi-slice campaign to completion or genuine failure, so the handoff receipt and continuation logic get exercised at least once.
4. Move retry, backoff, and provider health out of prose and into the adapter.
5. Then collapse the duplicated contract prose — after the above, you'll know which paragraphs correspond to something observable.

The architecture is sound and I wouldn't restructure it. The gap is that the specification has outrun the evidence by a wide margin, and the fastest way to close it is to make one campaign run end to end with honest numbers.

