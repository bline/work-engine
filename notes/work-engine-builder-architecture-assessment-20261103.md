# Assessment of the development system as the Work Engine architecture

Recorded 2026-11-03 for the maintainer, following the request to compare the
repository's architecture, Rust crates, root workflows and `devwits-auth`.
This is a dated architectural assessment. It records findings and a proposed
direction; it does not accept a replacement design or authorize implementation,
deployment, state migration or retirement of an existing owner.

**The system building the Rust implementation is already an operational
prototype of much of Work Engine. It is a credible basis for the product's
operating architecture, with substantial integration and qualification still
ahead.**

Here, the development system means the evolving root orchestration:
model-directed decomposition, skills, the coordination board, durable
assignments, context management, budget observation, native agent adapters and
the Rust `work-engine-manual-host`. The original workflow system has evolved
toward the intended product while building its replacement.

The strongest architectural result is the division of work. Models interpret
objectives, form bounded assignments, choose perspectives and revise routes.
Deterministic machinery handles observations and execution facts. Durable
artifacts preserve decisions, authority and results across sessions. This
corresponds closely to the model judgment, context economy and determinism
principles in [DESIGN.md](../DESIGN.md), with their rationale in
[PHILOSOPHY.md](../PHILOSOPHY.md).

| Intended capability | Existing development machinery | Remaining boundary |
| --- | --- | --- |
| Compose an organization around an objective | The root forms bounded assignments, selects builders and separate reviewers, retains useful contexts and revises the organization. | Organization candidates remain model-authored artifacts without an executable admission consumer. The example JSON has no mandatory schema. See [organization formation](../skills/manual-orchestration/references/organization-formation.md). |
| Spend inference on judgment and automate settled mechanics | The Rust manual host handles dispatch, observations, exact-turn collection, artifact capture and receipts, including real review workloads. | It calls a pinned Python adapter bundle and leaves decomposition, model selection, acceptance and integration with the root. See the [host contract](../skills/manual-orchestration/references/manual-host.md). |
| Preserve meaning across replaceable contexts | Continuations, context observations, same-session recovery and ownership records exist. | The outer helpers and Rust lifecycle remain separately qualified systems. See [outer continuity](../skills/manual-orchestration/references/same-session-context.md). |
| Preserve distinct evidence and decision owners | Root workflows and Rust domains distinguish review findings, claim establishment, acceptance and publication. | Their complete production composition remains unfinished. |
| Give the human an observable authority surface | `devwits-auth` supplies task projections, a mailbox and signed owner answers. | Its UI deployment does not establish qualified backend execution. The mailbox remains transport rather than decision authority. |

The current approach also supplies evidence for simplifying the planned
organization. The [supervisor contract](../skills/manual-orchestration/references/supervisor.md)
uses one top-level supervisor dispatching builders and separate reviewers,
explicitly motivated by the inference cost of additional supervisory layers.
That is relevant operational evidence when reconsidering the more elaborate
planning hierarchy in the
[planned architecture](../design/work-engine-planned-architecture.md).
It supports reevaluating the topology; it does not establish that every
workload benefits from the same topology or authorize changing an accepted
contract.

**The Rust implementation contains substantial domain machinery.** Representative
paths establish more than package scaffolding:

