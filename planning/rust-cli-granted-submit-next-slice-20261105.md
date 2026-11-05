# CLI granted command submission: proposed next slice

Recorded 2026-11-05. Proposal only. The current authorization covers merging the accepted granted-read CLI and presenting this budget; it does not admit command implementation or any native/provider work.

## Result and boundary

Add one explicitly invoked Rust command entry point for existing `submit_session_request`, using the accepted public `work_engine_server::LocalClient.command` and its qualified local service profile. Prefer a separate `work-engine-command` binary in the Rust workspace so the read CLI retains its current interface and allowlist. The implementation may reuse an existing package or introduce a small binary crate after bounded placement validation; it must not create another HTTP, grant, operation or reconciliation owner.

The operator supplies an existing exact domain request UUID, existing grant credential, session ID, workspace and input text via protected JSON file or stdin. Transport credential remains a separate private profile input. No credential issuance, implicit authority, secret-bearing argv, automatic retry, fallback, auto-selected target, or new request identity is introduced. Invalid invocation, shape and unsupported profiles fail before connecting. Domain authorization and payload/target binding stay with the accepted owner.

Machine output preserves the complete application envelope and distinguishes receipt/admission, native entry, uncertain entry, reconciliation, result and process closure. Human output preserves unknown/partial/unavailable states and terminal escaping. A transport timeout is not cancellation, proof of non-entry or settlement. Any explicit repeat uses the original identity and owner replay behavior; it does not make a second request. The new command adapter does not add a background retry/recovery loop.

## Admission dependencies

Require human approval of this exact proposal, fresh root/context/account/cost capacity, then-current compatible public server/client/owner source and exact qualified runtime/config/store bindings. Current controlled `offline-synthetic-v1` is the only admitted test profile: trusted explicit command capability, separate private command profile, query profile `bridge:null`, MCP disabled for CLI proof, synthetic native socket under test ownership. A profile name alone does not establish a synthetic endpoint.

The accepted external bridge is `fb54335946eaf20bdb70e0d849c3c9548aa6a9aff56b785467adb25c565d134a`; owner is `d5d51ab3a870a6efa82f5824a4e3cb9ba5468263724f6068c0d46b5ec586b8a2`. Bind the accepted external bridge's owner/source lineage and executable fixture at preflight; return a precise dependency if this cannot be established within the preparation guard. A merge-time fixture setup failure was traced to group-write permission on a root-owned evidence parent, corrected to0700; a fresh diagnostic confirmed shared-source bridge schema initialization. No bridge schema mismatch was established, and no owner changes are included. Do not silently narrow to fabricated owner responses. No AH2-T task/answer API or its separate funding is needed or supplied by this slice.

## Executable checkpoint and acceptance

Before 7 USD aggregate or 35 minutes from actual admission, demonstrate a real new command CLI calling the running accepted server with MCP disabled and a fixture-issued, existing grant for an explicitly controlled synthetic session. Independently observe the synthetic native fixture's entry counter, domain durable operation identity and returned owner observations; DTO equality alone is insufficient. Fixture grant issuance is test setup, not product grant issuance or a live authorization path.

Validate one admitted synthetic operation, exact original-identity repeats and concurrent repeats without a second entry, lost acknowledgement/uncertain entry, timeout/disconnect, explicit read/reconcile and restart with no invented retry or settlement. Use the accepted owner contract to distinguish outcomes it can establish from unsupported recovery claims. Cover wrong/missing/revoked/expired admission grants, target/payload conflicts, busy/unavailable owner, malformed inputs, private credential confidentiality, total deadline, complete machine envelopes and terminal-safe human output. Preserve the direct four-read, AH1 list/read and newly merged granted-read regressions.

Run relevant format, locked build, unit tests and strict no-deps Clippy, plus actual process shutdown/listener/helper checks under custody. Freeze exact source, generation-specific binaries, configuration, inputs, captured semantic output and process results. Preserve failed attempts and missing evidence. Obtain fresh separate Astra/high/default review with root-observed native readOnly/networkAccess:false before source acceptance. Provider independence is not claimed. Final result is private source and integration handoff; a later shared merge/push/deployment has separate authority.

## Budget and clocks

Recommend **18 USD total and 120 minutes from actual admission**, target 75 minutes, preserving the final 45 minutes and at least 6 USD for review, repair and closure. The clock starts only when human approval and accepted usable dependencies/capacity are both present; once admitted it never resets.

| Allocation | USD |
| --- | ---: |
| This merge/planning pass, final contract/runtime binding and checkpoint | 4 |
| Command adapter implementation | 4 |
| Focused executable, compatibility and process validation | 4 |
| Independent review and repairs | 4 |
| Collection, handoff and contingency | 2 |
| **Total** | **18** |

The preparation guard is **3.50 USD aggregate** including this merge/planning carry. If the accepted bridge/runtime/source binding or capacity cannot be established inside it, return the exact constraint and revised estimate before any builder/native fixture entry. The executable checkpoint is the earlier of **7 USD aggregate or 35 minutes**. Candidate implementation/validation must fit 12 USD aggregate, leaving 6 for final review/repair/closure. Stop and return measured outcome plus remaining work at a failed checkpoint, cost ceiling or hard deadline; no silent extension or transfer from peer grants.

All figures use existing speed-adjusted API-equivalent USD proxies, not measured subscription quota or an actual bill. Include this current merge/planning pass from 2026-11-05T00:18:57.180Z exactly once if funded, with its final reporting tail. The current supervisor 5 USD administrative guard is overlapping control, not an extra 5 added to 18. Earlier closed 12 USD granted-read implementation, 15 USD service implementation and 13.37 USD original query work are excluded. No peer funding is transferred.

After the contract is settled, provisionally use one directly supervised Terra/medium/default builder and a fresh separate Astra/high/default reviewer; preserve the user's Sol6.1/xhigh/default root settings. Select a stronger builder only for concrete remaining decisions rather than a domain label. No builder, listener or future waiting obligation is launched by this proposal.

## Later scope

Live/provider/Pi execution, grant issuance UI/CLI, provisioning, remote OAuth, task/approval answers, owner/schema changes, background command recovery and server deployment remain separate work. This proposal advances the accepted command surface through a narrow local adapter; it does not make the controlled synthetic qualification a production execution claim.

Evidence owners: current merge/qualification/accounting under `/home/bline/.local/state/work-engine/cli-granted-merge-next-20261105T0019`; original bounded source/review under `/home/bline/.local/state/work-engine/cli-granted-service-reads-20261104T2318`; accepted AH2 source/profile under `/home/bline/.local/state/work-engine/ah2c-server-20261104T2242`. Tier2 graph project home-bline-code-work-engine, generation u689e571450b08c56g2/index00:11:06. LocalClient.command exact source confirms submit/read allowlist; both-direction depth1 trace has one example caller and a spurious unrelated matches edge, so no complete caller claim is made. Unresolved call ranges in client/command/CLI were checked directly; new fixture absent at shared baseline was read from accepted source. Coverage best effort, not proof of completeness.
