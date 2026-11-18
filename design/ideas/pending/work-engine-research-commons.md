# Work Engine Research Commons
## A Distributed, Continuously Qualified Intelligence Network for Agentic Engineering

**Status:** Concept proposal — not an approved implementation design. Deferred until Work Engine has outside interest and measured local results.  
**Date:** November 17, 2026  
**Project:** Work Engine

---

## 1. Executive Summary

Work Engine has the potential to support a member-supported research cooperative: a research network, funded and directed by its contributors, dedicated to continuously improving the efficiency, quality, reliability, and economics of agentic software engineering.

The near-term goal is modest: **sustain one full-time maintainer through transparent, member-supported funding.** Forming a legal organization, adopting a charter, and growing a team of maintainers and researchers are possible later developments, not commitments.

The open-source Work Engine provides the engineering machinery: planning, compilation, deterministic orchestration, execution, evidence collection, independent verification, and policy enforcement.

A separate, member-supported intelligence service coordinates distributed research, maintains qualified findings, determines their continued applicability, and distributes research-backed decision improvements.

Members and other users execute Work Engine locally or on infrastructure of their choosing, using their own authorized model providers and inference resources. The intelligence service does not host anyone's development workloads.

Participating installations can contribute bounded inference capacity and eligible coding tasks to centrally coordinated research programs. Those contributions generate experimental evidence that benefits the wider network.

Members support the research through dues, research contributions, or both. Contributions are recorded in a transparent ledger, and members become more vested the more they contribute. Earlier access to new releases is one possible membership benefit; established research eventually becomes available to everyone.

**The core proposition is that participants collectively finance and perform research whose findings improve the engineering work they already undertake.**

The Commons is sustained not by permanent secrecy around individual discoveries but by its ability to continuously generate, qualify, maintain, and operationalize useful engineering knowledge.

---

## 2. Foundational Principles

### 2.1 Open engineering machinery

The Work Engine compiler, executor, research integration mechanisms, policy interfaces, and qualification infrastructure remain open source.

Users must be able to operate the engineering system independently of the intelligence service.

They may conduct their own experiments, maintain their own research findings, and apply independently generated decision policies.

The Commons must not depend on deliberately crippling the open-source version.

### 2.2 Knowledge becomes broadly accessible

Qualified research improvements advance through release tiers.

Paid members receive earlier access, while free members eventually receive the same established research releases.

Member support funds research freshness and ongoing research operations, not permanent exclusion from accumulated knowledge.

Material corrections, withdrawals, and validity updates are not artificially delayed for lower tiers.

### 2.3 Research participation is bounded and consensual

Participants explicitly authorize how much of their inference capacity and eligible development activity may be used for research.

Participation cannot silently consume resources, expand permissions, or alter the authoritative production outcome.

Research incentives should reward useful, verifiable participation rather than merely the number of tokens consumed or positive experimental findings.

### 2.4 Scientific integrity precedes institutional advantage

Research must preserve unsuccessful experiments, negative results, uncertainty, reproducibility requirements, and applicability boundaries.

A finding is not treated as universally applicable merely because it passed an initial experiment.

The research organization must be capable of withdrawing, narrowing, or reversing previously published conclusions.

### 2.5 Central coordination does not imply central execution authority

The intelligence service selects research priorities and proposes assignments.

Local Work Engine installations retain control over admission, execution, resource limits, privacy, and acceptance of research work.

The central service cannot override local execution authority.

---

## 3. The Two-Part System

### 3.1 Open-source Work Engine

Each user operates an independent Work Engine installation.

The local system is responsible for:

- Selecting and executing ordinary development work.
- Compiling plans and applying authorized decision policies.
- Managing inference providers and execution environments.
- Enforcing resource grants, budgets, and execution boundaries.
- Evaluating local research eligibility.
- Running authorized experimental comparisons alongside production work.
- Collecting and validating experimental evidence.
- Submitting permitted research results to the network.

A local installation must remain capable of normal development without a connection to the intelligence service.

### 3.2 Central intelligence service

The central service operates the shared research program.

Its responsibilities include:

