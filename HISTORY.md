# Work Engine History

## Purpose and scope

This document reconstructs how Work Engine emerged, how quickly its governing
ideas stabilized, which directions became durable capabilities, and which
routes were retired, deferred, or stopped.

It is a historical account, not a normative contract or current roadmap:

- [`DESIGN.md`](DESIGN.md) owns product doctrine and invariants.
- [`PHILOSOPHY.md`](PHILOSOPHY.md) explains the reasoning behind that doctrine.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) describes the current machinery.
- [`roadmap.md`](roadmap.md) owns current priorities and completion evidence.

The current quantitative snapshot in this document was taken on 2026-09-08 at
commit `3d128b7c7b34a8d9373d7cd236d46b12886369c7`. It supersedes the original
2026-08-25 snapshot at `5308ae55e5c4de2710c179da5caba8a0d63d3323` while
retaining that earlier state as a useful growth boundary. The working tree
contained substantial user-owned App Server, doctrine, documentation, and
migration work, so committed history and active development remain distinct.

The account uses two parallel historical streams:

- **Repository chronology** establishes when an idea became durable,
  executable, reviewable, or part of the repository's recorded state.
- **Conversation chronology** establishes when a problem, intuition, or
  abstraction shift was first articulated in the available ChatGPT memory
  record.

These streams answer different questions. A conversation may precede its
implementation, while a document may contain a polished formulation before a
later conversation captures the author's clearest compact recognition of what
it means. Conversation dates and quotations below are attributed to historical
records supplied by the project author; they are not independently recoverable
from Git.

---

## The short account

Work Engine is 22 days old as a distinct body of committed work, but it did not
begin as a blank-slate project. It emerged from Site2JSON after roughly nine
days of intensive conceptual and agent-assisted product development.

Its operational skeleton appeared almost fully formed on 2026-08-17. Its
governing philosophy crystallized about 55 hours later. Most subsequent growth
was the conversion of that doctrine into durable state, proposal and evidence
machinery, self-hosting checkpoints, compiled role environments, and an App
Server runtime capable of hosting supervisor work and retained review.

The central historical observation is:

> Work Engine discovered its philosophy before it built most of its machinery.

By the end of 2026-08-20, the repository had only 89 files and 13,646 text
lines, but 81.6% of the 2026-09-08 `DESIGN.md` and all of the current
`PHILOSOPHY.md` were already present. Five days later it had 731 committed files
and 104,210 text lines. Nineteen days after doctrine selection it had 1,725
committed files and 246,693 text lines.

The most precise origin statement supported by the combined record is:

> Site2JSON was conceived on August 8, entered Git on August 9, and generated
> Work Engine as an explicit subsystem on August 17.

The path from the first recorded Site2JSON concept to the current revisioned
research and App Server state therefore spans 31 days.

Work Engine also began producing accepted Work Engine code through its own
supervisor/builder lifecycle less than a day after its first Git appearance.
The first retained example reached an accepted receipt about 21 hours and 27
minutes after the initial Work Engine commit.

---

## Incubation inside Site2JSON

Site2JSON's Git history began on 2026-08-09. Its first recorded concept predates
that history: at 16:39 UTC on 2026-08-08, the project author described a general
browser-to-AI semantic renderer that would use Schema.org and JSON-LD first,
with site adapters extending the model only where necessary.

Less than an hour later, the conversation had already introduced declarative
adapters, provenance, DOM-visible authority, canonical versus compact output,
and fixture-based testing. The Git repository therefore records the first
durable implementation, not the beginning of the idea.

The early Site2JSON history contained several conceptual seeds that later
became central to Work Engine:

- truthful representation of observed, inferred, and unresolved state;
- provenance;
- granular user ownership of authored decisions;
- separation between automation and human authority; and
- more context-efficient AI-assisted engineering.

Site2JSON's four co-equal principles—Truth, Maintainability, Explainability,
and Aesthetics—were committed as a formal design-principles draft on
2026-08-15. Work Engine later inherited and adopted them.

