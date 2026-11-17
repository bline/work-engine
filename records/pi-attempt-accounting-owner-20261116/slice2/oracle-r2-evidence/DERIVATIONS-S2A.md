# Slice 2a authored derivations (finding-control cases)

Author: independent oracle (slice 2a), root `01a1e569`. This is an authored raw-policy argument, not consumer qualification. Dossiers are in `dossier/*.json` (schema `pi-accounting-oracle-dossier/1`, `dossier/dossier-1.schema.json`; all 12 validate). SourceRefs come from `tools/sourceref.py`, a navigation tool that computes nothing. Policy: BASE `43d0e7c8`, AMD `c8376f7a`, subject `spec/ck.json`, `contract/CONTRACT.md`, `ROOT-DECISIONS-R1.json`.

Families a control does not depend on are recorded as `RequiredUnmet(owner: oracle future batch)` with reason "outside slice-2a finding controls". Full S1 is out of scope.

## <a id="vc08"></a><a id="v03c"></a><a id="u06"></a><a id="v03a"></a>VC08, V03-control, U06, V03-api (R8-NAMESPACE-01)

**Raw facts (all four cases):**
- Launch is `openai-codex/gpt-6-astra` on the subscription lane.
- `S1/turn-1:1` is `subscription_proxy_admitted` with rid 1.
- `S1/turn-1:2` carries `subscription_proxy.rid` 1 and has no `usd`. It is a request in VC08 and an upstream_error in the others.

So both origins' references are in the **admission namespace** (NS1). The historical bundles declared a `dollar_ticket` scope, which is raw-inconsistent (NS2).

**Unit structure.** ROOT-DECISIONS-R1 Q-NAMESPACE is NOT ADMITTED, and root's 2026-11-16 reply chose option B. With no admitted scope and no admitted exact link, BASE §3 gives OriginBase per origin, so there are two units (NS3, PM2):
- `adm:S1/turn-1:rid=1` = {`:1`}. The admission row alone is clean subscription (as in N03-control-unlabelled-admission): codex, not_terminal, forwarding unknown, obligation none{subscription}, NoReservation.
- `row:S1/turn-1:line=2` = {`:2`}, which keeps the historical per-row lane determination:
  - VC08: codex, terminal, forwarded.
  - V03-control: codex, terminal (upstream_error), forwarding unknown.
  - U06 (unlabelled upstream_error) and V03-api (labelled openai/api on a subscription launch): PiApi AtLeast0, LaneContradiction, UnresolvedExposure, cause no_reservation.

**Attribution:** both units are unadmitted with candidate o (O-6/O-7), because no admission names S1 and o's tree contains S1.

**Money and standing** are identical to the grouped form: no owned spend, scope lows 0.
- VC08 and V03-control: established, admitted.
- U06 and V03-api: not_established (HistoricalUnadmitted, LaneContradiction, UnresolvedExposure), decision not_established.

## <a id="n03x"></a><a id="n03c"></a>N03-contradicted-admission, N03-control-unlabelled-admission

Each case has one origin, `subscription_proxy_admitted` rid 1, so the reference is in the admission namespace. The historical dollar_ticket contract and grant are withdrawn. The unit structure (a single origin) and the supplied money are unchanged. Only the namespace claims are derived.

## <a id="r08"></a>P130-R08-admissions-without-tree-still-block (R8-LOW-INVENTORY-01)

o (a1) and p (b1) both launched session c1, and neither tree contains S1. O-3 makes the attempt Uncertain, and O-3a keeps both admitting objectives as candidates. The usage origin `S1/turn-1:2` (complete 1000/100, 15M) is a candidate for both o and p, on the closed line in every grouping branch.

Under BASE §5 / SUPPORTED-LOW, U for `default/{o,p}/owned_closed` = {`S1/turn-1:2`}. Seed = ∅ because no owner is established. So single_floor = 0, low 0, no slot and no credit. The outstanding scopes are empty.

## <a id="r12"></a>P130-R12-untrusted-clock-keeps-candidate

The turn starts at 06:35:00, but the only row is dated 06:30:02. That violates L-2, so the clock is untrusted or contradicted and the interval is unplaceable (P-2). No admission names S1 (O-6), and O-7 keeps tree objective o. U(o, owned_closed) = {`S1/turn-1:1`}, Seed ∅, low 0.

## <a id="a04b"></a><a id="a04w"></a>P133-A04-cache-counts-both-absent, P133-A04-cache-write-absent-legacy

No admission names S1, and o's tree contains S1 with window [06:00, open), so the request at 06:30:02 is an unadmitted candidate of o. U(o, owned_closed) = {`S1/turn-1:1`}, Seed ∅, low 0. Membership depends only on candidate usage presence. The V-4 valuation amounts are not re-derived here.

## <a id="p130t"></a>P130-T-owned-before-window-summed (R8-PLACEMENT-01 positive)

Raw: reservation at 06:30:00, request at 06:30:02, o starts at 06:40, and a1 launched c1. If the declared dollar_ticket scope is admitted, the closed interval [06:30:00, 06:30:02] is **before** the window. Owner o is retained (O-1), with AttributionContradiction.

Without the scope, the reservation-only unit would straddle the start. The claim therefore stays `RequiredUnmet(root)` until root admits the proposed scope (see RESULT.md). The supplied value `before` agrees with the admitted branch. The Python and Rust placement predicates compute from the supplied grouping, which is the admitted branch.

## <a id="p130r02"></a>P130-R02-rounded-up-charge-admitted (R8-FIN-01 non-pilot control)

Raw `usd.charged_usd` has the exponent-form lexeme `1e-06`. That is an exact nano amount under MONEY producer-amount rules, so it is 1000 nano, retained by origin `S1/turn-1:2` in every branch (X-COMPONENTS). No supplied charge or high may fall below 1000.
