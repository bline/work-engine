# Engineering builder handoff receipt contract

Return one compact `handoff_receipt` beside the terminal `audit_receipt`. This object is non-durable context for a fresh builder; never append it to the metrics JSONL.

Include only:

- `slice_title`
- `slice_goal`
- `outcome`
- `durable_decisions`: authored architectural or ownership decisions that constrain later work
- `affected_boundaries`
- `placement_certificate`: trigger, producer, state owner, consumer, lifecycle, semantic consequence, downstream proof, and insufficient substitute
- `unresolved_concerns`
- `deferred_scope`

Keep observed outcomes, durable decisions, and unresolved concerns distinguishable. Exclude engine configuration, validation bookkeeping, individual commands, provider/model identity, token/cache/cost measurements, timing, call counts, and review statistics. Prefer 300-800 tokens; omit irrelevant narrative rather than required semantic state.
