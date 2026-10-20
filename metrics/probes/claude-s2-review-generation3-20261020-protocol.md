# Pre-registered protocol: S2 review pilot generation 3, 2026-10-20

Frozen before execution. Four conditions on the **same sealed key** as generation
2 (S2 candidate 2, `7fdf392f`, isolated snapshot with C3/C4/C5 repairs excluded
by reachability). Each condition is directly comparable to an existing
generation-2 arm. Written by the session that scored generation 2 and therefore
disqualified from being an arm.

Ground truth is unchanged: the four `gpt-6-astra`/`xhigh` findings, with the
generation-2 scoring classes **MATCH / PARTIAL / FALSE-CLEARANCE /
UNRESOLVED-DECLARED / NOVEL-VERIFIED / UNVERIFIED / MISS**.

## What each condition tests, and what each outcome would mean

### A - effort within a fixed model, isolating the confound I introduced

Generation 2 found thinking tokens ordering the F4 outcomes monotonically across
four arms (84,593 haiku MATCH, 46,448 sonnet UNRESOLVED, 29,984 opus-5 CLEARANCE,
25,775 opus-5-5 CLEARANCE) - but **confounded with model identity**. A is the
clean separation: same model, same prompt, only `--effort` changes.

| condition | arm | prompt | comparator |
|---|---|---|---|
| **A1** | `claude-opus-5` at **`max`** | unchanged | `opus-5` at `xhigh`, which FALSE-CLEARED F4 |
| **A2** | `claude-haiku-5-5` at **`medium`** | unchanged | `haiku-5-5` at `xhigh`, which MATCHED F4 |

- A1 catches F4 -> effort is a lever for Opus, and the clearance is not a fixed
  blind spot.
- A1 clears F4 again at the maximum available effort -> the clearance is
  effort-independent for this model, which is a much stronger claim than
  generation 2 can make.
- A2 loses F4 -> effort is a lever for Haiku too, supporting the monotonic
  reading over the model-identity reading.
- A2 keeps F4 at `medium` -> Haiku's match is not effort-driven, and the
  monotonic ordering is probably model identity after all.

### B - compiler-derived proof obligations

Tests the intervention argued for in `notes/s2-review-pilot-consequences-20261019.md`
section 5: that decomposing the contract's **already-stated** ordering into
explicit proof obligations is cheaper and more targeted than adding a reviewer.

| condition | arm | prompt | comparator |
|---|---|---|---|
| **B** | `claude-opus-5` at `xhigh` | `prompt-B-obligations.txt`, sha256 `c6cfa2876b447b3ce5729f2797b9a748be1de0b191044de2c718ebc8bf0195ec` | `opus-5` at `xhigh`, same effort, same model |

**Non-leak check, stated before running.** The appended obligations add no fact
absent from the contract. OB1 restates line 31's "Work cannot start until registry
insertion ... finish". OB2 directs enumeration to "the observation surfaces of the
result sink" - the sink is named in the plan at lines 9, 17 and 31 - and **does not
name `recv`, `results.pending`, or any mutex**. OB3 requires the
property/state/accessor triple from section 4 of the consequences note. The
defect's mechanism is not supplied; only the surface to enumerate is.

- B catches F4 -> the correspondence failure is addressable by contract
  decomposition, and the compiler path is supported by direct evidence.
- B clears F4 again -> obligation decomposition is insufficient on its own, and
  the problem is not merely that the arm chose the wrong property.

### C - concern scoping

Separates review framing from model capability, the confound recorded in
generation 1 and never resolved.

| condition | arm | prompt | comparator |
|---|---|---|---|
| **C** | `claude-sonnet-5-5` at `xhigh` | `prompt-C-concern.txt`, sha256 `2eb46b137cc0b59595f8c286543cf735f3c05d56d2ff88fdefa08ed75117db06` | `sonnet-5-5` at `xhigh`, which was UNRESOLVED-DECLARED on F4 and MISSED F3 |

The concern statement names happens-before relationships, concurrent
execution/publication/observation, ownership and release across cancellation,
timeout, drop and shutdown, and whether the close facts the plan requires can be
truthfully produced. **It names no defect and no file.**

- C moves F4 off UNRESOLVED-DECLARED toward MATCH, or recovers F3 - which **no
  arm in any generation has recovered** - then framing does real work and
  concern-scoped review is supported.
