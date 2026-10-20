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

## A2 result, and the A conclusion: F4 is model-determined, not effort-determined

`claude-haiku-5-5` at `--effort medium`: 16 calls, 267 s, **$0.1904**, 0.4833 Mwt,
47,144 thinking tokens (87% of output), 10 findings, verdict BLOCK, zero denials,
clean tree.

| reference | A2 at `medium` | haiku at `xhigh` (gen 2) |
|---|---|---|
| F1 drain panic | **MISS** | MATCH |
| F2 close timeliness | PARTIAL | PARTIAL |
| F3 join detached | MISS | PARTIAL |
| F4 spawn before register | **MATCH** | MATCH |

**The pre-registered A2 outcome: it kept F4 at `medium`.** So Haiku's match is not
effort-driven. Combined with A1, both sides of the A condition now agree:

| model | F4 across effort |
|---|---|
| `claude-opus-5` | CLEARED at `high`, CLEARED at `xhigh`, CLEARED at `max` |
| `claude-haiku-5-5` | MATCHED at `medium`, MATCHED at `xhigh` |

**F4 outcome tracks model identity and is stable across effort within a model.**
Effort did move other findings - Haiku **lost** F1 dropping from `xhigh` to
`medium` - so effort is a real lever on some findings and not on this one. The
generation-2 monotonic thinking-token ordering is therefore explained as model
identity with thinking volume as a proxy, not as reasoning quantity. Supporting
figure: Haiku at `medium` used **47,144** thinking tokens, more than `opus-5` at
`max` used (40,698), and reached the opposite F4 conclusion.

### The sharpest sentence in the pilot