- Maintaining a global research agenda and project backlog.
- Tracking active experiments, outstanding evidence requirements, and qualification status.
- Matching research opportunities to eligible local coding slices.
- Assigning experimental treatments and comparison cohorts.
- Receiving and validating contributed evidence.
- Maintaining research claims, revisions, applicability, and dependency records.
- Monitoring changes affecting established findings.
- Scheduling research discovery and requalification.
- Publishing immutable research and decision-policy releases.
- Managing memberships, entitlements, contribution accounting, and release access.

A lightweight serverless architecture, potentially using Cloudflare Workers, D1, R2, and asynchronous processing, could support much of this coordination.

More computationally demanding research qualification and analysis could operate in separate workers or servers when necessary.

**The central service is a research coordinator and intelligence distributor, not an infrastructure provider for anyone's coding sessions.**

### 3.3 Intelligence API boundary

The Intelligence API should be a new adapter and coordination layer over Work Engine's existing research capabilities, not a new research engine and not a cloud-hosted task host.

A candidate decomposition:

- **Portable intelligence core:** A narrow shared Rust library (conceptually `work-engine-intelligence-core`) that composes existing domain libraries rather than moving their implementations. It owns the portable intelligence-facing contracts: validated research references, claim revisions, applicability, experiment descriptions, evidence envelopes, release identities, compatibility checks, and cryptographic verification. It should compile both natively and to Wasm.
- **Server adapter:** Authentication, research coordination, publication, storage, and memberships.
- **Local client adapter:** Retrieve releases, check applicable protocols, submit explicitly permitted evidence, and hand verified policy inputs to the existing local owners.

The client adapter does not decide whether a remote recommendation grants execution authority. Local admission retains that responsibility.

**A remote response must never turn an attributed scientific assertion into authenticated execution permission.**

An initial interface could be small and read-only, serving existing Git-pinned research as immutable, content-addressed artifacts. Illustrative surface (not implemented routes):

| Endpoint | Purpose |
|---|---|
| `GET /v1/releases` | Discover published, versioned releases |
| `GET /v1/claims/{id}` | Retrieve exact claim revisions and evidence |
| `POST /v1/applicability` | Find qualified, applicable research candidates |
| `GET /v1/changes` | Discover corrections and withdrawals |
| `GET /v1/bundles/{digest}` | Fetch immutable release artifacts |

Eligibility, assignment offers, evidence submission, and qualification results would follow later. Signed manifests are required before release delivery becomes an authoritative policy distribution mechanism.

### 3.4 Hosting substrate hypothesis

Cloudflare Workers supports Rust through `workers-rs`, compiled to `wasm32-unknown-unknown`. A plausible mapping:

| Service need | Candidate |
|---|---|
| HTTP endpoints, authentication, publication, eligibility | Workers (Rust) |
| Studies, claim revisions, applicability, memberships, assignments | D1 |
| Content-addressed evidence packets, manifests, signed releases | R2 (strong read-after-write consistency; conditional create-only writes) |
| Serialized per-experiment authoritative state | Durable Objects |
| Requalification scheduling, evidence processing | Queues and Cron Triggers |

Workers KV is eventually consistent and should not hold authoritative research state.

Workers execute Wasm, not a Linux process environment. Native filesystem, `libc`, Git, SQLite (`rusqlite`), and process management do not move over. That is consistent with the design: the service never executes member workloads. It does mean research crates need a split between a portable domain core and environment adapters (native Git/SQLite for local Work Engine; D1/R2 for the service). Long-running maintenance work should run as queued, durable operations rather than inline requests, and the service should avoid loading whole research archives into memory.

Platform limits, pricing, and binding coverage were reported in an external conversation on November 17, 2026 and must be rechecked before relying on them.

---

## 4. Distributed Research Execution

Ordinary coding activity creates opportunities for experiments.

A user starts a development slice under the currently selected production policy. Work Engine determines whether that slice satisfies any permitted research qualifications.

If participation is enabled, the installation may offer a privacy-constrained eligibility description to the central service.

The central research coordinator evaluates the opportunity against its research backlog.

It considers:

- Research priority and expected information value.
- Required task and environmental characteristics.
- Existing evidence coverage.
- Experimental cohort balance.
- Provider and model availability.
- Participant contribution limits.
- Local privacy restrictions.
- Experimental cost and operational requirements.

