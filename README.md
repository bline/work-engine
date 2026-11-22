# Work Engine

Work Engine is an objective-driven system for engineering work with AI agents.
It gives models bounded decision authority, preserves the evidence and meaning
of their decisions, and keeps acceptance and publication under explicit operator
control. Software development is its first domain.

The central design rule is:

> **Contracts constrain what must remain true. Models choose how to make it true.**

An objective, its acceptance conditions, available capabilities, authority,
budget, and current evidence define the work. A model chooses and revises the
route. Deterministic machinery handles identities, state transitions, integrity,
limits, and checks whose answers are already mechanically decidable. A failed
route becomes evidence for choosing another route; unresolved work stays
unresolved in the record.

This repository contains a working campaign and slice workflow, a substantial
Rust runtime and state layer, and the designs and experiments extending them.
It is under active development. Implementation, bounded qualification, and
production readiness have distinct standing throughout the project.

## Demonstrated value

**About 17% lower cost per model call in the latest context-lifecycle
comparison.** Across five continuing development roots, observed
API-equivalent cost per call fell **16.6%** after native context refresh,
scoped document loading, and context-pressure delivery changes. Input cost per
call fell 11.5%, and mean input context fell 23.7%. The comparison includes
preparation and recovery calls. It measures those root sessions under a fixed
pricing model; task mix changed, so it does not isolate causation or establish
whole-project savings. The [measurement and aggregates](metrics/probes/context-cost-comparison-20261114.md)
make the scope and calculation inspectable.

**Useful engineering through the actual system.** In the
[first local development pilot](records/local-development-pilot-20261106/README.md),
an MCP request reached the Rust job server and native Pi runtime, which added
two regression tests covering eight timestamp boundary cases, reset and
recovered its context, and returned an exported patch. That patch passed all
37 crate tests, mutation checks, and separate review before local publication.
This establishes a bounded working path; the broader engineering integration
still has qualification work ahead.

These results give the project a practical direction: spend less on repeatedly
reconstructing state and coordinating machinery, while preserving the judgment,
evidence, and review needed to produce useful work.

## Research guides the next gains

The context result is one measured improvement in a larger program. The
[research map](metrics/register/README.md) connects questions, studies,
assessments, corrections, and their applicability to specific decisions. The
[efficiency ledger](metrics/efficiency-ledger.md) tracks the remaining levers:
deterministic coordination, reuse of settled evidence, smaller role context,
and reduced review and repair burden. Their expected gains overlap and remain
hypotheses until measured.

Research already changes implementation choices. A
[19-task randomized builder comparison](metrics/probes/dogfood-round2-builder-outcomes-20261104.md)
informed the default model selection; its small sample and inconclusive
statistical tests remain part of that decision's evidence. The register also
retains corrections when measurements disprove an earlier assumption. The aim
is a continuing cycle of research, bounded implementation, measurement, and
revision, with each result guiding the next investment.

## What has changed

The project has grown beyond a supervisor that runs coding slices:

- **Durable execution state.** Context lifecycle, queued input, operation
  identity, grants, uncertainty, recovery, and retirement have explicit owners.
  Provider sessions are realizations of that state, rather than the sole place
  where a task's meaning lives.
- **Native Rust hosts.** The workspace now includes lifecycle services, retained
  task and review hosts, claim/evidence storage, campaign and publication
  components, a skill compiler, operator queries, and a terminal interface.
  New first-party implementation follows the
  [Rust-first direction](docs/rust-development.md).
- **Evidence tied to its actual use.** Claims, review findings, retained basis
  records, and research delivery can refer to exact revisions and executions.
  A matching artifact or successful test does not by itself establish the
  authority, provenance, or production conditions needed by another consumer.
- **Research connected to the runtime.** The locally merged research task-host
  join retains native input and causal evidence and checks its connection to
  the original owner's delivery and recommendation. This is a private,
  controlled implementation milestone; the authentic research execution and
  production qualification remain open.
