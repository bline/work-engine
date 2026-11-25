# S3: live smoke of the Claude subscription container lane

2026-11-25, operator-approved. The approval covered up to 6 requests on the owner's
Claude Max subscription through `claude-sdk/`, with no API key and no fallback. The
run stopped at no failure and used **5 requests**. Lane at commit `006c9a67`, Pi
snap revision 93 (Pi 1.6.0), bundle digest `e6e4ad42fc1ff4c4…` (unchanged from
[S0](../pi-container-claude-sdk-s0-20261124/README.md)).

[drive.py](drive.py) ran each step through `role_host.py` with a disposable
state root and workspace, and stopped at the first failure. [evidence/](evidence/)
holds `results.jsonl` (per-step `status` summaries) and the host-only proxy logs.
The logs hold digests, verdicts and usage, and no prompt, response or credential.

| Step | Model and flags | Requests | Result |
| --- | --- | --- | --- |
| A launch | `claude-haiku-5-5`, `--tools read`, cap 2 | 2 | `read` tool round, then the file contents in the reply |
| B delivery, same session | Haiku, cap 2 | 1 | completed, turn 2 |
| C retire and custody | — | 0 | `idle_released`; the real token is in no file under the run directory |
| E | `claude-opus-5-5`, `--thinking high`, `--no-tools`, `--append-system-prompt` | 1 | completed |
| D | Opus, same, but `--system-prompt` (extension prompt replaced) | 1 | completed |

For every request:
- HTTP 200, disposition `completed`, `usage_status: reported`.
- Billing verdict `allowed`, with `anthropic-ratelimit-unified-status: allowed` and
  `anthropic-ratelimit-unified-overage-status: rejected`.
- The first `message_start` model was exactly the requested ID.
- `tool_names: []` for the Opus requests.
- Container removed.

## What this establishes, and what it doesn't

- **Live route evidence.** For these 5 requests on this account, the lane carried
  real traffic end to end: Claude Code ran in the networkless container with a
  placeholder credential, and the host injected the bearer. Both supported model
  IDs were served exactly, and the subscription verdict headers were present and
  allowed.
- **P7 billing question.** A request with Pi's system prompt replaced (step D)
  was also `allowed` with overage `rejected`. The extension README's 2026-10-04
  observation (`400 … extra usage` with Pi's default prompt) did not recur for
  this replaced prompt. That is one observation, and Anthropic's routing can
  change. The lane still refuses rather than falling back if it does.
  - Step D's request was not cached (`input_tokens: 423`, no cache writes).
  - Step E's was (`cache_creation_input_tokens: 913`).
- **Not established.**
  - Subscription quota consumption is not measured. The usage above is
    provider-reported tokens.
  - These 5 requests do not establish qualification for other models,
    sustained use, or a P7 receipt.
