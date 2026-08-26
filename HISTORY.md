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

The quantitative snapshot in this document was taken on 2026-08-25 at commit
`5308ae55e5c4de2710c179da5caba8a0d63d3323`. The working tree already contained
additional user-owned App Server and linguistic-register work, so committed
history and active development are kept distinct below.

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

Work Engine is only eight days old as a distinct body of committed work, but it
did not begin as a blank-slate project. It emerged from Site2JSON after roughly
nine days of intensive conceptual and agent-assisted product development.

Its operational skeleton appeared almost fully formed on 2026-08-17. Its
governing philosophy crystallized about 55 hours later. Most subsequent growth
was the conversion of that doctrine into durable state, proposal and evidence
machinery, self-hosting checkpoints, and an App Server runtime.

The central historical observation is:

> Work Engine discovered its philosophy before it built most of its machinery.

By the end of 2026-08-20, the repository had only 89 files and 13,646 text
lines, but 93.8% of the current `DESIGN.md` and all of the current
`PHILOSOPHY.md` were already present. Five days later it had 731 committed files
and 104,210 text lines.

The most precise origin statement supported by the combined record is:

> Site2JSON was conceived on August 8, entered Git on August 9, and generated
> Work Engine as an explicit subsystem on August 17.

The path from the first recorded Site2JSON concept to the current App Server
and context-lifecycle state therefore spans approximately 17 days.

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

## Conceptual chronology from the conversation record

Repository commits show when machinery and documents became durable. The
conversation record adds the following first-articulation landmarks:

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

Across committed `main` at the snapshot revision:

- 127 commits were reachable;
- 117,550 lines had been inserted and 13,340 deleted;
- 731 files and 104,210 text lines remained;
- one author identity appeared on all mainline commits;
- 72 commits, or 57% of the total, landed on August 22 and 23; and
- the median commit changed 448 lines, while 34 commits changed at least 1,000
  lines.

The change size and cadence are characteristic of agent-accelerated
development rather than conventional line-by-line human authorship.

### Major phases

The growth falls into seven recognizable phases.

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
7. **Runtime embodiment, August 25 onward.** App Server integration began
   turning the skill-and-artifact system into a retained role runtime with
   observable context lifecycle and restart-safe state.

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

Those files have been remarkably stable:

- the first canonical `DESIGN.md` contained 666 lines;
- the snapshot version contains 710 lines;
- only two commits have touched `DESIGN.md`;
- the first and snapshot `PHILOSOPHY.md` both contain 788 lines; and
- only the creation commit has touched `PHILOSOPHY.md`.

The one material later doctrine addition was the 45-line
**outcome-derived design** section on 2026-08-23. It sharpened the existing
position into the compact formulation:

> Define the space, not the solution. Expose the machine, not the route.

The vocabulary continued to improve, but the philosophical center did not
move. The root concepts crystallized within approximately 2.3 days of the first
Work Engine commit, before roughly 87% of the current repository text existed.

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
checkpoint history, and role lifecycle state.

### Proposal packets and controlled formation

Three speculative proposal documents appeared on August 20. By August 24, the
repository contained mechanically validated proposal packets, proposal
formation, raw idea intake, authority-controlled decisions, revision-bound
review artifacts, claim-lineage dogfood, and implementation-authorization
boundaries.

### Information and context lifetime

Context lifetime appeared in the canonical doctrine on August 20, contributed
to durable state and retained reviewer design, produced Wind Walker on August
24, and became a primary App Server implementation concern on August 25.

### Self-hosting

Less than a day after its first Git appearance, Work Engine used its own
machinery to produce an accepted provider-resolution implementation. It then
continued using that lifecycle to construct its receipt, persistence,
proposal, review, evidence, and runtime machinery. Its Git refs retain
candidate, accepted, and stopped checkpoint commits; its proposal system
records the decisions behind new skills; and its metrics record both accepted
and stopped campaign outcomes.

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

