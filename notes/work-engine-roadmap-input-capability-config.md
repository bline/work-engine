# Work Engine Roadmap Input — Capability Configuration Ownership

## Decision

Campaign configuration should own capability wiring so user prompts express intent rather than infrastructure details.

For Chrome Vision, a campaign should either reference a reusable Chrome Vision config file or provide an inline override. The normal case should be a file reference.

Example:

```yaml
capabilities:
  chrome_vision:
    config: work-engine/config/chrome-vision.yaml

  repository_evidence:
    provider: codebase-memory

  independent_review:
    provider: claude
```

Inline configuration should remain available as an escape hatch for campaign-specific behavior:

```yaml
capabilities:
  chrome_vision:
    config:
      version: 1
      endpoint: http://127.0.0.1:9222
      limits:
        events: 200
        textBytes: 14000
        evidenceBytes: 18000
        artifacts: 2
        endpointTimeoutMs: 5000
      artifactDirectory: work-engine/artifacts/chrome-vision
      targetAliases:
        page:
          type: page
          urlPattern: '^(?!chrome(?:-extension)?://)'
        sidePanel:
          type: page
          urlPattern: '^chrome-extension://[^/]+/editor/sidebar\.html(?:\?|$)'
```

## Ownership model

```text
user prompt
  → objective / desired outcome

campaign config
  → available capabilities + configuration references

capability config
  → transport/runtime limits, aliases, local defaults

project adapter
  → project-specific recovery semantics and behavior
```

For Site2JSON specifically:

- Chrome Vision generic transport/configuration remains under `work-engine/skills/chrome-vision`.
- Site2JSON-specific aliases/recovery composition remain in `scripts/site2json-chrome-vision.mjs`.
- Campaigns should not require the user to repeat Chrome Vision config paths in prompts.
- A campaign should expose Chrome Vision as an available capability; the builder decides when using it is justified by the objective and acceptance conditions.

## Roadmap implications

1. Add a generic `capabilities` section to the campaign schema/config model.
2. Allow capability entries to reference a config file or provide inline configuration.
3. Define path-base semantics explicitly for referenced config files.
4. Validate referenced capability configs during campaign preflight.
5. Preserve capability configuration identity/provenance in receipts.
6. Keep capability availability separate from mandatory workflow stages: declaring Chrome Vision does not require every slice to use it.
7. Make the prompt contract intent-focused; infrastructure wiring belongs in campaign/config ownership.

## Design principle

> Configuration should live with the layer that owns it. Prompts state intent; campaigns expose capabilities; capability configs define runtime mechanics; adapters own project-specific semantics.
