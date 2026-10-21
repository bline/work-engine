# S2 boundary audit: where an unrealizable obligation became an accepted slice

An audit of **implemented Work Engine mechanisms** against one historical case.
No model arms, no new inference. Separate from the Claude substitution pilot
because its subject is this repository's own execution governance.

Case: `rust-context-lifecycle-s2-20261007`, accepted. `s2-plan-v1` invariant 4
requires checking absolute deadlines before reporting success. The runtime cannot
obtain the authority expiry - `AuthorizedEntry` exposes five getters and carries
neither expiry nor wait budget (`lifecycle-store/src/lib.rs:44-61`), no public
store API returns an `AuthorityExpiresAt` for an attempt (`sqlite.rs:1289` is
internal), the runtime imports only the *types* (`clock.rs:3`), and the plan names
"a needed S1/core mutation" as a boundary-change stop. The slice was accepted with
the deadline machinery built and wired to nothing.

Each boundary below answers five questions: required property, existing
mechanism, evidence of exercise, the transition that let an unsupported
conclusion become authoritative, and **the smallest missing capability** - the
last asked deliberately so the audit does not convert four failures into four
subsystems.

## Boundary 1 - decomposition realizability

1. **Required property.** Every obligation assigned to a slice has its required
   inputs available from an authorized producer through a published interface the
   consumer may use.
2. **Existing mechanism.** None found. Greps over `skills/slice-supervisor/scripts`
   and `skills/slice-builder/scripts` for realizability, required-input or
   producer checks return only incidental word matches in receipt assembly.
3. **Exercise.** Not applicable; nothing to exercise.
4. **Transition.** Plan acceptance (`plan_acceptance: human_approval`) admitted a
   slice whose invariant 4 had no reachable input.
5. **Smallest missing capability.** A list diff at plan acceptance: the slice's
   required inputs against the published interfaces of its named producers.
   `AuthorizedEntry`'s five getters against invariant 4's demand would have failed
   it before a line was written. **But see boundary 4 - this may be redundant.**

## Boundary 2 - execution stop

1. **Required property.** When a slice meets a declared boundary-change stop, that
   event and its disposition are retained, and acceptance can distinguish a stop
   that was ignored from one that was referred and authorized.
2. **Existing mechanism.** One field: `stop_reason` on the terminal receipt
   (`skills/slice-supervisor/references/receipt-schema.md:17`), defined "string or
   null | **Null only for accepted slices**".
3. **Exercise.** Non-null in **1 of 32** metrics rows. S2: `null`. **No retained
   artifact in the campaign state records a stop or boundary-change event** -
   searched; the only matches are `stop_on`/`stopping_criteria` in agent-loop
   configuration, which is orchestration and unrelated.
4. **Transition.** The condition for a stop obtained, no stop was recorded, and a
   disconnected mechanism was implemented. **Evidentiary limit:** the absence of a
   record establishes that no stop event was retained in the searched artifacts.
   It does **not** establish that the builder recognised the condition and chose
   to proceed. Nothing downstream could observe that a stop had been due, which is
   the defect regardless of what the builder knew.
5. **Smallest missing capability.** Not "accepted with a stop reason" - a null
   `stop_reason` on a final receipt may be correct. What is missing is a **stop
   event with a disposition**, retained independently of the terminal receipt, so
   that two histories stop being indistinguishable: *stop met, ignored, accepted*
   versus *stop met, referred to owner, authorized, accepted*. Today both produce
   `stop_reason: null` and no event. This is a retained-history gap, not a schema
   contradiction - my earlier framing of it as the latter was wrong.

## Boundary 3 - finding attribution

1. **Required property.** A finding carries where the inadequacy lies, which
   authority may correct it, and what consequence is permitted - not only that a
   discrepancy exists.
2. **Existing mechanism.** None for this purpose. `receipt-schema.md` has
   "specialist dispositions", which record *which perspectives were selected*, not
   where a finding's fault lies. `codex-adversarial-review` has no attribution
   vocabulary, no contract-unrealizability class, and no false-clearance class.
3. **Exercise.** Across seventeen review runs on this subject, every one of the
   plan's three internal defects was attributed to the implementation, one
   prescribing removal of plan-mandated work. Recorded at
   `metrics/probes/claude-finding-attribution-20261020.md`.
4. **Transition.** A true finding with a wrong attribution entered remediation
   with the builder as its owner, and the review contract had no state in which to
   say otherwise.
5. **Smallest missing capability.** A nominated classification on each finding -
   candidate nonconformance / contract unrealizability / contract inconsistency /
   evidence insufficiency / unresolved attribution - that a reviewer may propose
   and may not resolve. The reviewer gains no authority; the finding stops
   silently selecting an owner.