Of 754 paths ever added to the standalone mainline, 48 no longer exist under
their original names. Twenty-three idea paths were losslessly moved into a
dated history directory, and two canonical documents were detected as exact
renames. Ten of the remaining paths were committed Python bytecode caches, one
was an archive, and one was a superseded campaign path. Raw path deletion
therefore substantially overstates conceptual abandonment.

### Current durable checkpoint inventory

The retained Work Engine checkpoint refs contained:

- 39 campaigns;
- 52 slices;
- 104 candidate checkpoint refs;
- 30 accepted slices;
- 3 explicitly stopped slices;
- 19 candidate-only slices; and
- 20 slices that required more than one candidate attempt.

The maximum recorded attempt count was ten.

The three explicitly stopped checkpoint slices all belonged to the
linguistic-register experiment. They stopped at evidence or transport gates.
One run rejected model output wrapped in an unexpected serialized object. A
later run incorporated one-level unwrapping, accepted 15 of 16 fresh samples,
and still stopped because one sample had 88 words against a frozen 90-word
minimum. Downstream semantic comparison was not run. These failures produced
the next experiment's contract rather than being silently repaired or erased.

### Recorded campaign terminals

The five committed metric files contained 41 terminal receipts:

- 27 accepted;
- 14 stopped.

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
telemetry is also incomplete: only 19 of 41 committed metric records contain a
builder wall-clock measurement. Those measured records total 12.19 builder
hours, but they cover only a subset of the development history and sometimes
overlap other provider measurements.

Clustering mainline commits into likely work sessions gives:

- approximately 40 hours using 90-minute session gaps;
- approximately 49 hours using two-hour gaps; and
- approximately 67 hours using four-hour gaps.

Allowing for work before the first commit, long-running agent work between
commits, upstream conceptual incubation, and the active uncommitted worktree,
the best estimate is:

> **Approximately 60–90 human-directed development hours**, of which
> **approximately 50–70 hours** are directly attributable to the committed
> Work Engine lineage.

Confidence is moderate to low. At least one experiment explicitly records that
manually supplied Git author timestamps were inaccurate for experimental
chronology.

It would be misleading to convert the 104,210 text lines directly into
traditional developer-hours. Much of the tree consists of agent-authored
documentation, schemas, fixtures, receipts, reviews, and experimental evidence.
A conventional team would likely need many hundreds of hours to reproduce the
surface area, but that is not the labor represented by this history.

---

## Repository composition at the snapshot

The committed tree contained:

- 731 files and 104,210 text lines;
- 323 Markdown files;
- 157 JSON files;
- 86 TypeScript files;
- 62 JavaScript module files;
- 51 Python files;
- 245 files under `skills/`;
- 142 files under `app-server/`;
- 110 files under `reviews/`;
- 91 files under `proposals/`; and
- 94 test-related files.

The current codebase index at the time of investigation contained 19,837 nodes
and 45,274 edges, including 1,134 functions and 609 methods. It showed an
implemented execution backbone, substantial skill and proposal machinery, and
an App Server runtime under rapid construction.

The graph's only recorded parse gap relevant to the broad App Server scope was
the first 20 lines of a generated Codex protocol binding. Deliberately excluded
`__pycache__` directories were also reported. Neither gap was used to establish
the historical claims in this document; Git and direct source observation were
the authoritative evidence for chronology and prose.

### Active state beyond the snapshot commit

At the time of investigation:

- `main` was six commits ahead of `origin/main`;
- 10 tracked files were modified;
- 17 files were untracked; and
- active work centered on App Server semantic-context machinery and the
  linguistic-register pilot.

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

The rapid surface-area growth remains a maintenance pressure: more than 90,000
text lines arrived after the doctrine stabilized, in approximately five days.
The open questions are now comparative. Which mechanisms create the leverage?
How much of the machinery is necessary? Do the demonstrated self-hosting
advantages generalize to other users, repositories, and objectives? Can the
system preserve its coherence over months rather than days? The historical
question is no longer whether the machinery works, but why it works, how much
is necessary, and how far the result generalizes.

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
