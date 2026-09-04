# Work Engine

Work Engine runs AI coding agents on real engineering work while keeping a named
human in charge of what gets accepted. Work is cut into bounded slices. Each
slice is planned, implemented, reviewed by a second agent that did not write it,
and checked by deterministic gates before a human decides whether to accept it.
Every slice leaves a durable record of the configuration it ran under, the
evidence it produced, who reviewed it, and how it ended.

The problem it addresses is that agent output is cheap to produce and expensive
to judge. An agent will return work that is formally plausible and contextually
wrong, and the cost of discovering that lands on whoever reviews it. Work Engine
attacks that asymmetry from three directions: it constrains what a slice is
allowed to change and claim, it requires independent review before acceptance
rather than after, and it treats stopping as a first-class outcome — an agent
that cannot get evidence for a claim records why it stopped instead of producing
something that looks finished. Acceptance authority is never delegated to the
agent.

## The loop

```text
campaign config  →  preflight        resolve the effective configuration, pin its digest
                 →  slice plan       bounded objective, explicit acceptance conditions
                 →  human acceptance of the plan
                 →  builder          one persistent agent implements one slice
                 →  independent      a separate agent, fresh process, read-only,
                    review           configured as a different provider from the builder
                 →  gates            deterministic checks with mechanically decidable results
                 →  checkpoint       immutable content identity in a private Git ref
                 →  human acceptance of the result
                 →  receipt          durable record appended to a metrics ledger
```

Four properties are load-bearing:

- **Acceptance authority is human.** Skills, gates, reviewers, and projections
  produce evidence. None of them accept a slice, publish to a branch, or widen
  their own authority.
- **Review independence is recorded, not assumed.** The reviewer's provider,
  isolation, and evidence class are written into the receipt. When the
  configured cross-provider reviewer is unavailable and the human authorizes a
  same-model fallback, the receipt records the downgraded evidence class and the
  amendment that caused it rather than claiming independence it did not have.
- **Stopping is truthful.** `stop_on` conditions are declared in the campaign.
  Stopped slices are recorded with their reason next to accepted ones in the
  same ledger.
- **Publication is explicit.** Accepted work lands in private `refs/work-engine/`
  checkpoints. Moving anything onto a branch is a separate, human-authorized
  step, and it must not touch the human's checkout, index, or unrelated files.

## Try it

Everything in this section runs offline, with no API key and no agent. It needs
Node.js 22+, and Python 3 with PyYAML for the graph commands.

```bash
npm install
npm test          # unit and integration tests for the supervisor, MCP adapter, and App Server runtime
```

Resolve a real campaign configuration. This binds the campaign file to its
canonical path and SHA-256, applies documented defaults, and reports which
fields were explicit versus defaulted — the same object a supervisor consumes:

```bash
npm run preflight            # campaigns/roadmap.yaml
node skills/slice-supervisor/scripts/campaign-preflight.mjs campaigns/review-remediation.yaml
```

Validate the role/invariant graph and confirm the checked-in generated views are
not stale:

```bash
python3 skills/agent-environment-graph/scripts/agent_environment_graph.py validate \
  --invariants docs/workflow-invariants.md --environments docs/agent-environments.yaml

python3 skills/agent-environment-graph/scripts/agent_environment_graph.py check \
  --invariants docs/workflow-invariants.md --environments docs/agent-environments.yaml \
  --rendered docs/agent-environment-graphs.md --role-output-dir docs/agent-environment-views
```

Read the receipts. These are the actual production records, one JSON object per
slice, and they are the fastest way to see what the system does and does not
claim:

```bash
cat metrics/*.jsonl docs/ai-workflow-metrics.jsonl app-server/docs/ai-workflow-metrics.jsonl \
  | python3 -c 'import json,sys
for l in sys.stdin:
    r=json.loads(l)
    print(r["status"], "|", r["run_id"], "|", r["slice_title"][:70])'
```

Running an actual campaign is different: it needs a coding-agent CLI, provider
credentials, and a human present to accept plans and results. The campaign files
in `campaigns/` are the entry points, and `skills/slice-supervisor/SKILL.md` is
the contract the supervising agent follows.

## Structure

| Path | What it holds |
| --- | --- |
| `DESIGN.md` | Normative doctrine: invariants, authority boundaries, design tests |
| `PHILOSOPHY.md` | Why that doctrine exists; explicitly non-normative |
| `ARCHITECTURE.md` | Current whole-system map, with an explicit maturity class per subsystem |
| `roadmap.md` | Product direction, remaining work, and completion evidence |
| `skills/` | Role and capability packages: contracts, adapters, schemas, tests |
| `app-server/` | App Server runtime: role hosting, services, protocol, isolated workspaces |
| `campaigns/` | Declarative campaign objectives and configuration |
| `metrics/`, `docs/ai-workflow-metrics.jsonl` | Slice receipts, one JSON object per slice |
| `docs/workflow-invariants.md` | Verified invariant catalog for the primary slice workflow |
| `planning/`, `proposals/`, `reviews/`, `ideas/` | Plans, formed candidate changes, review evidence tied to exact revisions, and speculative sources |

The repository is mid-migration. Today the canonical role contracts and
model-facing instructions live in `skills/`, and `app-server/` holds the runtime
those roles are being re-hosted in — where a role becomes a manifest-declared
instance backed by server-owned services rather than an agent-local skill
bundle. Both trees are live during the transition: `skills/` remains
authoritative for anything the migration has not explicitly moved, and the
migration plan
([`app-server/docs/skills-migration-plan.md`](app-server/docs/skills-migration-plan.md))
records per-slice ownership so no responsibility is retired before an accepted
successor exists.

## Deeper reading

Read in this order if you want the reasoning rather than the map:

1. [`PHILOSOPHY.md`](PHILOSOPHY.md) — why structure is placed only where it is load-bearing.
2. [`DESIGN.md`](DESIGN.md) — the contracts that structure produces, and the tests for whether something is really an invariant.
3. [`ARCHITECTURE.md`](ARCHITECTURE.md) — what currently exists, classified as implemented, active construction, formed direction, or exploratory.
4. [`app-server/docs/skills-migration-plan.md`](app-server/docs/skills-migration-plan.md) — the migration campaign, including the corrections it has had to record.
5. [`HISTORY.md`](HISTORY.md) — a self-audit of how the project actually developed: what stabilized early, which routes were retired or demoted, and a quantitative snapshot pinned to commit `5308ae5`. It states its own evidence classes, confidence, and limitations, and it does not flatter the result.

`AGENTS.md` is the entry point for an agent working in this repository.

The project also produces experimental evidence about agent behavior under
different instruction and review configurations; that work lives under
`skills/review-bench/`, `skills/linguistic-register-pilot/`, and `proposals/`.

## Status

Single maintainer, active development, no stability guarantees. The execution
backbone — campaign supervision, slice execution, gates, checkpoints, receipts —
is implemented and used to develop this repository. The planning, durable-state,
and control-plane layers are in varying states of construction; `ARCHITECTURE.md`
labels each one rather than presenting the intended system as finished.

## License

MIT. See [`LICENSE`](LICENSE).