## Boundary 4 - acceptance completeness

1. **Required property.** Acceptance establishes that each mandatory invariant has
   evidence, and distinguishes "no concern reported" from "no concern exists".
2. **Existing mechanism.** `unresolved_concerns` exists and is **actively used** -
   non-empty in 6 of 32 rows, carrying exactly this class of statement for S1, S4,
   S5 and S6/S7. So the system can represent and preserve a known unresolved
   concern.
3. **Exercise.** S2 recorded `unresolved_concerns: []`, `stop_reason: null`,
   `more_in_scope_work_remains: false`, 6 findings all `verified_resolved`, 0
   unresolved, and all three `validation_requirement_results` passed.
4. **Transition.** An empty list was treated as sufficient. **Correction to my
   first reading:** this was not a judgment failure inside a working mechanism.
   The list plausibly records what reviewers reported accurately; nothing obliged
   it to be complete. Raised by ChatGPT, and confirmed by (5).
5. **Smallest missing capability, and it is the cheapest fix in this audit.**
   **There is no correspondence obligation between a plan's invariants and the
   evidence accepted for it.** Searched `skills/` for any per-invariant or
   per-obligation evidence requirement: **zero hits.** Acceptance verifies three
   requirement *categories* - `semantic_proof`, `risk_proportional_checks`,
   `workspace_integrity` - each a list of commands and suites. `s2-plan-v1` states
   **seven numbered invariants**. Nothing maps one to the other.

   The fix is a mapping requirement: each invariant names its evidence, and
   **"none" is a visible state rather than an absent row.** Invariant 4 would have
   carried an empty evidence row through a human acceptance gate.

## The conclusion that changes what to build

Boundaries 1 and 4 are **the same correspondence checked at two different times** -
obligation against its required inputs before execution, obligation against its
evidence after. Boundary 4's version is strictly cheaper: it needs no model of
producers, interfaces or authority, only a table with a row per invariant, and an
unrealizable invariant announces itself by having nothing in its row.

So the realizability analyzer of boundary 1 is **deferrable, not eliminated** -
a correction to my first wording. The two act at different times and prevent
different losses:

- **Boundary 1** prevents spending a build on an obligation that cannot be
  satisfied. S2 spent five candidates and three repair rounds on a slice carrying
  an unsatisfiable invariant.
- **Boundary 4** prevents promoting an unestablished obligation into an accepted
  one.

Only the second is needed to stop *unsupported acceptance*, and it is far cheaper.
The economic order follows: implement the acceptance correspondence first, then
**measure** whether pre-execution checking avoids enough failed work to justify
its cost. That measurement does not exist and should not be assumed either way.

**The ledger must distinguish two dispositions, not one.** "Evidence absent" and
"evidence present but insufficient to establish the property" are different states,
and collapsing them lets a weak test receipt fill a row while establishing
nothing - which is the same true-but-insufficient failure this audit is about,
reproduced in the instrument built to detect it.

Two of the four boundaries need no new subsystem: boundary 4 needs a
correspondence obligation over artifacts that already exist, and boundary 3 needs a
vocabulary field. Boundary 2 needs a retained stop event, which is new state but
small. Boundary 1 is deferred pending a measurement of avoided waste.

## Scope and authority

This audit asserts nothing about S2's acceptance decision, which was human. It
authorizes no schema change, no skill change, and no reopening. It is evidence for
the decision-specific readiness work and the formed concern-scoped review
proposal, and it should not be read as approving either. Every source reference
above was read at the stated path; the metrics figures are from
`docs/ai-workflow-metrics.jsonl` as committed.

## Retrospective test of boundary 4's proposed safeguard

**Question:** could an invariant-to-evidence correspondence ledger have challenged
S2's acceptance using only information available at the time?

**Method.** A cold `claude-haiku-5-5`/`xhigh` arm, given the plan and the campaign's
retained acceptance-time artifacts, and **no implementation** - boundary 4's
question is whether the *record* supports the invariants. Four dispositions
available: ADEQUATE / INSUFFICIENT / ABSENT / INDETERMINATE, with the prompt rule
"Do not infer that an invariant holds because the slice was accepted, because
tests passed, or because no finding mentioned it."

### Attempt 1 VOID - my input selection, not the record

I supplied **22 of 136** artifacts, selected by a glob matching `candidate-[2-5]`
while the relevant files are named `candidate2`, `candidate3`. Every artifact the
arm reported as "referenced but not supplied" - four contract result records, the
F6 gate support script and its result - existed on disk. Result retracted:
"ADEQUATE 0 of 7" described a subset I chose, not the acceptance record.

