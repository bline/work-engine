# Pre-registration: campaign R2, measuring the read-set instruction change

Status: pre-registration, written 2026-10-22 **before campaign R2 is designed or
dispatched**. Baseline values below are computed from R1 now and frozen here, so
they cannot be chosen after R2's result is known.

Measurement role only. This arm does not build and does not review; it fixes the
metrics, the cohort rule and the falsification conditions in advance, and scores
afterwards.

## The treatment

Commit `cb61f807`, "Size manual-orchestration packets by their read set", adds to
`references/worker.md`:

> Read only what your packet's read list names; reading spends the same budget as
> writing. Prefer signatures (`grep -n 'pub fn'`) and targeted line ranges over
> whole files, batch independent reads, and commit after each independently
> passing unit.

and the corresponding paragraph to `references/coordinator.md`, telling the
coordinator to name the read set as precisely as the write scope.

## Why R1 cannot answer this

R1's post-change cohort is contaminated and unusable for this question:

1. **Defect R1-M-001.** Two copies of the skill existed; the `Skill` tool served
   `~/.claude/skills`, which never received `cb61f807`. `afd6ae453ed824ee9` (S3b)
   read only that copy *after* the divergence, so it ran after the change without
   receiving it. Cohort membership could not be read off commit time.
   **Resolved 2026-10-22**: `~/.claude/skills/manual-orchestration` is now a
   symlink to `skills/manual-orchestration`, so one text exists and divergence is
   structurally impossible. The previous copy is kept at
   `~/.local/state/work-engine/skill-backup-20261022/`.
2. **Task mismatch.** R1's remaining post-change workers (S3a–S3e) did
   validation, replay and report work, not crate building. Context before first
   code write is not comparable across those task types.

So the post-change arm has to be run, not recovered.

## Frozen baseline from R1 (pre-change builders)

Pre-change means the builder read the reference text before 2026-10-22T01:59:50Z,
when `cb61f807` reached the repo copy. All seven are Sonnet 5.5 high, trigger
150,000.

| builder | role | context at first code write | pre-write read calls |
| --- | --- | --- | --- |
| `a27d1ec9c1e4acca4` | S1 builder | 88,562 | 26 |
| `abeb3da4543a8545f` | S1 successor | 116,779 | 16 |
| `a0e59f73949c117b9` | S2 builder | **no code written** | 29 |
| `a68b863f021a779a8` | S2a successor | 100,144 | 15 |
| `aed2ceafc33b90035` | S2a-2 successor | 71,539 | 14 |
| `a5d3ffe262a707e38` | S2b builder | 103,954 | 14 |
| `a74b04b4ca3160232` | S2b-2 successor | 58,097 | 10 |

Frozen baseline values:

- **B1** context at first code write, the six that wrote: median **94,353**, mean
  89,846, range 58,097–116,779.
- **B2** builders producing no code at all: **1 of 7 (14.3%)**. `a0e59f73949c117b9`
  read to 145,858 and declared a handoff having written nothing, which is the
  failure mode the treatment text was written against.
- **B3** handoffs per Rust-building slice: **1.5** (3 handoffs over S1 and S2).
- **B4** Mwt per 1k Rust lines, code slices: S1 **1.26**, S2 **1.78**, mean 1.52.
- **B5** orchestration share of campaign cost: **36.2%**.
- **B6** pre-write read calls: median 15, range 10–29.

## Hypotheses and falsification

Each states the direction and what observation disconfirms it. Reported with n in
the same sentence.

- **H1** Context at first code write falls. *Supported* if the R2 median over
  code-slice builders is below 94,353. *Disconfirmed* if it is at or above it.
- **H2** No builder spends a whole context without writing code. *Supported* if
  0 of n. *Disconfirmed* by one or more instances.
- **H3** Handoffs per Rust-building slice fall below 1.5. *Disconfirmed* at or
  above 1.5.
- **H4** Code-slice cost falls below 1.52 Mwt/krs. *Disconfirmed* at or above.
- **H5** Orchestration share falls below 36.2%. Registered as a secondary; the
  treatment does not target it and no effect is the expected result.

**H1 is confounded by design and must not be read alone.** The treatment says
both "read less" and "commit after each independently passing unit". The second
clause moves the first write earlier without reducing reading, which lowers H1's
metric for a reason H1 does not claim. Disambiguation rule, registered now: a
fall in H1 **with pre-write read calls unchanged against B6** is earlier
committing, not leaner reading, and must be reported as such. Only a fall in both
supports the reading claim.

