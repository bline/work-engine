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