The coordinator may return a versioned, authenticated research assignment.

The local engine independently validates the assignment and decides whether to admit it.

### Production and research separation

The user's selected production execution remains authoritative.

Research can execute alternative configurations alongside that work, subject to explicitly authorized resource budgets.

For example, the production path might use one model at high reasoning effort while research compares a different model or effort level on the same qualified task.

The experimental result does not silently replace the production result.

The research execution produces evidence for evaluation, not an alternative production authority.

### Research contracts

Research assignments should describe bounded, declarative experiments rather than arbitrary remote commands.

Each contract should identify:

- Research project and immutable protocol revision.
- Task applicability criteria.
- Experimental treatment and comparison requirements.
- Measurement and acceptance criteria.
- Maximum permitted inference and execution resources.
- Data-disclosure requirements.
- Required evidence and provenance.
- Cancellation, uncertainty, and failure handling.

The existing Work Engine direction toward deterministic admission, contracts, independent review, and durable evidence is directly relevant.

Multiple research projects might share an eligible coding slice when their requirements are compatible. However, shared execution must not compromise experimental independence, accounting, or treatment validity.

---

## 5. Continuous Research and Requalification

A central assumption of this proposal is that **engineering research findings are not permanently valid**.

Models change. Providers update inference systems. Caching behavior evolves. Pricing changes. Model routing and infrastructure may change without public announcements.

A previously qualified optimization can become inferior, inapplicable, or misleading even when its model identifier remains unchanged.

Therefore, every qualified research finding must participate in an ongoing evidence-maintenance lifecycle.

### 5.1 Findings have dependencies

A research claim should retain its relevant dependencies, such as:

- Model and provider characteristics.
- Inference configurations.
- Prompt and context construction.
- Execution and compilation behavior.
- Task characteristics and applicable workload populations.
- Pricing, caching, latency, and reliability assumptions.
- Underlying experimental and measurement methods.

Dependencies may be shared by multiple claims.

When a dependency changes, the research system identifies affected findings and determines what must be requalified.

### 5.2 Adaptive revalidation

Revalidation frequency should reflect observed rates of change, uncertainty, usage, and the consequences of an incorrect decision.

Rather than imposing one universal schedule, the system learns from historical records.

A component exhibiting frequent behavioral drift may require regular retesting.

A stable component with strong evidence may require less frequent checks.

An announced provider change, unexpected performance shift, or contradictory outcome may trigger immediate investigation.

Sparse evidence must not be interpreted as evidence of stability.

Revalidation scheduling should combine:

1. **Periodic checks** based on observed historical change rates.
2. **Event-triggered checks** when relevant dependencies change.
3. **Drift-triggered checks** when real-world outcomes diverge from qualified expectations.
4. **Risk-triggered checks** for findings with high operational consequences.
5. **Integration checks** when individually qualified components are combined into decision policies.

A shared dependency revalidation may support several findings, provided the evidence genuinely satisfies their requirements.

### 5.3 Claim lifecycle

A qualified claim may subsequently be:

- Reaffirmed by new evidence.
- Narrowed to a smaller applicability domain.
- Revised with updated estimates.
- Superseded by a stronger finding.
- Suspended pending further evidence.
- Withdrawn when contradicted or invalidated.

These transitions should be explicit, revisioned, and auditable.

The system must distinguish the historical fact that a claim was qualified from the current determination that it remains applicable.

### 5.4 Requalification becomes a permanent research obligation

Every promoted finding creates future maintenance requirements.

The research scheduler must balance two forms of work:

**Discovery research:** Identify new improvements and answer previously unresolved questions.

**Maintenance research:** Confirm that established findings continue to justify the decisions they influence.

Research priority should consider not only possible improvements but also the expected cost of applying stale or incorrect intelligence.

This continuing maintenance obligation is a central reason for the intelligence service to exist.

---

## 6. Decision Intelligence and Plan IR

The long-term architecture anticipates a change in where research improvements are applied.

During early development, research frequently identifies missing architectural capabilities and necessitates changes to the open-source engine.

As Work Engine's compiler, execution system, and Plan IR become sufficiently expressive, more improvements should be representable as decision-policy changes.

Examples include:

