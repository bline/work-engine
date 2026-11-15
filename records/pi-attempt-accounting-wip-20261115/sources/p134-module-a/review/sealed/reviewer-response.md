```json
{
  "review_set_complete": true,
  "financial_contract_verdict": "REJECT",
  "s2a_financial_readiness": "NOT_READY",
  "architecture_assessment": {
    "result": "local_corrections",
    "reason": "The provisional-exposure measure, reservation-retention policy and downstream classification are viable. Contradictory exposure rules, an unsafe corpus expectation and incomplete financial interfaces need correction; replacing the overall architecture is not justified."
  },
  "examination": [
    {"area": 1, "result": "defect", "basis": "contract/CONTRACT.md:329-353,384-394; contract/TRANSITIONS.md:25,44-47; case:VC01-openai-usage-settlement. Validity alone correctly does not authorize replacement, but construction of replacement-dependent S output is not reconciled with S's declared inputs."},
    {"area": 2, "result": "defect", "basis": "contract/CONTRACT.md:431-435,508-515; case:P133-A02-error-status-with-priceable-usage. The main exposure table retains reservations correctly, but the earlier unconditional error-status rule can discard a larger usage amount."},
    {"area": 3, "result": "defect", "basis": "contract/CONTRACT.md:280-285,488-497,509; corpus/sample-cases.jsonl:32. Incomplete and unpriceable usage normally retains R; the other-provider corpus expectation nevertheless replaces R with a finite amount despite an unpriced identity."},
    {"area": 4, "result": "defect", "basis": "contract/CONTRACT.md:431-435,512; producer/pi_container.py.excerpts:1002-1004,1533-1570; case:P133-A02-error-status-with-priceable-usage. Supported error-status usage is legitimate accounting evidence, but this particular null-terminal fixture is not producer-writable."},
    {"area": 5, "result": "sound", "basis": "contract/CONTRACT.md:96-179,426-429; contract/RESERVATION-BOUNDS.md:15-24; producer/pi_container.py.excerpts:984-1020,1038-1068. Exact exposure is explicitly conditional provisional accounting, not an invoice or a reservation-derived billing bound."},
    {"area": 6, "result": "not_established", "basis": "contract/MONEY.md:12-40,46-54; producer/openai_api_rates.json:5-13; producer/anthropic_api_rates.json:18-37; case:P130-R02-rounded-up-charge-admitted. Exact current-rate arithmetic and directional exposure treatment check out; the full-range floating-point tolerance proof was not independently established."},
    {"area": 7, "result": "defect", "basis": "contract/CONTRACT.md:233,252-253,481-497,574-575; case:N06-null-cache-counts; case:P133-A04-mixed-observations; case:S02-control-error-status-after-reservation. Cache enumeration and mixed priced-high retention work, but all-null usage classification and malformed-field reporting are underspecified or inconsistent with expectations."},
    {"area": 8, "result": "defect", "basis": "contract/CONTRACT.md:307,317,329-334,526-529; case:R04-reused-reservation-id; case:P133-A09-conflicting-reservations-with-settlement. Maximum-with-blocking is appropriate for correlated conflicting evidence; evaluation of all rejection conditions against multiple reservations is undefined."},
    {"area": 9, "result": "defect", "basis": "contract/CONTRACT.md:337-349; producer/pi_container.py.excerpts:1497-1502,1549-1570; case:S06-string-http-status; case:A06-reservation-usage-null-status-no-id. The path-specific status rules are usable, but several reachability labels contradict the writer."},
    {"area": 10, "result": "sound", "basis": "contract/CONTRACT.md:41-43,494,539-540,582,642; case:Ha-inconsistent-cache-split-keeps-supported-low. Money depends on evidence, not diagnostic presence or temporal placement; the inconsistent split changes completeness directly, not through its diagnostic."},
    {"area": 11, "result": "sound", "basis": "contract/CONTRACT.md:666-670,709,736-745; case:VC01-openai-usage-settlement; case:VC04-upstream-error-keeps-reservation. Known refusal totals use one exposure per owned attempt plus residual commitments, not reservation plus settlement. General predecessor graph integrity was not qualified here."},
    {"area": 12, "result": "sound", "basis": "corpus/sample-cases.jsonl:1-56; contract/CONTRACT.md:222-227. No sampled current_producer case attaches a gate; sampled future cases are explicitly future_vertical and producer_unreachable with empty historical comparisons. Synthetic labeling does not establish gate authentication."}
  ],
  "architectural_challenges": [
    {"challenge": "exposure_vs_billing", "result": "sound", "basis": "contract/CONTRACT.md:96-97,121-151,413-429,736-745 explicitly limits Billing::Established to the usage-valuation standard and never produces ProviderBilling. Downstream admission is conservative policy, not invoice verification. The name Billing::Established is hazardous, but the stated semantics do not establish final provider billing."},
    {"challenge": "authenticated_bounds", "result": "defect", "basis": "contract/CONTRACT.md:197-216 acknowledges future signature verification but supplies no verified-authority result or explicit failed/unverified qualification state. case:P133-FG-VC14-authenticated-bound and case:P132-FG-VC01-gated admit with only writer/authority assertions and receipt hashes. Their synthetic labels are correct; authentication is not evidenced."},
    {"challenge": "conflicting_reservations", "result": "sound", "basis": "contract/CONTRACT.md:307,317,526-529; producer/pi_container.py.excerpts:989-992; case:R04-reused-reservation-id; case:C3-identical-reservation-copy. A reused rid in one incarnation is contradictory correlated evidence, not proof of independent liabilities: max with AtLeast and blocking is appropriate. Distinct rids/sources remain separate and identical repeats do not add money. Future ProducerAttempt precedence remains outside this qualification."}
  ],
  "findings": [
    {
      "id": "A-01",
      "severity": "P1",
      "status": "verified",
      "kind": "contract",
      "class": "local",
      "location": "contract/CONTRACT.md:431-435",
      "title": "The unconditional error-status rule contradicts retention of supported usage",
      "counterexample": "case:P133-A02-error-status-with-priceable-usage reports R=800000000 nano, charge=0 and 20000 output tokens at 50000 nano/token. Section 6.2 prescribes low 0/high AtLeast(800000000), whereas section 8 correctly requires low 1000000000/high AtLeast(1000000000). Following the earlier normative rule lowers conservative exposure.",
      "smallest_correction": "Limit section 6.2's zero-low/R-high formula to attempts without supported usage; otherwise reference the section 8 maximum and supported-low rule."
    },
    {
      "id": "A-02",
      "severity": "P1",
      "status": "verified",
      "kind": "corpus",
      "class": "local",
      "location": "corpus/sample-cases.jsonl:32",
      "title": "Other-provider fixture releases a reservation despite an unpriced canonical identity",
      "counterexample": "case:N03-api-row-other-provider launches anthropic/claude-fable-5-2 but its request and response models belong to the row's explicitly OpenAI family under MC-2. OpenAI has no claude-fable-5-2 rate. V-1/V-6 therefore cannot establish the identity set's valuation bound. Expected Eligible and Exact(28000000) replace R=300000000; the correct financial disposition is ineligible with AtLeast at least 300000000. Merely adding LaneContradiction does not repair the money.",
      "smallest_correction": "Correct the expectation to preserve R and unpriceable identity evidence; specify how E selects a usage schema when provider evidence contradicts launch facts rather than relying on the ambiguity note."
    },
    {
      "id": "A-03",
      "severity": "P1",
      "status": "verified",
      "kind": "contract",
      "class": "local",
      "location": "contract/CONTRACT.md:205-216",
      "title": "Gate qualification lacks an explicit authenticated-evidence prerequisite",
      "counterexample": "case:P133-FG-VC14-authenticated-bound contains an authority id, writer='authority', receipt hashes and a binding digest, but no signature verification result or authenticated channel evidence. The enumerated checks nevertheless produce an Exact commitment and admission. A self-asserted writer value must not substitute for a trusted verification result. case:P132-FG-VC01-gated has the same problem for release.",
      "smallest_correction": "Require an adapter-produced, authority-bound verification result before qualification; unverified or failed verification must leave commitments unresolved. Give synthetic positive fixtures explicit mocked trusted-verification evidence. This blocks later gate integration, not the gate-free C-K slice."
    },
    {
      "id": "A-04",
      "severity": "P2",
      "status": "verified",
      "kind": "contract",
      "class": "local",
      "location": "contract/CONTRACT.md:384-394",
      "title": "Replacement-dependent amount_source construction is not reconciled with stage interfaces",
      "counterexample": "case:VC01-openai-usage-settlement needs SettlementAmount, while case:U08-missing-observed-tier needs ReservationBound despite both having Valid(Usage). Section 6 makes S construct this distinction, but TRANSITIONS.md:25 excludes the valuation range from S and section 1 forbids reading V. Also, Outstanding is a unit variant at CONTRACT.md:376 although amount_source must be present for it.",
      "smallest_correction": "Specify a shared evidence-only usage-bound eligibility primitive available to S and V, or an explicit post-V financial finalization step; reconcile the Outstanding type and its serialized amount_source."
    },
    {
      "id": "A-05",
      "severity": "P2",
      "status": "verified",
      "kind": "contract",
      "class": "local",
      "location": "contract/CONTRACT.md:481-497",
      "title": "All-null and partially reported required usage counts lack a complete state mapping",
      "counterexample": "case:S02-control-error-status-after-reservation contains a present usage object with null input/output/cache counts and reasoning_tokens=0, yet expects NotReported. E-2 preserves those fields and NotReported is described only as no usage on the rows. The tables do not explicitly distinguish this producer placeholder from incomplete or unpriceable reported usage.",
      "smallest_correction": "Define usage-presence and count-state rules for absent/null/non-object usage and absent/null/malformed required counts, including mixed observations containing these states; encode required counts with explicit Field types."
    },
    {
      "id": "A-06",
      "severity": "P2",
      "status": "verified",
      "kind": "corpus",
      "class": "local",
      "location": "corpus/sample-cases.jsonl:4,19,41",
      "title": "Several null-terminal OpenAI fixtures contradict the deployed writer",
      "counterexample": "With terminal=null, producer/pi_container.py.excerpts:1549-1567 derives response={} and logs null model, tier and usage counts. VC03 nevertheless has non-null usage; S02-control has a response model and observed tier; P133-A02 has 20000 output tokens. The first two claim producer_reachable and the third says not_established. These exact rows are producer_unreachable. A non-200 response carrying a terminal SSE event can instead produce an error-status settlement with usage because parsing at :1533-1543 is not restricted to status 200.",
      "smallest_correction": "Correct exact-row reachability labels; retain clearly synthetic probes and add a writer-consistent error-status-with-terminal-usage control. Do not infer non-billing from either shape."
    },
    {
      "id": "A-07",
      "severity": "P2",
      "status": "verified",
      "kind": "corpus",
      "class": "local",
      "location": "corpus/sample-cases.jsonl:40",
      "title": "The incomplete-usage-above-reservation positive reachability claim is impossible",
      "counterexample": "case:P133-A01-incomplete-usage-above-reservation logs input_bound=6000 and R=$0.01 for gpt-6-astra. reserve_usd at producer/pi_container.py.excerpts:689-694 reserves at least 6000*12.5/1000000*0.5=$0.0375 before any output allowance. The financial reducer can process the synthetic row, but release 92a03789 cannot emit that reservation under the supplied rates.",
      "smallest_correction": "Mark the case producer_unreachable/adversarial_synthetic or construct a reservation and served-tier/usage combination that the reserve formula can actually produce."
    },
    {
      "id": "A-08",
      "severity": "P2",
      "status": "verified",
      "kind": "corpus",
      "class": "local",
      "location": "corpus/sample-cases.jsonl:45",
      "title": "Malformed cache evidence is not classified as required",
      "counterexample": "case:P133-A04-cached-malformed has cached_tokens='400', a Valid(Reservation) settlement and Unpriceable(MalformedCount). CONTRACT.md:574-575 requires MalformedEvidence for a malformed field not covered by settlement rejection. Expected blocking contains only UnresolvedExposure, although no settlement rejection covers this field.",
      "smallest_correction": "Add MalformedEvidence(cached_tokens), or explicitly narrow the normative classification rule if valuation-level malformed fields are intentionally excluded."
    },
    {
      "id": "A-09",
      "severity": "P2",
      "status": "verified",
      "kind": "contract",
      "class": "local",
      "location": "contract/CONTRACT.md:329-334",
      "title": "All-reasons rejection is undefined when reservation evidence conflicts",
      "counterexample": "case:P133-A09-conflicting-reservations-with-settlement has reservation amounts 500000000 and 300000000, and settlement.reserved=500000000. ReservationConflict is certain, but condition 4's equality with 'the reservation's amount' has no unique referent. Expected reasons contain only ReservationConflict; checking every reservation would additionally produce ReservedMismatch.",
      "smallest_correction": "Define which checks are inapplicable without a unique reservation, or define universal comparisons explicitly. Do not select the largest reservation as if it were the uniquely authenticated reservation."
    }
  ],
  "derivations": [
    {"case": "VC01-openai-usage-settlement", "agrees": true, "derived": "Valid(Usage); Closed/SettlementAmount; Eligible; V=1000*10000+100*50000=15000000/15000000 nano, complete; X=15000000/Exact(15000000), usage basis; corpus/sample-cases.jsonl:1, producer/openai_api_rates.json:8.", "disagreement": ""},
    {"case": "VC02-anthropic-usage-settlement", "agrees": true, "derived": "Valid(Usage); Closed/SettlementAmount; Eligible; V=1000*8000+500*40000=28000000/28000000 nano, complete, no writes to split; X=28000000/Exact(28000000), usage basis; corpus/sample-cases.jsonl:2, producer/anthropic_api_rates.json:18-23.", "disagreement": ""},
    {"case": "P130-R02-rounded-up-charge-admitted", "agrees": true, "derived": "Valid(Usage); Closed/SettlementAmount; Eligible; V=21*50*0.5=525/525 nano, complete; logged 1e-06 USD=1000 nano; X=525/Exact(1000), high ProxyCharge; corpus/sample-cases.jsonl:3, producer/openai_api_rates.json:5,10.", "disagreement": ""},
    {"case": "VC04-upstream-error-keeps-reservation", "agrees": true, "derived": "Valid(UpstreamError): absent status accepted and charge=R=300000000; Closed/ReservationConfirmed; Ineligible(NotUsageSettlement); V=NotReported; X=0/AtLeast(300000000), reservation basis; corpus/sample-cases.jsonl:5, producer/pi_container.py.excerpts:1497-1502.", "disagreement": ""},
    {"case": "S02-control-error-status-after-reservation", "agrees": false, "derived": "Valid(ErrorStatus): integer 429 and zero charge; Closed/ReservationBound; Ineligible(NotUsageSettlement); V state not determinable for the all-null placeholder object; no supported numeric usage; X=0/AtLeast(800000000), reservation basis; corpus/sample-cases.jsonl:19.", "disagreement": "Money agrees, but NotReported lacks an explicit placeholder mapping; the exact null-terminal row with non-null response_model/observed_tier is producer_unreachable, not producer_reachable."},
    {"case": "P133-A02-error-status-with-priceable-usage", "agrees": false, "derived": "Valid(ErrorStatus); Closed/ReservationBound; Ineligible(NotUsageSettlement); V=20000*50000=1000000000/1000000000 nano, incomplete; section 8 gives X=1000000000/AtLeast(max(800000000,1000000000)) with usage basis; corpus/sample-cases.jsonl:41.", "disagreement": "Expected money follows section 8, but section 6.2 contradicts it with 0/AtLeast(800000000); the exact null-terminal usage row is producer_unreachable rather than reachability not_established. It is correctly labeled synthetic."},
    {"case": "U08-missing-observed-tier", "agrees": true, "derived": "Valid(Usage): producer consistency uses max multiplier 2, price=30000000; Closed/ReservationBound; Ineligible(UsageUnpriceable); V=Unpriceable(TierUnestablished), supported none/priced_high 30000000; X=0/AtLeast(500000000), reservation basis; corpus/sample-cases.jsonl:28.", "disagreement": ""},
    {"case": "P130-R01-usage-incomplete-terminal", "agrees": true, "derived": "Valid(Usage): response.incomplete still supplies producer-priced usage; Closed/ReservationBound; Ineligible(UsageIncomplete); V=15000000/15000000 nano, incomplete; X=15000000/AtLeast(500000000), reservation basis; corpus/sample-cases.jsonl:26, producer/pi_container.py.excerpts:1542-1570.", "disagreement": ""},
    {"case": "P133-A01-incomplete-usage-above-reservation", "agrees": false, "derived": "Valid(Usage) under section 5; Closed/ReservationBound; Ineligible(UsageIncomplete); V=15000000/15000000 nano, incomplete; X=15000000/AtLeast(max(10000000,15000000))=AtLeast(15000000), usage basis on tie; corpus/sample-cases.jsonl:40.", "disagreement": "Attempt money agrees; producer_reachable is false because the logged 6000-token input bound alone requires a reservation of at least 37500000 nano."},
    {"case": "P2-usage-settlement-unpriced-identity-keeps-charge", "agrees": true, "derived": "Valid(Usage) against pinned astra: 12000*50000=600000000 nano; Closed/ReservationBound; Ineligible(UsageUnpriceable); V=Unpriceable(IdentityWithoutRate), supported none/priced_high 600000000; X=0/AtLeast(800000000), reservation basis; corpus/sample-cases.jsonl:30.", "disagreement": ""},
    {"case": "N01-identity-refusal-dearer-served", "agrees": true, "derived": "RS-3 refusal adds identity without changing exact rid join; Valid(Reservation), charge=R=50000000; Closed/ReservationConfirmed; Ineligible; V=10000*2000 to 10000*8000=20000000/80000000 nano, incomplete; X=20000000/AtLeast(80000000), usage basis; corpus/sample-cases.jsonl:37.", "disagreement": ""},
    {"case": "Ha-inconsistent-cache-split-keeps-supported-low", "agrees": true, "derived": "Valid(Reservation), charge=R=300000000; Closed/ReservationConfirmed; Ineligible; reported split 1000+500 exceeds writes 1000, so V=1000*8000+1000*(10000..16000)=18000000/24000000 nano, incomplete; X=18000000/AtLeast(300000000); corpus/sample-cases.jsonl:34.", "disagreement": ""},
    {"case": "N06-null-cache-counts", "agrees": true, "derived": "Valid(Usage), producer defaults null cache counts only for its 15000000-nano consistency price; Closed/SettlementAmount; Eligible; feasible vertices give V=6000000/17500000 nano, complete; X=6000000/Exact(17500000), usage basis; corpus/sample-cases.jsonl:36.", "disagreement": ""},
    {"case": "P133-A04-cache-write-absent-legacy", "agrees": true, "derived": "NotApplicable; None(Unbudgeted); Ineligible(NoReservation); cached=0 fixed, write ranges 0..1000, yielding V=15000000/17500000 nano, complete; X=15000000/Exact(17500000), usage basis; corpus/sample-cases.jsonl:42.", "disagreement": ""},
    {"case": "P133-A04-cache-counts-both-absent", "agrees": false, "derived": "NotApplicable; None(Unbudgeted); Ineligible(NoReservation); vertices (cached,write)=(1000,0),(0,0),(0,1000) give V=6000000/17500000 nano, complete; X=6000000/Exact(17500000), usage basis; corpus/sample-cases.jsonl:43.", "disagreement": "Attempt money agrees; exact-row reachability is producer_unreachable for release 92a03789, whose request writer always emits both cache keys, rather than not_established."},
    {"case": "P133-A04-cached-malformed", "agrees": false, "derived": "Valid(Reservation), charge=R=500000000; Closed/ReservationConfirmed; Ineligible(NotUsageSettlement); V=Unpriceable(MalformedCount), no supported low or priced high; X=0/AtLeast(500000000), reservation basis; corpus/sample-cases.jsonl:45.", "disagreement": "Money agrees; expected blocking omits MalformedEvidence required by CONTRACT.md:574-575. The writer can preserve a nonempty string cache count and take its reservation fallback; source-level writability is established."},
    {"case": "P133-A04-mixed-observations", "agrees": true, "derived": "Two distinct settlements produce Conflicting; Outstanding/ReservationBound; Ineligible; first observation V=15000000, second malformed, so V-8 gives Unpriceable with supported none/priced_high 15000000; X=0/AtLeast(500000000), reservation basis; corpus/sample-cases.jsonl:46.", "disagreement": ""},
    {"case": "P133-A05-model-case-variant", "agrees": false, "derived": "Rejected(ModelMismatch): GPT-6-Astra differs bytewise from pinned gpt-6-astra; Outstanding/ReservationBound; Ineligible; V=Unpriceable(IdentityWithoutRate), supported none/priced_high 15000000; X=0/AtLeast(500000000), reservation basis; corpus/sample-cases.jsonl:47.", "disagreement": "Attempt money agrees; the exact request model is producer_unreachable, not merely not_established, because check_request rejects it before reservation/forwarding (producer/pi_container.py.excerpts:533-534,1392-1395)."},
    {"case": "S06-string-http-status", "agrees": true, "derived": "Rejected(StatusType,Undercharge): string status invalid and zero charge below 15000000-501; Outstanding/ReservationBound; Ineligible; V=15000000/15000000 nano, incomplete because status is not integer 200; X=0/AtLeast(800000000), reservation basis; corpus/sample-cases.jsonl:16.", "disagreement": ""},
    {"case": "A06-reservation-usage-null-status-no-id", "agrees": true, "derived": "Rejected(StatusType), despite charge=R=250000000; Outstanding/ReservationBound; Ineligible; V=15000000/15000000 nano, incomplete; rejected exposure X=0/AtLeast(250000000), reservation basis; producer_unreachable label is correct; corpus/sample-cases.jsonl:21.", "disagreement": ""},
    {"case": "P133-A06-usage-settlement-status-absent", "agrees": true, "derived": "Rejected(StatusType), not Undercharge because charge=15000000; Outstanding/ReservationBound; Ineligible; V=15000000/15000000 nano, incomplete; X=0/AtLeast(500000000), reservation basis; producer_unreachable label is correct; corpus/sample-cases.jsonl:48.", "disagreement": ""},
    {"case": "P133-A06-error-status-null", "agrees": false, "derived": "Rejected(StatusType), zero charge does not cure absent integer status; Outstanding/ReservationBound; Ineligible; V state not determinable for the all-null placeholder object, no supported numeric usage; X=0/AtLeast(800000000), reservation basis; corpus/sample-cases.jsonl:49.", "disagreement": "Settlement and exposure agree; expected NotReported requires the missing all-null usage mapping. The producer_unreachable label is correct."},
    {"case": "R04-reused-reservation-id", "agrees": true, "derived": "One correlated rid group, conflicting reservations 800000000 and 100000000 nano; Absent; Outstanding/ReservationBound; Ineligible; V=NotReported; X=0/AtLeast(max(800000000,100000000))=AtLeast(800000000), reservation basis, ReservationConflict blocking; corpus/sample-cases.jsonl:8.", "disagreement": ""},
    {"case": "P133-A09-conflicting-reservations-with-settlement", "agrees": false, "derived": "Rejected with at least ReservationConflict; Outstanding/ReservationBound; Ineligible; V=15000000/15000000 nano, complete; X=0/AtLeast(max(500000000,300000000,15000000))=AtLeast(500000000), reservation basis; corpus/sample-cases.jsonl:50.", "disagreement": "Money agrees, but the exact rejection-reason set is not determinable: condition 4 does not specify comparison against conflicting reservation amounts, so expected ReservationConflict alone is not uniquely derived."}
  ],
  "s2a_blockers": {
    "financial_core": [
      "Resolve A-01's contradictory error-status exposure formulas before implementing a normative reducer.",
      "Correct the unsafe other-provider expectation and define contradictory-provider usage adaptation; it lies inside the proposed API/reservation grading subset (A-02).",
      "Specify replacement eligibility's shared computation and the complete Obligation/amount_source representation without violating stage dependencies (A-04).",
      "Define all-null/partial usage state mapping and applicability of rejection checks when reservations conflict; otherwise exact outputs require implementer policy choices (A-05, A-09).",
      "Reconcile malformed-evidence classification and producer-reachability metadata with the raw writer before treating the subset as an oracle (A-06 through A-08 and derivations).",
      "The review, freeze, owner-question disposition, crate location and authorization gates in S2A-ENTRY-CRITERIA.md:20-31 are not established by this review set; other modules' results are unavailable."
    ],
    "later_integration": [
      "Require authenticated authority verification before gate-based finite commitments or release; hashes and writer strings cannot qualify a gate (A-03).",
      "Specify how a provider-billing bound is translated into the provisional accounting commitment measure, including contradictory observed exposure above the asserted bound.",
      "Define future ProducerAttempt identity precedence and deduplication relative to legacy rid/source keys before admitting future attempt-id evidence.",
      "Qualify predecessor graph integrity, commitment ownership and checked aggregate arithmetic with Module B; this review establishes only the stated financial formula, not all integration combinations."
    ]
  },
  "minimum_required_revisions": [
    "Make section 6.2 agree with section 8 for supported error-status usage.",
    "Fix N03-api-row-other-provider so unresolved canonical pricing identity cannot release its reservation.",
    "Specify the eligibility primitive, Outstanding amount_source shape, usage-presence mapping and multi-reservation rejection applicability.",
    "Correct the identified corpus classification and reachability discrepancies; preserve separate synthetic and producer-writable controls.",
    "Before gate integration, add a trusted verification-result prerequisite and explicit synthetic verification fixtures.",
    "Supply a rigorous producer-consistency tolerance argument covering the actual pricing operations and accepted input domain rather than relying on an approximate operation count."
  ],
  "inadequately_examined": [
    "All 14 file terminators and the final review-set marker were present; hashes and advertised byte lengths were not independently recomputed.",
    "The 117 unsampled corpus cases, validator implementation/results and historical reader implementation were not available or verified.",
    "The full-range floating-point tolerance proof and arithmetic-failure output behavior were not exhaustively qualified.",
    "Producer excerpts omit some parsing helpers and constants, so provider-origin plausibility and all historical writer variants were not established.",
    "Lifecycle, temporal candidate selection, predecessor graph behavior and admission-authority implementation were not independently qualified beyond their financial interfaces.",
    "Gate receipt authentication cannot be established from the supplied assertions and hashes."
  ]
}
```

