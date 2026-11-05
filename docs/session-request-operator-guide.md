# Session request operator guide

This operating reference describes the capabilities qualified at source `c1d238775afd3b7e42f81086566d47634dcf1317`. Its design acceptance and evidence are recorded in the [dated decision](../records/rust-cli-operator-readiness-decision-20261105.md). It confers no new grant policy, deployment permission, runtime capability or live authority. Reassess affected claims when their owning implementation or operational assumptions change.

## Scope and qualified source

Reuse the existing local `session-bridge` issuer and exact submit/read adapters. No new issuer, credential service, HTTP endpoint, Rust wrapper or recovery state machine is justified by this inspection. The public service is qualified only for its controlled `offline-synthetic-v1` profile. A real native socket or provider cannot acquire that qualification by copying the profile label. First live execution remains blocked on the concrete operational qualification and human decisions below.

Source subject: accepted shared commit `c1d238775afd3b7e42f81086566d47634dcf1317`, tree `6bf1696153265a9633ceacadd0ce7cdcac3ef8e5`. Read from the clean private worktree `/home/bline/cbm-work/wt-cli-command-merge-20261105`, excluding concurrent RF, task/answer and Pi adapter work. Session-query and server command owner pin is `470a625b76d17a62a68659709d99d2767c944441be4cd2ddbff96bc6be12a12d`. Current common-source binary/test standing is owned by `/home/bline/.local/state/work-engine/cli-command-merge-next-20261105T0121/ROOT-ACCEPTANCE.json`, `RUNTIME-COMMON.json` and `MERGE-VERIFICATION.json`; this design reruns no runtime test and makes no broader native compatibility claim.

## Existing ownership and capability evidence

Paths below are relative to that exact source subject. References identify inspected owning functions, not a claim of exhaustive graph coverage.

| Boundary | Existing owner and source | What it establishes; limit |
| --- | --- | --- |
| Private store/config | `session-bridge/src/store.rs:33–149`, secure/config/initialize/open | Owner UID, private files/directories, store UUID/meta agreement and SQLite checks. Explicit local initialization creates durable grant/operation tables. It does not discover or create a native target. |
| Exact wire | `session-bridge/src/wire.rs:1–93`, wire/operation_digest | Canonical request/session UUID, UTF-8 payload <=32KiB without NUL, payload hash, fixed runtime header plus JSON and final newline, wire hash and operation identity. Raw text and its final newline matter. |
| Local preview/issue/revoke | `session-bridge/src/lib.rs:39–179`; `store.rs:217–291` | Preview canonicalizes workspace, reads input and computes exact wire; after config/store opening it makes no native call. Issue recomputes wire, checks reviewed hash, observes native target/history, creates random private bearer and durable exact request grant. Revoke blocks subsequent authorized reads/submits; no cancellation. |
| Native profile | `session-bridge/src/native.rs:289–336,452–578`, connect/preflight/resume/start | WebSocket over Unix stream, initialize/initialized, loaded/read/history and resume/turn/start with correlation. This is a Codex-shaped protocol, not proof that any installed endpoint or Pi speaks it. Preflight idle/history observations and cooperative locks do not fence unrelated native clients. |
| Domain admission/reconciliation | `session-bridge/src/lib.rs:180–415`, rpc/checked_projection/submit_with_native; `store.rs:274–310` | Grant binds original request/session/canonical workspace/payload/wire/operation. Admission checks expiry, reserves occupancy and durable identity before possible send. Existing operation is projected on exact repeat; reads reconcile without a new submit. Capability remains message admission only, with process closure unavailable. |
| Private server composition | `session-query/src/lib.rs:24–104`; `work-engine-server/README.md`, Granted commands; `work-engine-server/src/command.rs:1–34` | Separate query profile (`bridge:null`) and optional command profile with bridge binary/config pins. Current 470 owner pin is checked. Trusted operator controls files and parents; check/use races remain. |
| Public client/service | `work-engine-server/src/lib.rs:35–77`; command README and `work-engine-command/src/main.rs:10–154` | Public command allowlist is submit/read-operation/read-result; query API retains list/read-session. Separate service bearer authenticates transport only. Submit takes five exact string fields, no automatic retry, target selection or grant issuance. Full envelope and human uncertainty/limitations preserve owner standing. |
| Distinct provisioning owner | `session-bridge/README.md`; `session-bridge/src/lib.rs:54–102`; `session_events.py:2322–2333,2782–2785` | Optional pre-existing provisioning has its own policy, parent credential, child handle and held connection. It is disabled by default and needs its own qualified profile. It is not the legacy message-grant bearer or an automatically available LocalClient issuer. This design neither activates nor replaces it. |