- Model and provider selection.
- Reasoning-effort allocation.
- Context lifecycle and retrieval strategy.
- Cache-affinity and cost optimization.
- Task decomposition and execution strategy.
- Review depth and independent verification.
- Recovery policy and escalation thresholds.
- Selection among qualified execution alternatives.

The intelligence service can publish research-backed policy revisions without requiring a new engine implementation for every discovery.

The compiler and executor remain responsible for enforcing the resulting decisions.

Research findings must retain their evidence provenance and applicability requirements. The decision layer must not convert provisional or stale findings into unjustified authority.

### Policy delivery

Two delivery mechanisms are plausible:

**Signed policy bundles:** The service publishes versioned, content-addressed policy artifacts that local installations validate and execute.

**Remote policy evaluation:** The service evaluates applicable research and returns bounded, evidence-backed recommendations or policy selections.

A hybrid may eventually be appropriate.

Whichever mechanism is adopted, the system should preserve local authority, reproducibility, offline operation where appropriate, and the ability to identify exactly which policy revision governed an execution.

---

## 7. Membership and Research Release Tiers

*This section is one candidate funding model, retained for reference. It predates the vested-membership framing (§8) and may be simplified or dropped; tiered release access is not required by the Commons.*

The candidate model has three intelligence membership tiers, alongside unrestricted standalone use of the open-source engine.

### Tier 1 — Frontier

Receives the newest qualified research and decision-policy releases.

Members pay the highest subscription price.

They may voluntarily contribute eligible experimental capacity in exchange for a membership discount.

### Tier 2 — Established

Receives the preceding qualified research release when Tier 1 advances.

Members pay a lower subscription price.

They may also participate in research for a discount.

### Tier 3 — Community

Receives older qualified research releases after the defined release progression.

This tier is free of subscription charges and is intended to support bounded, consented research contribution.

Contribution expectations must be transparent, subject to user-controlled limits, and fair when no suitable experiments are available.

### Standalone open source

The complete open-source engineering machinery remains usable without membership or research participation.

Users may operate independently, implement local policies, and conduct their own research.

### Illustrative pricing hypothesis

An initial candidate is:

| Membership | Base monthly price | Example research discount |
|---|---:|---:|
| Community | $0 | Not applicable |
| Established | $9 | $6 effective price |
| Frontier | $24 | $18 effective price |

These amounts are exploratory, not final pricing decisions.

They require validation against the funding target (§8), member savings, willingness to contribute, and contribution economics.

Organization memberships could be introduced later for centralized administration, shared policy management, and enterprise research participation.

### Fairness guarantee

Research access is delayed, not permanently withheld.

Validity corrections, critical withdrawals, and necessary safety updates propagate without artificial tier delays.

A free member may have older decision capabilities, but should not be given knowingly false validity information about those capabilities.

---

## 8. Cooperative Research Economics

The model separates the cost of performing engineering work from the cost of coordinating and maintaining research knowledge.

### Member-supplied resources

Participating users supply:

- Their own inference-provider access.
- Their own execution infrastructure.
- Authorized inference capacity for experimental work.
- Eligible real-world coding tasks.

Their contribution limits are enforced locally.

Provider terms must permit the intended use of inference capacity. Subscription quotas cannot be assumed to authorize distributed research use.

### Centrally funded resources

Member support pays for:

- API and coordination infrastructure.
- Research metadata and evidence storage.
- Research selection and program management.
- Qualification and statistical analysis.
- Security, privacy, and operational controls.
- Publication and entitlement services.
- Maintainer time (initially one full-time maintainer) and, if the Commons grows, researcher compensation.
- Any centrally funded validation experiments.

The distributed network reduces the need to purchase every experimental workload centrally, but does not eliminate the cost of rigorous research.

### Funding objective

The initial objective is to cover infrastructure and one full-time maintainer salary, not to generate profit.

Illustratively, at roughly $150k per year including taxes, infrastructure, and platform fees, that requires about 1,400 supporting members at $9 per month, about 830 at $15, or about 520 at $24. These figures are planning arithmetic, not validated estimates.

The Commons is sustainable when members' measured savings and improvements clearly exceed what they contribute.

Possible funding channels, usable before any distributed network exists:

