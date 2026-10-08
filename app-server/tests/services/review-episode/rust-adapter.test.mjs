import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { createImplementationReviewService } from "../../../src/services/implementation-review/service.mjs";
import { canonicalJson, digest } from "../../../src/services/review-episode/contract.mjs";
import { createRustReviewEpisodeService, RustReviewEpisodeError } from "../../../src/services/review-episode/rust-adapter.mjs";

const binaryPath = process.env.WORK_ENGINE_REVIEW_EPISODE_TEST_BINARY;
const selectedTest = binaryPath ? test : test.skip;
const sha = (value) => createHash("sha256").update(value).digest("hex");
const reference = (owner, reference, revision, value) => ({owner, reference, revision,
  sha256: digest(value), freshness: "exact immutable revision"});

selectedTest("native adapter uses bound campaign owner, Rust writes, exact read and restart", async () => {
  const parent = await mkdtemp(path.join(os.tmpdir(), "review-episode-native-adapter-"));
  const root = path.join(parent, "native-root");
  try {
    const init = spawnSync(binaryPath, ["init-native", "--root", root]);
    assert.equal(init.status, 0, init.stderr?.toString());
    const expectedSha256 = sha(readFileSync(binaryPath));
    const campaignIdentity = {runId: "native-test", sliceNumber: 1,
      attemptId: "attempt-1", planVersion: "plan-1"};
    const obligationId = "implementation-review";
    const identity = {...campaignIdentity, reviewObligationId: obligationId,
      reviewEpisodeId: digest({identity: campaignIdentity, obligationId}).slice(0, 32)};
    const selection = {selectionId: "selection-1", specialists: [{obligationId, selection: "selected"}]};
    const campaign = {identity: campaignIdentity, reviewSelection: selection};
    const subject = {commit: "candidate", tree: "tree", patchIdentity: "patch"};
    const selectionRevision = digest(selection);
    const authority = {schemaVersion: 1,
      grantId: `grant:native-review:${digest({identity: campaignIdentity, obligationId})}`,
      identity,
      source: {owner: "slice-supervisor", reference: selection.selectionId,
        revision: selectionRevision, sha256: selectionRevision, freshness: "exact immutable revision"},
      writer: {actorId: `implementation-reviewer:${obligationId}`, provider: "claude", generation: 1,
        runtimeSession: reference("reviewer-runtime", "session-1", "generation-1", "session-1")},
      readers: ["reviewer", "builder", "supervisor"],
      initialSubject: reference("checkpoint", subject.commit, subject.tree, subject),
      predecessorRevision: null};
    const options = {binaryPath, expectedSha256, root,
      timeoutMs: Number(process.env.WORK_ENGINE_REVIEW_EPISODE_TEST_TIMEOUT_MS ?? 2_000),
      implementationReview: createImplementationReviewService()};
    assert.throws(() => createRustReviewEpisodeService({...options,
      expectedSha256: "f".repeat(64)}), /binary digest differs/);
    const unbound = createRustReviewEpisodeService(options);
    assert.throws(() => unbound.read({identity}), /unbound/);
    const service = createRustReviewEpisodeService(options);
    service.bindCampaignOwner(() => campaign);
    const begun = service.begin({authority, transitionId: "\ud800"});
    assert.equal(begun.phase, "initial_review");
    assert.equal(typeof begun.handledTransitions["\ud800"], "string");
    assert.deepEqual(service.resumeInitial({authority}), begun);
    const result = JSON.parse(await readFile(new URL("../../fixtures/implementation-review/acceptable-as-is.json", import.meta.url)));
    result.subject = subject;
    assert.deepEqual(service.validateResult({authority, expectedRevision: begun.revision, result}), result);
    const reported = service.transition({authority, expectedRevision: begun.revision,
      transitionId: "\ud801", action: "record_result", payload: {result, unresolvedQuestions: []}});
    assert.equal(reported.phase, "reported");
    assert.equal(service.history({identity}).length, 2);
    const restarted = createRustReviewEpisodeService(options);
    restarted.bindCampaignOwner(() => campaign);
    assert.deepEqual(restarted.read({identity, revision: begun.revision}), begun);
    assert.deepEqual(restarted.recover(identity), reported);
    assert.throws(() => restarted.transition({authority, expectedRevision: reported.revision,
      transitionId: "unavailable", action: "succeed_evidence", payload: {}}), /awaits RC admission/);
    const remediatedSubject = {commit: "candidate-2", tree: "tree-2", patchIdentity: "patch-2"};
    const remediatedReference = reference("checkpoint", remediatedSubject.commit,
      remediatedSubject.tree, remediatedSubject);
    const remediating = restarted.transition({authority, expectedRevision: reported.revision,
      transitionId: "remediation-subject", action: "record_remediation_subject",
      payload: {subject: remediatedReference}});
    const uncertain = restarted.transition({authority, expectedRevision: remediating.revision,
      transitionId: "uncertain", action: "mark_uncertain",
      payload: {reason: "controlled session loss", reconciliationAction: "replace writer"}});
    const successor = {...authority,
      writer: {...authority.writer, generation: 2,
        runtimeSession: reference("reviewer-runtime", "session-2", "generation-2", "session-2")},
      initialSubject: remediatedReference, predecessorRevision: uncertain.revision};
    const replaced = restarted.transition({authority: successor, expectedRevision: uncertain.revision,
      transitionId: "replace", action: "replace_writer",
      payload: {reason: "session unavailable", pendingAction: "reconcile exact state"}});
    assert.equal(replaced.writer.generation, 2);
    assert.throws(() => restarted.transition({authority, expectedRevision: replaced.revision,
      transitionId: "stale", action: "mark_uncertain",
      payload: {reason: "stale", reconciliationAction: "stop"}}), /Authority|writer generation/);
    const retired = restarted.transition({authority: successor, expectedRevision: replaced.revision,
      transitionId: "retire", action: "retire",
      payload: {outcome: "superseded", reason: "closed", protectedReferences: [remediatedReference]}});
    assert.equal(retired.status, "retired");
    assert.throws(() => restarted.read({identity: {...identity, reviewObligationId: "other"}}),
      RustReviewEpisodeError);
  } finally { await rm(parent, {recursive: true, force: true}); }
});