Here `session-bridge` paths mean `rust/crates/session-bridge`, and similarly for the other Rust crates. Python path is `skills/session-events/scripts/session_events.py`. The graph project was `home-bline-code-work-engine`, Tier2, generation `u39609e913a3e9c35g4`. Search/trace/snippet located cli and store.issue; inbound issue trace had unknown completeness. Coverage flagged unresolved calls in all relied-on code files; relevant owning source and direct calls were read explicitly. Missing query README was confirmed and is not evidence. These are bounded source findings, not an exhaustive repository absence claim.

## Operator flow and examples

These examples are shapes for a later authorized operator, never commands executed in this design slice. `<...>` values are non-secret placeholders, not working fixture credentials or target identities. Select controlled absolute paths and preserve exact bytes, ownership and permissions. Do not paste credentials into argv, board messages, logs, native input or review artifacts. The operator prepares credential-bearing JSON through an approved private channel.

1. Bind the accepted binary build/source hashes, owner hash, private bridge config/store UUID/database/lock directory/native socket, server config and both server profiles. Control the referenced files and parent directories throughout use. Initialize a new local store only if explicitly intended; never replace an existing store to clear uncertainty.

```text
<bridge> --config <new-private-config> store init --database <new-private-db> --lock-dir <private-existing-lock-dir> --native-socket <selected-unix-socket>
```

2. The human/operator selects the original request UUID, already loaded dedicated session UUID and canonical existing workspace. The human approves the exact runtime-authored wire, including the fixed header, target and payload bytes. Input is a UTF-8 file; preserve trailing newline and Unicode. Preview itself still needs an initialized readable private config/store; it does not establish native target health.

```text
<bridge> --config <config> grant preview --request-id <original-request-uuid> --session-id <selected-session-uuid> --workspace <canonical-workspace> --input-file <exact-input-file>
<bridge> --config <config> grant issue --request-id <same-request-uuid> --session-id <same-session-uuid> --workspace <same-workspace> --input-file <same-input-file> --admit-until <future-RFC3339-UTC> --credential-out <new-private-grant-file> --dedicated-session --reviewed-wire-sha256 <human-reviewed-wire-sha256>
```

The issuer checks hash equality and target preflight, but `--reviewed-wire-sha256` and `--dedicated-session` are caller assertions, not independently verified human approval or an exclusive native fence. The external operator owns the review/authority record and exclusive-use condition. This legacy grant reports runtime policy unavailable: it does not itself enforce provider budget, sandbox, network or downstream effects. A message-admission grant cannot silently authorize whatever the message asks the target to do.

3. Bind separate service transport configuration. Example client profile:

```json
{"version":1,"kind":"local-service","endpoint":"127.0.0.1:49312","credential_file":"/absolute/private/service-bearer","timeout_seconds":40}
```

The service bearer is 32–4096 printable ASCII bytes without newline, in its protected file. The issuer writes the domain bearer into a distinct private credential file. Transfer the bearer value into the protected submit JSON without changing the approved payload text. Do not put a credential-file path where the API expects the actual grant string. The command profile is a separate pinned session-query profile with bridge enabled; the query profile retains bridge:null. Explicit server command capability and qualified command profile are both required; liveness/readiness alone do not authorize a command.

```json
{"request_id":"<original-request-uuid>","grant_credential":"<domain-bearer-value>","session_id":"<selected-session-uuid>","workspace":"<canonical-workspace>","input_text":"<exact-reviewed-input-text>"}
```

```text
work-engine-command <client-profile> submit_session_request --json <private-submit-json>
```

