import { createHash, sign, verify } from "node:crypto";

export const OBSERVED_CONTEXT_SCHEMA_VERSION = 1;
export const OBSERVED_CONTEXT_TYPE = "work-engine.observed-context";

const DIGEST = /^[a-f0-9]{64}$/;
const ORIGINS = new Set([
  "application",
  "assistant",
  "developer",
  "human",
  "retrieved_content",
  "skill",
  "system",
  "tool",
]);
const TRUST_CLASSES = new Set([
  "attributed_evidence",
  "governing_instruction",
  "human_authority_input",
  "model_output",
  "trusted_application_data",
  "untrusted_data",
]);
const INSTRUCTION_APPLICABILITY = new Set([
  "contract_defined",
  "governing",
  "none",
]);
const SOURCE_INVENTORY_COMPLETENESS = new Set(["complete", "partial", "unknown"]);
const MANDATORY_UNKNOWNS = Object.freeze([
  "exact_effective_model_input",
  "hidden_reasoning_state",
  "provider_internal_instructions",
]);
const SIGNATURE_DOMAIN = Buffer.from("WORK_ENGINE_OBSERVED_CONTEXT_V1\0", "utf8");

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function requireDigest(value, label) {
  requireText(value, label);
  if (!DIGEST.test(value)) throw new TypeError(`${label} must be lowercase SHA-256`);
  return value;
}

