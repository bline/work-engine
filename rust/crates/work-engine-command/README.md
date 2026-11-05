# work-engine-command

Submit one request under an existing grant:

```text
work-engine-command PROFILE submit_session_request --json|--human INPUT
```

`PROFILE` is a private JSON file with `version: 1`, `kind: "local-service"`, loopback `endpoint`, absolute `credential_file` and `timeout_seconds` (1–60). The credential file holds the local service bearer, separately from the domain grant. `INPUT` is a private JSON file or `-` for standard input. It must contain exactly the string fields `request_id` (the original UUID), `grant_credential`, `session_id`, `workspace`, and `input_text`. Supply the grant and exact original request and payload; the adapter does not issue credentials or choose targets.

JSON output carries the complete service envelope when one was received. An owner result is an observation, not proof of completion or settlement. A timeout, disconnect or invalid response can leave entry unknown; inspect the original request through `work-engine-query` before deciding on an explicit repeat. The adapter never retries automatically. The timeout bounds the client wait, not the lifetime or cancellation of server-side helpers.
