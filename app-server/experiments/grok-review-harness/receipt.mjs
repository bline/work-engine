import { createHash } from "node:crypto";

import { readPacketEvidence } from "./packet.mjs";

export class ReviewReceiptError extends Error {}
export const GrokReviewReceiptError = ReviewReceiptError;

const VERDICTS = new Set(["acceptable_as_is", "remediation_required", "incomplete"]);
const SEVERITIES = new Set(["blocker", "high", "medium", "low", "info"]);
const CONFIDENCE = new Set(["high", "medium", "low"]);
const BASIS = new Set(["reproduced", "inferred"]);

function object(value, field) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new ReviewReceiptError(`${field} must be an object`);
  }
  return value;
}

function string(value, field) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new ReviewReceiptError(`${field} must be a non-empty string`);
  }
  return value;
}

function array(value, field) {
  if (!Array.isArray(value)) throw new ReviewReceiptError(`${field} must be an array`);
  return value;
}

function exactJson(text, field) {
  try {
    return JSON.parse(text.trim());
  } catch (error) {
    throw new ReviewReceiptError(`${field} is not one JSON value: ${error.message}`);
  }
}

export function parseGrokCliEnvelope(stdout) {
  const envelope = object(exactJson(stdout, "Grok CLI output"), "Grok CLI output");
  if (envelope.type === "error") {
    throw new GrokReviewReceiptError(`Grok CLI returned an error: ${envelope.message ?? "unknown error"}`);
  }
  string(envelope.sessionId, "Grok CLI sessionId");
  if (envelope.stopReason !== "end_turn") {
    throw new GrokReviewReceiptError(`Grok CLI stopReason must be end_turn, got ${envelope.stopReason}`);
  }
  string(envelope.text, "Grok CLI text");
  return envelope;
}

function citation(value, field, files) {
  const item = object(value, field);
  const filePath = string(item.path, `${field}.path`);
  const known = files.get(filePath);
  if (!known) throw new GrokReviewReceiptError(`${field}.path is not in the packet manifest: ${filePath}`);
  if (!Number.isInteger(item.start_line) || !Number.isInteger(item.end_line)) {
    throw new GrokReviewReceiptError(`${field} line bounds must be integers`);
  }
  if (item.start_line < 1 || item.end_line < item.start_line || item.end_line > known.line_count) {
    throw new GrokReviewReceiptError(`${field} line bounds exceed ${filePath}`);
  }
  return { path: filePath, start_line: item.start_line, end_line: item.end_line };
}

async function bindCitation(packet, item) {
  const text = await readPacketEvidence(packet.root, item.path);
  const lines = text.split("\n");
  const selected = lines.slice(item.start_line - 1, item.end_line).join("\n");
  return {
    ...item,
    evidence_sha256: createHash("sha256").update(selected).digest("hex"),
  };
}

export async function admitReview({
  packet,
  response,
  provider,
  sessionId,
  transportUsage = null,
}) {
  const result = typeof response === "string"
    ? object(exactJson(response, "reviewer response"), "reviewer response")
    : object(response, "reviewer response");
  const subject = object(result.subject, "subject");
  if (subject.commit !== packet.manifest.subject.commit
    || subject.tree !== packet.manifest.subject.tree
    || subject.patch_identity !== packet.manifest.subject.patch_identity) {
    throw new GrokReviewReceiptError("reviewer subject does not match the packet manifest");
  }
  if (!VERDICTS.has(result.verdict)) throw new GrokReviewReceiptError("verdict is not recognized");

  const files = new Map(packet.manifest.files.map((item) => [item.path, item]));
  const findings = [];
  for (const [index, rawFinding] of array(result.findings, "findings").entries()) {
    const field = `findings[${index}]`;
    const finding = object(rawFinding, field);
    string(finding.id, `${field}.id`);
    string(finding.title, `${field}.title`);
    if (!SEVERITIES.has(finding.severity)) throw new GrokReviewReceiptError(`${field}.severity is not recognized`);
    if (!BASIS.has(finding.reproduced_or_inferred)) {
      throw new GrokReviewReceiptError(`${field}.reproduced_or_inferred is not recognized`);
    }
    if (!CONFIDENCE.has(finding.confidence)) {
      throw new GrokReviewReceiptError(`${field}.confidence is not recognized`);
    }
    for (const key of ["observed", "violated_expectation", "consequence", "recommended_remediation"]) {
      string(finding[key], `${field}.${key}`);
    }
    const evidence = array(finding.evidence, `${field}.evidence`)
      .map((item, evidenceIndex) => citation(item, `${field}.evidence[${evidenceIndex}]`, files));
    if (evidence.length === 0) throw new GrokReviewReceiptError(`${field} has no evidence`);
    findings.push({
      ...finding,
      evidence: await Promise.all(evidence.map((item) => bindCitation(packet, item))),
    });
  }

  if (result.verdict === "remediation_required" && findings.length === 0) {
    throw new GrokReviewReceiptError("remediation_required requires at least one finding");
  }
  const decisiveEvidence = array(result.decisive_evidence, "decisive_evidence").map((raw, index) => {
    const item = object(raw, `decisive_evidence[${index}]`);
    return {
      ...citation(item, `decisive_evidence[${index}]`, files),
      reason: string(item.reason, `decisive_evidence[${index}].reason`),
    };
  });
  if (result.verdict === "acceptable_as_is" && decisiveEvidence.length === 0) {
    throw new GrokReviewReceiptError("acceptable_as_is requires decisive evidence");
  }

  const limitations = array(result.limitations, "limitations").map((item, index) => (
    string(item, `limitations[${index}]`)
  ));
  object(result.metrics, "metrics");

  return {
    schema_version: 1,
    provider: string(provider, "provider"),
    session_id: string(sessionId, "sessionId"),
    subject: { ...packet.manifest.subject },
    verdict: result.verdict,
    findings,
    decisive_evidence: await Promise.all(decisiveEvidence.map((item) => bindCitation(packet, item))),
    limitations,
    provider_metrics: result.metrics,
    transport_usage: transportUsage,
  };
}

export async function admitGrokReview({ packet, envelope }) {
  return admitReview({
    packet,
    response: envelope.text,
    provider: "grok-cli",
    sessionId: envelope.sessionId,
    transportUsage: envelope.usage ?? null,
  });
}