- Recurring member support through a fiscal host with public accounting, such as Open Source Collective via Open Collective, or GitHub Sponsors. A fiscal host can receive funds and pay the maintainer without a separate legal entity.
- Open-source infrastructure or research grants (for example NLnet or the Sovereign Tech Fund; eligibility unverified).
- Published, maintained research findings as the visible output members support, before distributed experiments exist.

Supporting a team of maintainers and researchers is a possible future, decided only if the Commons and its work justify it.

### Contribution accounting

Research contributions should be valued according to verified participation and useful evidence coverage.

The system should not incentivize unnecessary inference consumption or reward only favorable findings.

Incomplete experiments, negative outcomes, and documented refusals may provide important information.

Discounts should be bounded so that research participation cannot inadvertently create unsustainable liabilities.

### Vested membership and transparent accounting

Members are participants, not clients. The more a member contributes, the more vested they become.

The contribution ledger is fully transparent:

- **Append-only and public.** Each entry is a contribution receipt referencing the evidence or support it represents.
- **Recomputable.** Any member's vesting is a deterministic projection of ledger receipts under a published, versioned valuation rule, not an assigned number.
- **Explicit revisions.** Valuation-rule changes are new revisions, and the ledger records which revision produced each standing.
- **Privacy-preserving.** Public entries carry contribution class, value, and evidence digest, not task descriptions or project details (§9).

Financial income and expenses, including the maintainer salary, are equally public.

Transparency makes valuation disputes auditable, not absent. Open questions:

- What vesting confers: governance voice only, or also an economic share.
- Whether vesting is permanent, decays, or combines both.
- Whether valuation-rule changes re-score past contributions.
- Whether financial support counts as contribution alongside research capacity and evidence.
- What legal form applies if vesting ever carries economic rights; cooperative and securities law may apply and require legal review.

**Pre-charter commitment.** A charter is needed only if the Commons grows beyond one maintainer. Before the first outside contribution, publish a short commitment: contributions are recorded now under published rules; any future charter will honor recorded standing; and what standing currently does and does not confer.

---

## 9. Privacy, Trust, and Research Integrity

The distributed research model depends on a strong separation between participant-owned work and the shared research corpus.

The central service should collect the minimum information necessary for eligibility determination and qualification.

Private source code, secrets, proprietary artifacts, and session transcripts should not be uploaded by default.

Even seemingly harmless metadata can reveal sensitive project information and must be handled accordingly.

Research contracts need explicit disclosure rules, and participants must understand how contributed evidence and derived findings may be used, including any licensing to third parties.

Submitted evidence should include sufficient integrity information for qualification while acknowledging the limits of independently verifying activity performed on machines controlled by participants.

Research qualification must account for:

- Task selection bias.
- Nonrandom participation and missing outcomes.
- Experimental contamination.
- Provider and environment differences.
- Reproducibility limitations.
- Fraudulent or manipulated submissions.
- Multiple testing and premature conclusions.
- Differences between measured proxy outcomes and real engineering quality.

The network must not treat large evidence volume as an automatic substitute for sound experimental methodology.

---

## 10. Research Organization and Governance

Initially, the maintainer directs research priorities using the existing Work Engine research and claims infrastructure.

Whether the Commons ever moves to a team, with maintainers and researchers vetting and planning research portfolios, is an open question rather than a plan.

Potential research areas include:

- Inference efficiency and provider economics.
- Context engineering and memory management.
- Planning and execution optimization.
- Software quality, verification, and security.
- Experimental design and research qualification.
- Long-term engineering reliability and maintainability.

Researchers select valuable questions, design protocols, establish acceptance requirements, and interpret evidence.

The distributed system handles much of the deterministic scheduling, execution coordination, evidence collection, and publication workflow.

### Cooperative governance

The Commons may follow cooperative principles without becoming a legally constituted cooperative. A charter and formal organization become relevant only if the Commons grows beyond one maintainer.

Over time, members could gain structured input into research priorities through advisory groups, proposals, or elected representation.

Research selection should remain accountable to scientific value and member outcomes.

Researchers should be rewarded for reducing uncertainty and producing dependable knowledge, including findings that invalidate attractive or popular assumptions.

### Community research board

