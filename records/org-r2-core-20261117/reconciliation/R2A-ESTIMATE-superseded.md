# R2-A' budget estimate (root, 2026-11-17)

Scope (approved R2 design, `s2/ROOT-S2-ADDENDUM.md` items 3–9 and review §7 "second slice"): replace the
labelled assumed discharges for the separation, component-binding, authority-requirement and
realization-admission kinds with real checks. Mapping-rule kinds stay assumed (O7). No storage, grant issuance or
production admission.

Split into two slices so each review surface stays bounded:

| Slice | Content | Builder | Review | Remediation + recheck | Root | Total |
|---|---|---|---|---|---|---|
| A: admission core | New pure crate (organizational authority): owner-attested principal / vantage-actor / component-binding / realization-acceptance records (6.1 F4/F5, property-defined identities); observation trait + fixture source incl. Unknown; Gate O subset O3, O4, O5, O8, O9; Gate R R1–R4; states current/stale/revoked/unknown/refused; 6.1 §6 oracle classes; mutants | Opus high $10–14 | Fable xhigh (complex) $8–12 | $4–7 | $4–5 | **$26–38** |
| B: host boundary | Delivery host builds admission from an observation per boundary (reserve → observe-for-reservation → gate → regate at effect commit) for spawn and consumption; total discharge from gate outcomes replaces assumed discharges for the four kinds; same-observation/later-boundary negative; S1/S3 matrices re-run | Opus high $8–12 | Fable xhigh $8–12 | $4–7 | $4–5 | **$24–36** |

Total **$50–74**. Recommended grant: **$75**, as a new objective (current objective closes at about $88 of $100;
the remaining ~$12 is not carried).

Basis: this objective's actuals. S1 builder $10.47 / Fable review $12.14 / cold-cache recheck $7.06; S3 builder
$7.32 / review $6.92 / remediation ~$2 / recheck ~$1.9; S2 design $4.86 + review $7.74; root ~$5 per slice. Slice A is
larger than S3 (new crate and two gates); slice B touches the accepted vertical again.

Risks: the delivery-gate subsystem already carries one rejection warning (history r41); a second substantive
rejection there requires a design review (~$8) before further repair. Fable availability on the subscription is
shared with other sessions. Stop rule: after each slice's first review, stop and report if the remainder cannot
cover remediation, recheck and closure.

Billing: children run through Claude Code on the Claude subscription (`env -u ANTHROPIC_API_KEY`, logs show Claude
Max), equivalent to the `claude-sdk/` namespace; no `anthropic/` API-key billing.
