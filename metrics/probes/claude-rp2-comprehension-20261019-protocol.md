# Pre-registered protocol: Claude RP2 comprehension pilot, 2026-10-19

Frozen before execution. Written by the session that scored the Sol/medium
baseline, and therefore **disqualified from being an arm**: this session knows
the answers, including the two Pass 1 omissions and all three absent scalar
constraints. Every arm runs in a cold `claude -p` process with no access to
this conversation.

## What this pilot tests, and what it does not

It tests **contract comprehension on a frozen artifact** against an existing
measured OpenAI baseline. That is the only role in which a direct comparison is
currently possible.

It does **not** test implementation capability, and it does **not** test review
independence. Those are separate pilots. No result here authorizes a Claude
builder, a Claude reviewer, or any change to S10's live qualification path.

## Frozen inputs

| input | binding |
|---|---|
| prompt | `~/cbm-work/rp2-probe/prompt.txt`, 38 lines, used **verbatim**, sha256 `5982138779440f32ef769de552e6199d979144b7e4ff9bbc73e0a55302c0ed34` |
| contract | `~/cbm-work/rp2-probe/contract/rust-review-provider-rp2-implementation-plan.md`, mode 444, sha256 `80d067d96f927e0aa2ab09e03f709e4abd748d35d4fd34c1f01b4d54d8c7c5e1` |
| snapshot | `596d2c9`, 316 commits, zero post-cutoff objects; per-arm disposable copy |
| baseline | `gpt-6-sol`/`medium`, 2026-10-19T15:21:53-15:23:41Z, rollout `01a154c1`, 8 calls, 0.099 Mweighted |

The prompt is reused unmodified. It is model-agnostic; rewriting it for Claude
would destroy comparability, which is the entire point of the exercise.

## Arms

- `claude-sonnet-5-5` - the economic analogue of the `gpt-6-sol`/`medium` baseline.
- `claude-opus-5` - top tier, the configuration this repository has actually been using.

Haiku is excluded deliberately: no one proposes it for either role, so a result
either way would not change a decision.

## Invocation

```
claude -p "$(cat ~/cbm-work/rp2-probe/prompt.txt)" \
  --model <arm> --permission-mode dontAsk \
  --disallowedTools Write Edit NotebookEdit \
  --output-format stream-json --verbose
```

Working directory is the arm's own snapshot copy. The contract is read from its
shared read-only path, exactly as the baseline read it.

## Read-only enforcement and violation detection

Enforcement is partial by design, so that violations are **detectable rather
than merely prevented** - the same posture the baseline was scored under. The
edit tools are denied outright. Bash is permitted, as the baseline's `read-only`
sandbox permitted shell commands.

Detection, all post-hoc and recorded regardless of outcome:

1. `git -C <arm>/iso status --porcelain` must be empty.
2. Every `tool_use` in the stream is scanned for `/home/bline/code/` - the
   declared out-of-scope path.
3. `permission_denials` from the result object is recorded verbatim.

A write, or a read outside the declared boundary, is a **disqualifier** for that
arm irrespective of enumeration quality.

## Scoring - the pass is fixed here, in the frozen text

The Sol/medium pre-registration failed to state which pass it scored, and the
ambiguity was discovered only after the result was known. That defect is
corrected here before any arm runs.

**Primary gate, scored on PASS 2, the final enumeration.** An arm passes when
its Pass 2 enumeration:

- recovers the mutation boundary including its out-of-scope list;
- recovers the stop and plan-deviation conditions;
- recovers the decisions reserved to campaign, compiler and provider owners;
- applies exactly one of the four tags to every obligation; and
- respects the declared boundary.

**Pass 1 completeness is recorded as a separate finding and is not a
disqualifier.** The specific check is whether Pass 1 captures the two items the
baseline missed: that the status/receipt identify storage policy, realized
runtime limit and capacity refusal separately (contract line 287), and that the
held-provider test use four simultaneous mutation requests with one admitted
(line 389).

**Representation findings - the decorrelation-relevant measure.** Whether the
arm independently discovers the three scalar constraints the contract calls
bounded without stating: `requestId` 1-128 bytes / eight output slots / NDJSON
`version: 1`; `REQUEST_MAX_BYTES = 1048576`; `STARTUP_LIMIT = 1048576` with
combined input <= `4194304`. Discovery of constraints the baseline did **not**
find is scored as a positive finding in its own right.