function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((key) => !allowed.includes(key)).sort();
  if (unknown.length > 0) {
    throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
  }
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`,
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function deepFreeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}

function normalizeJson(value, label) {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new TypeError(`${label} numbers must be non-negative safe integers`);
    }
    return value;
  }
  if (Array.isArray(value)) return value.map((item, index) => normalizeJson(item, `${label}[${index}]`));
  requireRecord(value, label);
  const normalized = {};
  for (const key of Object.keys(value).sort()) {
    normalized[key] = normalizeJson(value[key], `${label}.${key}`);
  }
  return normalized;
}

function normalizeContentRef(value, label) {
  requireRecord(value, label);
  rejectUnknown(value, ["kind", "reference", "sha256"], label);
  return {
    kind: requireText(value.kind, `${label} kind`),
    reference: requireText(value.reference, `${label} reference`),
    sha256: requireDigest(value.sha256, `${label} sha256`),
  };
}

function normalizeVisibleItem(value, index) {
  const label = `visibleItems[${index}]`;
  requireRecord(value, label);
  rejectUnknown(value, [
    "identity", "origin", "trustClass", "instructionApplicability",
    "contentRef", "producer",
  ], label);
  requireText(value.identity, `${label} identity`);
  if (!ORIGINS.has(value.origin)) throw new TypeError(`${label} origin is unsupported`);
  if (!TRUST_CLASSES.has(value.trustClass)) {
    throw new TypeError(`${label} trustClass is unsupported`);
  }
  if (!INSTRUCTION_APPLICABILITY.has(value.instructionApplicability)) {
    throw new TypeError(`${label} instructionApplicability is unsupported`);
  }
  if ((value.trustClass === "untrusted_data"
      || ["assistant", "retrieved_content", "tool"].includes(value.origin))
      && value.instructionApplicability !== "none") {
    throw new TypeError(`${label} untrusted or produced content cannot govern instructions`);
  }
  if ((value.trustClass === "governing_instruction")
      !== (value.instructionApplicability === "governing")) {
    throw new TypeError(`${label} governing trust and applicability must agree`);
  }
  return {
    identity: value.identity,
    origin: value.origin,
    trustClass: value.trustClass,
    instructionApplicability: value.instructionApplicability,
    contentRef: normalizeContentRef(value.contentRef, `${label} contentRef`),
    producer: value.producer == null ? null : requireText(value.producer, `${label} producer`),
  };
}

function normalizeGoverningSource(value, index) {
  const label = `governingSources[${index}]`;
  requireRecord(value, label);
  rejectUnknown(value, ["identity", "owner", "contentRef"], label);
  return {
    identity: requireText(value.identity, `${label} identity`),
    owner: requireText(value.owner, `${label} owner`),
    contentRef: normalizeContentRef(value.contentRef, `${label} contentRef`),
  };
}

function normalizeSkill(value, index) {
  const label = `activatedSkills[${index}]`;
  requireRecord(value, label);
  rejectUnknown(value, ["name", "path", "sha256"], label);
  return {
    name: requireText(value.name, `${label} name`),
    path: requireText(value.path, `${label} path`),
    sha256: requireDigest(value.sha256, `${label} sha256`),
  };
}

function normalizeOmission(value, index) {
  const label = `omissions[${index}]`;
  requireRecord(value, label);
  rejectUnknown(value, ["scope", "reason"], label);
  return {
    scope: requireText(value.scope, `${label} scope`),
    reason: requireText(value.reason, `${label} reason`),
  };
}

function normalizeSource(input) {
  requireRecord(input, "observed context source");
  rejectUnknown(input, [
    "logicalRoleInstanceId", "runtimeBinding", "lastCompletedTurnId",
    "visibleItems", "governingSources", "activatedSkills", "lifecycleSnapshot",
    "expectedNextWork", "sourceInventoryCompleteness", "omissions", "unknowns",
  ], "observed context source");
  const binding = requireRecord(input.runtimeBinding, "runtime binding");
  rejectUnknown(binding, ["threadId", "bindingRevision", "environmentRevision"], "runtime binding");
  if (!Number.isSafeInteger(binding.bindingRevision) || binding.bindingRevision < 1) {
    throw new TypeError("runtime binding revision must be a positive safe integer");
  }
  const lifecycleSnapshot = normalizeJson(input.lifecycleSnapshot, "lifecycle snapshot");
  if (lifecycleSnapshot.schemaVersion !== 1) {
    throw new TypeError("lifecycle snapshot schema version is unsupported");
  }
  if (lifecycleSnapshot.threadId !== binding.threadId) {
    throw new TypeError("lifecycle snapshot thread does not match runtime binding");
  }
  const unknowns = [...new Set([
    ...MANDATORY_UNKNOWNS,
    ...(input.unknowns ?? []).map((value, index) => requireText(value, `unknowns[${index}]`)),
  ])].sort();
  const expectedNextWork = requireRecord(input.expectedNextWork, "expected next work");
  rejectUnknown(expectedNextWork, ["reference", "sha256"], "expected next work");
  if (!SOURCE_INVENTORY_COMPLETENESS.has(input.sourceInventoryCompleteness)) {
    throw new TypeError("source inventory completeness is unsupported");
  }
  const omissions = (input.omissions ?? []).map(normalizeOmission);
  if (input.sourceInventoryCompleteness !== "complete" && omissions.length === 0) {
    throw new TypeError("incomplete source inventory requires an explicit omission");
  }
  const visibleItems = (input.visibleItems ?? []).map(normalizeVisibleItem);
  const governingSources = (input.governingSources ?? []).map(normalizeGoverningSource);
  const activatedSkills = (input.activatedSkills ?? []).map(normalizeSkill);
  for (const [label, values, identity] of [
    ["visible item", visibleItems, (value) => value.identity],
    ["governing source", governingSources, (value) => value.identity],
    ["activated skill", activatedSkills, (value) => value.name],
  ]) {
    const identities = values.map(identity);
    if (new Set(identities).size !== identities.length) {
      throw new TypeError(`${label} identities must be unique`);
    }
  }
  return {
    logicalRoleInstanceId: requireText(input.logicalRoleInstanceId, "logical role instance id"),
    runtimeBinding: {
      threadId: requireText(binding.threadId, "runtime binding thread id"),
      bindingRevision: binding.bindingRevision,
      environmentRevision: requireText(binding.environmentRevision, "runtime environment revision"),
    },
    lastCompletedTurnId: input.lastCompletedTurnId == null
      ? null
      : requireText(input.lastCompletedTurnId, "last completed turn id"),
    visibleItems,
    governingSources,
    activatedSkills,
    lifecycleSnapshot,
    expectedNextWork: {
      reference: requireText(expectedNextWork.reference, "expected next work reference"),
      sha256: requireDigest(expectedNextWork.sha256, "expected next work sha256"),
    },
    sourceInventoryCompleteness: input.sourceInventoryCompleteness,
    omissions,
    unknowns,
  };
}

function digest(value) {
  return createHash("sha256").update(canonicalJson(value)).digest("hex");
}

function signedBytes(value) {
  return Buffer.concat([SIGNATURE_DOMAIN, Buffer.from(canonicalJson(value), "utf8")]);
}

export function projectObservedContext(source, {
  componentId,
  buildRevision,
  keyId,
  privateKey,
}) {
  const observedContext = normalizeSource(source);
  const payload = {
    schemaVersion: OBSERVED_CONTEXT_SCHEMA_VERSION,
    type: OBSERVED_CONTEXT_TYPE,
    construction: {
      componentId: requireText(componentId, "projector component id"),
      buildRevision: requireText(buildRevision, "projector build revision"),
    },
    sourceRevision: `sha256:${digest(observedContext)}`,
    observedContext,
  };
  const signature = sign(null, signedBytes(payload), privateKey).toString("base64");
  return deepFreeze({
    ...payload,
    attestation: {
      algorithm: "ed25519",
      keyId: requireText(keyId, "projector key id"),
      signature,
    },
  });
}

export function verifyObservedContextProjection(projection, { resolvePublicKey }) {
  try {
    requireRecord(projection, "observed context projection");
    rejectUnknown(projection, [
      "schemaVersion", "type", "construction", "sourceRevision",
      "observedContext", "attestation",
    ], "observed context projection");
    if (projection.schemaVersion !== OBSERVED_CONTEXT_SCHEMA_VERSION
        || projection.type !== OBSERVED_CONTEXT_TYPE) return false;
    const construction = requireRecord(projection.construction, "projection construction");
    rejectUnknown(construction, ["componentId", "buildRevision"], "projection construction");
    const normalizedConstruction = {
      componentId: requireText(construction.componentId, "projector component id"),
      buildRevision: requireText(construction.buildRevision, "projector build revision"),
    };
    const attestation = requireRecord(projection.attestation, "observed context attestation");
    rejectUnknown(attestation, ["algorithm", "keyId", "signature"], "observed context attestation");
    if (attestation.algorithm !== "ed25519") return false;
    requireText(attestation.keyId, "projector key id");
    requireText(attestation.signature, "projector signature");
    const observedContext = normalizeSource(projection.observedContext);
    if (projection.sourceRevision !== `sha256:${digest(observedContext)}`) return false;
    const payload = {
      schemaVersion: projection.schemaVersion,
      type: projection.type,
      construction: normalizedConstruction,
      sourceRevision: projection.sourceRevision,
      observedContext,
    };
    const publicKey = resolvePublicKey(attestation.keyId);
    if (!publicKey) return false;
    return verify(
      null,
      signedBytes(payload),
      publicKey,
      Buffer.from(attestation.signature, "base64"),
    );
  } catch {
    return false;
  }
}