No tracked Work Engine subtree existed before 2026-08-17. On that date it
appeared under `site2json/work-engine`. The first commit was not exploratory
scaffolding. It already contained:

- `slice-supervisor` and `slice-builder`;
- declarative campaign configuration;
- explicit objectives, approval boundaries, and stop conditions;
- separate supervisor and builder ownership;
- plan acceptance before implementation;
- validation and adversarial-review gates;
- durable receipts and metrics; and
- truthful accepted, stopped, and failed outcomes.

Work Engine therefore began as a practical answer to a concrete problem: how
to let models advance the Site2JSON roadmap without flooding the controlling
context or blurring ownership and authority.

### Standalone extraction

On 2026-08-20, Work Engine became a standalone repository. The Git topology
preserves two roots:

1. a standalone `Initial commit` containing the license; and
2. the earlier Site2JSON-derived Work Engine lineage, merged immediately
   afterward with its original dates preserved.

The extraction commit is named `Import Work Engine history from Site2JSON`.
This topology explains why the repository has two roots and why Work Engine
commits predate the nominal standalone repository creation.

---

## Conceptual and repository chronology

Repository commits show when machinery and documents became durable. The
conversation record adds first-articulation landmarks for the earlier period;
later entries below are repository transitions unless stated otherwise:

| Date | Recorded articulation or transition |
| --- | --- |
| 2026-08-08 | Site2JSON concept and its initial semantic-renderer boundaries |
| 2026-08-15 | Four Site2JSON design principles become durable in Git |
| 2026-08-17 | Work Engine becomes executable supervisor/builder infrastructure |
| 2026-08-18 | First retained accepted Work Engine slice produces Work Engine's own provider-resolution code |
| 2026-08-19 | Multidimensional repository analysis is described through pre-indexing, durable per-pass artifacts, and later synthesis |
| 2026-08-20 | Work Engine's formal design and philosophy become durable in Git |
| 2026-08-21 | The author describes the central discovery as “programming with structure instead of procedures” |
| 2026-08-22–23 | Claims become revisioned answers rather than evidence dumps; proposals, retained review state, and durable role state develop |
| 2026-08-24 | Context loss becomes an architectural validity problem rather than an operational annoyance |
| 2026-08-25 | App Server experiments establish fresh context windows on one thread, followed by the host-owned lifecycle architecture |
| 2026-08-26–27 | Context lifecycle becomes live App Server machinery; claim evidence gains transactional custody, bounded reads, observation intake, semantic compilation, verification, and its first authorized publication vertical |
| 2026-08-28 | Skill decomposition and deterministic compilation converge with Agent Environment Graphs and executable runtime requirements |
| 2026-08-29–31 | Immutable review subjects, provider-neutral review services, native reviewer runtimes, specialist review, claim-backed closure, isolated workspaces, extension bundles, and cross-session coordination form a hosted review stack |
| 2026-09-01 | Supervisor effects, capabilities, workspace publication, strategic reconciliation, operational coordination, and native review move behind App Server hosting |
| 2026-09-02–07 | Recovery work hardens authentication, result custody, remediation lineage, nested reload authority, operator control, generation/supervisor separation, and zero-finding review closure |
| 2026-09-08 | Revisioned execution environments are generalized into a recovery, comparison, forensics, and research architecture |

The especially revealing entry is the direct formulation recorded at 20:57
UTC on 2026-08-21:

> “I think this is like programming with structure instead of procedures.
> maybe structure is enough.”

The sentence is less polished than the doctrine, but historically more
revealing. It captures the central intuition in the author's own language:
structure may be sufficient to preserve correctness without prescribing the
route.

---

## Growth by day

The table follows commits reachable from the standalone `main` history. Text
line counts are repository snapshots at the last commit on each date.