- C changes nothing -> framing is not the lever, and the generation-2
  complementarity is better explained by model identity.

## Invocation

Identical to generation 2's fixed form, which produced zero denials across eight
runs:

```
claude -p "$(cat <prompt>)" --model <arm> --effort <level> \
  --max-budget-usd 12.00 --permission-mode bypassPermissions \
  --add-dir /home/bline/cbm-work/s2-review-pilot/contract \
  --disallowedTools Write Edit NotebookEdit \
  --output-format stream-json --verbose
```

Each condition gets its own disposable snapshot copy. Disqualifier unchanged: a
write to the snapshot, or a read outside the declared boundary.

## Execution order, chosen for quota attribution

Generation 2 established that **concurrent runs make per-run quota attribution
invalid**. A1 therefore runs **alone and first**, to obtain one clean per-run
five-hour-window reading. A2, B and C then run concurrently, and their quota draw
is reported as an **aggregate only**.

## Limits

n=1 per condition. A is two cells of a 2x2 that is not being completed. B tests
one obligation decomposition, not the space of them, and a negative result does
not condemn the compiler approach generally. C tests one concern statement; a
different wording could perform differently. None of this authorizes a Claude
reviewer, a change to S10, or compiler implementation.

## A1 result, 2026-10-20: the clearance is effort-independent

`claude-opus-5` at `--effort max`: 29 calls, 735 s, **$4.3651**, 0.7127 Mwt,
**40,698 thinking tokens** (76% of output, up 36% from `xhigh`'s 29,984),
6 findings, verdict BLOCK, zero denials, clean tree.

| reference | A1 at `max` |
|---|---|
| F1 drain panic | **MATCH** (its F5) |
| F2 close timeliness | MISS |
| F3 join detached | MISS |
| F4 spawn before register | **FALSE CLEARANCE** |

Recall 1/4, identical to the same model at `high` and at `xhigh`.

### The pre-registered outcome obtained: effort is not the lever

The protocol stated that if A1 "clears F4 again at the maximum available effort
-> the clearance is effort-independent for this model, which is a much stronger
claim than generation 2 can make." That is what happened. `claude-opus-5` has now
false-cleared F4 at **three ascending effort levels**:

| run | effort | thinking | how F4 was cleared |
|---|---|---|---|
| gen 1 | `high` | 39,468 | argued from `acknowledge_result` taking the registry lock |
| gen 2 | `xhigh` | 29,984 | enumerated all nine registry-lock sites, **conceded** the mutex does not stop the spawned task, cleared on a narrowed question |
| gen 3 A1 | `max` | 40,698 | bare assertion in a "what the candidate gets right" list: "`submit` is synchronous, registers under the registry lock, and returns an inert observer ticket" |

### The finding I did not anticipate: warrant quality does not rise with effort

The `xhigh` clearance was the **most** substantiated and the **most** checkable -
it named its enumerated set, which is what let the error be located in minutes.
The `max` clearance, with 36% more thinking, is a one-clause assertion inside a
list of strengths, carrying no enumerated set and no discriminating observation.

So raising effort produced a **less falsifiable** wrong claim. This matters
directly for the candidate addition in
`notes/s2-review-pilot-consequences-20261019.md`: it shows that requiring the
property/state/accessor triple cannot be obtained by buying more inference. The
triple has to be a **contract requirement on the judgment's form**, because
effort alone moved it in the wrong direction here.

It also qualifies the generation-2 observation that thinking tokens ordered the
F4 outcomes monotonically across arms. Within a single model across three effort
levels, thinking volume does **not** order outcome or warrant quality: 39,468
(clear), 29,984 (clear, best warrant), 40,698 (clear, worst warrant). The
monotonic cross-arm ordering is therefore more likely model identity than
reasoning quantity - which is the question A2 was designed to settle from the
other side.

### A1 also improved on the reference

Its F5 names a mechanism the reference only implied. The reference said the panic
"occurs while holding the registry mutex, preventing subsequent truthful
cleanup/reporting". A1 states why: the panic **poisons** `registry: Mutex<...>`,
so every subsequent `registry.lock().unwrap()` panics, losing exactly the
unresolved, child-exit and unsafe-at-close facts the plan requires to survive for
S4 - and it cites `s2-plan-v1.md:17` for that requirement. That is correct Rust
semantics and a genuine severity escalation argument, from the arm that graded the
same finding low at `xhigh`.