A2's F6 is the best F4 treatment any arm produced, the reference included. It
quotes plan line 31 verbatim, cites the `rt-multi-thread` requirement at plan line
63 and `rust/Cargo.toml:35` that makes the race reachable, locates the spawn at
`:448-477`, the provider call at `:454` and the insert at `:505-511`, explains why
the `:447` comment is wrong ("the spawned future never takes that lock and does
not wait on it"), identifies the publication window at `:496-502`, and then bounds
itself:

> "Registry-lock serialization still stops observers from seeing the task early,
> so the defect is a violated ordering contract plus a window of unowned provider
> work. **I did not find a demonstrated incorrect settlement.**"

That first clause is **the same observation `opus-5` used to clear the finding**.
Haiku reaches it, states it, and uses it to **bound the consequence** while the
ordering violation stands. `opus-5` reached it and treated it as establishing the
invariant.

This is the cleanest available illustration of the warrant-versus-property
distinction: identical evidence, correct in both arms, supporting a finding in one
and an exoneration in the other, because they differed on **which property the
contract required**. It also shows that the discriminating observation does not
have to be absent for a clearance to be wrong - `opus-5` had it.

### Independent corroboration of the child-slot finding

A2 also found it, unprompted: "`process.rs:159-161` counts all records against
`max_children`, so slots are never reclaimed during operation" - reached inside
its F4 on child-exit observation. That finding is now held by **two arms of
different model families' tiers** plus my own source verification, and by neither
the reference nor any other arm.

## C result: concern scoping works on F4 and fails on F3

`claude-sonnet-5-5` at `xhigh` with the concurrency/ordering/ownership concern:
15 calls, 346 s, **$0.9884**, 0.4140 Mwt, 34,181 thinking tokens, 8 in-concern
findings plus out-of-concern items noted in one line each, verdict BLOCK, zero
denials, clean tree.

| reference | sonnet unscoped (gen 2) | sonnet concern-scoped (C) |
|---|---|---|
| F1 drain panic | MATCH (low) | **MATCH** (medium), now naming mutex **poisoning** |
| F2 close timeliness | PARTIAL | PARTIAL |
| F3 join detached, false `joined` | MISS | **MISS** |
| F4 spawn before register | UNRESOLVED-DECLARED | **MATCH** |

### Framing moved F4, and made the review cheaper

The pre-registered success condition for C obtained on F4: the same model at the
same effort moved from "I could only reason from tokio semantics, not observe
them" to a full finding citing `:447-448`, `:505-511`, the absent start signal,
multi-thread reachability, and - a point no other arm made - that **every executor
test uses plain `#[tokio::test]`, a current-thread runtime**, so spawned work
cannot run before `submit` returns and the ordering is never forced by any test.
It also noted `drop(ticket)` is vacuous because `ExecutionTicket` is a plain
`AttemptId`.

And it did this **more cheaply than the unscoped run**: $0.9884 against $1.2369,
15 calls against 22, 34,181 thinking tokens against 46,448. Concern scoping was
not a cost-for-quality trade here; it was cheaper and better on the item it
targeted.

### F3 is framing-resistant, which is the stronger finding

The concern statement explicitly named "ownership and release of tasks,
**handles**, slots and child processes across **cancellation**, timeout, **drop**
and shutdown" - F3's exact subject matter. The arm then wrote about handle
ownership under drop ("Dropping the last executor handle drops the `JoinHandle`s,
which detaches the tasks instead of cancelling them") and **still did not reach
F3's mechanism**: that cancelling the drain future mid-await skips the restore at
`:313-320`, permanently detaching the collector, after which a later drain reports
`joined` from `results.pending`.

**Tally across all three generations: 11 Claude runs, 4 models, 4 effort levels,
and one concern scope that named F3's subject. Zero recoveries.** I predicted
before running that a C recovery of F3 would weaken the decorrelation argument.
It did not recover it, under the most favourable framing available, so the
argument is **strengthened**: F3 is a genuine blind spot that neither effort nor
scoping reaches.

### One imprecision worth recording

C's F6 says "Every registry read blocks on the lock, so this is not observable
through the executor API." That is **not quite right**: `ResultReceiver::recv` is
part of the executor's public API and reads `results.pending`
(`executor.rs:147`), never the registry, so a published result *is* observable
through the executor API during the window. Haiku's A2 phrasing was more precise
because it scoped the claim to the **task** rather than the API.

So three arms now reached the same observation and used it three ways: `opus-5` to
clear the finding, `haiku-5-5` to bound the consequence correctly, and C to bound
it with an overstated scope. The observation is easy to reach; stating exactly
what it does and does not cover is the hard part.

### Child-slot finding, third independent arm

C's F4: "Child slots are never released before terminal shutdown, and there is no
per-child reap or replacement." That is now **three arms** (`opus-5` at `xhigh`,
`haiku-5-5` at `medium`, and C) plus my own source verification, against zero from
the reference.

## B result: the decomposition caught what three effort levels could not

`claude-opus-5` at `xhigh` with derived obligations: 25 calls, 758 s, **$3.3718**,
0.5348 Mwt, 39,793 thinking tokens, verdict BLOCK, zero denials, clean tree.

**OB1 — NOT satisfied. OB2 — NOT satisfied. F4 is MATCHED.**

Same model, same effort, same sealed key as the generation-2 run that
**false-cleared** F4. The only changed variable is the contract decomposition.
The pre-registered success condition obtained: "B catches F4 -> the
correspondence failure is addressable by contract decomposition, and the compiler
path is supported by direct evidence."

### The model diagnosed its own prior failure

OB3 required the property/state/accessor triple for any obligation reported
SATISFIED. B reported none satisfied, and then volunteered this:

> "the property I *did* establish is the weaker one named under OB1 (atomic
> registry transition), ranging over the states `{open/closed gate} x {task
> absent, task registered, task registered+result pending, task acknowledged}`,
> over the accessor set `{submit, close_gate, close_and_drain, request_abort,
> owned_count, acknowledge_result}`. **That property is not the one invariant 1
> requires, and it does not cover accessors 1-3 above.**"

That accessor set is **exactly** the one the same model enumerated at `xhigh`
without obligations, when it cleared F4. Required to state the triple, it
identified the property mismatch itself. The triple is not merely a checking aid
for a later reader; here it was **self-diagnosing**.

### B's enumeration is better than mine, and corrects my own record

B enumerated six observation surfaces of the result sink with an explicit
exhaustiveness basis:

> "`ResultState`, its `pending` field, `ResultRecord`, and `ResultReceiver`'s
> `notifications`/`results` fields carry no `pub` or `pub(crate)` modifier, and
> `src/lib.rs:16-20` re-exports only the public type names... Rust privacy
> therefore confines every accessor to `src/executor.rs`, which makes a grep of
> that one file an exhaustive enumeration - that is the basis on which I treat the
> list as complete, and it is only valid at this revision."