- **A host application with isolated workloads.** The selected integration
  direction runs Work Engine on the host and workloads in Docker, initially
  one native Pi session per container. The Rust host owns deterministic control;
  models retain local planning and judgment inside their assigned boundary.
  The complete engineering path is still being qualified.

The architecture has also become more explicit about ownership. Objectives,
proposals, authority, evidence, review, runtime realization, and history have
separate meanings. A projection, transport, reviewer, or storage primitive gains
no decision authority merely by carrying their data. See the
[binding design](DESIGN.md) and
[planned architecture](design/work-engine-planned-architecture.md) for those
boundaries and their respective standing.

## How the pieces fit

```text
Operator objective and authority
              |
              v
Bounded work contract + current evidence + capabilities + limits
              |
              v
Model chooses and revises the route
              |
              v
Execution -> artifacts and observations -> review and checks
              |                                  |
              +------- durable state/history ----+
                                                 |
                                                 v
                            Authorized acceptance / publication
```

In the selected architecture, the deterministic host admits and tracks work,
binds the selected runtime, observes limits, and collects outcomes. Task and
review workflows own their semantic transitions. Provider and container adapters
realize the selected capabilities. Evidence and publication components preserve
attributable results without accepting their own work.

Context replacement follows the same division. A new conversation is useful
only when continuation can recover the objective, authority, commitments, and
unresolved obligations from durable owners. Lower token usage or a provider's
compaction event alone does not establish correct continuation. The
[Wind Walker skill](skills/wind-walker/SKILL.md) states that contract; the
[Rust lifecycle design](experiments/context-lifecycle-rust/DESIGN.md) develops
its runtime consequences.

The longer-term direction is reproducible execution: recover a meaningful
workflow coordinate from revision-bound state, or branch it under another
model, harness, or capability set for controlled comparison. The repository has
working pieces of this foundation; arbitrary workflow replay and a fully
integrated organizational control plane remain design and integration work.

## Why Pi and Docker

Pi supports both API keys and subscription quota through the same agent
harness. Both can provide the same tools, instructions, and containerized
workspace. Choosing how to pay for inference does not require choosing a
different execution environment.

Work Engine pairs that common harness with disposable Docker workloads and
durable state on the host. The selected architecture starts with one native
Pi session per container: models plan and execute within their assigned
boundaries, while the Rust host manages admission, execution limits, lifecycle,
and evidence under explicit operator authority.

That separation supports several practical advantages:

- **Enforceable role boundaries.** Builders and reviewers can receive different
  filesystem, tool, network, and credential access. Their permissions can be
  enforced by the execution environment as well as described in instructions.
- **Recovery beyond a conversation.** A container or model context can be
  replaced while durable owners retain task state, evidence, and unresolved
  obligations.
- **Independent review.** Reviewers can run in separate environments with
  explicit evidence access and recorded isolation conditions.
- **Controlled experiments.** Pinned images, workspace inputs, and configuration
  make it easier to compare models, instructions, and organizational designs
  while holding the surrounding environment consistent.
- **Less inference spent on machinery.** Deterministic host code handles routine
  lifecycle and bookkeeping operations, leaving models to exercise judgment.

This also connects organizational compilation to execution: a compiled role
can become a bounded runtime with concrete capabilities, limits, and identity.

These are architectural advantages, with qualification specific to each
execution profile. Docker alone does not establish security or reviewer
independence, and complete engineering integration remains in progress.
See the [Pi/Docker design](planning/s10-pi-docker-convergence-amendment-20261103.md)
and [bounded coding profiles](rust/crates/work-engine-manual-host/docs/pi-coding.md)
for implementation details and established limits.

## Current standing

Status as of November 21, 2026. The repository now contains an integrated
but unselected coding battery checkpoint: the Rust Host and native Pi ran a
scripted, credential-free Responses battery through repeated tool calls, a
real context reset, tool-free rehydration, and settlement. A separate control
refused a failed terminal response without retry. The
[Stage E qualification report](records/pi-coding-battery-stage-e-qualification-20261121.md)
records the exact tested revision, commands, binary and receipt digests, and
what can be independently rerun. This is component qualification, not a live
provider run or acceptance of a produced workspace.

