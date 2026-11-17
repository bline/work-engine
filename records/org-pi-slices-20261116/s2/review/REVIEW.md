revise_before_owner

# S2 independent design review: R2 admission proposal

Reviewer: separate Claude Code session, observed model `claude-fable-5-2` (requested effort xhigh; observed effort
not established). Independence: separate session and context; same provider family as the author. Advisory only: no
authority, acceptance or implementation is conveyed. Design disposition stays with Pi root `01a1e56e`; owner
questions stay with the operator.

Subject: `/home/bline/cbm-work/org-pi-slices-20261116/s2/design/PROPOSAL.md`, sha256
`4f0c9de08395adcacf9ee111641ecffef003fdb9d8cb7ac65a7f21ddaf8fa1f9` (recomputed at entry; equals `PROPOSAL.sha256`).
Source read in `/home/bline/cbm-work/wt-p1-vertical-20261113` (packet states the tree equals `1cc77114`; I did not
re-verify with git, which the packet forbids mutating and I did not need).

Process: the AGENTS.md board read was attempted and denied by the session permission mode, so board state is
`not established` for this review. Context and spend are observed by the parent; to me they are unavailable, not
zero. No builds, tests, network, Docker or git operations. Only this file was written.

Severity: **P1** makes approval unsafe or R2 unsatisfiable; **P2** must be corrected before the owner decides the
affected question or before the slice is dispatched; **P3** is an improvement. "Source-checked" means I read the
cited lines; "reasoning" means inference from them.

---

## 1. Verdict and summary

**`revise_before_owner`.** No P1. Eight P2, three P3.

The direction is sound and I would not change it: pure derivation, one organizational-authority decision owner,
per-effect admission at the consuming host, owner-attested records with actor-anchor separation (Option A), storage
kept out, production still refusing. D1, D2, D5 and D6 are decidable now.

The revision is needed because the questions that carry the cost and the consequence, **D3, D4, D7 and D8**, are
framed on premises the source does not support, and four decisions the owner must make are missing:

- The S1 delivery path supplies **no realization claim at all**, so the mismatch the proposal says will "refuse
  before spawn" does not exist as a `KnownIncompatibility` blocker on that path (F-1).
- The v1 realization vocabulary is closed to one provider class and one confinement class. The only second world
  the unchanged compiler can produce that the provider-disabled child could pass is a world with **no realization
  constraints**. D4 is therefore not an optional positive control: under D3 it is the precondition for re-running
  the S1 native matrix at all (F-1).
- The consumer seam passes one caller-built admission across two effects (spawn, consumption) and checks currency
  only at the second. That carries rev6 finding F3 forward in part (F-2). The test-oracle rule restates the rule
  rev6 finding F6 showed unimplementable (F-3).
- The observation contract states when there is no observation but not what an observation guarantees, which is the
  content of C5-B1 (F-4).

The revision is bounded. §8 lists it. In my judgment a root-authored addendum applying §8 does not need a second
independent review, because every item is either a source fact stated here or an explicit owner question.

---

## 2. Findings

| ID | Sev | Finding | Confidence |
|---|---|---|---|
| F-1 | P2 | D3 and D4 rest on wrong premises about the S1 path and the v1 vocabulary | high on facts, medium-high on consequence |
| F-2 | P2 | One admission spans two effects; the host does not own the boundary; rev6 F3 partly carried forward | high |
| F-3 | P2 | §5 restates the universal paired-control oracle that rev6 F6 refuted and 6.1 §6 withdrew | high |
| F-4 | P2 | Observation contract omits the source's obligations, so C5-B1 is hidden at the contract level | medium-high |
| F-5 | P2 | Blocker discharge has no stated owner or type, and the R2-A/R2-B cut runs through it | high on facts, medium on remedy |
| F-6 | P2 | Four owner decisions are missing | medium-high |
| F-7 | P2 | Lifecycle reconciliation was not done; a fixture epoch ledger is built before its owner is decided | medium |
| F-8 | P2 | R2-A scope says "O1–O9" but omits the records O1, O2 and O6 need; the estimate inherits that gap | medium-high |
| F-9 | P3 | Production instantiation and Q10/Q11 are close to vacuous in a fixture-only crate | medium |
| F-10 | P3 | Identity types are defined in log terms while storage is "undecided" | medium |
| F-11 | P3 | Smaller wording and consistency items | high |

