import { createHash } from "node:crypto";

export const EXTENSION_TERMINAL_STATES = Object.freeze(["cleaned", "retained"]);
export const EXTENSION_TRANSITIONS = Object.freeze({
  admitted: ["activated"], activated: ["executed"], executed: ["artifact_sealed"],
  artifact_sealed: ["detached"], detached: [...EXTENSION_TERMINAL_STATES],
});

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map((key) =>
    `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}

export function extensionDigest(value) {
  return createHash("sha256").update(canonicalJson(value)).digest("hex");
}

function text(value, label) {
  if (typeof value !== "string" || value.length === 0) throw new TypeError(`${label} is required`);
  return value;
}

export function validateRunEnvelope(run) {
  if (!run || run.network !== "sealed" || run.credentials !== "none") {
    throw new TypeError("extension run must use sealed networking and no projected credentials");
  }
  for (const field of ["repository_subject", "checkout", "artifact_root", "scratch_root",
    "retention", "namespace"]) text(run[field], `extension run ${field}`);
  if (!run.artifact_root.startsWith(`${run.namespace}/`)
      || !run.scratch_root.startsWith(`${run.namespace}/`)) {
    throw new TypeError("extension roots must be namespaced");
  }
  return structuredClone(run);
}

export function assertExtensionTransition(from, to) {
  if (!EXTENSION_TRANSITIONS[from]?.includes(to)) {
    throw new TypeError(`invalid extension transition: ${from} -> ${to}`);
  }
}

export function extensionReceipt(attachment, state, details = {}) {
  return Object.freeze({
    schema_version: 1, bundle_id: attachment.bundle_id,
    attachment_sha256: attachment.sha256, state,
    workload: details.workload ?? null,
    transport: details.transport ?? null,
    artifact: details.artifact ?? null,
    production_admission: false,
    publication_authorized: false,
  });
}