Error class: selecting inputs by pattern without enumerating what was available,
the same class as the `--add-dir` void earlier the same day. Missing check: list
the directory before globbing it. **Caught by the arm's own
`WHAT THE RECORD CANNOT SHOW` section**, which named the missing files by digest -
the second time that day an arm's required statement of its own limits detected an
experimenter error.

### Attempt 2, with 132 of 134 acceptance-time files

Excluded: the two source worktrees, deliberately, to keep this a record-only test.
Included: every `.json`, `.md`, `.log`, `.txt` and the four gate-runner scripts.
Pilot-output leak check 0. 23 calls, 364 s, **$0.4103**.

**ADEQUATE 0 · INSUFFICIENT 6 · INDETERMINATE 1 · ABSENT 0.**

Six times the evidence moved **nothing** to ADEQUATE. What changed is the
precision: each disposition now names what the record establishes and what it does
not.

| invariant | disposition | what the record does not establish |
|---|---|---|
| 1 reservation and eager ordering | INDETERMINATE | whether the six named tests assert *ordering* rather than outcome; the registry-insert and spawn order in source |
| 2 transport fact vs lifecycle observation | INSUFFICIENT | "establishes only local failure and unsettled ownership at the runtime boundary, by name" |
| 3 bounded result channel | INSUFFICIENT | "establishes reserved capacity and no saturation deadlock... does not establish **overflow or gap reporting**" |
| 4 clock seam and deadlines | INSUFFICIENT | "the restart clause has no S2 evidence" |
| 5 framing ownership | INSUFFICIENT | the reader/writer task source, a malformed-frame test, an audit of the `select!` sites |
| 6 launch gate and close record | INSUFFICIENT | "does not establish transport close or the rejected and attempted states" |
| 7 controlled peers | INSUFFICIENT | "does not establish production unreachability or that a peer alters only its own ledger" |

Invariant 3's gap independently matches the overflow finding three pilot arms
raised - reached from the acceptance record alone, with no source and no
knowledge of any defect.

### The closing line is the boundary-4 finding in its sharpest form

> "All invariants: the retained review records a status per finding and
> **`tests_run_by_reviewer: 0`**. The `verified_resolved` statuses cannot be traced
> to what the reviewer inspected, and the `no_open_findings` verdict is not
> counted as evidence for any invariant."

The reviewer ran zero tests, each `verified_resolved` is untraceable to an
inspection, and the campaign's closure rests on the absence of open findings. That
is the chain the correspondence requirement is meant to break, documented from the
campaign's own records for 41 cents.

### What this establishes, and what it does not

**Establishes:** a cheap perspective, using only retained acceptance evidence and
no implementation, finds that **no** mandatory invariant of an accepted slice was
adequately evidenced, and names the specific gap per invariant. The safeguard is
not hypothetical and not expensive.

**Does not establish:** that any invariant is *violated*. INSUFFICIENT is a
statement about the record, not the code - and the record's own
`vertical_semantic_test_passed: true` and `verified_resolved` entries may well be
true. The ledger's claim is that acceptance cannot be traced to them, which is
exactly the distinction boundary 4 exists to enforce.

It also does not establish that the required evidence was unavailable in 2026-10-07.
Several gaps - test bodies, a `select!` audit, a repository-wide sleep search - are
cheap artifacts that were simply never retained. That is the actionable half: the
fix is largely a retention requirement, not new analysis.

### Correction: `tests_run_by_reviewer: 0` is a conformance attestation, not a defect

**Claim.** I reported the ledger's closing line as "the reviewer ran zero tests",
framed as damning.

**Error class.** Reading a boundary-conformance counter as a performance measure.
In `s2-candidate5-contract-retained-result.json` the field sits beside
`source_mutations_by_reviewer = 0`. Both are attestations that the reviewer stayed
**read-only**, which `codex-adversarial-review` requires - the same constraint my
own Test R prompt imposed on every arm ("no builds, no test runs"). Zero is the
correct value. A non-zero value would be the finding.

**Consequence.** I presented conformance as misconduct and overstated the audit's
force. The surviving finding is narrower and is the one the ledger actually
supports: the record carries `finding_statuses[*].status = verified_resolved`
with **no field recording what the reviewer inspected to reach that status**. The
reviewer's conformance is attested; the basis of each resolution is not.

**Missing check.** Read the field's siblings before interpreting it.

Nothing in the audit establishes that the reviewer performed no source
inspection, that validators ran no tests, or that any invariant is violated. What
it establishes is that the retained evidence does not connect individual
invariants to the observations that warranted accepting them.

### What the correspondence requirement should actually demand