4. Keep original identity. Reconcile with `LocalClient.command("read_session_operation", {"request_id": original, "grant_credential": grant}, correlation)` and similarly `read_session_result` with text_offset/max_chars for bounded pagination, or the existing query CLI's granted read interface. Correlation string is transport metadata, not a new domain request ID. Inspect the complete envelope, returned owner errors, operation/native turn IDs, native status, reconciliation time, completion/result availability, redaction/truncation and limitations. An explicit repeat requires the same original request/grant/target/payload; it is not a retry mechanism to bypass an unresolved operation. Do not generate a replacement UUID, grant or store to force entry.

5. Admission expiry is not cancellation or a read deadline. `authorize` does not reject reads solely for elapsed expiry; submit checks expiry before first admission, while an already recorded exact repeat returns its existing projection. Revocation blocks authorized reads and submits and does not cancel admitted work. Preserve required observations before revocation when possible. Local `operation inspect --request-id <original>` reads a stored projection but cannot establish current native settlement. If reads are lost, retain uncertainty and hand the original identity to the operator rather than replacing state.

```text
<bridge> --config <config> operation inspect --request-id <original-request-uuid>
<bridge> --config <config> grant revoke --grant-id <issued-grant-id>
```

The operator owns server drain/collection, native target/process cleanup and evidence retention. A client timeout, disconnect, helper termination, revoked bearer or terminal turn is not proof of cancellation, process closure or cost settlement. Legacy direct grants provide no target-close command. Do not borrow provisioning close semantics for them.

## First live run: readiness contract, currently not ready

| Required bound fact | Owner / present standing |
| --- | --- |
| Actual provider/model/effort/tier, limits and effect permissions | Human authorizes; supervising runtime owner verifies actual configuration and available enforcement. Unselected. Existing message grant cannot establish them. |
| Installed native executable/version and actual Unix WebSocket endpoint/protocol | Native deployment owner qualifies it against the exact bridge build. Unknown; FakeNative/common-source synthetic tests do not establish this. Pi compatibility unestablished. |
| Dedicated session/workspace/checkpoint/config and exclusive-use arrangement | Human/native operator selects; runtime observer records loaded identity, canonical workspace, stable history and actual controls. Unselected. |
| Exact request UUID, payload bytes/hash, complete wire/hash, review authority, expiry and credential custody | Human approves exact subject; local issuer creates only the authorized exact grant after separately permitted preflight. None issued here. |
| Service profile authorization | Service owner qualifies the actual live composition/capability. `offline-synthetic-v1` is insufficient and must not be relabelled as live. Missing accepted live profile. |
| Cost/clock and reserve | Human funds separate live scope; root binds all actors, first funded time, deadline, review/closure reserve and owned spend observation. Neither the design nor this documentation publication funds a live run. |
| Native and domain effect observation | Supervising observer binds original request, grant, store, operation, native turn and exact terminal history/result; record unavailable usage and effect coverage. Unselected. At-most-once cooperating admission evidence is not independent proof of world effects. |
| Cleanup and unknown-outcome owner | Human names native cleanup/cancellation capabilities and a collection/escalation owner before entry. Retain an occupied uncertain operation, logs and original IDs on failure; never equate deadline/transport failure with non-entry. No owner/capability bound yet. |

Admission to a later live run requires these owned facts and explicit live authority; filling this table with fixture IDs is not sufficient. If the actual realization cannot establish required sandbox/effect/observation claims, report the acceptance condition unmet to its owner rather than weakening it or inventing a compatibility layer. AH/task/answer/RF/Pi work has separate authority and funding and is not a dependency by default.

## Acceptance and negative cases

The source design received fresh separate Astra/xhigh/default native readOnly/network-disabled review and root acceptance, as recorded in the dated decision. Publication changes only its heading, standing and evidence links; the operator examples and capability boundaries retain that reviewed meaning. Review is same-provider, not provider independence.

For any later implementation/qualification: use meaningful exact-byte vectors (Unicode/trailing newline/header changes), altered target/request/wire refusal, expired first admission versus existing-operation reads, revoked reads, wrong bearer role, rejected extra fields, partial/truncated/unavailable results, timeout/disconnect then original-identity reconciliation without another entry, and separate native closure/usage evidence. Existing synthetic tests retain bounded standing; the design and documentation publication performed no runtime tests, preview, issuer or native calls.
