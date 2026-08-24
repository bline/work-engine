# Project UI Review Capability

## Status

Exploratory project-facing capability pattern. The current `ui-design-principles` skill is a retained Site2JSON compatibility surface, not a generic Work Engine UI architecture owner.

## Idea

Create a high-reasoning review capability that combines:

- rendered interface evidence;
- repository/system-mechanism evidence; and
- the consuming project's design principles

to answer:

> Is this the right interface for the machinery it represents?

For material human-facing changes, preserve four co-equal judgment lenses:

- **Truth** — the interface corresponds to the real state, relationships,
  uncertainty, consequences, and authority of the machinery it represents.
- **Maintainability** — the human-facing model and its implementation remain
  structurally coherent as the product changes instead of accumulating
  duplicated concepts and one-off behavior.
- **Explainability** — a person can perceive the relevant distinctions,
  relationships, choices, and consequences without reconstructing hidden
  machinery.
- **Aesthetics** — hierarchy, composition, typography, color, motion, spacing,
  responsiveness, and sensory coherence clarify rather than obscure meaning.

For material human-facing review, assign one distinct reviewer perspective to
each lens and preserve four separately attributed judgments so no aspect can
disappear inside a broad UI judgment. All four perspectives are applicable when
the human-facing design itself is materially in scope; adaptive panel selection
does not use one strong dimension as a reason to omit another. An integrator may
preserve overlap and conflict but must not average away a material failure in
one lens. Reviewers remain diagnostic; their persistence, agreement, or
composition does not create proposal, implementation, or acceptance authority.

No independent fifth UI-design dimension has yet survived this bounded
falsification test:

1. Can a clearly bad interface fully satisfy Truth, Maintainability,
   Explainability, and Aesthetics?
2. Is any remaining failure actually a UI/product-design failure rather than
   strategy, ethics, domain correctness, architecture, law, or another owner's
   concern?

This is a current design hypothesis, not authority to exclude a material
concern. A candidate fifth dimension must be tested rather than renamed into
one of the four by assertion.

## Human access surface

Accessibility is the requirement that all four lenses remain valid across the
intended human access surface, not an optional fifth lens and not a feature
owned by an assumed default user. Relevant differences may include perception,
cognition, language, literacy, motor control, input modality, device,
environment, temporary impairment, familiarity, and expertise. This list is
illustrative rather than closed.

The product, legal, ethical, or other owning authority determines whom the
product is obligated or intended to serve. UI review tests the four lenses
across that established boundary and surfaces unstated exclusions or hidden
default-human assumptions. Specialized accessibility capabilities may produce
distinct evidence such as focus, keyboard, screen-reader, contrast, reflow,
motion, and target-size observations without acquiring a fifth design vote.

## Required inputs

### Rendered evidence

Potentially through the separately explored
[`ui-experience-evidence`](ui-experience-evidence.md) boundary:

- screenshots/crops;
- geometry and hierarchy;
- controls and state;
- styles;
- view predicates, navigation witnesses, interactions, and transitions;
- viewport/theme/accessibility evidence.

The reviewer asks evidence questions of a shared projection. It should not
need to reconstruct the same Chrome session, screenshots, provider analysis,
or accessibility observations independently for each lens.

### Mechanism evidence

Potentially from repository evidence:

- state transitions;
- ownership;
- underlying actions;
- provenance;
- uncertainty;
- reversibility;
- failure modes;
- user responsibility.

### Project design doctrine

The consuming project, not Work Engine, defines what good human-facing design means.

## Review depth

Useful intents include:

- **Focused review** — does the changed UI work, communicate, and integrate coherently?
- **Design review** — is the current interface abstraction itself appropriate for the underlying process?

The second may recommend changing the mental model, information architecture, or interaction shape rather than merely adjusting styling.

## Evidence boundary

Keep separate:

```text
observation
    what rendered/repository evidence establishes

interpretation
    why that may help or hinder the user

proposal
    a candidate better design
```

Without user evidence, predicted usability remains a design hypothesis.

## Does not own

This idea does not own:

- Work Engine Studio;
- project design doctrine;
- Chrome Vision;
- UI experience-evidence identity, capture, lineage, or freshness;
- repository evidence;
- implementation decisions.

It is a consuming review role/capability.

## Compact statement

> Observe the interface, understand the machinery, apply the project's design doctrine, and judge whether the interface faithfully and efficiently exposes that machinery.