### F-1 (P2). D3 and D4 rest on wrong premises

Source-checked facts:

1. Every binding in the S1 native suite's shared helper is built with `request(None)`
   (`work-engine-review-delivery/tests/native_vertical.rs:93-113`, also `:219`, `:274`, `:297`). A search of the whole
   delivery crate directory for the lowercase token `realization` matches only `src/binding.rs:112-113` and the
   helper definitions at `tests/common/mod.rs:73,77,82`. A claim can reach `prepare` only through
   `PreparationRequest.realization` (`work-engine-review-preparation/src/prepare.rs:45`), and a struct literal or the
   helper would contain that token, so within this crate the delivered path supplies no claim. With no claim,
   `realization_evaluation` is `None` (`prepare.rs:444-447`) and no `realization:*` blocker is emitted
   (`prepare.rs:525-542`). The `fake_fixture` mismatch exists only as a hand-built claim in a preparation test
   (`work-engine-review-preparation/tests/preparation.rs:439-466`).
2. The v1 vocabulary has exactly one provider class and one confinement class: `ProviderFamily { Anthropic }` and
   `ExecutionConfinement { GuardedSandbox }` (`work-engine-organization/src/schema.rs:129,133`); any other string is
   refused as unrecognized (`tests/rules.rs:481-484`).
3. `realizationConstraints` is optional per condition (`schema.rs:966-969`), and derivation emits a constraint only
   for a dimension some condition states (`derive.rs:951-959`).
4. Both frozen conditions state both dimensions, plus `mutation: none`
   (`tests/fixtures/frozen/requirements.json:30-45, 62-77`).
5. The S1 world is the frozen candidate (`tests/common/mod.rs:46-48, 147-148`), and the suite reaches the SDK
   through `deliver_and_consume` (`native_vertical.rs:140-145`), with one direct `run_native` call (`:432`).

Consequences (reasoning):

- Proposal §4.5 item 4 gives the wrong mechanism. Under D3 the frozen world would refuse because the unconditional
  `realization-admission:*` blockers (`prepare.rs:499-504`) have no truthful discharge, not because a
  `KnownIncompatibility` blocker exists. For `KnownIncompatibility` to mean anything on the vertical, R2-B must first
  make the **host** state what it is about to run and feed that into preparation. That scope item is absent from
  §4.5 and §7. Proposal §2's seam "(e) `KnownIncompatibility` blockers do not refuse" is true but incomplete: on the
  vertical they are never produced.
- Under D3 the frozen world refuses before spawn, so every native test that needs a spawned child (timeouts,
  trailing data, wrong session, limits) needs a world that admits. D4 is mandatory for "the full S1 matrix re-run",
  and its fallback ("accept pure-gate-only positive controls") would leave that matrix unrunnable through the gated
  entry. The proposal presents D4 as optional and says "the builder checks this first".
- By facts 2 and 3 the D4 world can only be one whose conditions state **no** realization constraint. The positive
  control then exercises no constraint row.
- "Truthfully satisfies" cannot hold for `mutation: none`: S1's `unclaimed` list says the SDK child is not confined.
  A fixture `RealizationAcceptance` stating `mutation: none` is a labelled assumption, not a truthful statement.
- D3 changes more of S1's accepted evidence than "the positive path stops passing": the matrix rows move to another
  world. That is consistent with S1's record (it closes the recorded `semantic_limit`), but the owner should approve
  it knowing the matrix is rebased, and R2-B's cost must include it.

Check that would have caught it: read `tests/common/mod.rs` and `schema.rs:125-135` before framing D3/D4. The
proposal lists the vocabulary as unchecked; it took four targeted reads here.

