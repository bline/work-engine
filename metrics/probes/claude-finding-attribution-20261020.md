# Finding attribution and remediation-authority failure, 2026-10-20

A result about **review contracts in general**, extracted from the S2 pilots
because it is not about any one compiler, model, or campaign. Recorded separately
so it is not read as a footnote to the obligation-compiler protocol.

Source arms: `metrics/probes/claude-s2-review-independence-20261019-protocol.md`,
`claude-s2-review-generation3-20261020-protocol.md`,
`claude-s2-builder-20261020-protocol.md`,
`claude-s2-obligation-compiler-20261020-protocol.md`. Board correction at
sequence 2211.

## The observation

`s2-plan-v1.md` contains at least three internal defects, verified against source
at `7fdf392f`:

1. **Unrealizable requirement.** Invariant 4 requires checking absolute deadlines;
   `AuthorizedEntry` exposes five getters and carries neither authority expiry nor
   wait budget (`lifecycle-store/src/lib.rs:44-61`); no public store API returns an
   `AuthorityExpiresAt` for an attempt (`sqlite.rs:1289` is internal); the runtime
   imports the *types* at `clock.rs:3` and has no value source; and the plan names
   "a needed S1/core mutation" as a boundary-change **stop**.
2. **Self-contradiction.** Integration-owner request 2 says the owned manifest
   "adds only `tokio.workspace = true`", while the mandated first increment
   requires a real disposable S1 store and a temporary directory, which cannot
   compile without dev-dependencies in that manifest.
3. **Undecidable attribution.** Root manifest and lock are excluded from the
   task-owned manifest, while the integration owner's Tokio pin into those same
   files is a precondition for every `--locked` gate, and both land as ordinary
   content in one candidate commit.

**Across seventeen Claude review runs on this subject - four models, four effort
levels, with and without obligations - the arms repeatedly detected these
discrepancies and attributed every one of them to the implementation.**

Measured over the five arms whose full findings are retained: plan-attribution
phrases 0, 0, 1, 0, 1; "revert/remove" remediation prescribed in all five.

The sharpest instance, from the `haiku-5-5` blind review of the Claude-built
tree: it stated the tension explicitly - "all 13 paths are inside my declared
boundary. The conflict is with the plan's narrower lane" - and then prescribed
**"revert the root files and the extra dev-dependencies"**, which is removal of
work defect 2 makes mandatory.

The only arm that attributed correctly was **E4**, which was forbidden from
reporting defects or dispositions and therefore had a conflict-referral channel
as its only outlet. It produced three plan-level conflicts, all three verified.

## Why this is a governance failure and not a recall statistic

A reviewer found a real discrepancy, classified it as an implementation defect,
and proposed a remediation that **would make the candidate less conformant**, by
removing dependencies the plan's own mandated proof requires. The finding was
true. The consequence drawn from it was wrong, and nothing in the review contract
could catch that, because the review contract's only output type is a finding
against the candidate.

So the system governs whether a finding is true and does not govern whether the
proposed consequence follows from it.

## Causal qualification, stated so this is not overread

E4 differed from the review arms in **task, instructions and perspective**, not
only in output schema. Its referrals establish the usefulness of that
configuration. They do **not** establish that adding a referral field to an
ordinary review repairs attribution. That is a testable hypothesis and it is
untested. The cheapest test: rerun one completed review arm with its prompt
unchanged except for an added disposition vocabulary, and score whether the same
discrepancies route differently.

## Candidate disposition vocabulary, for the formed concern-scoped review proposal

Not authorized here; recorded as input to that proposal.

| classification | meaning |
|---|---|
| candidate nonconformance | the accepted requirement is realizable and the implementation violates it |
| contract unrealizability | the requirement cannot be satisfied within admitted inputs or authority |
| contract inconsistency | accepted obligations conflict under the available realization |
| evidence insufficiency | available records cannot establish whether the requirement was met |
| unresolved attribution | the discrepancy is real; responsibility is not established |

A reviewer may **nominate** a classification. It must not acquire authority to
resolve a contract conflict by calling it a code defect, because that silently
expands or contradicts the builder's repair boundary.

## Four boundaries, each failing to route what it inherited

| boundary | failure | owner |
|---|---|---|
| decomposition | assigned a duty and withheld its required input | planning / contract compilation |
| execution | hit the named stop condition and implemented a disconnected mechanism instead of invoking it | execution governance |
| review | attributed a contract defect to the implementation and prescribed contradicting remediation | review contract |
| acceptance | closed with the issue unresolved through three repair rounds, a retained review and seven gates | readiness and verification |

The second is the one with an existing mechanism: the plan states the stop, and
the stop was not taken. An instruction to stop is not an enforced stop boundary.

## What a realizability check would need, and what is actually cheap about it

Required input -> authorized producer -> published interface -> consumer authority
-> availability at the point of discharge. Defect 1 is a producer/transport
failure for a single input, and comparing S2's required inputs against S1's
published interface would have failed it before a line was written -
`AuthorizedEntry`'s five getters against invariant 4's demand is a list diff.

**The check is cheap once the representation exists. Producing a correct
representation of semantic input requirements and real published interfaces is
not yet mechanical**, and that is the part organizational compilation still has to
establish. Claiming the check is cheap without that caveat would misprice it.

## Scope

The original review outputs are preserved unchanged; this adds evidence and
lineage. The sealed-key findings F1, F3 and F4 remain genuine code defects and
the recall measurements across all generations stand. Astra's reference F2 is
unaffected: it concerns close timeliness in `process.rs` against a local shutdown
policy deadline, not the entry's authority expiry, and folding it into this
reclassification is unestablished. This authorizes no change to `s2-plan-v1`, no
remediation, no alteration of `codex-adversarial-review` evidence classes, and no
change to S10.
