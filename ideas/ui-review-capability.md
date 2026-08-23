# Project UI Review Capability

## Status

Exploratory project-facing capability pattern. The current `ui-design-principles` skill is a retained Site2JSON compatibility surface, not a generic Work Engine UI architecture owner.

## Idea

Create a high-reasoning review role that combines:

- rendered interface evidence;
- repository/system-mechanism evidence; and
- the consuming project's design principles

to answer:

> Is this the right interface for the machinery it represents?

## Required inputs

### Rendered evidence

Potentially from Chrome Vision:

- screenshots/crops;
- geometry and hierarchy;
- controls and state;
- styles;
- interactions;
- viewport/theme/accessibility evidence.

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
- repository evidence;
- implementation decisions.

It is a consuming review role/capability.

## Compact statement

> Observe the interface, understand the machinery, apply the project's design doctrine, and judge whether the interface faithfully and efficiently exposes that machinery.