Not retention of everything - a test body is reconstructible from a pinned
revision, and duplicating it into every acceptance record buys nothing. What is
needed is a **relationship**, one row per mandatory invariant:

| field | content |
|---|---|
| required property | the invariant, in the plan's terms |
| observations relied upon | the exact tests, inspections or gate checks |
| pinned identity | artifact digest and revision at which they were taken |
| what they establish | stated, not implied by their passing |
| remaining gap | and its authorized disposition |

**And two levels of checking, which must not be conflated.**

- **Deterministic:** every mandatory invariant has a row, each cited artifact
  exists at the cited revision, and no row is empty. Cheap, mechanical, no
  inference. This alone would have surfaced invariant 4 at S2's acceptance gate.
- **Semantic:** whether the cited observations actually establish the required
  property. This is what the $0.41 arm did, and it still needs inference or
  authorized human judgment.

Treating the second as though the first guarantees it would reproduce, at the
level of the mechanism, exactly the true-but-insufficient failure the mechanism
exists to catch.

### The next test is prospective, not retrospective

This audit shows the mechanism can expose a deficiency in a historical record.
It does not show that it changes a decision at the moment the decision is made.
The test that would: require the correspondence rows for a candidate **approaching**
acceptance, before the acceptance decision, and measure two things - whether the
disposition changes, and **whether the supervisor can construct the rows from
existing artifacts without substantial additional inference.** The second is the
economic question; if row construction costs as much as a review, the mechanism
is not the cheap safeguard this audit suggests.

`manual-decomposition-s10-20261020` has slice C pending, which is the natural
subject. That is the campaign owner's decision, not mine, and this audit
authorizes nothing there.

### Correction: the deterministic gate as I specified it would have passed S2

**Claim.** I wrote that the deterministic check - "every mandatory invariant has a
row, each cited artifact exists at the cited revision, and no row is empty" -
"alone would have surfaced invariant 4 at S2's acceptance gate".