## Held constant

Changing two things at once would make the result unattributable:

- **Context trigger stays 150,000.** The open question of whether that trigger is
  too conservative — two of four R1 handoffs fired below it, and the lowest
  auto-compaction observed on this machine is 202,533 — is **registered as a
  separate campaign R3** and must not be varied inside R2.
- Models: Sonnet 5.5 high builders, Opus 5.5 tier-1 review, Fable 5.1 tier-2
  final. Any substitution voids the comparison with B1–B6.
- Parent-mediated handoff: predecessor writes a handoff file and returns
  `handoff_ready`; the coordinator launches the successor on it.
- One orchestrator session, worker-owned boundary declarations, per-worker context
  checks written to the campaign state directory.

## Required of the campaign for this to be scorable

1. At least **two Rust-building slices** of 2,000–5,000 added Rust lines each, so
   B4 has comparable denominators. Validation or report-only slices are recorded
   but excluded from H1, H3 and H4.
2. Every worker records the **sha256 of each reference file it actually read**,
   not only the commit its prompt pins. R1's prompts pinned `fa163bad` and nothing
   verified the text against the pin; that gap is what let R1-M-001 go unnoticed.
3. Each packet states its **read set** explicitly, and the slice record states
   **how many lines that read set comprises**, so pre-write reading can be
   normalised per 1k lines required to be read. Without this, a fall in H1 could
   be a smaller reading task rather than leaner reading.
4. Context checks continue at each phase boundary, with `input_tokens`,
   `recommendation` and the trigger in each record.
5. The branch, commit range and net added lines per language are recorded at the
   end, as R1 did.

## Confounds declared in advance

- **Between-campaign comparison.** R2 builds a different subject from R1. Pre-write
  reading depends on how much existing code the task requires reading, which is
  why item 3 above is mandatory rather than optional.
- **n is small.** R1 contributes 7 pre-change builders over 2 code slices. R2 will
  contribute a similar order. No result here will be more than suggestive, and
  every claim must carry its n.
- **Reviewers are inside the orchestration being measured.** Unchanged from R1.
  The only outside check is a direct gate re-run, which tier-2 should perform.
- **Instrument change.** The symlink means workers now receive one text where
  before they could receive either. This removes a confound rather than adding
  one, but it is a change between R1 and R2 and is declared here as such.
- **The measurement role shares a branch with the subject.** R1's integration
  base moved from `cb61f807` to `dc9fcc67` because this role committed between
  the decision and the cherry-pick. For R2, measurement commits should be held
  until the campaign's integration has landed, or made on a separate branch.

## Not in scope for R2

- Raising or lowering the context trigger (campaign R3).
- Any comparison with the pure-Codex baseline on handoff behaviour: Codex has no
  context-replacement primitive, which
  [`handoff-boundary-why-no-handoffs-20261021.md`](handoff-boundary-why-no-handoffs-20261021.md)
  establishes, so the arms differ in capability and not only in instruction.
- Plus-unit or quota claims. This arm consumes no Codex quota; weighted tokens are
  the only common unit.

## Amendment 1, 2026-10-22: `c56ab8ec` is a covariate, not a treatment change

`c56ab8ec` ("Make root context transfer a user-mediated procedure") landed 26
minutes after this registration was frozen. **Ruling: amend, do not revert.**

What I verified rather than accepted:

- The worker read set is `worker.md`, `call-cost.md`, `anthropic-host.md`. Basis:
  the exact `cat` commands issued by R1 builders `a27d1ec9c1e4acca4`,
  `a0e59f73949c117b9` and `a68b863f021a779a8`. **`context-boundary.md` is not in
  it**, so the +1,910 bytes there do not reach a worker.
- Byte deltas inside the worker read set: `worker.md` **+0**, `call-cost.md`
  **+0**, `anthropic-host.md` **+1,110**.
- The `cb61f807` read-set paragraphs in `worker.md` and `coordinator.md` are
  untouched. The worker row of the boundary table — "a single assignment is
  outgrowing its window mid-task → mid-task check, then continue or
  `handoff_ready`" — is unchanged.

The reported classification is right in substance but not in wording: this is not
"coordinator-side only". `context-boundary.md` and `anthropic-host.md` are
worker-read files; what is true is that **every changed line is root-scoped**.

