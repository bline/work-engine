# AH2-C shared Rust server result — 2026-11-04

Accepted private source **5ed0d1d81a309b951dae4e39805b279d68fc2e4b**,
tree **ec4330d7b65254687b78afae413e30cc2cd08df1**, from accepted68a056b9.
Seven server-crate paths compose existing granted submit/operation/result owners
through HTTP, MCP and the Rust client. Existing AH1 query behavior is preserved.
The source is reviewed and ready for integration; it has not been published to
the shared branch.

The selected profile is controlled **offline-synthetic-v1**. Trusted startup
must explicitly enable commands and supply a separate pinned command profile;
query-profile bridge remains null. LocalClient::command uses the original domain
request UUID and grant credential, separate from local transport authentication.
Exact retry keeps one operation; public reads reconcile owner state after restart.
Unknown, refused and unavailable do not imply cancellation or settlement. No
server grant issuer, native launcher or parallel operation journal was introduced.

Seven final custody checks passed: build, Rust tests, fmt, strict crate Clippy,
command conformance, pre-attempt fault and AH1 regression. Each binds897 identical
repository inputs before execution, matching the final source. Root verified190
retained runtime files. The process fixtures cover HTTP/MCP/Rust-client parity,
concurrent retry, conflicts/refusals, lost ACK/history outage, same-reference
restart, EOF drain and stranded admission. Native effect counts establish one
start on retry and zero starts for busy/pre-attempt refusal. Sampled leftovers
were zero; a complete inventory of short-lived helper PIDs is not claimed.

Separate Astra/high/default review ran with an independently observed
readOnly/network-disabled grant. Initial P2 AH2C-R1 found lost session_busy
semantics. Both adapters and actual-process coverage were repaired; the same
retained reviewer accepted the exact final subject with no open findings.
Validation2's missing source binding was superseded by the seven bound final
runs. Earlier failures remain preserved.

**Observed cost: $11.34 of the approved $35**, in speed-adjusted
API-equivalent proxy units. It includes preparation carry2.427675 once; the
pre-builder guard observed2.5102402 against3. Scope closed accepted23:23:44UTC,
within the unchanged3-hour implementation clock from22:51:22. Closing/reporting
tail is unsettled and remains attributed inside the original cap, not zero.
AH2-T30 and AH3 20 remain unapproved; peer grants are separate.

Pi/live/provider/provisioning and hostile-mutator behavior remain unqualified.
The native daemon is synthetic and response text is injected into a synthetic
transcript. Existing bridge check/use and unrelated-client idle-fence limitations,
non-hermetic toolchain, SQLite snapshot timing and AH1 post-call request-retention
limits remain. No general production or newer joined CLI/backend/S10 qualification.

Exact acceptance, validation, review and private full7path integration patch:
/home/bline/.local/state/work-engine/ah2c-server-20261104T2242/ROOT-ACCEPTANCE.json
/home/bline/.local/state/work-engine/ah2c-server-20261104T2242/ROOT-FINAL-VALIDATION.json
/home/bline/.local/state/work-engine/ah2c-server-20261104T2242/INTEGRATION-HANDOFF.md
/home/bline/.local/state/work-engine/ah2c-server-20261104T2242/FINAL-SOURCE-MANIFEST.json

Full patch SHA2563c42ae0051075786fa4292ffb3b15f0c65fdd5208296098e832fee1fd27d986d.
CLI received the requested native accepted handoff under its own12/90 grant.
Both source actors and all implementation/check observers are stopped/collected.
The root-owned12-hour role chatboard/context observer remains active on67206,
collector g185:194, with verified native delivery; it grants no new work authority.