The design’s central distinction is defensible: complete usage can justify replacing a reservation **within provisional accounting**, without proving invoice finality. Incomplete usage, missing tiers and unpriced identities generally preserve both the reservation floor and unresolved exposure. The requested OpenAI, Anthropic, cache-range and rounding calculations largely agree with the expected monetary amounts.

Nevertheless, approval would be premature. Two concrete problems affect the immediate financial core: the older error-status formula contradicts the newer usage-preserving rule, and the other-provider fixture expects a finite replacement despite an unpriced canonical identity. A correct implementation cannot satisfy both conflicting normative directions or safely reproduce that expectation.

Several smaller gaps also require decisions before exact-output Rust implementation: who computes replacement eligibility, how an outstanding obligation carries `amount_source`, how all-null usage becomes `NotReported`, and which rejection checks apply without a unique reservation. Producer reachability must be derived from the writer, not fixture labels; several supposedly reachable rows cannot be emitted as written.

Gate authentication is a separate, later integration blocker. Delegating verification to stage E is reasonable, but qualification must consume an explicitly trusted verification result. The synthetic positives currently contain assertions and hashes, not that result.

These defects warrant **REJECT / NOT_READY**, but not wholesale redesign. Local corrections can preserve the existing financial measure and stage separation. No conclusion here qualifies unsampled cases, establishes provider billing, or substitutes for Modules B and C.