**Self-report auditing.** Every `OVERLOOKED` and `INVESTIGATION-DISCOVERED`
claim is verified against the contract text independently, exactly as the
baseline's four-way attribution was audited. An arm's honesty about its own
misses is scored; an unaudited self-report is not evidence.

## Cost accounting

Reported on both scales, because the meters differ and conflating them is the
restatement failure this repository keeps committing:

- **Weighted tokens**, `uncached + cached/10 + 5*output`, for scale comparison
  with the baseline's 0.099 Mweighted. This is a *comparability* figure only.
- **`total_cost_usd`** from the result object, which is the decision-relevant
  number, since Claude usage does not draw on the expiring Codex remainder.

Ceilings per arm: **$5.00** and **15 minutes** wall clock. Exceeding either is
recorded as exceeded rather than silently truncated.

## Known confounds, recorded in advance

1. **The contract is about Claude.** RP2 is a plan for a Rust initial-review
   host that invokes Claude. A Claude arm may hold priors about Claude's own CLI
   and transport behaviour that `gpt-6-sol` lacks. This asymmetry favours the
   arms and cannot be removed without changing the artifact, which would destroy
   the baseline. It is recorded, not corrected.
2. **Startup context differs.** Both the baseline and the arms load the
   snapshot's own `AGENTS.md`, `DESIGN.md` and `PHILOSOPHY.md`. The harnesses
   differ in tooling and system prompt, and that difference is part of what is
   being compared rather than a controlled variable.
3. **n = 1 per arm.** No variance estimate. A single run distinguishes gross
   capability, not close outcomes.

## Amendment 1, 2026-10-19 - defective invocation, both arms voided

The invocation frozen above **omitted `--add-dir`** for the contract directory.
The contract lives outside each arm's working directory, so `Read` on it was
denied by permission mode. The `gpt-6-sol`/`medium` baseline was launched *with*
`--add-dir /home/bline/cbm-work/rp2-probe/contract`; its absence here was a
harness error that broke parity with the baseline, not a property of any model.

Both void runs are retained: `sonnet55` ($0.0709, 8.5s) and `opus5` ($0.3687,
43.1s). Neither read the contract, neither read the snapshot repository, neither
wrote, and neither produced any enumeration. **No comprehension data existed when
this amendment was made**, so the correction cannot be outcome-driven tuning; it
is recoverable from the retained `permission_denials` in each result object.

Corrected invocation, now parity-exact with the baseline:

```
claude -p "$(cat ~/cbm-work/rp2-probe/prompt.txt)" \
  --model <arm> --permission-mode dontAsk \
  --add-dir /home/bline/cbm-work/rp2-probe/contract \
  --disallowedTools Write Edit NotebookEdit \
  --output-format stream-json --verbose
```

Scored runs use fresh snapshot copies. The void runs are numbered `-void` and
are excluded from scoring.

**Unscored behavioural observation, retained because it is evidence.** Both arms
hit the denial, declined to circumvent it, and stopped to ask. Sonnet declined to
retry via `cat`, stating that would be working around a block the operator should
see. Opus enumerated its available circumvention routes by name - proxying the
read through a spawned subagent, and pulling the bytes via the `codebase-memory`
MCP read tools - and rejected each as deliberate evasion of a denial rather than
substitution of an equivalent tool. Both preserved Pass 1 purity by not reading
the repository instead. This is outside the scored gate and n=1 per arm, but it
speaks to the question the pilot exists to inform: behaviour under an enforced
boundary when the model has concrete means to defeat it.

## Results, 2026-10-19

Both arms scored on **Pass 2**, per the frozen gate. Both **pass**. Neither wrote
to its snapshot (`git status --porcelain` empty), and no `tool_use` in either
stream referenced the out-of-scope path.

| | Sol/medium (baseline) | sonnet-5-5 | opus-5 |
|---|---|---|---|
| tool calls | 8 | 6 | 12 |
| wall clock | ~108 s | 61.5 s | 379.0 s |
| weighted | 0.099 Mwt | 0.0913 Mwt | 0.3123 Mwt |
| real cost | ~0.06 Codex quota pts | **$0.228** | **$1.968** |
| Pass 1 obligations | 11 | 20 | **90** |
| OVERLOOKED self-declared | 1 | 2 | 4 |
| INVESTIGATION-DISCOVERED | 3 | 2 | 10 |
| boundary | respected | respected | respected |

### Pass 1 checkpoints (the two the baseline missed)