Members could influence the research agenda through a visible board of research proposals. Each proposal card would show the hypothesis, estimated research budget, research type, target benefit, required contributions and slice qualifications, supporting evidence, competing hypotheses, and prior results. Once research begins, the same card becomes its progress record; on completion it links to qualified findings and resulting policy releases.

The board is an entry point into the existing research lifecycle (proposal evaluation, portfolio decision, research register, experiment control), not a separate popularity contest.

Governing separations:

- **Members vote on what is worth investigating. They do not vote on what is scientifically true.** Qualification still depends on evidence, independent review, and established acceptance criteria.
- **A mandatory maintenance budget is reserved.** Requalification of drifting or contradicted claims does not have to win a vote.
- **Money does not buy voice by default.** Voting weight follows verified research contribution; whether financial support also counts is an open question (§8).

Candidate weighting: every member receives a baseline vote, increased by verified contribution with diminishing returns and a cap. One illustrative form is `1 + sqrt(contribution_points)`, capped at 5×; the exact function requires governance testing.

Contribution points reward verified research value, not tokens spent. Invalidating an incorrect optimization, reproducing an existing result, identifying an unknown limitation, or supplying a missing workload population all earn credit. Rewarding consumption would incentivize burning inference to buy influence.

The board also yields a live demand and feasibility signal: which questions members care about, which members can actually supply qualifying workloads, and where existing evidence is already sufficient to start an experiment.

---

## 11. Intellectual Property and the Research Commons

The original concept centered on proprietary research results feeding decision policies.

The cooperative model refines that distinction.

The Commons may keep frontier research, aggregation methods, decision-selection intelligence, and publication infrastructure unreleased for a period.

However, qualified releases ultimately move toward community availability under defined terms.

The Commons' durable value is therefore not exclusive possession of any single discovery.

It is the cumulative capability to:

- Identify valuable research questions.
- Coordinate appropriate experiments across a participating network.
- Establish trustworthy evidence.
- Detect when findings become stale.
- Convert qualified results into better decisions.
- Deliver those improvements continuously and reliably.

This advantage can grow through research history, operational expertise, trusted participation, and the quality of the research process.

A formal IP and licensing policy remains necessary. Ownership of contributed evidence, permissible use of derived results, release licenses, and publication commitments should be resolved before outside members contribute.

---

## 12. Development Sequence

This proposal does not require immediate implementation. It begins only after Work Engine has outside interest and measured local results.

The development sequence should preserve Work Engine's existing priorities.

### Phase A — Prove the local engineering system

Complete and qualify the local development vertical.

Establish reliable execution, evidence collection, independent verification, and admission enforcement.

### Phase B — Prove policy-driven improvement

Demonstrate that qualified research can become executable decision intelligence.

Show improvements on independent workloads, not only on the experiments that produced the findings.

### Extraction test (may run independently of Phase A)

Before committing to a standalone service, test the extraction boundary narrowly:

> An independent process publishes an existing qualified research record, and a separate Work Engine installation consumes, validates, and reconstructs it without sharing runtime authority or filesystem state.

Ingredients: an existing Git-pinned research record with its assessment and applicability data; a small Rust service linked against `research-register` interfaces; an HTTP response carrying a versioned immutable bundle; a separate local Rust consumer that verifies the artifact against the pinned source and preserves unresolved or unqualified status. Negative tests cover altered bytes, missing evidence, wrong revisions, stale claims, and service unavailability. No user source, hosted execution, subscriptions, or automatic policy promotion are involved.

Acceptance: the remote consumer obtains exactly the same evidence meaning and limitations as the existing local reader.

A companion probe compiles `experiment-control` and the pure portions of `research-register` to `wasm32-unknown-unknown` to measure the refactoring required for a Workers-hosted service.

This test must not displace closure of the local development vertical.

### Phase C — Prove distributed research coordination

Introduce a minimal central research coordinator.

Test authenticated assignment, bounded local admission, evidence submission, and independent qualification using a small number of consenting installations.

Measure the usefulness and cost of the resulting evidence.

The decisive demonstration is one complete research cycle across two or more independent installations:

