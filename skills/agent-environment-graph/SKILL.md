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
  --output docs/agent-environment-graphs.md
python3 skills/agent-environment-graph/scripts/agent_environment_graph.py check \
  --invariants docs/workflow-invariants.md \
  --environments docs/agent-environments.yaml \
  --rendered docs/agent-environment-graphs.md
```

The CLI owns duplicate-key detection, candidate extraction, wire/reference
validation, mechanically decidable graph analysis, stable rendering, and drift
detection. Treat analysis output as candidates, not semantic conclusions.

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
