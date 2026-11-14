# Evidence and claims

Authority: Binding Work Engine doctrine; part of [DESIGN.md](../../DESIGN.md).
Scope: the product contracts and design rules stated below.

Load when: Selecting or relying on evidence, producing receipts, or defining route-sensitive acceptance claims.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

### 3.9 Independently required production-path claims

Most contracts concern the resulting state or consequence. When multiple
production paths can establish the same required result, path selection remains
model judgment.

A Work Engine-authored production-path constraint is structural only when both
conditions hold:

1. A required claim quantifies over relevant production-path facts.
2. The claim is required by an authority or semantic owner other than the local
   constraint being justified, and a violating path causes a failure that can
   be named without referring merely to violation of that constraint.

The claim's identity includes its required subject, covered state, consumption
boundary, and consumer. Narrowing any of those dimensions changes the claim and
requires the authority that owns the affected acceptance condition; an actor
cannot replace an unestablished claim with a weaker one it can establish.

Three mechanisms may preserve or establish a production-path claim:

- **constraint** prevents an invalid path fact from occurring or propagating;
- **recording** supplies attributable evidence of what the acting role did; and
- **independent observation** supplies evidence owned outside the acting role
  of what occurred.

These mechanisms may be combined. Every required production-path claim needs at
least one admissible evidence mechanism: recording, independent observation, or
both, as its consumer requires. Constraint alone never establishes that the
protected property held.

Constrain the path only when a violating fact could cross the claim's
consumption boundary before available observation can support authorized
fencing, discard, repair, or replacement. Use recording when actor-attributed
conduct is sufficient; use independent observation when the consumer requires
attestation outside the actor or the actor cannot know the relevant world
history.

If no permitted realization can establish a proposed required claim, contract
formation must report the unrealizable condition to its owner rather than admit
it as executable or silently weaken it. If a bound realization cannot supply
the admissible evidence, report the claim as **unestablished**. This means the
acceptance condition that depends on the claim is unmet; it does not
independently halt unrelated work. Treat it as a truthful-failure or escalation
event for the owner of that acceptance condition. Proceeding without the claim,
narrowing or replacing it, or abandoning the requirement requires the authority
that the owning condition requires.

Context or reviewer freshness is not independently structural. It may be one
mechanism for establishing independence or another required property. Do not
generalize a bounded freshness need into compulsory context or reviewer
replacement.

This section governs whether Work Engine's own doctrine, runtime projections, or
mechanisms may promote a route choice into product structure. It does not limit
an operator's authority, when granted by the owning contract, to impose an
explicit route or route-sensitive acceptance condition. Such a requirement
derives authority from the operator, although its implementation should still
constrain no more than the requirement entails.

---

## 6. Evidence and capability selection

Evidence capabilities should normally be available together when doing so does not violate an authority or safety boundary.

The model should understand the purpose, strengths, costs, and limitations of each capability and choose according to the claim being established.

Do not force a capability sequence merely because one route is usually preferred. Do not make the model restart a reasoning process merely because the most useful evidence source changes.

### Capability choice versus fallback

These are distinct events:

1. **Capability selection** — the model chooses the evidence source best suited to a claim. No fallback necessarily occurred.
2. **Evidence fallback** — an earlier evidence route was unavailable, ambiguous, insufficient, or failed, so another route was added or substituted. Required fallback provenance must be recorded.
3. **Provider/configuration fallback** — the configured provider itself changes. Required configuration/provider provenance must be recorded.

Evidence-mode fallback inside a configured provider does not silently rewrite provider identity.

Audit contracts may require facts about how evidence was obtained. Those facts remain mandatory even though the choice of evidence route belongs to model judgment.

---

## 12. Provenance and receipts

Flexibility increases the importance of truthful provenance; it does not reduce it.

The audit record must describe what actually happened according to the owning receipt/schema contracts, including required route revisions, fallback transitions, provider/configuration changes, evidence modes, validation breadth, unavailable measurements, unresolved concerns, and the identity and establishment status of any production-path claim relied upon.

Important distinctions include:

- available capability versus capability actually used;
- capability selection versus fallback;
- evidence-mode fallback versus provider change;
- observed measurement versus inferred or unavailable measurement;
- current route versus historical route revision;
- actor-recorded conduct versus independently observed history;
- a false claim versus an unestablished claim; and
- a required claim versus a narrower claim with a different subject, covered
  state, consumption boundary, or consumer.

Never infer zero from unavailable evidence. Never rewrite provider identity because an already-configured provider used another authorized evidence capability. Never suppress a required fallback event because the final route succeeded.

The receipt records the path taken; it does not prescribe the path that must be taken next time.

For consequential semantic decisions, preserve only the decision support required by the owning artifact or contract: the accepted claim, decisive evidence, material uncertainty, and any invalidation condition that matters downstream. This is a decision receipt, not a reasoning transcript. When an existing certificate or receipt already carries that information, extend or generalize that artifact rather than inventing a parallel rationale schema.