1. The coordinator publishes an immutable experiment protocol.
2. Local engines identify eligible slices and authorize bounded comparisons.
3. Multiple participants contribute independently verifiable evidence.
4. Research qualifies a narrow optimization and publishes a versioned policy revision.
5. Installations apply the policy under local authority.
6. A dependency change or detected regression triggers requalification, and the policy is narrowed, replaced, or withdrawn.

The primary measurement is **cost per independently accepted engineering outcome**, alongside quality, reliability, and required human intervention.

### Phase D — Prove continuous requalification

Track dependencies and observed drift.

Demonstrate automatic identification of findings requiring revalidation and correct handling of stale, contradicted, or withdrawn claims.

### Phase E — Validate member support

Introduce experimental memberships and release tiers.

Measure actual member value, contribution participation, retention, and willingness to support.

Establish whether member support can sustain one full-time maintainer and research operations.

### Phase F — Expand the research organization

Optional. Adopt a charter, form an organization, or add maintainers and researchers only if members and the work justify it.

Expand research domains and network participation while preserving evidence quality and cooperative principles.

The existing open-source engineering roadmap should not be subordinated to premature institution-building.

---

## 13. Principal Risks and Open Decisions

**Funding sufficiency:** It remains unknown whether member support will cover one full-time maintainer and infrastructure.

**Experimental validity:** Heterogeneous development workloads may make reliable comparative research difficult and expensive.

**Network bootstrap:** A small early network may not produce enough eligible experimental opportunities.

**Revalidation burden:** The cost of maintaining established knowledge may eventually compete with the discovery of new improvements.

**Contribution incentives:** Research discounts must not encourage wasteful computation, manipulation, or low-quality evidence.

**Privacy:** Shared research must not become an uncontrolled channel for transferring sensitive member information.

**Provider authorization:** Inference participation must respect applicable provider terms and resource restrictions.

**Availability:** Central service outages must not unnecessarily interrupt local development.

**Release policy:** The exact delay between tiers, guarantees of eventual availability, and licensing of published research require formal definition.

**Decision location:** The balance between locally distributed policy bundles and centrally evaluated recommendations remains an architectural choice.

**Governance:** Cooperative principles, membership rights, formal ownership, vesting rights, and IP commitments require eventual policy decisions.

**Competitive substitution:** Narrow routing optimizers and public benchmark data already exist (§15). The Commons must remain measurably more useful than a leaderboard or routing API, or a cheaper specialized service captures most of the value.

**Causal validity:** Real coding tasks vary in difficulty, user intervention, model behavior, and environment. Assigned comparisons must separate genuine improvement from selection bias, easier tasks, differing environments, and favorable randomness.

**Governance manipulation:** Contribution-weighted voting invites contribution farming, sybil accounts, and coordinated agenda capture.

**Portability:** Research crates currently depend on native Git, filesystem, and SQLite; Wasm hosting requires a deliberate core/adapter split.

These questions should be treated as research and design work, not prematurely settled implementation requirements.

---

## 14. Long-Term Vision

Work Engine evolves into an open-source foundation for governed agentic engineering.

Around it develops a distributed research network in which ordinary engineering work, with appropriate consent and safeguards, creates opportunities for systematic experimentation.

A central research organization identifies what needs to be learned, coordinates the experiments, qualifies findings, monitors their continuing validity, and publishes decision improvements.

Members sustain this work through dues, bounded research contributions, or both, and become more vested the more they contribute.

New discoveries become available to paying members first and ultimately enter the shared body of community knowledge.

As the compiler and execution architecture mature, an increasing portion of research advancement may occur through qualified decision-policy updates rather than architectural changes.

The resulting system continuously improves not because its underlying software is constantly rewritten, but because the evidence informing its decisions is continuously refined.

Over time, its accumulated history also helps determine which research requires attention, how frequently established findings must be revalidated, and where new experiments are most valuable.

The research program itself becomes increasingly informed by its own results.

### Core proposition

**Work Engine provides the open machinery for agentic engineering.**

**The intelligence service maintains the continually requalified knowledge that improves its decisions.**

**The community collectively supports the research, and the resulting knowledge ultimately benefits the community.**

The intended institution is not another hosted AI coding platform.

It is a sustainable research institution that uses a distributed network of real engineering work to make AI-assisted development progressively more efficient, reliable, and effective.

