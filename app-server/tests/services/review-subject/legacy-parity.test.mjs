import assert from "node:assert/strict";
import { execFile, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { promisify } from "node:util";

import { createLegacyReviewSubjectBackend } from "../../../src/services/review-subject/legacy-backend-adapter.mjs";
import { createReviewSubjectService } from "../../../src/services/review-subject/service.mjs";

const execFileAsync = promisify(execFile);
const workspaceRoot = path.resolve(new URL("../../../..", import.meta.url).pathname);
const checkpointPath = path.join(workspaceRoot, "skills/slice-checkpoint/scripts/checkpoint.py");
const profilePath = path.join(workspaceRoot, "skills/code-change-profile/scripts/code_change_profile.py");
const profileFixture = path.join(
  workspaceRoot, "skills/code-change-profile/tests/fixtures/proposal-packets-accepted.json",
);

async function git(repository, ...args) {
  return (await execFileAsync("git", ["-C", repository, ...args], { encoding: "utf8" })).stdout.trim();
}

function directPython(script, input) {
  const result = spawnSync("python3", ["-c", script], {
    cwd: workspaceRoot,
    input: JSON.stringify(input),
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stderr, "");
  return JSON.parse(result.stdout);
}

test("host candidate receipt is exactly compatible with the legacy checkpoint owner", async (t) => {
  const repository = await mkdtemp(path.join(os.tmpdir(), "review-subject-parity."));
  t.after(() => rm(repository, { recursive: true, force: true }));
  await git(repository, "init", "--quiet", "--initial-branch=main");
  await git(repository, "config", "user.name", "Parity Test");
  await git(repository, "config", "user.email", "parity@example.invalid");
  await writeFile(path.join(repository, "task.txt"), "before\n");
  await git(repository, "add", "task.txt");
  await git(repository, "commit", "--quiet", "-m", "baseline");
  const baseline = await git(repository, "rev-parse", "HEAD");
  await writeFile(path.join(repository, "task.txt"), "after\n");
  const request = {
    schema_version: 1, repository, run_id: "parity", slice_number: 1, candidate_attempt: 1,
    baseline_commit_oid: baseline, parent_checkpoint_commit_oid: null,
    plan_version: "parity-plan", scope_revision: "parity-scope",
    gate_receipt_digest: "b".repeat(64), created_at: "2026-08-29T13:00:00+00:00",
    paths: [{ path: "task.txt", action: "include", attribution: "task_owned" }],
  };
  const script = `
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("legacy_checkpoint", ${JSON.stringify(checkpointPath)})
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
print(json.dumps(module.create_candidate(json.load(sys.stdin)), sort_keys=True, separators=(",", ":")))
`;
  const legacy = directPython(script, request);
  const service = createReviewSubjectService({ workspaceRoot });
  const mediated = await service.createCandidate(request);
  assert.deepEqual(mediated, legacy);
  const candidateSubject = {
    schema_version: 2, construction_method: "slice_checkpoint_candidate_receipt",
    evidence_cutoff: mediated.created_at, checkpoint: mediated,
  };
  const candidateProfile = await service.createPhysicalProfile({ subject: candidateSubject });
  assert.equal(candidateProfile.subject.checkpoint.checkpoint_commit_oid, mediated.checkpoint_commit_oid);
  assert.equal(candidateProfile.subject.checkpoint.manifest_digest, mediated.manifest_digest);

  const transitionScript = `
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("legacy_checkpoint", ${JSON.stringify(checkpointPath)})
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
value = json.load(sys.stdin)
print(json.dumps(module.transition(value["candidate"], "stopped", expected_accepted=None), sort_keys=True, separators=(",", ":")))
`;
  const legacyStopped = directPython(transitionScript, { candidate: legacy });
  const mediatedStopped = await service.transitionCandidate({ candidate: mediated, kind: "stopped" });
  assert.deepEqual(mediatedStopped, legacyStopped);

  const validateScript = `
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("legacy_checkpoint", ${JSON.stringify(checkpointPath)})
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
value = json.load(sys.stdin)
print(json.dumps(module.validate_lifecycle_receipt(value["receipt"], "stopped", require_paths=True), sort_keys=True, separators=(",", ":")))
`;
  const legacyValidated = directPython(validateScript, { receipt: legacyStopped });
  assert.deepEqual(
    await service.validateCheckpoint({ receipt: mediatedStopped, kind: "stopped", requirePaths: true }),
    legacyValidated,
  );
});

async function failingBackendFixture(t, { checkpointSource, profileSource }) {
  const root = await mkdtemp(path.join(os.tmpdir(), "review-subject-backend-failure."));
  t.after(() => rm(root, { recursive: true, force: true }));
  const checkpointDirectory = path.join(root, "skills/slice-checkpoint/scripts");
  const profileDirectory = path.join(root, "skills/code-change-profile/scripts");
  await mkdir(checkpointDirectory, { recursive: true });
  await mkdir(profileDirectory, { recursive: true });
  await writeFile(path.join(checkpointDirectory, "checkpoint.py"), checkpointSource);
  const checkpointDigest = createHash("sha256").update(checkpointSource).digest("hex");
  await writeFile(
    path.join(profileDirectory, "code_change_profile.py"),
    profileSource.replace("CHECKPOINT_DIGEST", checkpointDigest),
  );
  return root;
}

async function assertNormalizedBackendFailure(root) {
  const bridge = path.join(workspaceRoot, "app-server/scripts/review-subject-backend.py");
  const checkpoint = await readFile(path.join(root, "skills/slice-checkpoint/scripts/checkpoint.py"));
  const profile = await readFile(path.join(root, "skills/code-change-profile/scripts/code_change_profile.py"));
  const result = spawnSync("python3", [bridge, "--workspace-root", root], {
    input: JSON.stringify({
      schema_version: 1,
      backend: "work-engine.review-subject.legacy-v1",
      operation: "create_candidate",
      expected_backend_sha256: {
        checkpoint: createHash("sha256").update(checkpoint).digest("hex"),
        physical_profile: createHash("sha256").update(profile).digest("hex"),
      },
      input: { request: {} },
    }),
    encoding: "utf8",
  });
  assert.equal(result.status, 2);
  assert.equal(result.stdout, "");
  assert.match(result.stderr, /^review-subject: /);
  assert.equal(result.stderr.trim().split("\n").length, 1);
  assert.equal(result.stderr.length <= 2065, true);
  assert.doesNotMatch(result.stderr, /Traceback \(most recent call last\)/);

  const backend = createLegacyReviewSubjectBackend({
    workspaceRoot: root,
    bridgePath: bridge,
  });
  await assert.rejects(
    backend.invoke({ operation: "create_candidate", input: { request: {} } }),
    (error) => {
      assert.match(error.message, /^review-subject backend failed: review-subject: /);
      assert.doesNotMatch(error.message, /Traceback \(most recent call last\)/);
      assert.doesNotMatch(error.message, /exit 1$/);
      return true;
    },
  );
}

test("backend normalizes dynamic-load and dispatch exceptions without traceback leakage", async (t) => {
  const syntaxFailure = await failingBackendFixture(t, {
    checkpointSource: "def broken(:\n",
    profileSource: 'CHECKPOINT_VALIDATOR_SHA256 = "CHECKPOINT_DIGEST"\n',
  });
  await assertNormalizedBackendFailure(syntaxFailure);

  const dispatchFailure = await failingBackendFixture(t, {
    checkpointSource: "VALUE = 1\n",
    profileSource: 'CHECKPOINT_VALIDATOR_SHA256 = "CHECKPOINT_DIGEST"\n',
  });
  await assertNormalizedBackendFailure(dispatchFailure);
});

test("host physical profile is byte-compatible with the legacy analyzer fixture", async () => {
  const subject = JSON.parse(await readFile(profileFixture, "utf8"));
  const direct = spawnSync("python3", [profilePath, "profile", "--receipt", profileFixture, "--repository", workspaceRoot], {
    cwd: workspaceRoot, encoding: "utf8",
  });
  assert.equal(direct.status, 0, direct.stderr);
  const service = createReviewSubjectService({ workspaceRoot });
  const mediated = await service.createPhysicalProfile({ subject, repository: workspaceRoot });
  assert.deepEqual(mediated, JSON.parse(direct.stdout));
});

test("adapter refuses forged backend identity before exposing a result", async () => {
  const backend = createLegacyReviewSubjectBackend({
    workspaceRoot,
    invokeProcess: async () => ({
      code: 0,
      signal: null,
      stderr: Buffer.alloc(0),
      stdout: Buffer.from(JSON.stringify({
        schema_version: 1,
        backend: "work-engine.review-subject.legacy-v1",
        backend_sha256: { checkpoint: "0".repeat(64), physical_profile: "0".repeat(64) },
        operation: "create_candidate",
        result: {},
      })),
    }),
  });
  await assert.rejects(
    backend.invoke({ operation: "create_candidate", input: { request: {} } }),
    /backend identity mismatch/,
  );
});

test("profile validation refuses resigned physical contradictions", async () => {
  const subject = JSON.parse(await readFile(profileFixture, "utf8"));
  const service = createReviewSubjectService({ workspaceRoot });
  const profile = await service.createPhysicalProfile({ subject, repository: workspaceRoot });
  profile.observations.file_count.value += 1;
  await assert.rejects(service.validatePhysicalProfile({ profile }), /profile digest/);
});
