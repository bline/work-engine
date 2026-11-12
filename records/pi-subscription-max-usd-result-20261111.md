# Pi subscription container cost-proxy limit — result

November 11, 2026. Implemented locally; commit and skill deployment remain pending.

The user requested `--max-usd` support and explicitly selected an API-equivalent cost-proxy stopping limit. Subscription containers now accept a finite positive threshold, with launch-pinned default API-equivalent rates. Only one request may be in flight per session. Reaching the threshold or losing accounting prevents further requests and retained delivery. One admitted response can overshoot; billing and subscription quota remain unknown. Existing API reservation/output limits retain their semantics. Subscription output caps, strict tiers, service tiers and API-dollar objective admission remain refused.

Host-only durable request/turn reservations, process-shared turn exclusion and fail-closed replay preserve uncertainty across launcher loss, truncated streams, malformed logs and agent prompt deletion. Status and the ledger retain separately labelled proxy measurements in the Codex pool.

Exact accepted private checkpoint: `5a3fd0ad1d9445d0c4519eacfe8017437293ecee`, tree `6911574e8ee8fdd6607fa62b37480dd724322660`; task patch SHA256 `278d5b582032a6178a91766615fe14edb0ec93bb8821bb47cd8effa7b1fabc7d`, plan `builder-packet-v1`, scope `subscription-proxy-v1-host-only-marker`.

195 tests passed (43 subscription,43 API container,27 Anthropic,21 host,61 ledger), plus diffcheck and CLI synopsis check. Custody summary SHA256 `ca01cc259654d3eba46eb98fd6661cf4d2642a9283e3cb0ac2f97e42114632ae`; seven records bind ten inputs each. Root verified record/output hashes and every input against both the candidate and integrated files; shared CLI/diff checks also passed. Tests use synthetic credentials and local mocks.

Separate initial Astra/high/default/read-only review, retained through two repairs, accepted this exact checkpoint. Initial findings were truncated HTTP framing and concurrent delivery budgets; re-review caught agent-writable crash custody. Both final findings are resolved, with no new actionable issue. Runtime identity and actual read-only grants were independently observed; retained rechecks create no new independence claim. Agent-instruction-review applied to changed guidance and CLI help, with no separate instruction finding.

Integrated exactly five reviewed files into the working tree at shared HEAD `45155c0259d6925540d913c97930d4a732ddc91c`, preserving the index and unrelated work. No live provider request, R-2 integration/distribution, push or skill deployment occurred. Prior candidates and unsuccessful review verdicts remain preserved.

Working authority, gate verification, review reports, integration receipt, budget and process dispositions are under `/home/bline/cbm-work/pi-subscription-max-usd-20261111/`. The supervisor set an $8 speed-adjusted engineering proxy ceiling with a $5 checkpoint, separate from this feature's default API-equivalent valuation. Last measured before integration: $7.8610784; closure measurement and reporting tail are recorded in that archive, with no claim of settled total.
