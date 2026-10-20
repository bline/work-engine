# Pre-registered protocol: blind obligation compiler, 2026-10-20

Frozen before execution. Generations 2-3 and Test S established that appending
proof obligations recovers defects that general review misses, with a
specificity result: B's registration-shaped obligations caught F4 and missed F3;
D's ownership-shaped obligations caught F3 and missed F4 while looking at the F4
code region for an unrelated reason. Both obligation sets were **hand-authored by
me, knowing where the implementations failed.**

This is the arm that removes the human compiler. It is the measurement the
economic case depends on, and it is the distinction between an effective prompt
and an organizational compiler.

## E1 - compile obligations with no sight of any implementation

| element | binding |
|---|---|
| model | `claude-opus-5` at `xhigh` - matched to B and D, so authorship is the only changed variable |
| input | `s2-plan-v1.md` (sha256 `98390bd9...`) plus the repository **at the accepted predecessor baseline** |
| snapshot | `2bbc95c0` only |
| prompt | 36 lines, sha256 `32b915d2ce351adbf22dc8361785dd49eeff4aa55a9ef2baf69543cd669445bc` |

**Isolation verified before launch.** `5e0377d0` (Sol attempt 1), `7fdf392f`
(attempt 2) and `4071955` (accepted) are all **absent**. All four of
`clock.rs`, `executor.rs`, `framing.rs`, `process.rs` are **absent** from the
tree - the slice has not been built. E1 therefore cannot see a candidate, a
review, a finding, or any of B's or D's obligations.

E1 is told what makes an obligation useful - that discharging it honestly must be
able to fail, and that it should force a property over a set of states with a
stated exhaustiveness basis - because that is general doctrine this repository
already holds, not information about any defect. It is told nothing about
registration ordering, ownership across cancellation, or any invariant being more
at risk than another.

## E2 - execute the compiled obligations blind

`claude-opus-5` at `xhigh`, the Test R base prompt (sha256 `27b666a4...`)
with **E1's output appended verbatim**, against the sealed key `7fdf392f`.
Identical to B and D in every respect except who wrote the obligations.

**E1's output is frozen by sha256 and committed before E2 runs.** No editing, no
selection, no reordering, no removal of obligations I judge weak. If E1 emits a
poor set, E2 executes the poor set.

## Scoring

**E1, before E2 runs.** For each emitted obligation: can honest discharge fail?
Does it name a property, a state set, and an exhaustiveness basis? And the
decisive question - **which families does the set cover?** Specifically, whether
it contains an obligation that would force the registration/visibility question
B targeted, and one that would force the ownership-across-cancellation question D
targeted. Scored by me from the text, before any candidate is involved.

**E2.** All four reference findings under the frozen generation-2 classes
(MATCH / PARTIAL / FALSE-CLEARANCE / UNRESOLVED-DECLARED / NOVEL-VERIFIED /
UNVERIFIED / MISS), plus novel findings verified against source, plus the
combined cost of E1 + E2 against B's $3.3718 and D's $3.3569.

## Pre-registered outcomes

- **E1 emits both families and E2 recovers F3 and F4** - the strongest available
  result for the compiler path. It would mean obligation authorship is separable
  from defect knowledge, and the amortisation argument becomes testable rather
  than hypothetical.
- **E1 emits both families, E2 recovers neither** - obligation *execution* is the
  bottleneck, not authorship, and the B/D results depended on wording precision a
  general compiler does not reach.
- **E1 emits one family only** - a decomposition-**completeness** failure, which
  is a real finding even if E2 performs well on what it was given. This is the
  outcome I consider most likely: covering seven invariants uniformly is a
  different task from targeting the two that failed, and a compiler with no
  failure signal has no reason to weight them.
- **E2 false-clears something** - worse than a miss, per the generation-2
  finding, and it would indicate that obligations can license confident wrong
  conclusions when they are not the right obligations.

## Limits

n=1 at each stage. One plan. E1 is the same model family as E2, so this does not
test a cheap compiler feeding an expensive reviewer or the reverse - only whether
the authorship step can be performed without defect knowledge at all. A
**revision-validity** constraint surfaced unprompted in both B and D, each of
which scoped its exhaustiveness claim as "only valid at this revision"; if
compiled obligations are to amortise across candidates, that rebinding is a
separate unsolved problem and this arm does not address it.