**Bias direction: against H1.** The worker read set is 1,110 bytes larger, so
context at first code write is pushed up, not down. The change cannot manufacture
support for H1; if H1 is supported, it is supported against this headwind. That
is why reverting is the worse trade — it would withdraw a root-transfer
capability to protect an experiment of roughly three builders per slice from a
change that cannot touch the measured path.

**R2-M-001 (P3)**, residual: `anthropic-host.md`, a worker-read file, now carries
a root-transfer procedure workers cannot execute — about 280 tokens of every
worker's startup reading, in the skill whose change under test tells workers to
read only what their packet names. The remedy is to move it out of the worker
read set, and that must wait until R2 finishes, because applying it changes the
measured text.

### Standing rule for further skill changes

Registered now so each change is classified by rule rather than argued after the
fact:

- **Treatment-relevant** — any change to `worker.md` or `call-cost.md`, or to
  anything in `anthropic-host.md` naming read sets, packet sizing, mid-task check
  cadence or worker handoff declaration. Such a change pauses R2 or voids the
  comparison with B1–B6, and must be declared before dispatch.
- **Covariate** — changes confined to root, coordinator or supervisor procedure.
  Recorded with the commit, the per-file byte delta *inside the worker read set*,
  and the bias direction.

This applies to the unapplied v2 skill diff too: classify it by this rule before
applying it, and prefer holding it until R2 finishes.

## Amendment 2, 2026-10-22: the size criterion named the wrong quantity

R2's intended subject is a controller and projector of **369 JavaScript lines**
against a criterion of two Rust slices of 2,000–5,000 lines. That subject cannot
test R2 as registered — and the criterion as written named the wrong quantity.

Why the subject fails: B1's 94,353 came from builders reading a 2,700-line Rust
crate plus contract sections. A much smaller read set lowers context at first
code write for a reason H1 does not claim. H2's failure mode — a builder
consuming its context without writing — only arises under a large read set, so
0 of n is the expected result whatever the instruction says. H3's handoffs come
from context pressure that small slices do not generate. H4 has no comparable
Rust denominator.

**Criterion restated.** Was: two Rust-building slices of 2,000–5,000 added Rust
lines each. Now: each scored code slice must give its builders a **read set of at
least 2,000 existing lines** and add at least 1,000 lines in one language, with
at least three code-slice builders in total. H4 is scored only if the added lines
are Rust; otherwise H4 is `not established` and H1–H3 can still be scored.

The reason is that the causal driver of B1, B2 and B3 is the size of the read set
a builder is *given*, not the size of its output. The original criterion named
the output, and so would have excluded a qualifying experiment and admitted a
non-qualifying one. **This is a change to acceptance criteria made after learning
that the intended subject does not fit, and before any R2 data exists.** It is
stated as an amendment, not presented as the original criterion.

**Ruling.** If the scout finds no slice meeting the restated criterion, do not
bend the registration to fit the subject. Run the port as a delivery campaign,
instrumented identically, recorded as an observational cohort labelled **not
R2**, and leave R2 frozen and pending. Several small observational campaigns can
be pooled later; one bent registration cannot be unbent.

Instrument it either way: per-builder context at first code write, per-builder
pre-write read calls, the read set each packet names and how many existing lines
it comprises, context checks at each phase boundary, and handoffs per slice with
declaration reading and turnaround.

## Amendment 3, 2026-10-22: two of the seven B1 values were instrument artifacts

**The wrong claim.** B1 was registered as median **94,353**, mean 89,846, range
58,097–116,779 over the six pre-change builders that wrote code, and B6 as median
15, range 10–29 over seven.

**The error class.** The first-code-write detector accepted any `Bash` command
containing both a `>` character and the substring `.rs`. A command that merely
*mentioned* a `.rs` path while redirecting somewhere else — `2>/dev/null`,
`> /tmp/x`, `> result-b-v1.md` — therefore counted as a code write. The detector
also counted a `Cargo.toml` write as a code write.

**Corrected per builder.**

| builder | as registered | corrected | why |
| --- | --- | --- | --- |
| `a27d1ec9c1e4acca4` | 88,562 | **113,522** | 88,562 was `cat > …/Cargo.toml` at call 24; the first `.rs` write is call 28 |
| `a74b04b4ca3160232` | 58,097 | **77,556** | 58,097 wrote `/tmp/x` and a `mutate.py` under the run directory; the first `.rs` write is a heredoc Python patch of `tests/rederive.rs` at call 38 |

