import { createHash } from "node:crypto";

export const EXTERNAL_BOOTSTRAP_PROVENANCE = "external_bootstrap_wind_walker";
export const EXTERNAL_BOOTSTRAP_PACKET_KIND = "work-engine.external-bootstrap-evidence-packet";

const requireRecord = (value, label) => {
  if (!value || Array.isArray(value) || typeof value !== "object") throw new TypeError(label + " must be an object");
  return value;
};
const requireText = (value, label) => {
  if (typeof value !== "string" || value.trim() === "") throw new TypeError(label + " must be a non-empty string");
  return value;
};
const requireSha256 = (value, label) => {
  requireText(value, label);
  if (!/^[0-9a-f]{64}$/.test(value)) throw new TypeError(label + " must be a SHA-256 digest");
  return value;
};
const canonicalJson = (value) => {
  if (Array.isArray(value)) return "[" + value.map(canonicalJson).join(",") + "]";
  if (value && typeof value === "object") return "{" + Object.keys(value).sort().map(
    (key) => JSON.stringify(key) + ":" + canonicalJson(value[key]),
  ).join(",") + "}";
  return JSON.stringify(value);
};
const freeze = (value) => {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
};
const normalizeIdentity = (value) => {
  requireRecord(value, "slice identity");
  const identity = {runId: requireText(value.runId, "slice identity runId"),
    sliceNumber: value.sliceNumber, attemptId: requireText(value.attemptId, "slice identity attemptId"),
    planVersion: requireText(value.planVersion, "slice identity planVersion")};
  if (!Number.isInteger(identity.sliceNumber) || identity.sliceNumber < 1) {
    throw new TypeError("slice identity sliceNumber must be a positive integer");
  }
  return freeze(identity);
};

const exact = (value, fields, label) => {
  requireRecord(value, label);
  const expected = new Set(fields);
  if (Object.keys(value).some((field) => !expected.has(field))
      || fields.some((field) => !Object.hasOwn(value, field))) {
    throw new TypeError(label + " fields are invalid");
  }
  return value;
};

const packetDigest = (packet) => createHash("sha256").update(canonicalJson(packet)).digest("hex");

export function validateExternalBootstrapPacket(value) {
  const packet = structuredClone(value);
  exact(packet, ["schema_version", "kind", "provenance", "identity",
    "expected_campaign_revision", "objective_boundary_baseline", "task_manifest",
    "checkpoint", "gate", "incident_refs", "limitations", "whole_packet_sha256"],
  "external bootstrap packet");
  if (packet.schema_version !== 1 || packet.kind !== EXTERNAL_BOOTSTRAP_PACKET_KIND) {
    throw new TypeError("external bootstrap packet schema or kind is invalid");
  }
  if (packet.provenance !== EXTERNAL_BOOTSTRAP_PROVENANCE) {
    throw new TypeError("external bootstrap packet provenance is invalid");
  }
  packet.identity = normalizeIdentity(packet.identity);
  requireSha256(packet.expected_campaign_revision, "external bootstrap expected campaign revision");
  exact(packet.objective_boundary_baseline, ["objective", "accepted_boundary", "baseline"],
    "external bootstrap objective boundary baseline");
  requireText(packet.objective_boundary_baseline.objective, "external bootstrap objective");
  exact(packet.objective_boundary_baseline.accepted_boundary, ["reference", "sha256"],
    "external bootstrap accepted boundary");
  requireText(packet.objective_boundary_baseline.accepted_boundary.reference, "external bootstrap accepted boundary reference");
  requireSha256(packet.objective_boundary_baseline.accepted_boundary.sha256, "external bootstrap accepted boundary digest");
  exact(packet.objective_boundary_baseline.baseline, ["commit", "tree"], "external bootstrap baseline");
  for (const field of ["commit", "tree"]) requireText(packet.objective_boundary_baseline.baseline[field], "external bootstrap baseline " + field);
  if (!Array.isArray(packet.task_manifest) || packet.task_manifest.length === 0) throw new TypeError("external bootstrap task manifest must be nonempty");
  packet.task_manifest.forEach((item, index) => {
    exact(item, ["path", "action"], "external bootstrap task manifest[" + index + "]");
    requireText(item.path, "external bootstrap task manifest path");
    if (!["add", "modify", "delete"].includes(item.action)) throw new TypeError("external bootstrap task manifest action is invalid");
  });
  exact(packet.checkpoint, ["parent_commit", "commit", "tree", "ref", "task_patch_sha256"], "external bootstrap checkpoint");
  for (const field of ["parent_commit", "commit", "tree", "ref"]) requireText(packet.checkpoint[field], "external bootstrap checkpoint " + field);
  requireSha256(packet.checkpoint.task_patch_sha256, "external bootstrap task patch digest");
  exact(packet.gate, ["manifest_sha256", "receipt_sha256", "test_counts", "workspace_integrity_sha256"], "external bootstrap gate");
  for (const field of ["manifest_sha256", "receipt_sha256", "workspace_integrity_sha256"]) requireSha256(packet.gate[field], "external bootstrap gate " + field);
  exact(packet.gate.test_counts, ["passed", "failed"], "external bootstrap test counts");
  for (const field of ["passed", "failed"]) {
    if (!Number.isSafeInteger(packet.gate.test_counts[field]) || packet.gate.test_counts[field] < 0) throw new TypeError("external bootstrap test counts are invalid");
  }
  requireRecord(packet.incident_refs, "external bootstrap incident refs");
  if (Object.keys(packet.incident_refs).length === 0
      || Object.values(packet.incident_refs).some((item) => typeof item !== "string" || !item.trim())) {
    throw new TypeError("external bootstrap incident refs must be nonempty strings");
  }
  const requiredLimitations = ["app_server_lifecycle_proof_unavailable",
    "app_server_mailbox_proof_unavailable", "app_server_delivery_output_proof_unavailable"];
  if (!Array.isArray(packet.limitations)
      || packet.limitations.length !== requiredLimitations.length
      || requiredLimitations.some((item) => !packet.limitations.includes(item))) {
    throw new TypeError("external bootstrap packet must explicitly preserve unavailable App Server proof limitations");
  }
  requireSha256(packet.whole_packet_sha256, "external bootstrap whole packet digest");
  const {whole_packet_sha256: claimed, ...unsigned} = packet;
  if (packetDigest(unsigned) !== claimed) throw new Error("external bootstrap whole-packet integrity failed");
  return freeze(packet);
}

export function buildExternalBootstrapPacket(input) {
  const unsigned = structuredClone(input);
  return validateExternalBootstrapPacket({...unsigned, whole_packet_sha256: packetDigest(unsigned)});
}