| Area | What exists | Remaining boundary |
| --- | --- | --- |
| Campaign and slice execution | Used to develop this repository: configuration preflight, bounded builders, configured review, gates, private checkpoints, recovery, and terminal receipts. | Standalone installation, release, and operational completion remain roadmap work. |
| Rust lifecycle and hosts | Implemented state, provider/session, task, review, claims, compiler, and publication components, with focused tests and controlled qualification. | Qualification and deployment are profile-specific; these crates do not imply one generally available production runtime. |
| Research lifecycle | Revision-bound research/basis records, register reading and current-use projections, preparation/adoption persistence, native input retention, and the merged causal task-host join. | Producer build provenance, effective tool/access evidence, authentic research validation, and exact installation/execution approvals remain prerequisites. The first real context-policy revalidation, bounded execution, and feedback/baseline comparison remain open (H2 and M4/M5). |
| Pi/Docker coding integration | Host-controlled native Pi with bounded offline Completions and Responses battery evidence: tool effects, large history, reset, and continuation. The current design keeps real provider credentials on the host. | Checked build/test evidence tied to one export, live evidence-size limits, and live route qualification remain open. |
| Hosted review and custody | Unselected exact-subject and dispatch binding, bounded Git snapshot capture, and synthetic reviewer custody controls. | Actual pre-exposure custody and an independently checked evidence reader are not established; no hosted reviewer route has been admitted by this work. |
| Proposal and planning capabilities | Durable packets, formation, decision recording, strategic reconciliation, and bounded claim/review-state capabilities. | The broader evaluation, portfolio, organizational, and learning architecture is only partly implemented. |
| Operator surfaces | Local query/server interfaces and terminal UI components. | Commands, provisioning, and live execution require their own qualified profiles; a read interface supplies no execution grant. |

The research join was merged locally at `dda3f744`. Its
[delivery composition](rust/crates/work-engine-task-host/src/research_delivery.rs)
and [causal retention](rust/crates/lifecycle-store/src/native_causal.rs) are
implementation references. The earlier
[research progress record](records/research-lifecycle-implementation-progress-20261113.md)
provides milestone history, not the standing of every subsequent merge.

The [Pi/Docker convergence amendment](planning/s10-pi-docker-convergence-amendment-20261103.md)
records the integration objective and original component boundaries. Current
controlled work does not close the full engineering vertical or the standalone
release conditions. [ARCHITECTURE.md](ARCHITECTURE.md) maps the broader system;
[roadmap.md](roadmap.md) records strategic direction and remaining outcomes.
Dated records preserve what was established at the time they were written.

## The implemented slice workflow

A configured campaign is one concrete way to use Work Engine:

```text
campaign configuration and preflight
    -> bounded plan and its authorized acceptance
    -> coherent builder implementation
    -> configured review and remediation
    -> deterministic gates and immutable checkpoint
    -> authorized result acceptance
    -> durable terminal receipt
```

The campaign contract selects approval boundaries, review requirements, limits,
and stop conditions. When independence is required, the receipt must establish
its actual provider, isolation, and evidence class. A fallback records its real
standing and authority rather than inheriting the original independence claim.
Review need and reviewer context lifetime depend on the governing contract and
the consequences of the work.

Private checkpoints live under `refs/work-engine/`. User-visible commits and
remote publication are separate authorized consequences. A stopped or failed
slice retains its reason and evidence alongside successful terminals; missing
proof does not become a success-shaped receipt. The
[slice-supervisor contract](skills/slice-supervisor/SKILL.md) owns this workflow.

## Contribute

Work Engine offers concrete systems problems with a working repository to
measure against. Contributions can improve the engine that helps build the
engine, and leave evidence that others can check and extend.

- **Rust runtime and recovery:** make task execution, durable state, and context
  replacement easier to operate and qualify. Start with the
  [lifecycle service](rust/crates/lifecycle-service/README.md) and
  [local server](rust/crates/work-engine-server/README.md).
