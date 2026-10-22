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
