# Scripted native-host probe

This **test-only** mod replaces every `turn.step` response. It makes no model
requests and must never be loaded into a working agent. Use it with the
production adapter in a disposable directory. It writes `probe.json` there.

The driver calls the real registered tool, ends the requesting turn, observes
the automatic recovery turn, and inspects the adapter's status. It checks that
the provider session changed, the old conversation marker disappeared, and the
predecessor recovery pointer and Wind Walker instruction reached the new
conversation. A second user message repeats the cycle in the same process.
It does not perform semantic recovery or publish a real checkpoint.

From the Work Engine repository, record the absolute plugin path and change to
a fresh temporary directory:

```sh
context_plugin="$PWD/skills/manual-orchestration/claude-context-plugin"
context_probe_dir="$(mktemp -d)"
cd "$context_probe_dir"
cat | claude -p --input-format stream-json --output-format stream-json --verbose \
  --allowedTools mcp__work-engine-context__new_context \
  --plugin-dir "$context_plugin" \
  --plugin-dir "$context_plugin/tests/native-driver" \
  --setting-sources '' --strict-mcp-config --mcp-config '{"mcpServers":{}}' \
  --system-prompt 'Isolated deterministic context probe. No external work.'
```

Send this input and keep stdin open until `probe.json` contains a
`recovery_turn` and `adapter_status`:

```json
{"type":"user","message":{"role":"user","content":"PRE_CLEAR_ONLY_MARKER native context probe"}}
```

Send it again for a second cycle; close stdin with Ctrl-D after the observations
settle. Do not use `--tools ''`: that also filters out the registered mod tool.
For the interactive surface, omit `cat |`, `-p`, and the input/output format
flags, then type the marker as ordinary text. Closed-input negative cases use
the same headless command with input redirected from a completed file.

The [dated probe](../../../../../metrics/probes/claude-native-new-context-20261115.md)
records observed results and distinguishes scripted native-host behavior from
live-model recovery.