- **Research and evaluation:** reproduce a finding, test an efficiency
  hypothesis, or challenge an assumption with a bounded comparison. Start with
  the [research map](metrics/register/README.md) and
  [efficiency ledger](metrics/efficiency-ledger.md).
- **Operator experience:** help people understand what an agent is doing,
  what it has established, and what needs their decision. The
  [architecture map](ARCHITECTURE.md) and [roadmap](roadmap.md) show the
  integration work ahead.

A useful first contribution can be a focused regression test, a reproducible
measurement, a clearer operator view, or a repair to one recovery boundary.
Read the [design contracts](DESIGN.md), choose a bounded problem, and make the
result and its limits easy to inspect. Agent contributors also start with
[AGENTS.md](AGENTS.md). The local checks below provide an entry point.

## Explore locally

For the JavaScript workflow tools, use Node.js 22 or newer:

```bash
npm install
npm run preflight       # resolve campaigns/roadmap.yaml
npm run test:supervisor # focused preflight tests
```

The root `npm test` runs the JavaScript supervisor, MCP, Chrome Vision, and
App Server suites. It does not run the Rust workspace tests. Provider-backed
integration tests and actual campaigns have separate environment and authority
requirements.

The Rust workspace declares Rust 1.92 or newer. A focused library test entry is:

```bash
cargo test --manifest-path rust/Cargo.toml -p lifecycle-core
```

Cargo may need to fetch dependencies on the first run. Other crates have their
own features, native dependencies, fixtures, and qualification requirements;
consult the relevant component documentation before running a host or enabling
a controlled profile. Useful entry points include the
[lifecycle service](rust/crates/lifecycle-service/README.md),
[local server](rust/crates/work-engine-server/README.md), and
[review/completion host](rust/crates/work-engine-review-host/README.md).

Actual inference or campaign execution needs a selected coding-agent runtime,
provider access, an authorized work contract, and the applicable runtime
profile. Example configurations and test fixtures are implementation material;
they do not grant deployment, credentials, acceptance, or publication authority.

## Repository map

| Path | Purpose |
| --- | --- |
| [DESIGN.md](DESIGN.md) | Index of binding contracts in `docs/contracts/`. |
| [PHILOSOPHY.md](PHILOSOPHY.md) | Non-normative rationale for the design. |
| [ARCHITECTURE.md](ARCHITECTURE.md), [roadmap.md](roadmap.md) | System map, strategic direction, and remaining work. |
| [design/](design/README.md) | Architecture views, proposals for the future system, and exploratory synthesis with explicit standing. |
| [rust/](rust/Cargo.toml) | First-party Rust workspace and component implementations. |
| [skills/](skills/) | Role and capability contracts, adapters, schemas, and tests. |
| [app-server/](app-server/) | Existing JavaScript runtime, protocols, services, and migration work. |
| [planning/](planning/), [proposals/](proposals/), [reviews/](reviews/), [ideas/](ideas/) | Implementation plans, candidate changes, revision-bound review evidence, and raw ideas. |
| [records/](records/README.md) | Dated decisions, campaign results, and historical qualification records. |
| [experiments/](experiments/) | Bounded experimental implementations and retained evidence. |
| [campaigns/](campaigns/), [metrics/](metrics/) | Declarative campaigns and execution/accounting records. |
| [docs/repository-layout.md](docs/repository-layout.md) | Placement and standing rules for repository artifacts. |
| [docs/model-selection.md](docs/model-selection.md) | Revisable model and effort guidance with provenance and limitations. |

Existing components retain their authority until an accepted replacement takes
over. The Rust-first direction permits simpler replacements while preserving
approved semantics, data integrity, provenance, and qualification evidence.
Internal interfaces have no general backward-compatibility guarantee.

[AGENTS.md](AGENTS.md) is the entry point for agents working in the repository.
[HISTORY.md](HISTORY.md) provides a dated self-audit of the project's development.

## License

MIT. See [LICENSE](LICENSE).