| Date | Commits | Files at day end | Text lines | Main development |
| --- | ---: | ---: | ---: | --- |
| 2026-08-17 | 1 | 11 | 735 | Supervisor/builder engine, configuration, receipts |
| 2026-08-18 | 6 | 28 | 3,258 | Placement reasoning, deterministic gates, evidence-provider work |
| 2026-08-19 | 2 | 49 | 5,586 | Repository search, Chrome Vision, extensive run review |
| 2026-08-20 | 11 | 89 | 13,646 | Doctrine crystallization, standalone extraction, proposal ideas |
| 2026-08-21 | 13 | 233 | 38,802 | Checkpoints, review bench, strategic roles, durable state |
| 2026-08-22 | 33 | 368 | 57,174 | Proposal packets, proposal formation, role environments, evidence lineage |
| 2026-08-23 | 39 | 477 | 71,597 | Claim lineage, retained review state, instruction review, code profiles |
| 2026-08-24 | 11 | 588 | 87,361 | Wind Walker, idea intake, production claim evidence |
| 2026-08-25 | 11 | 731 | 104,210 | App Server runtime and semantic context lifecycle |
| 2026-08-26 | 7 | 1,459 | 185,537 | Linguistic-register research corpus, reloadable host, live context lifecycle, claim-evidence store and reads |
| 2026-08-27 | 3 | 1,469 | 187,154 | Observation intake, semantic claim compiler/verifier, authorized claim publication |
| 2026-08-28 | 6 | 1,516 | 199,476 | Instruction-closure corpus, skill compiler, graph convergence, runtime requirements |
| 2026-08-29 | 13 | 1,550 | 208,373 | Review subjects, role-free skill import, transport experiments, paired calibration, coordination board |
| 2026-08-30 | 11 | 1,636 | 220,377 | Isolated workspaces, provider-neutral review, native reviewer runtime |
| 2026-08-31 | 13 | 1,686 | 225,741 | Specialist review, claim-backed review closure, extension bundles, reliability |
| 2026-09-01 | 10 | 1,709 | 233,838 | App Server supervisor inhabitation, effects, capabilities, publication, native reviews |
| 2026-09-02 | 3 | 1,709 | 234,535 | Native-review authentication and result recovery |
| 2026-09-03 | 3 | 1,710 | 234,807 | Research relocation and pilot revision candidates |
| 2026-09-04 | 4 | 1,712 | 235,052 | Charter relocation and public repository documentation |
| 2026-09-05 | 13 | 1,721 | 241,667 | Review recovery, remediation lineage, nested reload authority, operator control |
| 2026-09-06 | 5 | 1,724 | 244,390 | Operational-state separation, continuity fixes, runtime and architecture ideas |
| 2026-09-07 | 2 | 1,724 | 245,016 | Context-migration recovery and zero-finding review closure |
| 2026-09-08 | 1 | 1,725 | 246,693 | Revisioned research and execution architecture |

Across committed `main` at the snapshot revision:

- 221 commits were reachable;
- 266,369 lines had been inserted and 19,676 deleted across commit patches;
- 1,725 files and 246,693 text lines remained;
- one author identity appeared on all mainline commits;
- 72 commits, or 32.6% of the total, landed on August 22 and 23; and
- the median commit changed 447 lines, while 58 commits changed at least 1,000
  lines.

The change size and cadence are characteristic of agent-accelerated
development rather than conventional line-by-line human authorship.

### Major phases

The growth falls into twelve recognizable phases.

1. **Site2JSON incubation, August 8–16.** Conceptual and product work exposed recurring
   questions about truth, provenance, ownership, AI context, and decision
   authority.
2. **Operational genesis, August 17.** The supervisor/builder engine, campaign
   contract, receipts, and metrics appeared together; within 24 hours, that
   engine was producing accepted changes to itself.
3. **Placement and efficiency, August 18–19.** Work concentrated on repository
   reconnaissance, deterministic gates, provider boundaries, Codebase Memory,
   Chrome Vision, and reducing model context.
4. **Doctrine and extraction, August 20.** Competing design formulations were
   synthesized into the canonical design and philosophy, and Work Engine became
   a standalone repository.
5. **Durable control machinery, August 21.** Checkpoints, completion
   publication, strategic planning, role environments, scheduling, review
   benchmarking, and durable state broadened the execution backbone.