It found **three** unguarded readers that leak a `TaskResult` before registration:
`ResultReceiver::recv` (`:150`), `ResultReceiver::pending` (`:160`), and
`OwnedExecutor::pending_results` (`:233`); and correctly separated
`acknowledge_result` and `close_and_drain`, which do serialize behind the registry
lock.

**This corrects section 4 of `notes/s2-review-pilot-consequences-20261019.md`,
which stated the relevant set had "exactly two members ... two lines of grep."**
Verified: `:158` and `:230` both lock `results.pending`, both are `pub`, both
return full result values, and my grep missed them because it required `pending`
and `.lock()` on one line while Rust method chains span lines.

Fifth pattern-matching failure of mine this session, and the most pointed: I used
that grep as **the worked example** for why the accessor-set check is mechanical,
and the example was itself produced by a defective mechanical check. B's
exhaustiveness basis - module privacy confines the accessor set to one file - is
**sounder than the one I demonstrated**, because it argues from the language's
visibility rules rather than from a text pattern.

So the candidate survives and its demonstration does not: the triple works, and
my instance of it was wrong. A deterministic accessor check is only as good as its
exhaustiveness argument, which is precisely the field the candidate requires and
which my own example omitted.

### B also reached C's independent point

"every executor test runs single-threaded while the manifest pins
`rt-multi-thread`", tied to `DESIGN.md` §3.9 and §12 and `PHILOSOPHY.md` §18 -
"treating missing attestation as truth repeats the same epistemic error as
inferring zero from an unavailable measurement." Two arms, different conditions,
same observation about why the test suite cannot detect the defect.

## Generation 3 conclusion, all four conditions returned

| condition | changed variable | F4 | F3 | cost |
| --- | --- | --- | --- | --- |
| **A1** `opus-5` @ `max` | effort up from `xhigh` | FALSE CLEARANCE | MISS | $4.3651 |
| **A2** `haiku-5-5` @ `medium` | effort down from `xhigh` | MATCH | MISS | $0.1904 |
| **C** `sonnet-5-5` @ `xhigh` | concern scope | MATCH (from UNRESOLVED) | MISS | $0.9884 |
| **B** `opus-5` @ `xhigh` | derived obligations | **MATCH** (from CLEARANCE) | MISS | $3.3718 |

Generation-3 inference total: **$8.9157** list-equivalent, four serial runs.

### What the generation establishes

1. **Effort is not the lever for F4.** `opus-5` false-cleared at `high`, `xhigh`
   and `max`; `haiku-5-5` matched at `medium` and `xhigh`. The outcome tracks
   model identity, not reasoning allocation. The generation-2 observation that
   thinking tokens ordered the four arms monotonically is explained as model
   identity with thinking volume as a proxy - Haiku at `medium` spent 47,144
   thinking tokens, more than `opus-5` at `max`.
2. **Warrant quality does not rise with effort either.** The `max` clearance, with
   36% more thinking than `xhigh`, is a one-clause assertion where `xhigh`'s was a
   checkable enumeration. More reasoning bought a *less* falsifiable wrong answer.
3. **Representation moved the judgment where effort could not.** Same model, same
   effort, same sealed key: obligations turned a false clearance into a match, and
   OB3's property/state/accessor triple made the model locate its own prior
   property mismatch.
4. **Concern scoping also moves F4, and costs less** than the unscoped run it
   replaced (15 calls against 22, $0.9884).
5. **F3 is intervention-resistant.** Eleven Claude runs, four models, four effort
   levels, an explicit concern statement naming cancellation, handle ownership and
   drop, and a decomposed contract: none recovered the mechanism. Better
   decomposition does not dissolve every blind spot, and the open question is
   which intervention a given failure warrants - not whether to buy more
   reasoning.

### What it does not establish

n=1 per cell, so stochastic variation is not isolated; one defect on one frozen
tree, so nothing here generalises to an unseen contract; and the obligations were
hand-derived by a model of the same family at comparable cost, so the economic
claim - that a cheap compiler can produce sufficiently correct decompositions for
a cheap reviewer to execute - remains entirely untested. The next decisive
experiment is matched arms on a *different* frozen defect with obligations
compiled from the contract without sight of the answer key, then the same
obligations given to `haiku-5-5` or `sonnet-5-5`.

Authorizes no Claude reviewer in any campaign, no change to S10, and no
alteration of `codex-adversarial-review` evidence classes.