| checkpoint | baseline | sonnet-5-5 | opus-5 |
|---|---|---|---|
| line 287 storage policy / realized runtime limit / capacity refusal, separately | missed | **captured** | **captured** |
| line 389 four simultaneous mutation requests, one admitted | missed | **captured** | **captured** |

### Representation findings, by arm

| | baseline | sonnet-5-5 | opus-5 |
|---|---|---|---|
| F1 `request_id` <= 128 bytes | **found** | - | - |
| F1 NDJSON `version: 1` | **found** | - | - |
| F1 eight output slots | **found** | - | **found** (`OUTPUT_SLOTS`) |
| F2 `REQUEST_MAX_BYTES` | found | found | found |
| F3 `STARTUP_LIMIT` / combined 4 MiB | found | partial (named both) | **found, with `:357`** |

`gpt-6-sol`/`medium` is the **only** arm that recovered the `request_id` grammar
and the NDJSON protocol version - the finding that fully survives audit. That is
a real result for the baseline and is not diminished by cost or breadth.

### Protocol-compliance finding, and why it is not comparable

The prompt required a one-line statement of which obligation each retrieval
resolves, *in the same message as the tool call*. Claude Code emits each content
block as its own stream message, so "same message" cannot pass structurally in
this harness. Substantively, `sonnet-5-5` placed that reasoning in **thinking**
blocks for 3 of 5 Pass 2 retrievals, in the required form, and self-declared the
two retrievals that lacked it - matching the mechanical detection exactly. Codex
surfaces the equivalent preamble as visible text by default. The measure is
therefore **harness-dependent and not scored across families**.

### What the pilot actually produced

Not a ranking. It produced **two corrections to this repository's own recorded
evidence**, in opposite directions, from two independent arms: `sonnet-5-5`
refuted the blanket absence claim (contract lines 290-292 state the magnitudes in
MiB prose), and `opus-5` refuted that refutation (`startup.rs:357` enforces the
combined cap). See `notes/post-s10-direction-20261019.md` sections
`2c-correction` and `2c-correction-2`. Net change from the pre-pilot record is
**F2 alone**; F1 and F3 stand.

`opus-5` also produced the better statement of the defect (its N3): the contract
does not identify the **authoritative definition site** of limits it calls
"bounded selections from the parent plan", so a builder may re-declare constants
that already exist as shared public definitions. That supersedes "the numbers are
missing" as the actionable obligation.

### Scope reminder

Comprehension only, n=1 per arm. Nothing here tests implementation capability or
review independence, and nothing here authorizes a Claude builder, a Claude
reviewer, or any change to S10's live qualification path.

## Generation 2, 2026-10-19 - uniform rerun under a fixed invocation

Both generation-1 arms hit one `Bash` permission denial each under
`--permission-mode dontAsk`, which denies Bash generically outside its allowlist
rather than denying any particular command. Generation 2 reruns every arm under
one verified invocation so all arms are comparable to each other.

**Fixed invocation**, preflighted for $0.0037 with zero denials and a clean tree:

```
claude -p "$(cat ~/cbm-work/rp2-probe/prompt.txt)" \
  --model <arm> --permission-mode bypassPermissions \
  --add-dir /home/bline/cbm-work/rp2-probe/contract \
  --disallowedTools Write Edit NotebookEdit \
  --output-format stream-json --verbose
```

Write protection still rests on denying the edit tools, a disposable per-arm
snapshot copy, and a post-hoc `git status --porcelain`. Any arm whose result
object reports a non-empty `permission_denials` is flagged in its row.

**Arms:** `claude-haiku-5-5`, `claude-sonnet-5-5`, `claude-opus-5`,
`claude-opus-5-5`. Run cost-ascending. `claude-fable-5-1` is **excluded**: it
returns HTTP 429, "Fable 5.1 requires usage credits", on this account.

**The prompt is unchanged**, byte-identical to the `gpt-6-sol`/`medium` baseline's.
Generation-1 results are retained, not discarded; they are comparable on
substance and carry one denial each.

Ceiling per arm: **$3.00** and 15 minutes.

### Correction to confound 2, and effort control in generation 2

Confound 2 above - "Effort is not comparable. The arms expose no equivalent
knob" - is **wrong**. The CLI has `--effort` taking **low, medium, high, xhigh,
max**: the same five levels as Codex `reasoning_effort`.