### F-2 (P2). One admission, two effects; host does not own the boundary

Proposal §4.2 and §4.4 define an admission as binding "one boundary, single use", and say "a new boundary needs a
new admission". §4.5 then passes **one** `FixtureEffectAdmission` into `deliver_and_consume`, checks digests and
discharge coverage before spawn, recomputes state against a new observation only before consumption, and consumes
the epoch once, at consumption.

- Delivery (spawning a session and giving it the contract; in production, a paid provider call that exposes the
  subject) and consumption (advancing episode state) are two effects under one admission. By the proposal's own
  §4.4 that needs two admissions, or delivery must be declared not a boundary. Neither is stated.
- The pre-spawn check has no fresh observation and no epoch check. A stale or revoked admission still spawns.
- Reuse is detected at consumption, after the second child has run. §4.5 item 3 says this makes S1 F-7's
  no-auto-replay "a host property"; for the delivery effect it does not.
- `run_native` is a public spawn route that takes no admission (`native.rs:1026-1029`; the docstring at
  `:1200-1204` promises no other route to a *consumption*, not to a spawn). "On refusal no native child is spawned"
  holds only for callers of `deliver_and_consume`.
- Rev6 F3's recommendation was a host-owned operation that "constructs the request/epoch, obtains the observation,
  runs both gates and consumes authorization, with no caller path to reuse an observation for a later boundary"
  (`review-report.md:59`). Amendment 6.1 §3 answers with reserve, `observe_for(&reservation)` sealing the boundary
  into the observation, gates, then regate inside the effect commit (`AMENDMENT-6.1.md:161-179`). The proposal keeps
  the regate and the epoch record but drops the reservation and the sealing: §4.1's observation carries no boundary,
  the admission is built by the caller, and Q1–Q14 contain no same-observation/later-epoch case (6.1's B12b,
  `AMENDMENT-6.1.md:185`). The counterexample F3 actually reproduced (`review-report.md:55`) is therefore not
  excluded by the proposed R2-A API and not tested.

So the proposal's §1 statement that token-lifetime enforcement is replaced by 6.1 F3's host boundary is only partly
true of what §4.5 specifies.

### F-3 (P2). The oracle rule restates rev6 F6

Proposal §5: "Each negative case has a paired positive control that differs only in the varied input, and each
asserts that the **intended** check refused while the others passed." Rev6 F6 found exactly this universal rule not
implementable (`review-report.md:82-91`), and 6.1 §6 withdrew it for five oracle classes
(`AMENDMENT-6.1.md:236-250`). Three of the proposal's own rows contradict the rule:

- Q9 (`Unknown`): there is no observation, so no check outcomes exist.
- Q2(b) and any principal or binding revocation: O4 and O8 legitimately fail together.
- Q11: production refuses O5 and the binding checks together.

The proposal adopts 6.1 for F3, F4 and F5 and is silent on F6. A builder would have to invent exceptions.

### F-4 (P2). The observation contract hides C5-B1 instead of fencing it

Keeping storage out of R2 is the right boundary. It is sound only if the contract states what the eventual source
must guarantee, because that guarantee is what C5-B1 says passive file readers cannot give (`assignment-log.md`
C5/C6 entry: readers can observe `Activate` before its fsync).

§4.1 lists when there is no observation. It does not say what `Committed` or `Revoked` at revision N warrants. As
written, a passive reader that reports an activation before it is durable satisfies §4.1. Missing obligations on
the source, in contract terms:

- an issuance is reported `Committed` only once its activation is durable;
- a reported revision is never retracted by later recovery, and revisions are totally ordered;
- an uncertain revocation keeps fencing (rev6 §3.3, `STAGE0-CONTRACT-r6.md:417-424`; owner ruling in
  `AMENDMENT-6.1.md:14-17`);
- one observation is one state, never a mixture.

§4.1 also over-specifies: "while a write is in progress ... there is no observation" is the serialized log's global
rule, with the availability cost the rev6 review reproduced as H5 (`review-report.md:252-256`). A protected service
need not behave that way. State the outcome (no decision on an unpublished or uncertain state) and leave the
mechanism to the storage decision.

