import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { promisify } from "node:util";

import { ClaimEvidenceError } from "./contract.mjs";
import { observationEventIdentity, normalizeObservation } from "./observation.mjs";

const execFileAsync = promisify(execFile);
const GIT_TIMEOUT_MS = 5_000;
const GIT_MAX_BUFFER = 4 * 1024 * 1024;

function requireText(value, label) {
  if (typeof value !== "string" || value.length === 0) throw new ClaimEvidenceError(`${label} must be a nonempty string`);
  return value;
}

function objectIdentity(value, label) {
  requireText(value, label);
  if (!/^[0-9a-f]{40,128}$/.test(value)) throw new ClaimEvidenceError(`${label} must be an opaque lowercase hexadecimal Git object identity`);
  return value;
}

async function git(repositoryPath, arguments_, { binary = false } = {}) {
  try {
    const result = await execFileAsync("git", ["-C", repositoryPath, ...arguments_], {
      encoding: binary ? "buffer" : "utf8",
      timeout: GIT_TIMEOUT_MS,
      maxBuffer: GIT_MAX_BUFFER,
      windowsHide: true,
    });
    return result.stdout;
  } catch (error) {
    const detail = error?.killed ? "timed out" : "failed";
    throw new ClaimEvidenceError(`Git checkpoint verification ${detail}`);
  }
}

export async function observeGitCheckpoint({
  repositoryPath,
  expectedCommit,
  expectedTree,
  producerIdentity,
  observedAt,
  providerSequence = null,
  origin,
  subject,
  evidenceBaseline,
  executableGeneration,
  adapterVersion,
}) {
  requireText(repositoryPath, "Git repository path");
  objectIdentity(expectedCommit, "expected Git commit");
  objectIdentity(expectedTree, "expected Git tree");
  requireText(producerIdentity, "Git checkpoint producer identity");

  const objectFormat = (await git(repositoryPath, ["rev-parse", "--show-object-format"])).trim();
  requireText(objectFormat, "Git object format");
  const commit = (await git(repositoryPath, [
    "rev-parse", "--verify", "--end-of-options", `${expectedCommit}^{commit}`,
  ])).trim();
  if (commit !== expectedCommit) throw new ClaimEvidenceError("Git checkpoint commit identity mismatch");
  const tree = (await git(repositoryPath, [
    "rev-parse", "--verify", "--end-of-options", `${expectedCommit}^{tree}`,
  ])).trim();
  if (tree !== expectedTree) throw new ClaimEvidenceError("Git checkpoint tree identity mismatch");
  const commitContent = await git(repositoryPath, ["cat-file", "commit", expectedCommit], { binary: true });
  const artifactDigest = createHash("sha256").update(commitContent).digest("hex");
  const sourceIdentity = `${origin.reference}:git+${objectFormat}:${commit}:${tree}`;

  return normalizeObservation({
    event_identity: observationEventIdentity(producerIdentity, sourceIdentity),
    producer: { identity: producerIdentity, kind: "deterministic_git_checkpoint_adapter" },
    origin,
    subject,
    evidence_baseline: evidenceBaseline,
    artifact: {
      kind: "git_checkpoint",
      reference: `git+${objectFormat}:${commit}`,
      digest: { algorithm: "sha256", value: artifactDigest },
      verification: "verified",
      checkpoint: { object_format: objectFormat, commit, tree },
    },
    observed_at: observedAt,
    provider_sequence: providerSequence,
    completeness: "available",
    exclusions: [],
    collection_failures: [],
    executable_generation: executableGeneration,
    adapter_version: adapterVersion,
  });
}
