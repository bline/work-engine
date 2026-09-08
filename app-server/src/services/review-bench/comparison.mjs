import { requireBench } from "./contract.mjs";

const SEVERITY_RANK = { low: 1, medium: 2, high: 3, critical: 4 };
const RESOURCES = ["wall_clock_seconds", "cost_usd", "input_tokens", "output_tokens"];
const round6 = (value) => Math.round(value * 1_000_000) / 1_000_000;
const ratio = (numerator, denominator) => denominator ? round6(numerator / denominator) : null;
const sortedObject = (entries) => Object.fromEntries([...entries].sort(([left], [right]) => left.localeCompare(right)));

export function reviewBenchConfigurationKey(result) {
  const reviewer = result.reviewer;
  return [reviewer.provider, reviewer.model, reviewer.harness, reviewer.reasoning_effort,
    reviewer.review_protocol, String(reviewer.pass_count), reviewer.aggregation_strategy,
    [...reviewer.aggregation_members].sort().join(","), [...reviewer.tool_access].sort().join(",")].join("|");
}

function newAggregate(result) {
  const reviewer = result.reviewer;
  return {
    provider: reviewer.provider, model: reviewer.model, harness: reviewer.harness,
    inference_family: reviewer.inference_family, reasoning_effort: reviewer.reasoning_effort,
    review_protocol: reviewer.review_protocol, tool_access: [...reviewer.tool_access].sort(),
    pass_count: reviewer.pass_count, aggregation_strategy: reviewer.aggregation_strategy,
    aggregation_members: [...reviewer.aggregation_members].sort(), attempts: 0, cases: new Set(),
    truth_opportunities: 0, truth_detected: 0, blocking_opportunities: 0, blocking_detected: 0,
    reported_findings: 0, verified_claims: 0, reviewer_observations: 0,
    true_positive_findings: 0, false_positive_findings: 0, duplicate_findings: 0,
    nonblocking_observation_findings: 0, evidence_valid_findings: 0,
    severity_overstatements: 0, severity_understatements: 0,
    blocking_label_overstatements: 0, blocking_label_understatements: 0,
    false_accepts: 0, false_blocks: 0, blocked_unverified: 0,
    resource_totals: Object.create(null), resource_observations: Object.create(null),
  };
}

function updateAggregate(aggregate, result, scoring, truthFindings, expectedVerdict) {
  const detected = new Set(); aggregate.attempts += 1; aggregate.cases.add(result.case_id);
  aggregate.truth_opportunities += truthFindings.size;
  const blockingTruth = new Set([...truthFindings].filter(([, finding]) => finding.blocking).map(([id]) => id));
  aggregate.blocking_opportunities += blockingTruth.size; aggregate.reported_findings += result.findings.length;
  aggregate.verified_claims += (result.verified_claims ?? []).length; aggregate.reviewer_observations += (result.observations ?? []).length;
  const resultFindings = new Map(result.findings.map((finding) => [finding.finding_id, finding]));
  for (const disposition of scoring.finding_dispositions) {
    if (disposition.disposition === "true_positive") {
      aggregate.true_positive_findings += 1; disposition.truth_finding_ids.forEach((id) => detected.add(id));
      const reported = resultFindings.get(disposition.result_finding_id);
      const matched = disposition.truth_finding_ids.map((id) => truthFindings.get(id));
      const expectedSeverity = matched.reduce((best, finding) => SEVERITY_RANK[finding.severity] > SEVERITY_RANK[best.severity] ? finding : best).severity;
      if (SEVERITY_RANK[reported.severity] > SEVERITY_RANK[expectedSeverity]) aggregate.severity_overstatements += 1;
      else if (SEVERITY_RANK[reported.severity] < SEVERITY_RANK[expectedSeverity]) aggregate.severity_understatements += 1;
      const expectedBlocking = matched.some((finding) => finding.blocking);
      if (reported.blocking && !expectedBlocking) aggregate.blocking_label_overstatements += 1;
      else if (!reported.blocking && expectedBlocking) aggregate.blocking_label_understatements += 1;
    } else if (disposition.disposition === "false_positive") aggregate.false_positive_findings += 1;
    else if (disposition.disposition === "duplicate") aggregate.duplicate_findings += 1;
    else aggregate.nonblocking_observation_findings += 1;
    if (disposition.evidence_valid) aggregate.evidence_valid_findings += 1;
  }
  aggregate.truth_detected += detected.size;
  aggregate.blocking_detected += [...detected].filter((id) => blockingTruth.has(id)).length;
  if (expectedVerdict !== "accepted" && result.verdict === "accepted") aggregate.false_accepts += 1;
  if (expectedVerdict === "accepted" && result.verdict !== "accepted") aggregate.false_blocks += 1;
  if (result.verdict === "blocked_unverified") aggregate.blocked_unverified += 1;
  for (const resource of RESOURCES) if (resource in result.timing) {
    aggregate.resource_totals[resource] = (aggregate.resource_totals[resource] ?? 0) + result.timing[resource];
    aggregate.resource_observations[resource] = (aggregate.resource_observations[resource] ?? 0) + 1;
  }
  return detected;
}

