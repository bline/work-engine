---
name: ui-design-principles
description: Design or review Site2JSON product, interface, implementation, documentation, and presentation decisions through its four co-equal principles of Truth, Maintainability, Explainability, and Aesthetics. Use for Site2JSON design work or critique; do not apply as a generic visual-style guide.
---

# Site2JSON Design Principles

Shape Site2JSON so its internal structure, human-facing expression, and continued
evolution reinforce one another. Apply Truth, Maintainability, Explainability,
and Aesthetics as one design system. They are equal principles, not a hierarchy,
and none is permission to silently sacrifice another.

This skill reconstructs the draft doctrine in
`../site2json/DESIGN_PRINCIPLES.md`. It preserves that document's design intent;
it does not make this package the authority to amend the source doctrine.

## Truth

Keep the UI as close as practical to the actual underlying process. Do not
replace real system behavior with a simpler fiction when the reality can be
represented clearly.

Where materially relevant, keep uncertainty, competing assignments,
provenance, confidence, nested relationships, variant resolution, drift,
fallbacks, arbitration, and unresolved state legible. Let users distinguish:

- what the system observed;
- what it inferred;
- why it made a decision;
- which alternatives existed;
- where resulting data came from; and
- what remains uncertain.

Expose implementation detail only when it helps make real behavior legible.
Represent actual internal state and actual events; do not use visualization,
motion, labels, or narrative cues to simulate a process that did not occur. The
interface represents the machinery rather than decorating a fictional account
of it.

## Maintainability

Do not make users responsible for state they did not author or explicitly take
over. Attach responsibility to the smallest meaningful decision: changing one
field, relationship, transform, override, or other decision does not imply
ownership of its enclosing extractor.

Keep automatically inferred, user-reviewed, user-modified, explicitly pinned,
automatically maintainable, and approval-bound states distinguishable. Let the
system continue maintaining untouched state whenever it can do so safely. Do
not turn users into janitors for automation.

Carry the same principle into the implementation. Keep code clear, modular,
understandable, refactorable, and tested. Keep documentation aligned with the
implemented system; material drift is a defect. Place necessary complexity in
the system when that avoids transferring needless cognitive or maintenance
burden to users, future developers, or documentation readers.

## Explainability

Make the product explain itself through ordinary use. A user should be able to
begin understanding it without first mastering an external manual.

Reveal important relationships directly through suitable combinations of
spatial arrangement, labels, synchronized highlighting, motion, interaction,
progressive disclosure, contextual explanation, visible provenance, and
meaningful states or icons. Prefer perceivable correspondence over requiring
the user to reconstruct relationships from disconnected views. In particular,
make the relationship among source page, semantic model, and extracted output
available when it matters.

Make errors and uncertainty explanatory. Provide enough context to understand
what happened, where and why it happened, how the user's model contributed,
and what meaningful action remains. Repeated interaction should build the
user's capability rather than preserve avoidable opacity.

Documentation should deepen understanding, add rigor, and serve as reference.
It does not compensate for an interface that could reasonably explain itself
but does not.

## Aesthetics

Treat aesthetics as a first-class property of the whole project: interface,
architecture, code, documentation, terminology, organization, developer
experience, distribution, public presentation, demonstrations, motion,
interaction, and their coherence as a whole.

Build something worth understanding and caring for. Coherent internal design
should invite stewardship and extension; refinement and maintenance should
remain creatively satisfying rather than making replacement attractive merely
because the mature system is unpleasant to inhabit.

Use visual hierarchy, spacing, movement, timing, weight, restraint, contrast,
color, and composition to improve perception and experience. Motion should
communicate state or process. Color should communicate meaning. Layout should
reveal relationships. Emphasis should direct attention toward what matters.

Beauty must not conceal complexity, uncertainty, provenance, or failure. It
should make truth easier to see. Aesthetics means coherent, comprehensible form,
not surface prettiness.

## Resolve decisions through the combined system

Evaluate unclear decisions against all four principles and prefer forms that
satisfy several at once:

- Truth determines what must be represented faithfully.
- Explainability determines how relevant reality becomes understandable.
- Maintainability determines where responsibility and complexity live over time.
- Aesthetics determines how coherent form supports perception, experience, and care.

Treat an apparent conflict as an unresolved design problem. A truthful but
needlessly incomprehensible result, an explainable result that shifts avoidable
maintenance to the user, a beautiful result that misleads, or a mechanically
maintainable result that is hostile to understand and extend has not satisfied
the system.

When reviewing an existing artifact, distinguish directly observed behavior
from inferred consequences and unknowns. Report which principles a design
supports, where they reinforce one another, and where a material conflict
remains. Do not manufacture a violation from personal taste or promote one
current visual or implementation mechanism into universal doctrine.