With those obligations written, D6 becomes a real decision: the owner approves the acceptance criteria for the
later storage choice. Without them, D6 defers the problem without recording it in the contract.

### F-5 (P2). Discharge: no owner, no type, and the slice cut runs through it

- §4.2 puts `discharges[blocker id -> check id]` inside the admission; §4.5 has the host check that discharges cover
  the blockers; Q13 "removes one discharge from an otherwise complete admission". That reads as a map presented to
  the host. If the party seeking admission can author it, the completeness check verifies a self-authored claim. If
  only the gate can build it, Q13 needs a test-only constructor. The proposal does not say which.
- Blocker ids are `format!` strings (`prepare.rs:451-534`) and the class enum has two variants
  (`prepare.rs:202-207`). Mapping a blocker to its discharging check means dispatching on string prefixes. One id
  already has the form `realization:<cond>:realization:provider_family` (`prepare.rs:534` with
  `realization.rs:196`).
- R2-A builds "the blocker-discharge map" with dependencies that exclude the preparation crate (§7). The "stable
  blocker ids exported as a discharge vocabulary" arrive in R2-B. R2-A would encode another crate's string format
  without depending on it, one slice before that format is stabilized.

Remedy (reasoning): a closed `BlockerKind` in the preparation crate; a total function from kind to the check or
checks that discharge it, owned by the admission contract; the **host** derives discharge from gate outcomes and
refuses any unknown kind. The map is an output, never an input.

### F-6 (P2). Missing owner decisions

1. **The three unowned mapping rules** (`scope-digest-to-subject`, `problem-scope-to-episode-identity`,
   `consumption-evidence-mapping`; `prepare.rs:451-485`). The proposal discharges them in the fixture by the labelled
   declaration and refuses them in production as `mapping_unowned`. Consequence: after D5, D6, D7 and issued records,
   production admission is still unreachable. §6.1 lists this as a gap; no question asks the owner to name owners
   for the rules or to accept that state. The admission also "binds the subject" only as the caller's claim
   (`prepare.rs:20-21, 40`).
2. **Realization identity and the evidence for R3.** `realizationDigest` is never defined for the Pi vertical: what
   it covers, and who computes it. If the admission's author supplies it, R4 compares two caller-supplied values.
   R3 compares the *declared* actor with the bound actor; the rev6 review's J6 says declared equality "is not an
   observation of who actually executed" (`review-report.md:457-463`). Evidence-and-claims §3.9 requires a recording
   or independent-observation mechanism for a required production-path claim; constraint alone does not establish
   it (`evidence-and-claims.md:37-40`). PLAN R2 asks qualification to bind the *actual* actor. The contract should
   say which mechanism R3 will use in production (host observation of the spawned process, compared with the
   owner-attested binding) and record that the fixture uses declared values. This is different from Option B:
   observing who ran, to compare with an attested binding, does not derive authority from observation.
3. **Is delivery a boundary?** See F-2.
4. **Standing of Amendment 6.1.** The proposal adopts its F3, F4 and F5 text. 6.1 is unaccepted and its review was
   never launched. Approving this contract accepts those parts; the owner should do that knowingly. This review
   covers them only as restated in the proposal.

### F-7 (P2). Lifecycle reconciliation not done; epoch ledger precedes its owner

PLAN R2 requires reconciling with "existing admission/publication/lifecycle owners" and forbids a parallel
scheduling owner. The proposal states that lifecycle-* sources were not read and marks the lifecycle option in D7
`not established`. Source-checked pointers (I read only these passages, about 90 lines; the relationship is
`not established`, not absent):

- `lifecycle-core/src/reducer.rs:79-84` `AdmissionOwner { Domain{generation}, Transition{id}, Fenced }`, and
  `:99-108` `CommandAdmission` carrying a `principal: String` "supplied by trusted composition". That is a second
  principal notion the §18 `Principal` record must be related to.
