# Production-path claims

Authority: Non-normative explanation; part of [PHILOSOPHY.md](../../PHILOSOPHY.md).
Scope: reasoning behind doctrine, without adding requirements.

Load when: Understanding why an acceptance claim may depend on execution history or independent observation.

Section numbers retain their original citation identities. Links identify
related topics; they do not require reading every linked document.

## 18. Production-path claims, consumption, and establishment

The route-invariance test needs one careful qualification. Most required
properties concern the resulting artifact or consequence, but some acceptance
claims quantify over how that result was produced. Independence,
preregistration, evidence custody, treatment exposure, fallback provenance, and
effect duplication can all make production history relevant.

That does not make every preference about production history structural. A
route-sensitive claim earns contractual force only when it is already required
by an owner outside the local mechanism seeking to constrain the route, and a
violating route causes a failure that can be named without saying merely that
the route rule was broken. Otherwise any preferred procedure could authorize
itself by inventing a claim whose only consumer is the procedure.

### Claim locus and the recovery frontier are different questions

One question is where the fact asserted by the claim exists.

A **path-constitutive claim** is about the execution itself. Whether a reviewer
was exposed to builder reasoning, whether a specification author saw outcomes,
or whether evidence remained under required custody cannot be established from
the terminal artifact alone. The relevant evidence must come from the runtime,
harness, custody system, or another observer of the production path.

A **world-history claim** is about what happened outside the actor. Whether an
external effect occurred twice is not necessarily visible in a terminal
snapshot, but an independently owned effect ledger may establish it. The
acting role's own log is still an attributable record of what the role reports;
it is not independent observation of the world merely because it has ledger
shape.

The second question is where a violation becomes consequential. The important
boundary is consumption, not elapsed time in isolation. A path must be
constrained when a violating fact could enter the state covered by the claim
before available evidence allows it to be fenced, discarded, repaired, or
replaced. Faster detection matters because it can move that frontier, not
because latency is an independent design value.

Consumption is relative to the exact claim. A contaminated reviewer episode
cannot be made uncontaminated, but a new isolated reviewer may still establish
a new claim if the owning acceptance condition permits replacement and the
discarded episode's output has not entered the state covered by the original
claim. If the output has already crossed that boundary, replacement may no
longer restore the broader claim. Substituting a narrower claim changes what
the consumer required and therefore belongs to that consumer, not to the actor
whose available evidence supports only the weaker statement.

### Constraint, recording, and observation answer different questions

Constraint preserves a required property by preventing an invalid fact from
occurring or propagating. Recording provides evidence of what the actor says it
did. Independent observation provides evidence of what occurred from a source
outside the actor's authority in the respect the claim depends on. These are
composable mechanisms rather than steps or a ladder. A single claim may need
more than one.

For example, independent review may use capability constraints to prevent
mutation and realization evidence to attest which capabilities were actually
granted. Preregistration may constrain outcome visibility and preserve an
independently attributable frozen specification. An at-most-once effect may use
fencing or idempotency to prevent duplication and an independently owned effect
ledger to establish world history. Fallback provenance may need only an
actor-attributed route record when that is all its consumer requires.

The evidence form follows the acceptance claim. Self-recording is sufficient
when attributable actor conduct is the required fact. It is insufficient when
the consumer requires evidence independent of the actor or when the actor
cannot know the world history being asserted. Conversely, independent
observation does not repair a violation that has already crossed an
irreversible consumption boundary; it may establish only that the required
claim is false.

### Truth, establishment, and authority must remain separate

A constraint can preserve a property without producing admissible evidence
that it held. A realization can therefore leave a claim unestablished even
when the protected property may in fact be true. Treating missing attestation
as truth repeats the same epistemic error as inferring zero from an unavailable
measurement.

An unestablished claim is not an invitation for the actor to weaken the claim,
declare success, or manufacture a global halt. It means the acceptance
condition depending on that claim has not been met. The owner of that condition
retains the authority to require another realization, permit a replacement
subject, change or waive the condition, proceed under explicitly different
acceptance, or allow the objective to fail truthfully. A requirement that no
permitted realization could establish is instead a contract-formation defect;
it should be exposed before a particular realization is treated as executable.

This reasoning is intentionally more expansive than the corresponding rule in
`DESIGN.md`. Runtime projections need the applicable claim, boundary, evidence
and authority consequences, not a general procedure for classifying every
case.
