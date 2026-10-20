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