6. **Proposal and evidence systems, August 22–24.** Ideas became durable
   proposal packets, formation and intake capabilities, authority-controlled
   decisions, claim lineage, retained review state, instruction review, code
   profiles, and claim evidence.
7. **Runtime and claim embodiment, August 25–27.** App Server integration turned
   the skill-and-artifact system into a retained role runtime with live context
   transitions, restart-safe state, and a transactional claim-evidence service.
8. **Compilation and role closure, August 28.** Structured skill sources,
   deterministic compilation, Agent Environment Graph validation, and runtime
   requirements connected authored contracts to executable roles.
9. **Hosted review-stack migration, August 29–31.** Immutable subjects,
   provider-neutral review services, native and specialist reviewers, isolated
   workspaces, claim-backed closure, and extension bundles moved review into the
   App Server boundary.
10. **Supervisor inhabitation, September 1.** Effects, capabilities, workspace
    publication, strategic reconciliation, operational coordination, and native
    review were composed into an App Server-hosted supervisor environment.
11. **Recovery and operator-control correction, September 2–7.** Authentication,
    result recovery, replay refusal, retained remediation, nested reloads,
    operator interruption, operational-state separation, and zero-finding
    closure exposed and repaired the consequences of hosting real work.
12. **Revisioned execution as research architecture, September 8.** The
    repository connected durable execution history to reproducible recovery,
    comparison, forensics, behavioral experiments, and architecture research.

---

## Self-hosting from the first day

The initial Work Engine skeleton could not have been built by a system that did
not yet exist. After that bootstrap, however, self-hosting began almost
immediately.

The earliest retained, provenance-backed accepted example is slice 1 of run
`0dc2a3fc-60e1-4fa7-81c1-9b78187da264`, accepted at
2026-08-18 14:23 MDT. Its title was **Static reconnaissance-provider
abstraction**, and its goal was to implement the roadmap's static
reconnaissance-provider abstraction behind the existing evidence-skill
contract. The campaign objective was explicitly **Advance the work-engine
roadmap**. It used `slice-builder` as the builder and reached procedural plan
acceptance before implementation.

The accepted task patch covered six Work Engine paths. Four already had
baseline content and were revised; the two clearest newly produced executable
artifacts were:

- `skills/slice-builder/scripts/resolve_provider.py`, an 80-line deterministic,
  fail-closed provider-to-adapter resolver; and
- `skills/slice-builder/tests/test_resolve_provider.py`, its 81-line focused
  test suite.

The slice also updated the builder instructions, supervisor configuration
reference, roadmap campaign, and roadmap. Its validation record includes the
focused resolver tests, deterministic gates, 15 Work Engine tests, 71
repository tests, freshness checks, and adversarial review. One blocking review
finding was repaired before acceptance; the final receipt records no unresolved
concerns.

This accepted receipt appeared about 21 hours and 27 minutes after Work
Engine's first Git commit. The corresponding Site2JSON commit,
`719aa39c2cf382bb88d7446391b58fc2f71eb04d`, landed about 23 hours and 41
minutes after that first commit. Its standalone rewritten counterpart is
`47edc446a6980752eb467b4beeffe32113152ec0`.

Earlier August 17 campaign records exist, but they stopped for human judgment
or unresolved architecture and contain no retained accepted implementation.
The August 18 slice is therefore the earliest *durably provable* production of
Work Engine code by Work Engine—not necessarily the first unrecorded attempt.

This changes the interpretation of the repository. Self-hosting was not a late
demonstration added after the architecture matured. Almost from day one, Work
Engine was both the object being designed and part of the means used to design,
implement, validate, review, and accept it.

---

## How quickly the core concepts crystallized

The answer differs for operational structure and philosophical formulation.

### Operational structure: day one

The first Work Engine commit already contained the ideas that still define the
execution backbone:

- objective-driven campaigns;
- bounded slices;
- separate supervisor and builder ownership;
- planning before mutation;
- explicit authority and approval;
- deterministic validation where possible;
- receipts rather than transcript retention; and
- truthful stopping.

The first implementation was more procedural than the current doctrine, but
the durable ownership boundaries were already recognizable.

### Philosophical formulation: approximately 55 hours

The canonical doctrine appeared in the 2026-08-20 `progression of design`
commit. That commit contained four design variants and five philosophy
variants. One design and one philosophy were selected as the canonical files
one minute later.

Those files have been remarkably stable at their philosophical center:

- the first canonical `DESIGN.md` contained 666 lines;
- the snapshot version contains 816 lines;
- three mainline commits appear in `DESIGN.md`'s named history, including its
  canonical selection;
- the first and snapshot `PHILOSOPHY.md` both contain 788 lines; and
- only the creation commit has touched `PHILOSOPHY.md`.

The first material later doctrine addition was the 45-line **outcome-derived
design** section on 2026-08-23. It sharpened the existing position into the
compact formulation:

> Define the space, not the solution. Expose the machine, not the route.

On 2026-09-08 the design expanded again to distinguish independently required
production-path claims from local procedural preferences. That addition made
execution sequence, capability grants, custody, and evidence history part of
the result only when an independently owned acceptance claim actually depends
on them. It qualified route-invariance without abandoning it.

The vocabulary and boundary cases continued to improve, but the philosophical
center did not move. The root concepts crystallized within approximately 2.3
days of the first Work Engine commit, before roughly 94.5% of the current
repository text existed.

The conversation record makes the sequence more nuanced. Formal doctrine was
durable on August 20, while the author's most compact direct recognition—
“programming with structure instead of procedures”—was recorded on August 21.
Git answers when the structured formulation landed; conversation history shows
when its meaning was articulated most plainly. Neither timestamp should be
misrepresented as the only moment of discovery.

---

## Ideas that prospered

### Supervisor and builder separation

The original distinction between campaign control and coherent slice execution
survived. It remains the backbone described in `ARCHITECTURE.md`: the
supervisor owns configuration, lifecycle, acceptance, limits, continuation,
and receipts; the builder owns repository understanding, placement,
implementation, validation, and repair.

### Deterministic gates

The early goal of removing fully determined work from model reasoning became a
large family of scripts, schemas, digest checks, gate runners, validators, and
atomic persistence boundaries.

### Audit receipts and compact handoffs

The early separation between durable audit evidence and small continuation
context expanded into terminal receipts, private checkpoints, continuation
state, Git-ref history, completion offers, strategic handoffs, and lifecycle
records.

### Placement and independent falsification

Placement analysis and fresh adversarial review survived, but their status
changed. They became capabilities selected according to consequence rather
than a mandatory universal sequence.

### Durable state and recovery

Durable state moved from an idea to a reusable compare-and-swap primitive, then
became the basis for active-slice recovery, retained review episodes,
checkpoint history, role lifecycle state, a transactional claim store, hosted
review recovery, generation state, operator coordination, and cross-session
resource claims.

### Claim evidence as a service

Claim-centered evidence progressed from proposal dogfood into an App
Server-backed service. Between August 26 and 31 it gained transactional SQLite
custody, bounded reads, immutable observations, a semantic compiler and verifier
shadow path, authorized publication, exact role projections, and claim-backed
native-review closure. The result preserves the distinction between evidence,
claims, reliance, authority, and workflow acceptance rather than collapsing
them into one registry.

### Proposal packets and controlled formation

Three speculative proposal documents appeared on August 20. By August 24, the
repository contained mechanically validated proposal packets, proposal
formation, raw idea intake, authority-controlled decisions, revision-bound
review artifacts, claim-lineage dogfood, and implementation-authorization
boundaries.

### Information and context lifetime

Context lifetime appeared in the canonical doctrine on August 20, contributed
to durable state and retained reviewer design, produced Wind Walker on August
24, and became a primary App Server implementation concern on August 25. By
September 7 it included live transition leases, durable input custody,
reloadable generations, exact nested-turn correlation, operator interruption,
and explicit separation between executable-generation and supervisor
operational state.

