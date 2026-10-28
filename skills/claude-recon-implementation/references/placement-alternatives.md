# Map placement alternatives shallowly

Before implementation reconnaissance, use one read-only Claude process to map architectural placement. This call is a routing pass, not an implementation survey. It must inspect canonical design/roadmap evidence, module ownership boundaries, and only the minimum symbols needed to distinguish plausible homes. Do not ask it to trace every candidate deeply.

Prefer the native-Claude transport launcher. The command below shows the
underlying Claude invocation; wrap it as specified in the transport reference:

```bash
claude -p \
  --effort medium \
  --model sonnet \
  --no-session-persistence \
  --tools "mcp__codebase-memory-mcp,Read,Glob,Grep" \
  --output-format json \
  --json-schema '<placement-schema>' \
  --dangerously-skip-permissions \
  '<bounded placement prompt>'
```

When the placement premise depends on a known external artifact, add its
smallest stable containing directory to the same invocation:

```bash
  --add-dir '<bounded-external-artifact-root>'
```

Tell Claude in the bounded prompt which certificate clause requires that path,
that indexed structure remains primary for repository code understanding, and
that filesystem observation must not broaden beyond the named artifact. Do not
wait for an index-only call to fail before making an already-required external
artifact readable.


Require a JSON schema with:

- `semantic_outcome`: the user-visible or runtime consequence the slice must make true, independent of implementation shape.
- `placement_candidates`: at most three compact candidate objects naming the boundary, owner, producer, consumer, lifecycle, evidence for and against, downstream proof, and insufficient substitute.
- `discriminators`: at most five facts or questions that distinguish candidates.
- `ranges`: at most eight minimum canonical-document or ownership ranges Codex must read to choose responsibly.
- `blockers`: at most three unresolved premise conflicts, preconditions, or decisions.
- `placement_risk`: `low`, `medium`, or `high`, with evidence.
- `statistics`: available call statistics.

Do not request or return a singular recommended slice, complete call paths, exact changed-file lists, or a full implementation plan in this round. Ranking candidates is allowed only when the evidence and uncertainty remain visible.