test("actual child malformed replies fail closed before facade projection", async (t) => {
  const parent = await mkdtemp(path.join(os.tmpdir(), "review-episode-native-bad-reply-"));
  t.after(() => rm(parent, {recursive: true, force: true}));
  const fake = path.join(parent, "fake-native-child");
  const root = path.join(parent, "native-root");
  await mkdir(root);
  await writeFile(fake, `#!/usr/bin/env node
const fs = require("node:fs");
const crypto = require("node:crypto");
const canonical = (value) => Array.isArray(value) ? '[' + value.map(canonical).join(',') + ']'
  : value && typeof value === 'object' ? '{' + Object.keys(value).sort().map((key) =>
    JSON.stringify(key) + ':' + canonical(value[key])).join(',') + '}' : JSON.stringify(value);
const digest = (value) => crypto.createHash('sha256').update(canonical(value)).digest('hex');
const input = fs.readFileSync(0);
const request = JSON.parse(input.subarray(4).toString('utf8'));
const mode = process.env.WORK_ENGINE_TEST_REPLY_MODE;
if (mode === 'malformed_utf8') {
  const output = Buffer.alloc(5); output.writeUInt32BE(1); output[4] = 255;
  process.stdout.write(output); process.exit(0);
}
const response = {version: 2, domain: 'review-episode', requestId: request.requestId,
  source: 'native_host', profile: 'native-host-v1', operation: request.operation,
  requestDigest: digest(request), rootSelectionDigest: request.rootSelectionDigest,
  status: 'observed', observedRevision: null, state: null};
if (mode === 'wrong_operation') response.operation = 'history';
if (mode === 'wrong_status') response.status = 'applied';
if (mode === 'missing_state') delete response.state;
if (mode === 'invalid_state') response.state = {revision: 'a'.repeat(64)};
if (mode === 'extra_error' || mode === 'unknown_kind' || mode === 'missing_error_message'
    || mode === 'duplicate_error') {
  response.status = 'error'; response.kind = mode === 'unknown_kind' ? 'Unknown' : 'ResultContract';
  response.message = 'controlled error'; delete response.state; delete response.observedRevision;
  if (mode === 'extra_error') response.unexpected = true;
  if (mode === 'missing_error_message') delete response.message;
}
let bodyText = canonical(response);
if (mode === 'duplicate_error') bodyText = bodyText.replace('"kind":"ResultContract"',
  '"kind":"Io","kind":"ResultContract"');
if (mode === 'duplicate_success') bodyText = bodyText.replace('"state":null', '"state":null,"state":null');
if (mode === 'bom') bodyText = String.fromCharCode(0xfeff) + bodyText;
const body = Buffer.from(bodyText);
const output = Buffer.alloc(4); output.writeUInt32BE(body.length);
process.stdout.write(Buffer.concat([output, body]));
`, {mode: 0o700});
  const binarySha = sha(await readFile(fake));
  const basis = {profile: "native-host-v1", protocol: 2, codec: "review-episode-js-json-v1",
    root, executableSha256: binarySha};
  await writeFile(path.join(root, ".review-episode-native-host-v1"),
    canonicalJson({...basis, selectionDigest: digest(basis)}));
  const campaignIdentity = {runId: "bad-reply", sliceNumber: 1, attemptId: "one", planVersion: "v1"};
  const obligationId = "implementation-review";
  const identity = {...campaignIdentity, reviewObligationId: obligationId,
    reviewEpisodeId: digest({identity: campaignIdentity, obligationId}).slice(0, 32)};
  const campaign = {identity: campaignIdentity, reviewSelection: {selectionId: "selected",
    specialists: [{obligationId, selection: "selected"}]}};
  const service = createRustReviewEpisodeService({binaryPath: fake, expectedSha256: binarySha, root,
    implementationReview: createImplementationReviewService(), timeoutMs: 2_000});
  service.bindCampaignOwner(() => campaign);
  assert.equal(service.read({identity}), null);
  for (const mode of ["malformed_utf8", "wrong_operation", "wrong_status", "missing_state",
    "invalid_state", "extra_error", "unknown_kind", "missing_error_message",
    "duplicate_error", "duplicate_success", "bom"]) {
    process.env.WORK_ENGINE_TEST_REPLY_MODE = mode;
    try {
      assert.throws(() => service.read({identity}), (error) => error instanceof RustReviewEpisodeError
        && (mode === "bom" ? error.kind === "Protocol"
          : ["Protocol", "Transport"].includes(error.kind)), mode);
    } finally { delete process.env.WORK_ENGINE_TEST_REPLY_MODE; }
  }
});

