import {execFileSync} from "node:child_process";
import {createHash} from "node:crypto";

const INVENTORY_PATH = "app-server/migrations/skills/portfolio-inventory-v1.json";

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort()
    .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}
const sha256 = (value) => createHash("sha256").update(value).digest("hex");
const digest = (value) => sha256(canonicalJson(value));

function git(repository, args, {encoding = "utf8"} = {}) {
  return execFileSync("git", ["-C", repository, ...args], {
    encoding, maxBuffer: 32 * 1024 * 1024,
  });
}

function commit(repository, revision, label) {
  if (typeof revision !== "string" || !revision.trim()) throw new TypeError(`${label} is required`);
  const value = git(repository, ["rev-parse", `${revision}^{commit}`]).trim();
  if (!/^[0-9a-f]{40}$/.test(value)) throw new Error(`${label} did not resolve to a Git commit`);
  return value;
}

function blob(repository, revision, filePath) {
  try { return git(repository, ["show", `${revision}:${filePath}`], {encoding: null}); }
  catch (error) {
    if (error?.status === 128) return null;
    throw error;
  }
}

function inventory(repository, revision, inventoryPath) {
  const bytes = blob(repository, revision, inventoryPath);
  if (bytes === null) throw new Error(`skills migration inventory is absent at ${revision}`);
  let document;
  try { document = JSON.parse(bytes.toString("utf8")); }
  catch { throw new Error(`skills migration inventory is invalid JSON at ${revision}`); }
  if (!Array.isArray(document?.packages)) throw new Error("skills migration inventory packages are invalid");
  return {bytes, document};
}

function flatten(document) {
  const entries = new Map();
  for (const record of document.packages) {
    if (typeof record?.name !== "string" || !record.name.trim()) {
      throw new Error("skills migration inventory package name is invalid");
    }
    for (const [kind, values] of [["source", [record.source]], ["artifact", record.artifacts], ["test", record.tests]]) {
      if (!Array.isArray(values)) throw new Error(`skills migration inventory ${record.name}.${kind} entries are invalid`);
      for (const entry of values) {
        if (typeof entry?.path !== "string" || !entry.path.trim()
            || typeof entry?.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(entry.sha256)) {
          throw new Error(`skills migration inventory ${record.name}.${kind} entry is invalid`);
        }
        if (entries.has(entry.path)) throw new Error(`skills migration inventory duplicates ${entry.path}`);
        entries.set(entry.path, {package: record.name, kind, entry});
      }
    }
  }
  return entries;
}

function withoutIntegrity(document) {
  const copy = structuredClone(document);
  for (const record of copy.packages) {
    delete record.source.sha256;
    for (const entry of [...record.artifacts, ...record.tests]) delete entry.sha256;
  }
  return copy;
}

export function classifySkillsMigrationIntegrity({repository, baselineRevision, candidateRevision,
  acceptedPaths, inventoryPath = INVENTORY_PATH} = {}) {
  if (typeof repository !== "string" || !repository.trim()) throw new TypeError("repository is required");
  if (!Array.isArray(acceptedPaths)
      || acceptedPaths.some((value) => typeof value !== "string" || !value.trim())) {
    throw new TypeError("acceptedPaths must be an array of repository paths");
  }
  const baselineCommit = commit(repository, baselineRevision, "baselineRevision");
  const candidateCommit = commit(repository, candidateRevision, "candidateRevision");
  const baseline = inventory(repository, baselineCommit, inventoryPath);
  const candidate = inventory(repository, candidateCommit, inventoryPath);
  const baselineEntries = flatten(baseline.document);
  const candidateEntries = flatten(candidate.document);
  const accepted = new Set(acceptedPaths);
  const paths = [...new Set([...baselineEntries.keys(), ...candidateEntries.keys()])]
    .filter((filePath) => baselineEntries.get(filePath)?.entry.sha256
      !== candidateEntries.get(filePath)?.entry.sha256 || accepted.has(filePath))
    .sort();
  const semanticProjectionEqual = canonicalJson(withoutIntegrity(baseline.document))
    === canonicalJson(withoutIntegrity(candidate.document));
  const changes = [];
  const violations = [];
  for (const filePath of paths) {
    const before = baselineEntries.get(filePath) ?? null;
    const after = candidateEntries.get(filePath) ?? null;
    const baselineBytes = blob(repository, baselineCommit, filePath);
    const candidateBytes = blob(repository, candidateCommit, filePath);
    const baselineActual = baselineBytes === null ? null : sha256(baselineBytes);
    const candidateActual = candidateBytes === null ? null : sha256(candidateBytes);
    const recordedChanged = before?.entry.sha256 !== after?.entry.sha256;
    const sourceChanged = baselineActual !== candidateActual;
    if (!recordedChanged && after?.entry.sha256 === candidateActual) continue;
    let classification;
    if (!before || !after || baselineBytes === null || candidateBytes === null) {
      classification = "inventory_or_source_membership_changed";
    } else if (after.entry.sha256 !== candidateActual) {
      classification = "stale_candidate_binding";
    } else if (!sourceChanged && recordedChanged) {
      classification = "inherited_baseline_debt_reconciliation";
    } else if (sourceChanged && accepted.has(filePath)) {
      classification = "in_scope_derived_update";
    } else if (sourceChanged) {
      classification = "out_of_scope_source_change";
    } else classification = "unchanged_current_binding";
    const change = Object.freeze({path: filePath,
      package: after?.package ?? before?.package ?? null,
      kind: after?.kind ?? before?.kind ?? null,
      classification,
      sourceChanged,
      acceptedSourcePath: accepted.has(filePath),
      baselineRecordedSha256: before?.entry.sha256 ?? null,
      baselineActualSha256: baselineActual,
      candidateRecordedSha256: after?.entry.sha256 ?? null,
      candidateActualSha256: candidateActual});
    changes.push(change);
    if (["inventory_or_source_membership_changed", "stale_candidate_binding",
      "out_of_scope_source_change"].includes(classification)) {
      violations.push(Object.freeze({path: filePath, classification}));
    }
  }
  if (!semanticProjectionEqual) {
    violations.push(Object.freeze({path: inventoryPath, classification: "semantic_inventory_change"}));
  }
  const body = Object.freeze({schemaVersion: 1,
    inventoryPath,
    baselineCommit,
    candidateCommit,
    acceptedPaths: Object.freeze([...accepted].sort()),
    baselineInventorySha256: sha256(baseline.bytes),
    candidateInventorySha256: sha256(candidate.bytes),
    semanticProjectionEqual,
    changes: Object.freeze(changes),
    violations: Object.freeze(violations),
    verdict: violations.length === 0 ? "mechanically_attributed" : "requires_disposition"});
  return Object.freeze({...body, receiptDigest: digest(body)});
}

export const skillsMigrationInventoryPath = INVENTORY_PATH;