- `review-episode-core/src/reducer.rs:105-116, 140-151`: the episode owner already refuses a moved revision and
  detects a handled `transition_id`. For the consumption effect that is single-use and staleness, owned today. The
  proposal's stale condition "the episode revision moved" (§4.4) duplicates it, against its own §2 rule that
  admission must not duplicate these checks.
- `work-engine-review-delivery/src/lib.rs:51-54` `Transport { attempt_id, generation }` is a third identity already
  on the delivery envelope.

R2-B nonetheless builds a fixture epoch ledger whose production home is deferred (D7) and whose only consumers are
Q5 and mutant (g). This is the nearest analogue in the proposal to the S1 lesson (a root-authored production-path
mechanism consumed only by its own gate; `ROOT-S1-DESIGN-DECISION.json` `basis`). Either decide D7 now, after
reading the lifecycle owners, or remove the ledger from R2 and record delivery-boundary single-use as a gap.

### F-8 (P2). R2-A scope and estimate

§4.3 says Gate O runs O1–O9. O1 needs an admission lineage with a committed head of purpose `execution`; O2 needs a
committed semantic acceptance; O6 needs revision 4's E5 disposition rule (`STAGE0-CONTRACT-r6.md:618-633`). Those
are rev6 §4.2 record types. R2-A's scope lists only the §18 records. The stale condition "the admission head is
superseded" also needs the lineage. §8 says rev6 §4.4–§4.6 were not read and does not list §4.1–§4.2 as read. A
builder would have to invent the missing records or silently drop checks.

PLAN R2 names what qualification must bind: principal, actor, subject, operation, authority revision, realization,
and stale/revoked/unknown. O3, O4, O5, O8, O9 and R1–R4 cover that. O1, O2, O6 and O7 can wait for the publication
mechanism that defines their records.

Estimate: R2-A at $8–11 builder is `unverified` by the proposal and by me; the packet excludes telemetry, so I
compared no node actuals. As a size reference only, 6.1's own decomposition puts Gate O and `satisfies` at 500–800
lines and tests at 2,000–3,000 (`AMENDMENT-6.1.md:289-291`, itself labelled not established). The scope gap above
is a larger risk to the estimate than the arithmetic.

### F-9 (P3). Production instantiation in a fixture-only crate

