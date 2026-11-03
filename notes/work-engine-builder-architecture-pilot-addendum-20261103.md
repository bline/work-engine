# Controlled pilot addendum to the architecture assessment

Recorded 2026-11-03 as a supplement to the
[development system architecture assessment](work-engine-builder-architecture-assessment-20261103.md).
The original assessment keeps its evidence cutoff and conclusions.

The prospective
[current host versus manual orchestration pilot](../metrics/probes/manual-host-controlled-20261103-prereg.md)
is prepared and registered in local commit
`72c05cf7323b961892e32eac6a500efc46a3b5a7`. It directly addresses the open
economic question: whether the deterministic outer host reduces root inference
cost while preserving the checks, evidence and judgment required of manual
orchestration.

- **Design:** twelve nominal episodes in six matched pairs, comparing the
  current host with manual use of the same underlying orchestration scripts.
  Each episode begins with a fresh supervisor. The paired tasks comprise two
  source reviews and four small experimental workloads.
- **Primary measure:** paired root API-equivalent inference cost, including
  startup, ordinary repairs and closure. Quality, total actor cost, root calls
  and elapsed time are also recorded. API-equivalent cost is a modeled cost
  proxy rather than an invoice or subscription-quota measurement.
- **Proposed budget:** $100–150 expected, with a $200 whole-study maximum,
  including a $20 preparation and qualification allocation. These amounts use
  the same modeled API-equivalent USD basis.
- **Preparation:** fixture compilation, Clippy, formatting and registration
  checks were reported passing. The
  [registration receipt](../metrics/probes/manual-host-controlled-20261103-registration.json)
  records successful specification readback, fixture compilation without
  running tasks, and zero native roles created.
- **Status at registration:** no paid trial episodes had started. The frozen
  protocol requires separate owner approval and successful qualification before
  allocation or episode entry. Registration checks do not establish runtime
  qualification. Later approval, qualification and execution remain with the
  pilot's owner and its separate decision records.

This is the prospective comparison missing from the earlier retrospective
observations. Its matched tasks and complete accounting can strengthen or
weaken the proposed cost benefit once outcomes exist. Its scope is a small
lifecycle-workload pilot; it cannot establish savings for long engineering
campaigns, the earlier 40 percent root-time hypothesis, or completion of the
Rust runtime integration.

The architecture assessment therefore retains its economic conclusion: there
is narrower evidence for less generated orchestration code, while end-to-end
net benefit remains unestablished. The pilot adds a concrete planned test of
that conclusion, with no observed savings implied by registration. Its eventual
results belong in a separate dated result record.
