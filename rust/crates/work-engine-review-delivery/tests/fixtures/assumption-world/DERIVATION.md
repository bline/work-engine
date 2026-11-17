# Assumption world (R2-S second world)

**Standing: labelled synthetic test fixture. An assumption world, not a qualified organization.**

`requirements.json` is the frozen vertical's requirements
(`rust/crates/work-engine-organization/tests/fixtures/frozen/requirements.json`) with the
`realizationConstraints` member removed from both acceptance conditions
(`cond-builder-projection`, `cond-campaign-terminalization`). Nothing else differs: same problem
scope, obligation, normative values (including `mutation: none`), authorities and policy. It is
compiled by the unchanged v1 organization compiler (`derive`, profile `fixed-slice-review-v1`).

Derivation, checked by `assumption_world_is_the_frozen_world_without_realization_constraints`
in `tests/native_vertical.rs` (semantic JSON equality, not byte equality):

```text
value = parse(frozen/requirements.json)
for condition in value.acceptanceConditions: delete condition.realizationConstraints
assert parse(assumption-world/requirements.json) == value
```

Why this world: the v1 vocabulary has one provider class (`anthropic`) and one confinement class
(`guarded_sandbox`), and `realizationConstraints` is optional per condition. The R2-S host
describes what it runs truthfully for those two dimensions (provider-disabled Pi SDK child, not
confined), so a world that states either constraint is a known realization incompatibility. The
only admitting world the unchanged compiler can produce is one that states neither
(S2 review F-1; root addendum item 2).

What is assumed, and what is not established:

- The SDK child is **not confined**. The world simply states no confinement requirement.
- `mutation`: the host makes a scoped claim, `ReadOnlyClaimed`. **Claimed**: no tool is granted,
  hence the model has no mutation route. **Backed by construction** (the adapter creates the
  session with `noTools` and an empty tool list) and by **refusal at allocation**
  (`BoundAttempt::bind`) and **at request** (`verify`) of any allocated, active or advertised tool.
  **Not established**: process-level writes by the unconfined child. The evidence gap is the
  `realization-admission` blocker; the realization checks themselves stay never-dischargeable
  (root ruling, option 1).
- S-B: separation, observer identity, component binding, authority requirement and realization
  admission are discharged by the host's own fixture-domain S-A admission, keyed on check ids
  (`gate::discharge`). That admission decides over the **fixture authority world**
  (`tests/common/mod.rs`, module `authority`): synthetic owner attestations, including a
  `RealizationAcceptance` of the host's realization digest whose `provides` (mutation `none`,
  fresh continuity, verified artifacts, execution-evidence custody, independent observer) is a
  fixture statement, not an established property of the unconfined child. So a realization
  admission here closes the evidence gap only at fixture standing.
- The three mapping-rule blockers still pass only by a labelled fixture `AssumedDischarge`
  (`gate::ASSUMED_DISCHARGE_LABEL`), never by an admission (no owner for those rules, O7).

The frozen world remains the negative: refused before spawn by known realization incompatibility.
