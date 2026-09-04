# Contributing

This repository is maintained by one person and is under active development.
Issues and pull requests are welcome; please read this first, because the
acceptance model here is deliberate and differs from most projects.

## Acceptance authority

Acceptance is held by the maintainer. Passing tests, a clean diff, or an
approving review from any tool does not accept a change. This is the same rule
the system itself enforces on its own agents, and it applies to human
contributions for the same reason: the cost of a plausible-but-wrong change
falls on whoever has to discover it later.

## Before opening a pull request

Open an issue first for anything beyond an obvious fix. The most useful thing
you can bring is the reasoning: what outcome you want, what currently prevents
it, and what would have to remain true for the change to be correct.

Run the checks locally:

```bash
npm install
npm test
```

If your change touches role contracts, invariants, or the role environment,
regenerate and re-verify the projections:

```bash
python3 skills/agent-environment-graph/scripts/agent_environment_graph.py check \
  --invariants docs/workflow-invariants.md --environments docs/agent-environments.yaml \
  --rendered docs/agent-environment-graphs.md --role-output-dir docs/agent-environment-views
```

## Which documents own what

Before changing behavior, check which document owns the thing you are changing.
[`ARCHITECTURE.md`](ARCHITECTURE.md) has the repository map and per-subsystem
ownership. In short:

- [`DESIGN.md`](DESIGN.md) owns invariants and authority boundaries. Changing one
  is a contract change, not an ordinary patch, and needs its own discussion.
- [`PHILOSOPHY.md`](PHILOSOPHY.md) is non-normative. Do not derive requirements
  from it.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) describes what currently exists and must
  not silently promote a proposal or an idea into an invariant.
- `skills/` and `app-server/` own their local runtime and artifact semantics.

A change that makes one of these documents disagree with another is a defect
even if the code works.

## AI-assisted contributions

They are welcome and are how most of this repository was written. Two
requirements:

1. **Say so.** Note in the pull request that the change is AI-assisted, and
   which tool produced it.
2. **Bring the evidence, not just the diff.** State what you verified, how, and
   what you did not verify. If you could not establish something, say that
   rather than leaving it implied. An unverified claim presented as settled is
   the specific failure this project exists to prevent, and it is the fastest
   way to have a pull request closed.