- **Lifecycle and retained tasks:** the task host composes real task-owner
  commits with lifecycle admission, replacement requests and recovery-only
  behavior. The lifecycle plan records installed native execution against an
  offline provider fixture, including cuts around owner commit and restart
  without provider replay. S9 and full S10 acceptance remain open. See
  [task dispatch](../rust/crates/work-engine-task-host/src/lib.rs) and the
  [current qualification boundary](../experiments/context-lifecycle-rust/IMPLEMENTATION_PLAN.md#s9--fresh-rust-development-startup-and-deferred-historical-migration).
- **Review and evidence:** `review-workflow` composes campaign, episode, claim
  and execution-evidence owners. Its initial join reads through those owners
  rather than accepting a caller's success assertion. Installed-provider
  qualification remains narrower than a completed live review. See the
  [review join](../rust/crates/review-workflow/src/initial.rs) and
  [RP3 result](../records/rust-review-provider-rp3-validation.md).
- **Publication:** W1 implements checkpoint acceptance, publication offers,
  guarded Git updates and crash reconciliation through Rust owners. Its
  demonstrated authority profile is an offline fixture profile, and its result
  identifies a remaining standalone-binary episode-read boundary. See the
  [W1 result](../records/rust-workspace-w1-result.md).
- **Compilation:** deterministic skill and extension compilation exists.
  `work-engine-compiler` packages and checks authored inputs; it does not
  implement the model-directed organizational compiler described in the
  architecture. See the [C4 result](../records/rust-compiler-c4-result.md).

**The main gap is joining the outer development system to those Rust owners.**
The outer system is operational and adaptable. The Rust domains provide
explicit ownership, transactional state and recovery mechanisms. They have
not yet become one runtime for the complete workflow.

The manual host's
[dependency manifest](../rust/crates/work-engine-manual-host/Cargo.toml)
has no direct dependency on the Rust lifecycle, campaign, review or publication
crates. Its
[implementation](../rust/crates/work-engine-manual-host/src/lib.rs)
binds requests, journals uncertain create/start effects and collects exact
native turns through its adapter bundle. Its contract explicitly leaves the
product lifecycle/task/review hosts separate. These are concrete reasons to
treat it as a promising composition point whose current existence does not
establish the full integration. Absence of direct dependencies alone would not
prove that no protocol integration exists.

`devwits-auth` contributes an important operator boundary. Its local
`HANDOFF.md` records the deployed UI and mailbox, a historical signed-answer
round trip, and the narrower scope of the latest production browser check.
`laptop/owner_answer.py::verify_answer` verifies exact answer bytes, the named
request, a pinned approval key and WebAuthn evidence;
`laptop/approval_sync.py::pull` verifies before posting a board reply. A Rust
execution consumer still needs its own current-authority and effect-admission
checks. The UI can remain a projection and authority surface while those
decisions enter the existing semantic owners.

**The cost architecture is promising; net savings remain unestablished by the
comparisons examined.** The
[bookkeeping-host pilot](../metrics/deterministic-bookkeeping-host-pilot-20261102.md)
observed five root lifecycle calls in both arms. Root-plus-child cost was
$1.268571 manually and $1.377917 hosted, using the repository's speed-adjusted
API-equivalent cost proxy. The small, unmatched comparison establishes no
advantage. The
[ordinary-use observation](../metrics/bookkeeping-host-ordinary-use-20261102.md)
then exposed root-written inspection programs and receipt-consumption friction.
The November 3
[ordinary-use repairs](../records/manual-host-ordinary-use-repairs-result-20261103.md)
and [receipt repairs](../records/bookkeeping-host-repairs-result-20261103.md)
address several of those problems without claiming measured campaign savings.

A separate
[efficiency observation](../metrics/manual-host-efficiency-20261103.md)
landed while this assessment was being saved. Across unequal review workloads,
its current-host sample has fewer root-authored Python bodies and fewer
associated root calls than its immediate predecessor. That strengthens the
narrower evidence for reducing generated glue. The retrospective does not
establish causal time or cost savings against manual orchestration. It is a
later measurement supplement to the original assessment, not a matched trial.

The resulting maturity assessment is:

| Area | Assessment |
| --- | --- |
| Operating approach | Demonstrated on real engineering work. |
| Rust domain foundations | Substantial implementation with meaningful bounded qualification. |
| Unified recoverable runtime | Partial. |
| General organizational execution | Supplied largely by model judgment and skill contracts. |
| Economic benefit | Narrow evidence for less generated glue; end-to-end net benefit remains unestablished. |

**The proposed direction is to join the current operating approach to the
existing Rust owners.** Decomposition, architectural judgment and semantic
acceptance can remain with their authorized model or human owners. Dispatch,
delivery, custody, observation and recovery need explicit runtime ownership.
The existing domain boundaries provide a basis for that composition without
turning the manual host's operational registry into a second owner of campaign,
lifecycle, claim or publication meaning.

A decisive next demonstration would use one representative engineering task:

1. Begin with an approved objective and a root-formed organization.
2. Execute through the Rust task, lifecycle, review and evidence owners.
3. Replace the supervising conversation while child work remains outstanding.
4. Recover the exact authorization, results and uncertain effects without
   duplicate execution or a weakened acceptance condition.
5. Complete the applicable human decision and publication boundary, and measure
   total coordination, implementation, review, repair and recovery cost against
   a comparable route.

That would demonstrate the development system becoming the product. The
implemented machinery is sufficient to pursue this convergence now. A single
completion percentage would hide the integration and qualification work that
still determines whether the replacement is usable.

The original assessment used the working tree observed at Work Engine HEAD
`afc3be261b0124c3800f3ad82807c7d0625e53d8`. The save-time parent is
`332cf63b4462e52a4b4b75431a7c6c91439d028f`, which adds the later efficiency
observation. External `devwits-auth` sources were read at local HEAD
`119c8669c0d5bccd1acc613eeb14566a057132cc` under
`/home/bline/code/devwits-auth`. Some observed Work Engine source, including W1
publication and completion-host files, was uncommitted; this note's commit does
not publish or qualify that implementation. The conclusions use current-source
inspection and existing dated qualification records, without a new runtime
qualification or an exhaustive audit of every crate.

Repository retrieval used Tier 2 graph-assisted verification for project
`home-bline-code-work-engine`, store generation `ud92146f74df51684g2`, with
index metadata dated `2026-11-03T13:41:09Z`. Relevant symbol pages and source
snippets were inspected. An ambiguous cross-crate call trace was replaced by
direct source and dependency-manifest inspection. Coverage checks returned no
recorded gaps for the checked Work Engine evidence paths; that best-effort
signal does not establish completeness. `devwits-auth` was not indexed and was
read directly. Qualification results cited above retain their own source,
profile, authority and uncertainty boundaries.
