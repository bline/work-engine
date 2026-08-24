Ah—yes. Static profiles aren’t what you meant.

There is runtime control in the App Server, but it is split across two mechanisms:

1. `skills/config/write` can enable or disable a skill while the App Server is running.
2. `turn/start` can explicitly inject a particular skill by path for that turn.

For example, your supervisor could select the mock skill at runtime:

```json
{
  "method": "turn/start",
  "id": 101,
  "params": {
    "threadId": "thread-1",
    "input": [
      {
        "type": "text",
        "text": "$checkpoint Execute this as a dry run."
      },
      {
        "type": "skill",
        "name": "checkpoint",
        "path": "/work-engine/skills/mock/checkpoint/SKILL.md"
      }
    ]
  }
}
```

The `skill` input item causes App Server to inject the full instructions from that exact path. That means your engine can resolve:

```text
capability: checkpoint
mode: dry-run
→ /skills/mock/checkpoint/SKILL.md
```

instead of relying on Codex’s implicit skill discovery. [App Server skill invocation](https://learn.chatgpt.com/docs/app-server#skills)

You can also change the globally enabled state at runtime:

```json
{
  "method": "skills/config/write",
  "id": 26,
  "params": {
    "path": "/work-engine/skills/real/checkpoint/SKILL.md",
    "enabled": false
  }
}
```

Then refresh discovery with:

```json
{
  "method": "skills/list",
  "id": 27,
  "params": {
    "cwds": ["/work-engine"],
    "forceReload": true
  }
}
```

App Server also emits `skills/changed` when watched skill files change. [Runtime skill configuration](https://learn.chatgpt.com/docs/app-server#skills)

The limitation is that `skills/config/write` appears to modify persistent configuration globally—it is not documented as a thread-local capability set. So toggling it around concurrent runs could race:

```text
production thread ── expects real skill
dry-run thread ───── disables real skill globally
```

I would avoid that mechanism for concurrent environment switching.

The better runtime design is probably:

```text
runtime environment
    capability: checkpoint
    implementation: mock
           │
           ▼
supervisor resolves exact SKILL.md path
           │
           ▼
turn/start receives explicit skill item
```

Even better: keep one semantic skill and switch the actual tools at thread creation. App Server has experimental `dynamicTools` on `thread/start`, so the same skill contract could receive either:

```text
checkpoint_write → real implementation
```

or:

```text
checkpoint_write → receipt-producing mock
```

Those dynamic tools are thread-associated and persist in thread metadata, which is much closer to the environment semantics you want. [App Server dynamic tools](https://learn.chatgpt.com/docs/app-server#dynamic-tool-calls-experimental)

So the strongest arrangement is:

* Skill = stable behavioral/capability contract.
* Dynamic tool = runtime-selected implementation.
* Thread environment = real or mock tool bindings.
* Dry-run tool returns structurally valid simulated receipts.
* Real side-effect tool is never supplied to the dry-run thread.

That gives you genuine runtime dependency injection. You would not need mock copies of most skills—only mock implementations of their effectful tools. The skill stays invariant while its available mechanisms vary by environment.

