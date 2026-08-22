---
name: agent-environment-graph
description: Build, update, render, or audit Work Engine Agent Environment Graphs while preserving canonical ownership and human authority over contract changes.
---

# Agent Environment Graph

Use the repository's invariant catalog and role-environment projection as the
canonical inputs. Rendered graphs and semantic-judgment artifacts are views and
run evidence; neither is a competing source of truth.

Run all commands from the repository root. The deterministic boundary is:

```bash
python3 skills/agent-environment-graph/scripts/agent_environment_graph.py validate \
  --invariants docs/workflow-invariants.md \
  --environments docs/agent-environments.yaml
python3 skills/agent-environment-graph/scripts/agent_environment_graph.py analyze \
  --invariants docs/workflow-invariants.md \
  --environments docs/agent-environments.yaml
python3 skills/agent-environment-graph/scripts/agent_environment_graph.py render \
  --invariants docs/workflow-invariants.md \
  --environments docs/agent-environments.yaml \
  --output docs/agent-environment-graphs.md \
  --role-output-dir docs/agent-environment-views
python3 skills/agent-environment-graph/scripts/agent_environment_graph.py check \
  --invariants docs/workflow-invariants.md \
  --environments docs/agent-environments.yaml \
  --rendered docs/agent-environment-graphs.md \
  --role-output-dir docs/agent-environment-views
```

The CLI owns duplicate-key detection, candidate extraction, wire/reference
validation, mechanically decidable graph analysis, stable rendering, and drift
detection. Treat analysis output as candidates, not semantic conclusions.

Rendered Mermaid views follow a bounded presentation contract:

- invariant nodes keep their stable `INV-*` ID and derive a short display label
  from the canonical catalog condition; do not maintain hand-written graph labels;
- invariant colors derive only from the catalog's existing `Class` value, while
  role, capability, ownership, observation, prohibition, and mediated-transition
  nodes use their consistent relation colors; use only basic GitHub circle color
  families so diagrams and Markdown tables can share one generated legend;
- use top-to-bottom subgraphs and invisible ordering edges, with one visible
  entry edge per relation group so high-degree roles do not create unreadable
  fan-out; and
- preserve the exhaustive role-to-target edge set in generated relation tables
  and the role × relation matrix. The simplified diagram is an overview, not the
  complete edge ledger; table markers use literal Unicode circles plus semantic
  text so neither GitHub shortcode support nor color perception is required. Keep
  each marker and label together with non-breaking characters.

Color and shortened text are presentation metadata, never new semantic truth.
If a desired label or color cannot be derived from a canonical field, update the
owning input through the authority process or leave the view unclassified.

Render a navigable page hierarchy: the top-level output owns the shared legend,
cross-role matrix, summary, findings, and role links; `role-output-dir/README.md`
is the GitHub directory index; and one slugged page per role owns that role's
contract summary, graph, complete relation ledger, and applicable authority,
observation, mutation, transition, and independence details. `check` validates
the entire generated set. `render` may remove only stale Markdown pages in the
explicit role output directory that carry this generator's ownership marker;
preserve every unmarked file.

Read [references/semantic-judgment.md](references/semantic-judgment.md) before
classifying invariant status, causal parents, ownership, authority,
conditionality, or equivalence, or before updating either canonical input.
Validate any judgment artifact with `validate-judgments`. A genuine contract
change requires recorded human approval; tool or model identity never supplies
that authority.

Do not infer doctrine from the rendered Markdown. Update the owning canonical
input, validate it, and regenerate the view. If the requested change has no
clear canonical owner, stop and request an ownership decision rather than
creating another graph dataset.