**Refuted by this audit's own result.** Ledger v2, given the complete record,
returned **ABSENT 0**. Every one of the seven invariants had *something* citable.
Invariant 4 had the S2-C2-F2 repair summary ("monotonic post-await deadline
classification and latched late disposition across drains") and the
`ready_exit_after_cutoff_and_later_drain_keep_late_disposition` test. A row for
invariant 4 would have been non-empty, its citations exist at their revisions, and
the gate would have passed.

Attempt 1 returned ABSENT 1 only because I had withheld most of the record. So
the single state my deterministic gate detects is the state that **did not occur
here**, and it appeared in my data only as an artifact of my own input error.

**Consequence.** The cheap deterministic layer does **not** substitute for the
semantic pass on this case. The $0.4103 judgment did the work: it is what
distinguished "the F2 repair establishes post-await monotonic classification" from
"invariant 4 as stated also requires wall-time sampling for S1 authority checks,
the rule that a late close does not extend proof authority, and restart
non-persistence - none of which is named". No structural check reaches that.

**Error class.** Proposing a mechanism and asserting its sufficiency without
running it against the case that motivated it. This is the same
true-but-insufficient shape the audit is about, now in the safeguard I proposed -
the fourth level at which it has appeared today, after a review judgment, an
experimenter's verdict and a campaign's acceptance.

### The corrected design

**ABSENT is a valid ledger state, not a condition to forbid.** Requiring a
non-empty row rewards filling it with a weak citation, which is the failure mode
S2's seven green gates against seven invariants already demonstrates.

| layer | what it establishes | cost |
|---|---|---|
| **deterministic** | every mandatory invariant has a row; the row carries a disposition; references resolve at their cited revision, or the row carries an explanation of what is missing | no inference |
| **semantic** | which of **SUPPORTED / INSUFFICIENT / ABSENT / INDETERMINATE** holds, with its basis | inference or authorized human judgment |

The deterministic layer guarantees **complete accounting**. It does not and cannot
guarantee **sufficient evidence**. On this case it would have caught nothing; its
value is preventing an invariant from silently vanishing, which is a different and
still worthwhile protection.

So the honest economic statement is: **the audit priced the semantic layer at
$0.4103 for seven invariants and did not price the deterministic layer at all,
because the deterministic layer would not have fired.** Assembling the references,
verifying provenance and resolving disputed dispositions remain unmeasured, and
the supervisor floor measured today is ~1 Mweighted per slice against which any of
this must be charged.

## Adjudication of the seven dispositions against S2's own scope

The question: was the $0.4103 assessor detecting genuine evidence gaps, or applying
requirements S2 did not own? Checked against the plan's deferred-scope paragraph -
"S4 service/store composition, durable result-ingress/write-failure handling,
authenticated command boundary, S5 native provider, live proof, S9
migration/rollback, activation, Q3/Q4 and publication are deferred."

| gap the ledger named | S2-owned? | basis |
|---|---|---|
| inv 2: port evidence extension only establishes local failure | **yes** | integration request 3 asks for the extension in `lifecycle-provider/src/lib.rs`, inside the 13-path boundary. "S5 native provider" is deferred; the **port contract** is S2's |
| inv 3: overflow and gap reporting | **yes** | invariant 3 states "Preserve and report overflow/gap explicitly"; T32 is in the plan's own scenario list |
| inv 4: wall-time sampling for S1 authority checks | **yes** | and this is the realizability conflict - S2 owns the sampling and cannot reach the value |
| inv 4: late close must not extend proof authority | **yes** | invariant 4's own text |
| inv 4: restart never persists a monotonic instant | **yes** | a type-design property of S2's clock seam. Not covered by "durable result-ingress" or "S9 migration/rollback" |
| inv 5: framing task, malformed-frame test, `select!` audit | **yes** | invariant 5's own text, in the 13-path boundary |
| inv 6: transport close, rejected and attempted states | **yes** | invariant 6 states "close transport" and names the full disposition vocabulary |
| inv 7: production unreachability, peer alters only its ledger | **yes** | invariant 7's own text; the plan's `workspace_integrity` gate even names "no fixture provider in production capability" |

**None is deferred. The assessor was not over-reaching.**

Two supporting observations:

1. **The restart clause is the one where the two runs disagreed, and the complete-record
   run was the more accurate.** Attempt 1 called it "persistence scope the record
   defers to S4"; attempt 2 called it "no S2 evidence". Attempt 2 is right - and it
   is independently confirmed by both Claude builder arms, which implemented it as
   a type-design property: private fields, no constructor from data, no `Serialize`,
   epoch-tagged instants whose `duration_since` returns `None` across epochs. An
   obligation two independent builders discharged in source is an obligation S2
   owns.
2. The remaining uncertainty is not scope but **adjudication of adequacy**, which
   is a judgment and which this audit does not settle. The dispositions are
   defensible against scope; whether each is the *correct* adequacy call is a
   separate question an authority would own.

## The lever is obligation granularity, not ledger sophistication

Invariant 4 as written contains **four** distinct obligations - wall-time sampling
for S1 authority checks, post-await deadline checks, late-close recording without
extended authority, and restart non-persistence. The record offered evidence for
the second and that evidence was accepted for the whole invariant.

A finer-grained plan representation would have made that substitution impossible
without stating it: four rows, four dispositions, and the F2 repair discharging
exactly one of them.

This is **not** a free deterministic win. Deriving correct sub-obligations is
semantic work, and deriving them wrongly reproduces E1's coverage failure -
E1 held the right cancellation property and bound it to the wrong subject class.
But it relocates the lever: from making the evidence ledger cleverer to making
the accepted obligations finer, which is a Plan IR and compiler concern rather
than an acceptance-mechanism concern.

That also explains why the deterministic gate could not fire here. It was checking
one row for a property that was really four.

## Candidate principle, recorded as formation evidence only

**Every compiled obligation must have an independently attributable disposition.
Its satisfaction may support another obligation only through an explicit,
warranted relationship.**

**Corrected from "independently decidable", which this pilot's own evidence
refutes.** Some obligations are inherently relational and cannot be decided in
isolation - and the clearest case is the one that worked. F3 ranges over the
registry row, the `ClosedTask` local and the loop local at once, and the
obligations that recovered it (D's OB1/OB2, and OB39 alone in E6) required
tracing handle ownership *across* cancellation of enclosing futures. A
decidability criterion would have disqualified the only obligations that
succeeded. Attributable disposition is the property that matters: each obligation
carries its own disposition, evidence and explicitly represented dependencies,
and no successful check silently discharges an unrelated requirement.

Invariant 4 is the instance. Four constituent obligations - wall-time sampling for
authority checks, post-await classification, late close without extended proof
authority, restart non-persistence. The campaign produced legitimate evidence for
the second. That evidence then stood for the whole.

**The evidence was not wrong. Its scope of authority was wrong.** Which is a
different failure from inadequate testing: a test can be correctly executed, its
receipt authentic and its result accurate, and the acceptance decision still
unjustified.

Under the principle, an invariant's aggregate judgment is **deterministic** once
its constituents and their required relationships are represented - it cannot
claim satisfaction unless every constituent is adequately established or
legitimately disposed of. The hard inference moves upstream, to producing a
decomposition that is complete and semantically faithful. That is a Plan IR
concern.

This is formation evidence for Plan IR and implementation-contract compilation.
It authorizes neither.

## The gaps do not authorize repairs

All eight gaps are S2-owned. **That does not make all eight implementation
defects.** Each could be any of:

- behaviour already correct, lacking evidence;
- genuinely missing functionality;
- an execution contract that needs correcting - invariant 4's authority-expiry
  clause is the standing example, since S2 cannot reach the value and the plan
  names that mutation as a stop.

The obligation-to-evidence relationship establishes **what is not established**.
It does not select a remedy or an owner. Keeping that boundary is what prevents
the review-attribution failure recorded in
`metrics/probes/claude-finding-attribution-20261020.md` from reappearing inside
the acceptance mechanism - a reviewer prescribing removal of plan-mandated work
is the same error as a ledger prescribing an implementation fix for an
unrealizable requirement.

## What the investigation established, at the level the evidence carries

Four independent demonstrations of one failure, at four levels:

| level | instance |
|---|---|
| a review judgment | Opus established a true synchronisation property over one accessor set; the required property was a different one (generation 3, arm B) |
| an experimenter's verdict | I verified the inner-timeout property and recorded the outer-cancellation property (Test S, F3 CLEAN, corrected) |
| a campaign's acceptance | evidence for one of invariant 4's four clauses stood for all four |
| a proposed safeguard | I asserted a deterministic gate would have caught it; ledger v2's ABSENT 0 shows it would have passed |

In every case a locally valid result was promoted beyond what it warranted. The
cheapest interventions found are, in order: **finer obligations in the plan**, and
**recording what each inspection was relied upon to establish** - a correction to
my earlier "recording what was inspected". Provenance is not the scarce field.
`S2-C2-F2`'s repair record has perfectly good provenance; what was overstated is
what it was relied upon to establish. A record can carry exact digests, revisions
and timestamps for a test and still warrant the wrong conclusion, which is the
invariant-4 failure precisely. The missing field is the one connecting an
observation to the specific clause it discharges.

Neither requires a more capable model, and that is the economic point: once these
relationships are durable and mechanically checkable, each new model does not have
to reconstruct them through inference.

## A counter-instance: a deterministic check that did fire

Recorded so the refutation above is not overgeneralised into "deterministic checks
are useless". They catch what they are shaped to catch.

`manual-decomposition-s10-20261020/slices/C2`, same day, different campaign: the
artifact-manifest digest check reported **eight mismatches**. Cause, per
`handoff-amendment-v2.md`: the result writer enumerated files while its SQLite
connections were still open, so it bound eight transient `-wal` and `-shm`
sidecars that SQLite then removed on close. V2 closes each connection before
enumeration and excludes sidecars; verification opens copies in immutable
read-only mode, leaving zero sidecars. 150 source-manifest paths, 20 request
digests and four stopped-database integrity checks pass. `result-v1.json` is
preserved unchanged.

**Two things this establishes.**

1. **A digest-and-existence check fires on a real receipt error.** What it cannot
   do is adjudicate scope of authority - whether cited evidence establishes the
   clause it is offered for. Both facts now have instances on the same day: this
   check caught a perturbed inventory; my proposed gate would have passed S2's
   invariant 4 because the row was non-empty and its citations resolved.
   Structural completeness and semantic adequacy are independent dimensions, and
   each layer is worth having for what it alone detects.
2. **A distinct error class, not the one this audit is about.** This is not a
   true-but-insufficient conclusion. It is an **observation that perturbed its own
   subject**: the receipt writer created the artifacts it then bound as evidence.
   Worth naming separately - measurement artifacts entering an evidence inventory
   - because no amount of obligation decomposition addresses it, and the fix is
   ordering, not representation.

**And the lineage discipline is what S2's record lacked.** V1 is preserved, the
amendment names the cause, states what did and did not change, and bounds the
consequence explicitly - "no substantive effect on the C2 behavioral conclusions
or process disposition; no test rerun was needed". Compare S2, where closure
appears only in the terminal audit and no field records what the reviewer
inspected. The contrast is evidence that the retention the audit asks for is
achievable within the current artifact conventions, by a session that was not
asked to do it.

## Capability and efficiency are separate, and every measurement says representation bought the first by paying for the second

E6 is strong evidence that execution organisation changes **what a model
establishes**: same `haiku-5-5`, same `xhigh`, same OB39 wording, F3 recovered
alone and missed among 25. It is **not** evidence that representation reduces the
inference needed to establish the same result, and the two measurements I have
both go the other way for equivalent scope.

| comparison | capability | cost for equivalent scope |
|---|---|---|
| E2-haiku (31 in one context) vs E8 (31 across four) | recall 1/4 -> **2/4** | $0.7243 -> **$2.2509**, 3.1x |
| E5 (25 obligations) vs E6 (OB39 alone) | F3 miss -> **match** | $0.5542 for 25 in one context -> $0.3697 for **1** alone |

So the honest summary is: **better representation improved what was established
and raised the cost of establishing an equivalent scope, in both cases I
measured.** Nothing recorded here supports "structured representation is cheaper".
What it supports is "structured representation establishes more, and the price of
that is unpriced against an equivalent-coverage baseline".

That is a real constraint on the economic hypothesis, not a caveat on it.

**Withdrawn: the $9.24 figure.** I quoted 25 x $0.3697 as the cost of discharging
E5's packet one obligation at a time. That is a linear extrapolation and the
measured data contradicts it. E8 discharged **31 obligations across four contexts
for $2.2509** - about $0.073 per obligation, a fifth of the per-obligation rate
the extrapolation assumes - because cost is dominated by a **per-context** term,
not a per-obligation one. E5 then measured **25 obligations in one context at
$0.5542**, cheaper in absolute terms than E6's single obligation at $0.3697 plus
any second context.

So cost does not scale with obligation count in any simple way, and no measured
figure supports a 17x penalty for full decomposition. The only measured
multi-context total is E8's $2.2509 against E2-haiku's $0.7243 for the same
content: **3.1x**. That is the number, and it is the one comparison where both
arrangements discharged the same obligation set.

## What the deterministic machinery should do, stated precisely

The five recorded failures are all errors in the relationship between an
observation and a conclusion. That suggests the machinery's job is **not** to
determine whether each conclusion is true. It is to **preserve the boundaries of
what has been established** - which is a far more achievable property, and the
one every instance today argues for:

- a completed execution does not imply satisfaction of every accepted obligation;
- an authentic evidence receipt does not imply the evidence supports every
  property associated with it;
- a positive judgment does not discharge another property without an explicit
  warranted relationship;
- an unresolved obligation cannot silently disappear from the accepted
  consequence.

Each of those can be enforced once the relationships are represented, **without
solving the underlying semantic question**. The distinction worth keeping:
deterministically establishing correctness is not available; deterministically
preventing unsupported claims of correctness is.

## Control condition for D, so two benefits are not conflated

The prospective and retrospective routes must reach the **same underlying
artifacts** and be evaluated against the **same obligations**. If the prospective
process also produces *better* evidence, a cheaper judgment could be credited to
cheaper assessment when it was really improved evidence. Both are worth having;
attributing one to the other would reproduce, in the measurement, the same
scope-of-authority error this audit is about.

## A hazard for adaptive perspective routing, with a data instance

Raised by ChatGPT at close: historical success cannot be the only criterion for
choosing which perspectives to fund, or the system becomes efficient at finding
familiar defects while systematically ignoring novel ones.

**This session supplies the instance that makes it concrete rather than
cautionary.** From the dispositions recorded across the protocol files - my own
records, not re-derived from the streams:

- **F1** (concurrent-drain panic) was matched by nearly every arm that looked at
  the process side: all four generation-2 arms, A1, C, B, D, E5, and partition
  P3. It is the cheapest finding in the pilot.
- **F3** (handle detached on drain cancellation) was matched by **two** arms in
  the whole investigation - D, with hand-authored ownership obligations, and E6,
  with one obligation and nothing competing.

A router allocating perspectives by historical recall would therefore fund
F1-class concerns heavily and F3-class concerns barely - **the exact inversion of
where help was needed.** F1 needed no help. F3 needed a dedicated obligation and
an uncontested context, and even then two of seventeen runs found it.

So exploitation metrics are **anti-correlated with where additional perspectives
pay** on this subject, and recall history is close to the worst single signal
available for that decision.

**And exploration is not a weighting problem.** My first wording - "an
exploration term whose cost and authority are explicitly governed" - still frames
it as a coefficient in a score over history. No score over historical data
reaches a concern that was never represented in it. The instance is in this
session: E4's three CONFLICTS are **contract** defects, and no reviewer-recall
statistic over any history would have surfaced them, because no reviewer was ever
asked. Exploration needs an independent justification, not a different formula.

**Two routing decisions, needing different evidence:**

| decision | question | evidence that bears on it |
|---|---|---|
| perspective formation | what material uncertainty requires independent investigation? | not recall history; this is the exploration problem |
| model selection | which configuration can establish the admitted property at acceptable cost? | E6 is exactly this evidence - `haiku-5-5` can discharge OB39 alone for $0.37 |

Collapsing them is how a system becomes efficient at familiar defects. E6
answers the second question and says nothing about the first.

## The three-way relationship this investigation found

Stated because the pilot kept treating these as one variable and they are not:

- **Representation determines what can be asked.** E1 held the right cancellation
  property and bound it to the wrong subject class, so the question was never put.
- **Organisation influences what gets examined.** E6: identical obligation text,
  one obligation instead of 25, F3 from miss to match at the same model and
  effort.
- **Evidence determines what can legitimately be concluded.** The acceptance
  ledger: 0 of 7 invariants adequately evidenced, with no implementation read and
  no defect discovered.

**Model capability operates inside all three.**

**Correction to my first wording, "was never the binding constraint on any of
them".** Error class: generalising from the F3 and F4 outcomes to the whole
investigation. Counter-instance from this pilot's own record: in the RP2
comprehension arms, **six Claude runs at two effort levels all failed to recover
the `requestId` 128-byte grammar, `OUTPUT_SLOTS` and the NDJSON version, which
`gpt-6-sol`/medium recovered in 8 calls.** Nobody showed that to be a
representation or organisation failure; it may be capability, and it was never
isolated. Consequence: a stronger claim than the evidence supports, in the
sentence meant to summarise the investigation. Missing check: look for a cell
where the intervention was not tried before claiming the variable never bound.

The defensible version, which is also the more useful one:

> **Apparent model incapability can be a failure of representation or execution
> organisation, so increasing model capability is not necessarily the remedy.**

That is testable and actionable. "Capability never binds" is neither.

## Correction to a figure quoted at close

The closing summary cites "thirteen recorded corrections". The measured figure is
**19 correction commits of 56 commits** on 2026-10-20, counted from the commit
log. Thirteen was my own earlier in-conversation estimate, made before I counted.

## Pre-registered stopping condition for D

Recorded now so it cannot be adjusted later: **if prospectively recorded
correspondence costs more fully loaded inference without improving warranted
decisions, that is a valid negative result and is to be reported as one.**

Controls, all of which must hold or the comparison is uninterpretable:

1. Independently adjudicable obligations frozen **before** execution.
2. **Perspective count and grouping identical** between the two arms - E8 shows
   organisation is a large separate effect.
3. Both arms reach the **same underlying artifacts** and are evaluated against the
   **same obligations**, so improved evidence production is not credited as
   cheaper assessment.
4. The prospective route is charged for **generating and validating** the
   relationships, not only for the final assessment.
5. The compared quantity is **inference per adequately warranted decision**, not
   receipt completeness and not any single token category.
6. **Decision, warrant quality and economics are scored separately**, never
   collapsed:

   | dimension | measurement | failure it prevents |
   |---|---|---|
   | decision | did the authorized disposition change? | treating an unchanged decision as no benefit |
   | warrant quality | was the decision more adequately supported? | treating more citations as more establishment |
   | economics | fully loaded inference to reach that warranted decision | pricing volume instead of consequence |

   Control 5 puts warrant in the denominator without saying how it is measured,
   which is not enough. **Warrant quality is scored by the triple** - does each
   disposition name the property, the states it ranges over, and the basis on
   which its enumeration is complete - not by citation count or length.

   **Completing the rubric, because as written it would misscore the defect class
   this pilot is about.** A warrant is complete *relative to the property being
   established*. For F4, state and observation surfaces sufficed. For F3, the
   property also required the **transitions** that produce those states -
   cancellation of the enclosing future - and E5 enumerated the state correctly
   while attributing it to one cause. So for a property concerning ownership,
   ordering, cancellation or lifecycle evidence, the exhaustiveness field must
   cover the transitions into each relevant state and not only the states and
   their accessors. This is recorded in
   `metrics/probes/claude-s2-obligation-compiler-20261020-protocol.md` and was
   missing here; the two artifacts now agree.

   **Volume did not track warrant quality in either direction, which is the
   defensible claim.** I first wrote "anti-correlated", which asserts an inverse
   relationship from two instances. Instances against volume: generation-3 A1 used
   36% more thinking at `max` than at `xhigh` and produced a *less* falsifiable
   clearance; E2 returned 13 findings against D's 7 at 1/4 recall against 2/4.
   Instance the other way: `haiku-5-5` on the same 31 obligations returned **16**
   findings to `opus-5`'s 13, with per-obligation dispositions carrying an explicit
   Basis column where the Opus arm wrote prose, at a fifth of the cost. **Volume is
   not a proxy for quality in either direction**, which is a stronger and more
   useful conclusion than an inverse relationship.

No further retrospective S2 arm. That subject has yielded what it has, and
additional arms would increasingly tailor the method to defects already
understood.