### Compiled roles and hosted review

The August 28–September 1 migration turned repository-local skills into
validated runtime realizations. Structured sources compile through Agent
Environment Graph relations into bounded role requirements; App Server services
now host immutable review subjects, provider-neutral review episodes, native and
specialist reviewers, isolated workspaces, publication, supervisor effects,
and operational coordination. Run-scoped extension bundles preserve a separate
path for experimental capabilities without silently expanding the stable core.

### Self-hosting

Less than a day after its first Git appearance, Work Engine used its own
machinery to produce an accepted provider-resolution implementation. It then
continued using that lifecycle to construct its receipt, persistence,
proposal, review, evidence, compilation, and runtime machinery. By September 1
the App Server was not merely an experimental host: accepted migration slices
had placed supervisor effects, capabilities, fenced workspace publication,
strategic reconciliation, operational coordination, and native reviews behind
its service boundary. Its Git refs retain candidate, accepted, and stopped
checkpoint commits; its proposal system records the decisions behind new
skills; and its metrics record both accepted and stopped campaign outcomes.

---

## Routes that failed, narrowed, or were demoted

Work Engine has rejected methods more often than objectives.

### Fixed route and evidence taxonomies

The initial optimization roadmap used named direct and falsified-placement
routes, hard evidence-cardinality limits, and more rigid provider sequences.
The project later recognized that these rules could turn current experience
into policy. Route identities were opened and evidence selection was returned
to model judgment inside provenance and authority contracts.

### Mandatory Codebase Memory substrate

The proposal to make Codebase Memory the default mandatory repository substrate
was demoted. Codebase Memory remains a strong current indexed capability, but
its availability does not make it an invariant route.

### Fixed escalation ladders

Fixed adaptive-escalation procedure was replaced with evidence-based escalation
consequences and truthful provenance. Escalation remains reachable without
being triggered merely by time, size, or inconvenience.

### Ritual reviewer replacement

Repeatedly restarting reviewers was rejected. The current doctrine distinguishes
fresh initial independence from the value of retaining an isolated reviewer
through remediation.

### Site2JSON implementation work

The original Work Engine optimization roadmap included Site2JSON ESM migration.
Standalone extraction made the ownership error visible, and that work returned
to the Site2JSON roadmap.

### Research directions

Model-choice benchmarking and the semantic architectural-memory overlay were
moved to research rather than retained as product-completion gates. Permanent
shared placement for claim-centered evidence lineage was explicitly deferred
pending further dogfood.

### Provider and batch experiments

Native Claude transport failover, OpenRouter routing attestation, batch
adaptation, and paired-review calibration were implemented as explicit,
provenance-bearing experiments. The production direction did not become a
universal provider ladder. The later App Server stack instead admitted exact
review profiles and hosted retained native review behind authority, subject,
and replay fences. Experimental transport work remained evidence about a route,
not doctrine requiring that route.

### Root-level research ownership

Research-foundation and community-charter material briefly appeared at the
repository root before moving under `app-server/docs/research/`. A broad
OpenCode-as-substrate idea was later removed while a narrower substrate
evaluation and provider-turn-harness direction remained. These corrections
changed ownership and framing rather than erasing the underlying research.

These are route revisions, not evidence that the larger objectives were
abandoned.

---

## Dead ends and stopped work

There is no single honest dead-end count because the repository distinguishes
discarded drafts, stopped executions, provider failures, deferred proposals,
and rejected meanings.

### Documentary alternatives: approximately 11

Seven sibling design and philosophy drafts lost the August 20 canonical
selection. Four older historical design documents were briefly imported and
then removed. These are discarded formulations, not abandoned core concepts.

Of 1,770 paths ever added to the standalone mainline, 71 no longer exist under
their original names. The earlier set already included lossless idea moves,
canonical renames, generated bytecode caches, an archive, and a superseded
campaign path. The later set adds deliberate research and charter relocations
and a narrowed provider-substrate proposal. Raw path deletion therefore still
substantially overstates conceptual abandonment.