The other five were adjudicated and stand: `abeb3da4543a8545f` 116,779
(`cat > tests/support/mod.rs`), `a68b863f021a779a8` 100,144 (heredoc Python patch
of `src/plan.rs`), `aed2ceafc33b90035` 71,539 (`cat >> tests/evaluate_outcomes.rs`),
`a5d3ffe262a707e38` 103,954 (`Write src/check.rs`). `a0e59f73949c117b9` has **no
write candidate of any kind** in its transcript, so B2's instance is re-verified
rather than merely re-asserted.

**Corrected baselines.** **B1** median **102,049**, mean 97,249, range
71,539–116,779, n=6, defined as the first write to a `.rs` file. **B6** median
**15.5**, mean 18.2, range 14–28, n=6, over the builders that wrote Rust source;
the registered n=7 had pooled the non-writer's 29 with `a74b04b4`'s wrong 10.
B2 (1 of 7), B3, B4 and B5 do not use this detector and are unchanged.

**Downstream consequence.** H1's bar sat **7,696 tokens below** the true
pre-change median, making H1 harder to support than the data warranted.
Correcting it moves the bar in the direction that favours the treatment, which is
why the next paragraph exists.

**Sequence disclosure.** This correction was found while instrumenting the P1
builder and was written **after** P1's first code write had been measured. At the
time of amendment P1 stood at 81,055 with 26 pre-write read calls. P1 is an
observational cohort labelled **not R2** and is not scored against B1 as an R2
result; the order of discovery is recorded so a reader can judge it.

**Check that caught it, and a second defect.** The detector flagged a `sed -n`
read containing `2>/dev/null` as P1's first code write. A rewritten detector then
disagreed with the frozen table on two builders — and its own `git commit` test
matched only the literal string, missing `git -C <dir> commit`. Neither detector
version is reliable alone. Every value above comes from listing all candidate
write calls per builder and **reading the matched command**, which is what rule 5
requires and what the original pass did not do.

## Amendment 4, 2026-10-23: the standing rule does not classify the instrument, and a concurrent write scope overlaps a frozen read set

**Trigger.** Two remediation packets for the outside-review findings (`$R/fix-a`,
`$R/fix-b`, `$R = ~/.local/state/work-engine/r2-pressure-controller-20261022`)
were frozen at base `624ec60e` at about 01:10Z and dispatched alongside the R2
planner, which is counting read sets at the same commit. This amendment is
written **before** either builder's first write.

**P1-M-001 applied and passed.** Every read-set path named in the three packets
exists at the required base and was counted: 15 of 15 present, n=15. This is the
first use of the remedy registered after P1's packet named 176 lines absent at
base.

### Gap 1: the standing rule has no class for the instrument

Amendment 1's rule sorts a change into *treatment-relevant* (worker-read text) or
*covariate* (root, coordinator or supervisor **procedure**).
`skills/manual-orchestration/scripts/integration_readiness.py` is neither. It is
not read by workers and it is not procedure text; it is the gate that decides
whether a slice integrates, and both R1 and R2 are scored through it.

**A third class is registered: *instrument*.** Any change to a script or test
under `skills/manual-orchestration/scripts/` or `skills/manual-orchestration/tests/`
that gates, measures or records builder output. Recorded with the commit, what
the gate accepted before and after, and the direction of bias.

**First entry, status `pending` (packet frozen, builder not finished).** Fix-b
implements EXT-003: re-verify the pinned binary's digest immediately before each
execution instead of once before `plan`. R1 integrated under the one-check gate;
R2 would integrate under the per-execution gate. The change is **strictly
stricter** — it rejects a superset of what the old gate rejected — so any R1↔R2
comparison of integration-pass rate or of post-integration defects is biased
**against** R2, the treatment arm. No R1 integration is retroactively invalid:
the finding is that the old gate *could* report a digest for a binary other than
the one executed, not that it did. Whether it did in R1 is `not established`.

### Gap 2: R2-M-002, a concurrent write scope may not intersect a frozen read set

P1-M-001 checks that a read set **exists** at base. It does not check that
something else may **change** it while the packet is live. Fix-b declares the
write scope `rust/crates/decision-readiness/**`. R2's read sets, frozen in
`$R/scout2/result-v1.json`, draw on four files inside that scope:

| slice | read total | inside fix-b write scope | share |
|---|---|---|---|
| E1 | 3,491 | 1,878 (`validate.rs` 1,220, `model.rs` 611, `digest.rs` 47) | 53.8% |
| E2 | 2,547 | 2,024 (`evaluate.rs` 1,413, `model.rs` 611) | 79.5% |
| P1 | 2,461 | 611 (`model.rs`) | 24.8% |

E2's margin over the 2,000-line criterion is 547 lines while 79.5% of its read
set is editable by a slice running concurrently from the same base. The three
findings fix-b must repair are in `check.rs`, `cli.rs` and `lib.rs`; none of the
four overlapping files needs to be touched, so the exposure is scope breadth, not
intent.

**R2-M-002 (P2, coordinator).** Before dispatching any slice concurrently with a
frozen read set, intersect the slice's declared write scope with that read set
and either narrow the scope to the paths the work names or re-count the read set
at the later HEAD. Reported to the coordinator at chatboard sequence 2494 with
the remedy: narrow fix-b to `src/check.rs`, `src/cli.rs`, `src/lib.rs` and the
test path, or land fix-b first and count at post-fix HEAD.

**Effect on R2's status.** R2 stays unfrozen on the user-approved subject. The
qualification figures above are the scout's and remain `unverified` for added
lines; if fix-b lands inside the overlap, the planner's `freeze-check.json` must
be recomputed before any builder is scored against the size criterion.

## Amendment 5, 2026-10-23: `skills/coordination-channels` is a covariate

**Classified under amendment 1's standing rule before use.** The skill is
root- and coordinator-scoped procedure, so it is a **covariate**, not a
treatment change. Recorded with the commit, the byte delta inside the worker
read set, and the bias direction.

- **Commit** `5793fda3`, adding `skills/coordination-channels/SKILL.md` (91
  lines) as a standalone skill.
- **Byte delta inside the worker read set: zero, by construction.** The skill is
  not referenced from `skills/manual-orchestration/` at all —
  `grep -rn coordination-channels skills/manual-orchestration/` returns nothing
  — and the four worker-read files are byte-identical before and after:
  `SKILL.md` 86ec40bc…, `worker.md` 802ac8d8…, `call-cost.md` af4d3bb9…,
  `anthropic-host.md` 84cfc80b…. These are the same digests fix-a and the R2
  planner recorded when they read those files.
- **Byte delta outside it:** one new file, 91 lines, loaded only by a root or
  coordinator that names its path.
- **Worker-facing adoption is deferred until R2 ends.** The skill's own first
  paragraph says so. Referencing it from a worker packet, a shared instruction
  or anything a worker inherits would make it treatment-relevant and pause R2 or
  void the comparison with B1–B6. Verifying that is a precondition of every R2
  dispatch, not a one-time check.

**Bias direction: `not established`, and recorded as uncertain rather than
rounded to zero.** Zero worker-text delta does **not** imply zero experimental
effect. Better coordination at the root can change dispatch order, packet
content, how early a scope collision is caught, how often a worker is corrected
mid-run, and therefore cost, handoffs and outcomes — all of which R2 measures.
The direction is plausibly favourable to the treatment arm (fewer wasted
dispatches) but could run the other way (more mid-run corrections raising worker
context). No measurement separates these, so no sign is claimed.

**What would settle it.** R2 slices dispatched with and without root-side
channel use, which this campaign is not sized to provide. Until then the
covariate is a named uncertainty in R2's limits section, not a quantified
adjustment.

### Amendment 5 addendum, 2026-10-23: the non-discoverability basis was too narrow

**The wrong claim.** That `skills/coordination-channels` is not discoverable to
a worker, verified by listing `~/.claude/skills` and `~/.codex/skills`.

**The error class.** An absence claim whose stated search domain was narrower
than the real one. Claude resolves skills from **four** classes, not one:

| class | location | definitions |
|---|---|---|
| A | `~/.claude/skills/<name>/SKILL.md` | 12 |
| B | `~/.claude/skills/synced/<uuid>/<name>/SKILL.md` (server-synced) | 26 |
| C | `~/.claude/plugins` | 33 |
| D | project-local `.claude/skills` | absent |

**The downstream consequence.** None to the conclusion, which survives a
complete search: `coordination-channels` appears in **no** class A–D and in
neither Codex directory. But amendment 5's zero-delta argument rested on a
two-directory check, so the basis is now the enumeration above.

**The check that would have caught it.** Search for `SKILL.md` across the whole
agent configuration root rather than listing the directory one expects.

