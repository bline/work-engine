# Repository Instructions

Read and follow [core contracts](docs/contracts/core.md) before repository work.
The [design index](DESIGN.md) routes the rest of the binding doctrine by decision;
[philosophy](PHILOSOPHY.md) is explanatory and non-normative. Scoped documents
state their authority, scope, and loading condition. Their contracts remain binding
when applicable; a link is not an instruction to recursively load every document.

## Loading map

| Situation | Read |
| --- | --- |
| Every new session or context window | This file and [core contracts](docs/contracts/core.md); perform the board read below |
| Resuming unfinished work after restart, reset, or compaction; preparing replacement | Invoke `$wind-walker` and follow [context recovery](docs/contracts/context-recovery.md) before continuing |
| Choosing architecture, semantic ownership, boundaries, or product structure | [System design](docs/contracts/system-design.md) |
| Authoring or reviewing agent instructions, skills, or loading rules | [Instruction design](docs/contracts/instruction-design.md) |
| Selecting or relying on evidence, writing receipts, defining route-sensitive acceptance | [Evidence and claims](docs/contracts/evidence-and-claims.md) |
| Coordinating agents, conducting review, retaining or replacing context | [Context and review lifetimes](docs/contracts/context-and-review.md) and the active role's skill |
| Coordinating agents or shared resources; an active human-supervised role with a configured durable scheduler | [Repository coordination](docs/agent-coordination.md) |
| A decision needs the reasoning behind a contract | The relevant topic from [PHILOSOPHY.md](PHILOSOPHY.md) |

Load applicable task and role references before the decisions they govern. Multiple
rows may apply. Bounded implementation and validation do not require the full design
or rationale corpus. Design and instruction review need the scoped doctrine and
relevant rationale, rather than every topic. Role and campaign contracts continue
to own their specific obligations and acceptance conditions.

## Shared-session awareness

Concurrent sessions share files, not conversation context. At startup, after context
recovery, before claiming a shared resource, and before paid or externally
consequential operations, read the board:

```bash
python3 skills/durable-state/scripts/codex_chatboard.py --repository . read --limit 20 --active-claims
```

Use [repository coordination](docs/agent-coordination.md) before claims, launches,
transitions, or resource mutation. Claims are advisory coordination, not authority.

## Context pressure: every role

At startup and after recovery, establish delivery of this session's context
pressure through an owned chatboard `wait` with a verified context binding and
output collector. This applies to every active role, including standalone roots,
workers and reviewers: a board-topic subscription alone produces no context
telemetry, and an uncollected alert cannot inform preservation before rollover.
Use [session control](skills/manual-orchestration/references/context-control.md)
when establishing or repairing delivery. If the route is unavailable, record
the gap and own an explicit check before the next substantial phase; unknown
headroom is not spare capacity. Existing authority and collection owners remain.
When a reading calls for same-session replacement, the host's native control
(in Pi, `new_context`) discards history without a summary, so preserve
meaning first through `$wind-walker`;
[native recovery](skills/manual-orchestration/references/native-context.md)
owns each host's invocation.

## Codebase Memory

Codebase Memory (`codebase-memory-mcp`) holds indexed code graphs of
repositories: symbols, callers and callees, imports and change impact. A graph
query can answer a structural question for a fraction of the context that
reading files to the same answer costs, for a builder as much as for its
parent. Source files remain the authority for exact text and for anything the
index has not covered. The [repository-search contract](skills/repo-search/SKILL.md)
owns evidence tiers, coverage and fallback; the `codebase-memory` skill
describes the tools.

The main checkout and each worktree are separate projects, and an index can
lag its tree, so a result from the wrong project or generation describes other
source. Check the project root and `index_status` before relying on results,
and `check_index_coverage` for cited paths. `list_projects` pages its output.
In Pi, the core tools are declared as `mcp__codebase_memory__<tool>` and the
rest are callable from `codemode` scripts; a Pi subagent gets them only when
its agent's `tools` list names `mcp:codebase-memory` entries
([Pi host](skills/manual-orchestration/references/pi-host.md#pi-as-the-supervising-host)).

## Shared build directory

Operator decision 2026-11-20: where the user-level `~/.cargo/config.toml` sets
`build.target-dir` (it does on the operator's laptop), every worktree builds
into that one directory, so Cargo's lock runs builds one at a time and disk
stays bounded; a lock wait is expected. Do not point a build elsewhere with
`CARGO_TARGET_DIR` or `--target-dir`. The pinned-tool provisioning scripts are
the exception: they build privately so context observation never waits on that
lock. Binaries there belong to whichever worktree built last: copy one out and
pin its digest before relying on it. `/tmp` is a small RAM-backed tmpfs with a
per-user quota shared by every session; filling it fails other sessions' writes
(`EDQUOT`), so keep builds and large scratch output out of it.

## Pre-release compatibility policy
Work Engine is under active development. Internal implementations and
interfaces have no backward-compatibility guarantee unless an explicit consumer
or owner-approved contract requires one.  Prefer the simplest architecture that
satisfies current requirements and the intended system design.  Existing code
is reusable implementation material, not architectural authority. It may be
modified, replaced, or removed.  Preserve approved semantics, data integrity,
provenance, and qualification evidence—not obsolete implementation behavior.
Any recommendation to preserve compatibility must identify the specific
consumer, the consequence of breaking it, and the cost of maintaining it.

## Model-selection guidance
[Model selection](docs/model-selection.md) provides current defaults and their
tradeoffs when choosing an engineering model. It is decision guidance, not an
override of explicit user choices or campaign/provider contracts; consult it
when that choice is in scope, rather than inferring a selection from inheritance.

## Rust development direction

[Rust development](docs/rust-development.md) records the user's Rust-first
direction for new first-party implementation and proposed shared-code boundaries.
Use it when choosing implementation language or reusable package placement;
existing components and their authority remain until accepted replacement.

## Where files go

Readers judge a file's standing from where it sits. Put a new file where
[repository layout](docs/repository-layout.md) says, and follow its rules when
moving or citing one.