### Current durable checkpoint inventory

The retained Work Engine checkpoint refs contained:

- 82 campaigns;
- 117 slices;
- 243 candidate checkpoint refs;
- 84 accepted slices;
- 5 explicitly stopped slices;
- 28 candidate-only slices; and
- 60 slices that required more than one candidate attempt.

The maximum recorded attempt count was eleven.

Four explicitly stopped checkpoint slices belong to the linguistic-register
experiment. They stopped at construction, evidence, or transport gates. One
run rejected model output wrapped in an unexpected serialized object. A later
run incorporated one-level unwrapping, accepted 15 of 16 fresh samples, and
still stopped because one sample had 88 words against a frozen 90-word minimum.
Downstream semantic comparison was not run. The fifth stopped checkpoint is an
App Server skills-migration integration slice. These outcomes remained durable
rather than being rewritten as success.

### Recorded campaign terminals

The seven committed metric files contained 70 terminal receipts:

- 55 accepted;
- 15 stopped.

Most stopped outcomes were not conceptual failures. Recorded reasons included
provider quota or infrastructure failure, insufficient evidence, unresolved
architectural ownership, a required human choice, and an intentionally reached
phase boundary.

### Proposal decisions

Eight proposal decision records existed in the snapshot tree. Seven approved
proposal meaning, usually with provisional placement. One deferred permanent
claim-lineage placement for more dogfood. None formally rejected the underlying
proposal meaning.

The project is young enough that many directions remain exploratory, formed,
or unexercised rather than failed.

---

## Estimated development effort

Git timestamps are activity evidence, not time tracking. Work Engine's own
telemetry is also incomplete: only 38 of 70 committed metric records contain a
builder wall-clock measurement. Those measured records total 65.92 builder
hours, but they cover only a subset of the development history and sometimes
overlap other provider measurements.

Clustering mainline commits into likely work sessions gives:

- approximately 82 hours using 90-minute session gaps;
- approximately 104 hours using two-hour gaps; and
- approximately 145 hours using four-hour gaps.

Allowing for work before the first commit, long-running agent work between
commits, upstream conceptual incubation, and the active uncommitted worktree,
the best estimate is:

> **Approximately 100–170 human-directed development hours**, of which
> **approximately 85–145 hours** are directly attributable to the committed
> Work Engine lineage.

Confidence is moderate to low. At least one experiment explicitly records that
manually supplied Git author timestamps were inaccurate for experimental
chronology.

It would be misleading to convert the 246,693 text lines directly into
traditional developer-hours. Much of the tree consists of agent-authored
documentation, schemas, fixtures, receipts, reviews, and experimental evidence.
A conventional team would likely need many hundreds of hours to reproduce the
surface area, but that is not the labor represented by this history.

---

## Repository composition at the snapshot

The committed tree contained:

- 1,725 files and 246,693 text lines;
- 411 Markdown files;
- 722 JSON files;
- 88 TypeScript files;
- 222 JavaScript module files;
- 104 Python files;
- 959 files under `skills/`;
- 404 files under `app-server/`;
- 115 files under `reviews/`;
- 103 files under `proposals/`; and
- 196 test-related files.

The current codebase index at the time of investigation contained 28,946 nodes
and 77,157 edges, including 2,606 functions and 1,028 methods. It showed an
implemented execution backbone, substantial skill and proposal machinery, and
an App Server runtime that now hosts compiled roles, review, claims, workspace,
and supervisor capabilities.

The graph reported parse gaps in the short generated Codex protocol index and
one linguistic-register preregistration file. Deliberately excluded
`__pycache__` directories were also reported. None of those gaps was used to
establish the historical claims in this document; Git and direct source
observation were authoritative for chronology and prose.

### Active state beyond the snapshot commit

At the time of investigation:

- `main` was two commits ahead of `origin/main`;
- 60 tracked files were modified before this history edit;
- 16 files were untracked; and
- active work centered on App Server executable-generation maintenance,
  skills-migration integrity, operator ingress, provider-turn hosting,
  structural planning, and production-path claim doctrine.