**A related fact worth recording**, because it bears on what a worker receives:
the listing delivered to the fix-b builder named **13 skills**, while the
discovery surface is **71 definitions**. Not everything discoverable is listed.
So the per-dispatch verification amendment 5 requires must read **the worker's
own listing**, not the filesystem — the filesystem is the wrong instrument for
that question, and a clean filesystem check is necessary but not sufficient.


## Amendment 6, 2026-10-23: what counts toward the 2,000-line read set

Board sequence 2574 (coordinator `c1d31e88`) asks for a ruling before E2 is
scored: do **predecessor output**, the **campaign contract**, and **DESIGN.md**
count toward amendment 2's "read set of at least 2,000 existing lines"? E2's
dispatch packet records its own criterion status as `not established` pending
this, so the ruling decides whether E2 is a scored R2 slice at all.

**Columns, re-counted here at E2's base `f2fe643d` rather than taken from the
packet.**

| column | lines | basis |
|---|---|---|
| repo | 439 | 221 + 26 + 84 + 108; `proposal-evaluation.md` counted at base |
| `contract-v2.md` ranges | 283 | packet-stated ranges |
| `contract-v2-errata-1.md` 1-33 | 33 | packet-stated range |
| predecessor output (E1) | **1,672** | counted at base: `src/**` 1,473 + `tests/support/mod.rs` 199 — exact match to the coordinator's figure |
| DESIGN.md | 817 | counted at base |

### The ruling, in three parts

**1. Predecessor output counts.** It exists at base, so P1-M-001 is satisfied
and verified; the packet names it in the read set; and builder 1 recorded
reading about **1,603 of its 1,672** lines. Amendment 2 grounds the threshold in
the size of the read set a builder is *given*, because pre-write reading is what
drives B1-B3. 1,672 lines of Rust cost the same context whoever wrote them.
Provenance appears neither in the criterion nor in the mechanism it names.

**2. Contract and errata count (316).** Amendment 2 derives the threshold from
B1, whose read set it describes as "a 2,700-line Rust crate **plus contract
sections**". Campaign-authored spec text was inside the basis that set the
number, so excluding it here would hold R2 to a stricter rule than the baseline
it is compared against.

**3. DESIGN.md does not count (817).** It is the `AGENTS.md` doctrine read,
present in every session in both arms, and it was **not** itemised in the basis
from which 2,000 was derived. Adding a constant 817 to every slice while the
threshold stays 2,000 would lower the bar by 817 rather than measure the slice.
It remains a separate column — which is what the dispatch packet already
instructs ("DESIGN.md as its own line").

### Effect

**E2 meets the criterion at 2,427** (439 + 316 + 1,672). The conclusion does not
depend on part 2: repo plus predecessor alone is **2,111**. It depends entirely
on part 1 — without predecessor output the maximum reachable count is 1,572
*including* DESIGN.md and 755 under this ruling. **Part 1 is load-bearing and is
the part to attack if this ruling is wrong.**

### The cost of part 1, recorded as a covariate, not a disqualification

A chained slice's read set is partly an **outcome** of the slice before it, so
eligibility becomes endogenous to the campaign: E2 cleared the threshold because
E1 emitted 1,672 lines. Two consequences, registered now.

- **E2's B1-B3 values carry a provenance flag.** 1,672 of its ~2,427 read lines
  are same-campaign, same-contract Rust written by a Sonnet builder, which may be
  cheaper to read than unfamiliar repository code. Any comparison against B1's
  pre-existing-crate baseline must state this; it is not corrected for.
- **A slice whose read set is predominantly predecessor output is not
  independent evidence** for H1-H3. If P1 also chains, R2's scored slices are not
  three independent draws, and n must be stated as such rather than as 3.

### Also settled, being measurable at the same time

E2 added **3,598 Rust lines** (src 1,750, tests 1,848; basis
`git diff --numstat f2fe643d..HEAD`, all files Rust), so the >= 1,000-added-lines
criterion is met and H4 is scorable for this slice. E2 ran **three builders**
(two handoffs) and E1 three, so the >= 3-code-slice-builders criterion is met.

### A defect in this prereg's own JSON, found while ruling

`requirements_for_scorability[0]` in the `.json` sibling still reads "at least
two Rust-building slices of 2000-5000 added Rust lines each" — the criterion
**amendment 2 replaced**. The `.md` carries the restated criterion; the `.json`
does not, so a reader applying the JSON would use the superseded rule. It is
marked superseded rather than rewritten, because a registration is append-only.