`binding` and `native` exist only under the `fixture` feature (`work-engine-review-delivery/src/lib.rs:36-41`;
`Cargo.toml:13-17`). There is no production entry in the delivery crate, so §4.5 item 5 ("the production path is
unchanged") is true because none exists, and Q10's compile-fail test would assert against an entry that would have
to be written for the test. `Store::admit` already refuses every v1 candidate under O5 (`store.rs:246-270`), so a
production Gate O adds refusal reasons and no reachable behaviour.

A policy type parameter is cheap, and one gate implementation for both domains is a real benefit, so I would keep
`P`. I would drop the uninhabited `EffectAdmission<Production>`, state Q10 as feature-gating plus the absence of a
production constructor (the existing pattern at `work-engine-authority-store/src/fixture.rs:9-37`), and state Q11
as "existing refusal tests unchanged".

### F-10 (P3). Identity types defined in log terms

`ObservationRevision{position, head}` and `IssuanceId` (6.1 F5: the frame digest "which includes position and
predecessor", `AMENDMENT-6.1.md:218`) are serialized-log concepts. With storage undecided, define them by property:
an issuance identity unique per act of issuing, and a totally ordered revision with a state digest.

### F-11 (P3). Smaller items

- "Optionally, `Store::admit` delegates its O5 computation": optional coupling of P1-F history to the new owner.
  Decide it. Pre-release policy allows retiring `Store::admit`; leaving two "ordinary admission" entry points sits
  badly with one owner.
- Three meanings of "admission" would coexist in the vertical: the episode's evidence admissions
  (`binding.rs:51`; `preparation/src/fixture.rs:33`), P1-F's `FixtureAdmission`, and `EffectAdmission`. The
  `consumption-evidence-mapping` blocker is about the first. Name them apart.
- D1 bundles three decisions (crate as home of the decision logic; Gate R at the host; blockers as discharge
  obligations). A crate is an implementation home; the owner is the organizational authority. Split the question.
- Citations: "matching strings are not a mapping" is `prepare.rs:484`, not `:483`; "execution proximity" is
  invariant 4, not among the cited 2, 3, 11, 12 (`authority-and-ownership.md:598-620`). Neither changes a
  conclusion.
- §3's `unverified` classification of separation: I loaded §3.9 (`evidence-and-claims.md:11-47`). The separation
  requirement originates in accepted requirements (`enforce.rs:269-283`; provenance rows at `prepare.rs:422-427`),
  an owner other than the gate, and its violation is nameable without the gate (an actor accepts its own work). I
  concur that it is independently required. Its evidence mechanism is the open part (F-6 item 2).

---

## 3. Q1: does the proposal satisfy R2 as written?

Partly.

| R2 requirement | Standing |
|---|---|
| Owners reconciled with existing admission and publication owners | Met for `slice-campaign`, `session-task-admission`, `review-episode-core`, `work-engine-authority-store` |
| Reconciled with lifecycle owners | `not established` (F-7) |
| No parallel grant owner | Met in substance; P1-F `Store::admit` left undecided (F-11) |
| No parallel scheduling owner | Not met while R2-B builds an epoch ledger with no decided owner (F-7) |
| Derivation separate from publication and per-effect admission | Met |
| Qualification binds principal, actor, subject, operation, authority revision, realization | Bound as identities; actor and realization are declared values with no stated evidence mechanism (F-6); operation boundary incoherent across two effects (F-2) |
| Stale, revoked, unknown handled | Defined; enforced at consumption only (F-2) |
| Fixture identity cannot satisfy a production binding | Met by feature gating and absence of a constructor (F-9) |

Code-seam citations I checked and found accurate:

- `work-engine-organization/src/enforce.rs:257, 269-283` (one profile; separation rows `None`); `:226, 248`.
- `work-engine-authority-store/src/store.rs:205-218, 233, 242-270, 272-276`; `lib.rs:1-7`; `fixture.rs:1-37`.
- `work-engine-review-preparation/src/prepare.rs:23-28, 451-541` (all seven blocker groups);
  `realization.rs:1-9, 26-47`; `fixture.rs:45-98`.
- `work-engine-review-delivery/src/native.rs:1206-1210, 1224`; `binding.rs:112-117` (equality only).
- `review-episode-core/src/reducer.rs:105-148`.
- `session-task-admission/src/admission.rs:1-31`; `native_authority.rs:1-2`; `pi_pilot.rs:1-2`.
- `slice-campaign/src/admission.rs:4-18`; `installed_authority.rs:41, 254` (declarations only);
  `job_admission.rs:110` (declaration only).
- `design/architecture/organizational-compilation.md:385-456`; `runtime-realization.md:132-210`;
  `authority-and-ownership.md:285-315, 554-624`.

Citations that are inaccurate or incomplete: §4.5 item 4 and §2 seam (e) (F-1); "the delivery host" as a production
boundary (F-9); the two line slips in F-11.

Not checked: the handoff-rev7 and PLAN-R2 line citations, the two `design/ideas/pending` status blocks, and the
proposal's absence claim for Gate O/Gate R mentions.

## 4. Q2: are rev6 defects carried forward? Is the storage boundary sound?

| Rev6 item | Carried forward? |
|---|---|
| F1 content before activation | Not in code (storage out of scope). Must appear as a source obligation (F-4) |
| F2 `Abort` uncertainty | Same |
| F3 observation boundary lifetime | **Partly carried** (F-2): no reservation, no sealing, caller-built admission, B12b untested |
| F4 decision-kind coverage | Avoided by removing `decisions` (6.1 F4) |
| F5 revocation identity | Addressed by 6.1 F5; identity is log-shaped (F-10) |
| F6 test oracle | **Carried** (F-3) |
| F7 loss model, growth | Not applicable without storage |
| C5-B1 | Not carried into code. Hidden at the contract level until source obligations are stated (F-4) |

The "observation contract, storage undecided" boundary is sound in structure: gate logic is pure over one
observation and the fixture source is in memory. As written it hides the storage problem, because the contract
would accept the reader C5-B1 rules out.

## 5. Q3: is it the simplest design?

Not yet. Machinery I would remove or defer, in order of weight:

1. The fixture epoch ledger (F-7).
2. O1, O2, O6, O7 and the records they imply (F-8).
3. The discharge map as presented data (F-5).
4. `EffectAdmission<Production>` and the Q10 compile-fail against a non-existent entry (F-9).
5. The optional `Store::admit` delegation (F-11).

Closest to the S1 lesson: the ledger (item 1), and the positive end-to-end path as a whole. After D3 and D4 its
discharges are all fixture-authored: the separation attestation, fixture principal and binding records, a fixture
realization acceptance, and the declaration for the three unowned mappings. That is acceptable as a paired control
for the negatives. It should be named an assumed discharge at fixture standing, not described as a realization the
child "truthfully satisfies".

## 6. Q4 and Q5: owner decisions

| # | Right question? | Assessment |
|---|---|---|
| D1 | Yes, but bundled | Sound. Split into three; the third (blockers as discharge obligations) needs F-5 resolved |
| D2 | Yes | Sound. Option A's cost is stated: two vantages under one account are never separate. On the Pi/Docker path that means no production separation between service-run vantages until distinct accounts or a key-anchored kind exist. The owner should confirm that consequence, not only the mechanism |
| D3 | Yes | Recommendation sound in intent. Framing wrong on mechanism and incomplete on consequence (F-1). Needs F-2 item decided |
| D4 | Mis-framed | Mandatory under D3, not optional; feasibility is answered here (constraint-free world only); "truthfully" must become "by labelled assumption" |
| D5 | Yes | Defer is sound |
| D6 | Yes | Sound once the source obligations are part of what is approved (F-4) |
| D7 | Yes | Cannot be decided from this proposal (lifecycle unread). Deferring is sound only if the ledger leaves R2 (F-7) |
| D8 | Yes | See §7; estimates `unverified` and scope under-specified (F-8) |

Missing: the four decisions in F-6.

**Q5.** D3 is consistent with R1's accepted exit and with `S1-ACCEPTANCE.json`: the record names the behaviour as
its `semantic_limit`, and PLAN R1 says the mismatch "stays unsatisfied". D3 closes that limit. It does not
contradict the acceptance, which stands for checkpoint `1cc77114`. It is safe inside R2-B only with these
conditions, none of which the proposal states:

1. the D4 world is fixed before dispatch as the constraint-free variant, and labelled;
2. the host derives the realization description from what it will run and feeds it to preparation;
3. the owner acknowledges that the S1 matrix is rebased onto the new world, with the frozen world as the negative;
4. `run_native` takes the admission or stops being public, so "no spawn on refusal" is a property of the crate;
5. delivery is decided to be a boundary or not (F-2).

## 7. Q6: slices

R2-A and R2-B are coherent as layers. Three problems:

- R2-A alone has no consumer. Its whole outcome is a pure crate exercised by its own tests until R2-B lands.
- The discharge vocabulary is consumed in R2-A and stabilized in R2-B (F-5).
- R2-B carries the unstated work in F-1 (host realization description, test migration) and F-2.

**A smaller first slice exists.** Cut by consequence, seam first:

**R2-S: refusal seam with assumed discharges.**

- Preparation crate: closed `BlockerKind` beside the existing ids. No change to `prepare`'s meaning.
- Delivery host: derive the realization description from `NativeConfig` and pass it to preparation, so
  `realization_evaluation` exists on the delivered path. Before spawn, refuse any `KnownIncompatibility` and any
  blocker kind without a discharge. Before consumption, evaluate again. In this slice the only discharges are a
  **labelled fixture assumption set**, total over kinds, with the standing of `FixtureDeclaration`. It is called an
  assumed discharge, not an admission.
- Second world: the frozen requirements without `realizationConstraints`, compiled by the unchanged compiler. Move
  the native suite to it. The frozen world becomes the negative: spawn count 0, `consumer_invocations() == 0`.
- Tests: proposal Q3, Q12, Q13 and the cross-module part of Q14; mutants (d) and (e); the S1 matrix on the new
  world.
- Excluded: records, Gate O/R, stale/revoked/unknown, epochs.

Outcome: S1's semantic limit is closed with a consumer consequence, and the blocker vocabulary is fixed as the
interface the admission core must satisfy. It does **not** meet R2's exit by itself; that needs the second slice.

Estimate: builder $9–13, review $12–15, remediation $3–5, about $24–33. `unverified`: derived from the packet's
per-slice figures and the relative size of the change, with no node actuals read.

**Second slice: admission core.** The proposal's R2-A reduced to O3, O4, O5, O8, O9, R1–R4, the five states, the
host-owned boundary of F-2, and 6.1's oracle classes; it replaces the assumed discharges for the separation,
component-binding, authority and realization-admission kinds. It now has a waiting consumer. Not estimated here.

If the owner keeps the proposal's order, R2-A should still be trimmed per F-8 and should include the boundary
function's signature, so that R2-B does not have to reopen the core.

Confidence in the seam-first recommendation: medium. It rests on my reading that the unresolved risk sits in the
seam (F-1, F-2, F-5) rather than in gate logic that rev6 already reviewed twice.

## 8. Revisions requested before the owner decides

1. Correct §4.5 item 4 and §2 seam (e): no claim on the S1 path; add the host-derived realization description to
   scope (F-1).
2. Restate D4 as mandatory under D3, with the constraint-free world and "labelled assumption" wording (F-1).
3. Decide and state whether delivery is a boundary. Make the host build the admission from an observation source,
   or state why a caller-built admission is acceptable. Add the same-observation/later-epoch case (F-2).
4. Replace §5's universal oracle sentence with 6.1 §6's classes (F-3).
5. Add the source obligations to §4.1 and remove "write in progress" as a mechanism (F-4).
6. Specify discharge as a host-derived total function over a closed kind (F-5).
7. Add the four owner questions in F-6.
8. Either decide D7 after reading the lifecycle owners, or remove the epoch ledger from R2 (F-7).
9. Reduce Gate O to the checks R2 names, or add the missing records to scope and re-estimate (F-8).
10. Re-slice or justify the order (§7). State the estimates' basis against node actuals, or keep them `unverified`.

## 9. What I read and did not read

Read: the packet; `manual-orchestration/SKILL.md`, `worker.md`, `call-cost.md`; `AGENTS.md`; `core.md`;
`system-design.md`; `evidence-and-claims.md` §3.9; the proposal in full; PLAN.md in full; `S1-ACCEPTANCE.json`;
`ROOT-S1-DESIGN-DECISION.json`; `assignment-log.md`; the rev6 review report (findings, Q, claims, hypotheses and
judgments); `STAGE0-CONTRACT-r6.md` §3.3, §4.3, §8–9, §18; `AMENDMENT-6.1.md` header and §2–§9; the source ranges
cited above.

Not read: `AMENDMENT-6.1.md` §1; rev6 §4.1–§4.2 and §4.4–§4.6; corrections r2 and the C5/C6 feasibility document
(known from the log only); S1 `REVIEW-1.md`, `RECHECK-1.md`, `MATRIX.md`; `attempt.rs` beyond line 60;
`work-engine-review-delivery/tests/binding.rs` and `tests/pure.rs` (searched, not read); lifecycle sources beyond
the two passages in F-7; `installed_authority.rs` and `job_admission.rs` beyond the cited declarations;
`handoff-rev7`, PLAN-R2.md and the pending idea documents; all telemetry JSON; the board (read denied).

Generalisations about the S1 suite rest on its shared helper and the crate-wide search stated in F-1, not on reading
every test.