The active tree also contained an accepted S13 receipt for provider-neutral
Review Bench validation, blinded rendering, supplied-observation inventory,
and descriptive comparison through bounded experimental App Server machinery.
That receipt explicitly withheld production-review authority. Its publication
to `main` was pending and is therefore not included in the commit-bound counts
above.

This document does not treat those user-owned working-tree changes as accepted
mainline history.

---

## What kind of project Work Engine has become

Work Engine began as an efficiency mechanism: give a capable model enough
execution reach to advance a roadmap while keeping the supervisor's context
small.

Within roughly three days, the project recognized a more general problem.
Overly detailed workflows can suppress the judgment that capable models
contribute. That recognition converted an orchestration tool into a theory of
model-centered systems.

The conversation record gives this transition a contemporary name:

> programming with structure instead of procedures

Everything since then can be read as an attempt to reconcile two pressures:

1. give the model genuine freedom to choose and revise a route; and
2. make authority, evidence, state, failure, and provenance mechanically
   truthful.

This explains the project's apparent paradox. It is philosophically skeptical
of procedure, yet full of schemas, validators, state machines, receipts, and
gates. Those mechanisms are not intended to decide what the model should do.
They are intended to make invalid or dishonest states unreachable while
leaving valid routes open.

The greatest strength in the history is early conceptual coherence joined to
immediate self-hosting. Work Engine is not merely an untested theory: within
its first day it was producing accepted, tested changes to itself, and it
continued building substantial parts of its own machinery through its own
lifecycle.

The rapid surface-area growth remains a maintenance pressure: more than 233,000
text lines arrived after the doctrine stabilized, in approximately nineteen
days. The second half of the record also changes the standard of evidence. The
App Server migration demonstrated that a capability can work in isolation yet
still require repeated correction when it becomes responsible for real
authority, retained provider sessions, recovery, and operator control.

The open questions are now comparative and longitudinal. Which mechanisms
create the leverage? How much of the machinery is necessary? Which recovery
properties survive interruption and replacement? Do the demonstrated
self-hosting advantages generalize to other users, repositories, and
objectives? Can the system preserve its coherence over months rather than days?
The revisioned research architecture committed on September 8 makes the
repository's own execution history a possible substrate for answering those
questions rather than only a record that the questions exist.

---

## Evidence and limitations

This account used four evidence classes:

1. **Git history** from both `/home/bline/code/site2json` and the standalone
   Work Engine repository for dates, ancestry, path changes, growth, and
   retained Work Engine refs.
2. **Conversation history** supplied by the project author for pre-Git origin,
   first-articulation dates, direct quotations, and abstraction shifts that Git
   cannot represent.
3. **Direct source observation** for design documents, roadmaps, proposal
   decisions, metric receipts, stopped-experiment summaries, and current
   architecture descriptions.
4. **Codebase Memory structural evidence** for the current implementation
   inventory and selected App Server boundaries, checked against generation
   and coverage metadata.

The main limitations are:

- commit time is not labor time;
- conversation timestamps establish recorded articulation, not necessarily the
  first private thought or the exclusive moment an idea was discovered;
- the supplied ChatGPT memory chronology is attributed historical evidence and
  was not independently verified against an exported conversation archive
  during the repository investigation;
- rewritten extraction history preserves content and dates but not the exact
  uncommitted process that preceded the first Work Engine commit;
- the self-hosting priority claim is based on an exhaustive review of retained
  historical metrics and Work Engine refs; an earlier unrecorded self-hosted
  attempt remains possible;
- checkpoint refs include machine-generated candidate and terminal commits and
  must not be counted as ordinary authored product commits;
- candidate-only slices are unresolved historical states, not automatically
  failures;
- clean index coverage metadata is best-effort and is not proof of
  completeness; and
- the repository was changing during the investigation, so this document binds
  its quantitative claims to the snapshot revision named above.