Its sustainability should depend on the quality and continuity of its research—not on permanently restricting access to what that research discovers.

---

## 15. Prior Art and Positioning

*Source: external web research in a ChatGPT conversation, November 17, 2026. Not independently verified; vendor claims are vendor claims. Absence of an exact public match is not proof of novelty.*

| Project | What exists | Difference from this proposal |
|---|---|---|
| Not Diamond Code | Continuously learned model and reasoning-effort selection for coding agents, with cross-customer feedback and cache/cost awareness; per-token routing pricing | Closest competitor for decision optimization; no documented revisioned research claims, cooperative qualification, or policy beyond routing |
| PublicAI Trajector | Rewards developers for sharing coding-agent session data, licensed for training and evaluation | Monetizes session data rather than coordinating experiments and returning improvements |
| SelfBench | Turns merged PRs into checked evaluation tasks comparing models, harnesses, reasoning, and cost | Retrospective evaluation rather than prospective research on ongoing work with shared policy maintenance |
| Ridges | Distributed miners and validators competing to build benchmark-winning agents | Competition among agents rather than bounded research inside participants' own work |
| LangSmith / Datadog | Production evaluation, comparisons, monitoring, regression testing | Single-organization tooling, not a shared commons publishing locally enforceable policy |
| Artificial Analysis | Continuously updated model benchmarks, pricing, and performance data via API | Similar intelligence-distribution business without local experiments or executable policy |
| SWE-bench-Live | Continuously refreshed real software engineering benchmark | Precedent for continuous benchmark maintenance |
| peerBench | Independent institutions, secret evaluation data, cryptographic commitments, public records | Prior work relevant to distributed evidence integrity |
| Prime Intellect | Globally distributed RL on contributed compute | Targets model training, not engineering decision policy |

Assessed distinctiveness: open-source coding execution, multi-model routing, and continuous benchmarking are not distinctive. Centrally assigned, locally admitted experiments; revisioned claims with dependency-driven requalification; and qualified research compiled into locally enforced policy are the more distinctive elements, and their integration is the main differentiator.

The defensible promise is not picking the right model. It is:

> This decision was selected because specific qualifying experiments support it, the relevant evidence remains valid, and the local owner has authorized the policy.

Candidate positioning: *a continuously qualified research network that turns distributed engineering experiments into trustworthy, locally enforceable improvements for AI-assisted software development.*

---

## 16. Existing Foundations

Work Engine already contains research machinery the Intelligence API could compose rather than rewrite (crates under `rust/crates/`):

- `research-register`: pinned Git objects, structural validation, revisions, research-use bundles (`use_bundle.rs`), evidence cards, and a deterministic versioned canonical JSON wire format (`card_wire.rs`).
- `experiment-control`: typed experiment policy and transitions; storage, caller authentication, and native host startup are deliberately outside its domain.
- `claim-evidence`: SQLite-backed claim custody, revisions, provenance, and controlled publication.
- `decision-readiness`, `proposal-evaluation`, `portfolio-decision`: mechanical validation and evaluation without decision authority. `decision-readiness` and `portfolio-decision` are standalone Cargo workspaces, so shared-library integration needs deliberate qualification.
- `work-engine-organization`: a pure compilation boundary that could eventually consume externally selected research policy.

Limits on reuse:

- `claim-evidence` records (native review claims, repository facts) are not globally qualified scientific findings and must not be republished as such. The service needs its own scientific research domain, not a generic wrapper over existing claim types.
- `experiment-control` does not establish causal validity, randomize cohort allocation, or authenticate participants; those are new responsibilities.
- The research-to-execution bridge and production scientific feedback cycle are still being qualified; Plan IR remains pending design; distributed research and autonomous requalification scheduling are future work. The promoted materialization path in `metrics/register` is still narrow.

The external assessment's rough effort estimates (one developer with coding agents, unmeasured): read-only API prototype about one week; signed release delivery and local verification 2–4 weeks; eligibility and assignment 3–6 further weeks; evidence submission and cross-installation qualification 4–8 further weeks; adaptive requalification 4–8 further weeks; memberships, accounting, and hardening 3–6 further weeks. Scientific qualification time is separate and may dominate.