const faultBinary = process.env.WORK_ENGINE_REVIEW_EPISODE_FAULT_TEST_BINARY;
const faultTest = faultBinary ? test : test.skip;
faultTest("actual adapter reconciles before-commit, after-commit, unknown-ack and partial-reply cuts", async (t) => {
  for (const cut of ["before_commit", "after_commit", "after_commit_unknown", "after_partial_reply"]) {
    const parent = await mkdtemp(path.join(os.tmpdir(), `review-episode-adapter-${cut}-`));
    t.after(() => rm(parent, {recursive: true, force: true}));
    const root = path.join(parent, "native-root");
    const barrier = path.join(parent, "barrier");
    await mkdir(barrier);
    const init = spawnSync(faultBinary, ["init-native", "--root", root]);
    assert.equal(init.status, 0, init.stderr?.toString());
    const expectedSha256 = sha(await readFile(faultBinary));
    const campaignIdentity = {runId: `adapter-cut-${cut}`, sliceNumber: 1,
      attemptId: "attempt-1", planVersion: "plan-1"};
    const obligationId = "implementation-review";
    const identity = {...campaignIdentity, reviewObligationId: obligationId,
      reviewEpisodeId: digest({identity: campaignIdentity, obligationId}).slice(0, 32)};
    const selection = {selectionId: "selection-1", specialists: [{obligationId, selection: "selected"}]};
    const subject = {commit: "candidate", tree: "tree", patchIdentity: "patch"};
    const authority = {schemaVersion: 1, grantId: `grant:native-review:${digest({identity: campaignIdentity, obligationId})}`,
      identity, source: {owner: "slice-supervisor", reference: selection.selectionId,
        revision: digest(selection), sha256: digest(selection), freshness: "exact immutable revision"},
      writer: {actorId: `implementation-reviewer:${obligationId}`, provider: "claude", generation: 1,
        runtimeSession: reference("reviewer-runtime", "session-1", "generation-1", "session-1")},
      readers: ["reviewer", "builder", "supervisor"],
      initialSubject: reference("checkpoint", subject.commit, subject.tree, subject),
      predecessorRevision: null};
    const service = createRustReviewEpisodeService({binaryPath: faultBinary, expectedSha256, root,
      timeoutMs: 3_000, implementationReview: createImplementationReviewService()});
    service.bindCampaignOwner(() => ({identity: campaignIdentity, reviewSelection: selection}));
    const killer = cut === "after_commit_unknown" ? null : spawn(process.execPath, [new URL("../../fixtures/review-episode-rust/fault-killer.mjs",
      import.meta.url).pathname, barrier, cut, String(process.pid), faultBinary],
    {stdio: ["ignore", "pipe", "pipe"]});
    let killerStderr = "";
    killer?.stderr.on("data", (chunk) => {killerStderr += chunk.toString();});
    process.env.REVIEW_EPISODE_FAULT_CUT = cut;
    process.env.REVIEW_EPISODE_FAULT_DIR = barrier;
    process.env.REVIEW_EPISODE_FAULT_OPERATION = "begin";
    let error;
    try { service.begin({authority, transitionId: "exact-begin"}); }
    catch (caught) {error = caught;}
    finally {delete process.env.REVIEW_EPISODE_FAULT_CUT; delete process.env.REVIEW_EPISODE_FAULT_DIR;
      delete process.env.REVIEW_EPISODE_FAULT_OPERATION;}
    assert.ok(error instanceof RustReviewEpisodeError, `${cut}: write unexpectedly completed`);
    assert.equal(error.kind, cut === "after_commit_unknown" ? "OutcomeUnknown" : "Transport");
    assert.deepEqual(error.possibleCommit?.identity, identity);
    assert.equal(error.possibleCommit?.transitionId, "exact-begin");
    assert.equal(error.possibleCommit?.contentDigest,
      digest({action: "begin", unresolvedQuestions: []}));
    if (killer) {
      const killerExit = await new Promise((resolve) => killer.once("exit", (code, signal) => resolve({code, signal})));
      assert.equal(killerExit.code, 0, `${cut}: barrier killer failed ${JSON.stringify(killerExit)} ${killerStderr}`);
      const acknowledged = JSON.parse(await readFile(path.join(barrier, `${cut}.killed`), "utf8"));
      assert.equal(acknowledged.readyObserved, true);
    }
    const reconciled = service.reconcilePossibleCommit(error);
    const committed = cut !== "before_commit";
    assert.equal(reconciled.status, committed ? "committed" : "absent", cut);
    const firstCommittedRevision = reconciled.committedRevision;
    if (committed) {
      const conflictingContext = new RustReviewEpisodeError("Transport", "controlled conflicting content", {
        transport: error.transport, possibleCommit: {...error.possibleCommit,
          contentDigest: "f".repeat(64)}});
      assert.throws(() => service.reconcilePossibleCommit(conflictingContext), (caught) =>
        caught instanceof RustReviewEpisodeError && caught.kind === "InvalidCommand");
    }
    const begun = service.begin({authority, transitionId: "exact-begin"});
    assert.equal(reconciled.committedRevision, committed ? begun.revision : null);
    if (committed) assert.equal(firstCommittedRevision, begun.revision);
    assert.throws(() => service.begin({authority, transitionId: "exact-begin",
      unresolvedQuestions: ["different content"]}), RustReviewEpisodeError);
    assert.equal(service.history({identity}).length, 1, cut);
    assert.equal(service.recover(identity).revision, begun.revision);
  }
});