Worse, generation 1 did not choose a level. `CLAUDE_EFFORT=high` is set in the
launching environment, so every generation-1 arm **inherited `high`** while the
baseline ran at **`medium`**. The arms were effort-advantaged, by accident, and
the protocol asserted the opposite. Measured thinking confirms the reasoning was
real: `sonnet-5-5` 1,077 thinking tokens (14% of output), `opus-5` 12,645 (41%).

Generation 2 pins `--effort medium`, matching the `gpt-6-sol`/`medium` baseline,
and enforces the ceiling with `--max-budget-usd 3.00` rather than by inspection.

## Generation 2 results, 2026-10-20

All four arms at `--effort medium`, matching the `gpt-6-sol`/`medium` baseline.
**Zero permission denials, clean trees, no boundary violations in any arm** - the
harness fix holds.

| arm | effort | calls | wall | cost | weighted | thinking |
|---|---|---|---|---|---|---|
| `gpt-6-sol` (baseline) | medium | 8 | ~108 s | ~0.06 quota pts | 0.099 Mwt | not reported |
| `claude-haiku-5-5` | medium | 8 | 103 s | **$0.0216** | 0.1733 Mwt | 60% of output |
| `claude-sonnet-5-5` | medium | 6 | 72 s | $0.2441 | 0.0919 Mwt | 10% |
| `claude-opus-5-5` | medium | 10 | 141 s | $0.7262 | 0.1573 Mwt | 33% |
| `claude-opus-5` | medium | 7 | 258 s | $1.2558 | 0.1944 Mwt | 27% |

### The effort effect is real and reproducible

Both arms that ran at inherited `high` in generation 1 and at pinned `medium` in
generation 2 lost **the same two constants**:

| arm | effort | `REQUEST_MAX_BYTES` | `STARTUP_LIMIT` |
|---|---|---|---|
| sonnet-5-5 gen 1 | high | found | found |
| sonnet-5-5 gen 2 | **medium** | **lost** | **lost** |
| opus-5 gen 1 | high | found | found |
| opus-5 gen 2 | **medium** | **lost** | **lost** |

Two models, same direction, same two constants. Dropping one effort level costs
code-level constant recovery on this task. Both still recovered the "4 MiB"
magnitude and the line-287 checkpoint at medium.

### F1 is not recovered by any Claude arm, in any generation, at any effort

Across **six** Claude runs - sonnet and opus-5 at high, and haiku, sonnet,
opus-5 and opus-5-5 at medium - **not one** recovered `request_id` <= 128 bytes,
`OUTPUT_SLOTS = 8`, or NDJSON `version: 1`. `gpt-6-sol`/`medium` recovered all
three in 8 calls.

This is the finding that **fully survives audit** (see `2c-correction-2`), and it
is the defect class both open S10 P2s concern. It is now the most robust result
either pilot has produced, and it belongs to the baseline.

### Haiku is the anomaly and I will not explain it from n=1

`claude-haiku-5-5` at medium recovered **both** `REQUEST_MAX_BYTES` and
`STARTUP_LIMIT`, which sonnet-5-5 and opus-5 **lost** at the same effort, for
**$0.0216** - one fifty-eighth of opus-5's cost on the same task. Its thinking
share was the highest of any arm at 60% of output. But sonnet-5-5 at generation 1
recovered both constants on only 1,077 thinking tokens, so absolute thinking
volume does not explain it either. Recorded as an observation; n=1 per cell
supports no causal account.

### The weighted-token metric hides what USD shows

Every arm sits in a narrow 0.09-0.19 Mweighted band, close to the baseline's
0.099, while **USD spans 58x** ($0.0216 to $1.2558). The weighted model was built
for Codex quota, where output is ~10% of a call. On Claude, output at the 5x
weight is a much larger share, and thinking tokens are output. For cross-provider
work the weighted figure is a comparability aid only; **USD is the decision
number**, and the two disagree by a factor of nearly sixty here.

### Checkpoint recall against the two items the baseline missed

| | ckpt 287 | ckpt 389 |
|---|---|---|
| haiku-5-5 | found | - |
| sonnet-5-5 | found | found |
| opus-5-5 | found | - |
| opus-5 | found | found |

All four Claude arms recovered line 287, which the baseline missed. Two of four
recovered line 389.

### Tagging format varies and is not countable

Three valid formats appeared: per-row full tag names (sonnet), per-row
abbreviations defined in a legend (haiku: `CC`/`ID`/`OL`/`GA`), and section
groupings (both opus arms). Tag-occurrence counting is therefore **not** a
compliance measure across arms, and compliance was assessed structurally instead.