function renderAggregate(aggregate) {
  const rendered = {};
  for (const [key, value] of Object.entries(aggregate)) if (!["cases", "resource_totals", "resource_observations"].includes(key)) rendered[key] = value;
  rendered.case_count = aggregate.cases.size; rendered.defect_recall = ratio(aggregate.truth_detected, aggregate.truth_opportunities);
  rendered.blocking_recall = ratio(aggregate.blocking_detected, aggregate.blocking_opportunities);
  rendered.finding_precision = ratio(aggregate.true_positive_findings, aggregate.true_positive_findings + aggregate.false_positive_findings);
  rendered.evidence_validity = ratio(aggregate.evidence_valid_findings, aggregate.reported_findings); rendered.resources = {};
  for (const resource of RESOURCES) { const observations=aggregate.resource_observations[resource]??0, total=aggregate.resource_totals[resource]??0; rendered.resources[resource]={observations,total:observations?total:null,average:observations?round6(total/observations):null}; }
  const detected=aggregate.truth_detected, attempts=aggregate.attempts;
  rendered.resource_efficiency = {
    cost_usd_per_truth_detected: detected && aggregate.resource_observations.cost_usd === attempts ? round6(aggregate.resource_totals.cost_usd / detected) : null,
    wall_clock_seconds_per_truth_detected: detected && aggregate.resource_observations.wall_clock_seconds === attempts ? round6(aggregate.resource_totals.wall_clock_seconds / detected) : null,
    total_tokens_per_truth_detected: detected && aggregate.resource_observations.input_tokens === attempts && aggregate.resource_observations.output_tokens === attempts ? round6((aggregate.resource_totals.input_tokens + aggregate.resource_totals.output_tokens) / detected) : null,
  };
  return rendered;
}

export function compareReviewBenchResults(corpus, truth, results, scorings) {
  const scoringByResult = new Map(scorings.map((scoring) => [scoring.result_id, scoring]));
  requireBench(scoringByResult.size === scorings.length, "duplicate scoring result_id");
  requireBench(results.length === scoringByResult.size && results.every((result) => scoringByResult.has(result.result_id)), "every result must have exactly one scoring artifact");
  const truthByCase=new Map(truth.cases.map((item)=>[item.case_id,item])), corpusByCase=new Map(corpus.cases.map((item)=>[item.case_id,item]));
  const aggregates=new Map(), routes=new Map(), detections=new Map(), falsePositives=new Map();
  for(const result of results){const key=reviewBenchConfigurationKey(result);if(!aggregates.has(key))aggregates.set(key,newAggregate(result));const scoring=scoringByResult.get(result.result_id), truthCase=truthByCase.get(result.case_id), truthFindings=new Map(truthCase.findings.map((x)=>[x.truth_id,x]));const detected=updateAggregate(aggregates.get(key),result,scoring,truthFindings,truthCase.expected_verdict);const route=corpusByCase.get(result.case_id).change_profile.route_class;if(!routes.has(route))routes.set(route,new Map());if(!routes.get(route).has(key))routes.get(route).set(key,newAggregate(result));updateAggregate(routes.get(route).get(key),result,scoring,truthFindings,truthCase.expected_verdict);if(!detections.has(key))detections.set(key,new Map());if(!detections.get(key).has(result.case_id))detections.get(key).set(result.case_id,new Set());detected.forEach((id)=>detections.get(key).get(result.case_id).add(id));if(!falsePositives.has(key))falsePositives.set(key,new Map());falsePositives.get(key).set(result.case_id,(falsePositives.get(key).get(result.case_id)??0)+scoring.finding_dispositions.filter((x)=>x.disposition==="false_positive").length);}
  const configurations=sortedObject([...aggregates].map(([key,value])=>[key,renderAggregate(value)])); const byRoute=sortedObject([...routes].map(([route,items])=>[route,sortedObject([...items].map(([key,value])=>[key,renderAggregate(value)]))]));
  const pairwise=[], keys=[...detections.keys()].sort(), allTruth=new Map(truth.cases.map((item)=>[item.case_id,new Set(item.findings.map((x)=>x.truth_id))]));
  for(let i=0;i<keys.length;i++)for(const right of keys.slice(i+1)){const left=keys[i], shared=[...detections.get(left).keys()].filter((id)=>detections.get(right).has(id)).sort();let lu=0,ru=0,joint=0,miss=0,opportunities=0,lfp=0,rfp=0;for(const caseId of shared){const l=detections.get(left).get(caseId),r=detections.get(right).get(caseId),truthIds=allTruth.get(caseId);opportunities+=truthIds.size;lu += [...l].filter((x)=>!r.has(x)).length;ru += [...r].filter((x)=>!l.has(x)).length;joint += [...l].filter((x)=>r.has(x)).length;miss += [...truthIds].filter((x)=>!l.has(x)&&!r.has(x)).length;lfp += falsePositives.get(left).get(caseId)??0;rfp += falsePositives.get(right).get(caseId)??0;}pairwise.push({left,right,shared_cases:shared.length,truth_unique_to_left:lu,truth_unique_to_right:ru,truth_jointly_detected:joint,truth_jointly_missed:miss,truth_opportunities:opportunities,union_recall:ratio(opportunities-miss,opportunities),joint_miss_rate:ratio(miss,opportunities),right_conditional_recall_given_left_miss:ratio(ru,ru+miss),left_conditional_recall_given_right_miss:ratio(lu,lu+miss),left_false_positive_findings:lfp,right_false_positive_findings:rfp,right_incremental_truth_per_additional_false_positive:ratio(ru,Math.max(0,rfp-lfp)),left_incremental_truth_per_additional_false_positive:ratio(lu,Math.max(0,lfp-rfp)),interpretation:"descriptive union across attempts; adjudicate before policy use"});}
  return {artifact_type:"review_bench_report_v1",schema_version:1,bench_id:corpus.bench_id,truth_artifact_type:truth.artifact_type,truth_adjudication_amendments:(truth.adjudication_amendments??[]).map((x)=>x.amendment_id),status:"descriptive_only",configurations,by_route_class:byRoute,pairwise_complementarity:pairwise,limitations:["Metrics are descriptive and do not establish statistical equivalence.","Repeated attempts within a case are correlated.","Pairwise detection uses the union across attempts for each configuration and case.","Ground truth and finding correspondence depend on recorded adjudication."]};
}
