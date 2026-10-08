import { spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { constants, closeSync, openSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";

import { canonicalJson, digest, freeze, identityKey, validateAuthority, validateIdentity,
  validateState } from "./contract.mjs";
import { validateImplementationReviewResult } from "../implementation-review/contract.mjs";
import { ReviewEpisodeResultError } from "./service.mjs";

const MARKER = ".review-episode-native-host-v1";
const HEX = /^[0-9a-f]{64}$/;
const MAX_REQUEST = 8 * 1024 * 1024;
const MAX_RESPONSE = 32 * 1024 * 1024;
const NATIVE_ERRORS = new Set(["Framing", "Admission", "ResponseTooLarge", "Io",
  "InvalidCommand", "ResultContract", "Authority", "RevisionConflict", "StateIntegrity",
  "Path", "Schema", "Busy", "OutcomeUnknown"]);
const RESPONSE_BASE = ["version", "domain", "requestId", "source", "profile", "operation",
  "requestDigest", "rootSelectionDigest", "status"];
const decoder = new TextDecoder("utf-8", {fatal: true, ignoreBOM: true});

export class RustReviewEpisodeError extends Error {
  constructor(kind, message, {transport = null, possibleCommit = null} = {}) {
    super(message); this.name = "RustReviewEpisodeError"; this.kind = kind;
    this.transport = transport === null ? null : freeze(transport);
    this.possibleCommit = possibleCommit === null ? null : freeze(possibleCommit);
  }
}

function exact(value, keys, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)
      || Object.keys(value).sort().join("\0") !== [...keys].sort().join("\0")) {
    throw new RustReviewEpisodeError("Protocol", `${label} shape differs`);
  }
}

function selected(binaryPath, root, expectedSha256) {
  if (!path.isAbsolute(binaryPath) || !path.isAbsolute(root) || !HEX.test(expectedSha256)) {
    throw new RustReviewEpisodeError("Selection", "native binary, root and digest must be explicitly selected");
  }
  const actual = createHash("sha256").update(readFileSync(binaryPath)).digest("hex");
  if (actual !== expectedSha256) throw new RustReviewEpisodeError("Selection", "native binary digest differs");
  const markerSource = readFileSync(path.join(root, MARKER), "utf8");
  const marker = JSON.parse(markerSource);
  exact(marker, ["profile", "protocol", "codec", "root", "executableSha256", "selectionDigest"], "native root marker");
  const {selectionDigest, ...basis} = marker;
  if (canonicalJson(marker) !== markerSource || marker.profile !== "native-host-v1"
      || marker.protocol !== 2 || marker.codec !== "review-episode-js-json-v1"
      || marker.root !== root || marker.executableSha256 !== actual
      || selectionDigest !== digest(basis)) {
    throw new RustReviewEpisodeError("Selection", "native root selection differs");
  }
  return marker;
}

function fdForDescriptor(descriptor) {
  const file = path.join(os.tmpdir(), `review-episode-admission-${randomUUID()}`);
  let writable;
  try {
    writable = openSync(file, constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY, 0o600);
    writeFileSync(writable, canonicalJson(descriptor));
    closeSync(writable); writable = undefined;
    const readable = openSync(file, constants.O_RDONLY);
    unlinkSync(file);
    return readable;
  } finally {
    if (writable !== undefined) closeSync(writable);
    try { unlinkSync(file); } catch (error) { if (error?.code !== "ENOENT") throw error; }
  }
}

function frame(value) {
  const body = Buffer.from(canonicalJson(value));
  if (body.length === 0 || body.length > MAX_REQUEST) throw new RustReviewEpisodeError("RequestTooLarge", "native request exceeds limit");
  const prefix = Buffer.alloc(4); prefix.writeUInt32BE(body.length);
  return Buffer.concat([prefix, body]);
}

function decode(output, request) {
  if (!Buffer.isBuffer(output) || output.length < 4) throw new RustReviewEpisodeError("Transport", "native response is incomplete");
  const length = output.readUInt32BE(0);
  if (length === 0 || length > MAX_RESPONSE || output.length !== length + 4) throw new RustReviewEpisodeError("Transport", "native response frame differs");
  let response;
  let source;
  try { source = decoder.decode(output.subarray(4)); }
  catch { throw new RustReviewEpisodeError("Transport", "native response is not UTF-8"); }
  if (source[0] !== "{") {
    throw new RustReviewEpisodeError("Protocol", "native response wire is not canonical");
  }
  try { response = JSON.parse(source); }
  catch { throw new RustReviewEpisodeError("Transport", "native response is not JSON"); }
  if (!response || typeof response !== "object" || Array.isArray(response)) {
    throw new RustReviewEpisodeError("Protocol", "native response shape differs");
  }
  let canonical;
  try { canonical = canonicalJson(response); }
  catch { throw new RustReviewEpisodeError("Protocol", "native response nesting is invalid"); }
  if (canonical !== source) {
    throw new RustReviewEpisodeError("Protocol", "native response wire is not canonical");
  }
  if (response.version !== 2 || response.domain !== "review-episode"
      || response.source !== "native_host" || response.profile !== "native-host-v1"
      || response.requestId !== request.requestId || response.operation !== request.operation
      || response.rootSelectionDigest !== request.rootSelectionDigest
      || response.requestDigest !== digest(request)) {
    throw new RustReviewEpisodeError("Protocol", "native response binding differs");
  }
  if (response.status === "error") {
    exact(response, [...RESPONSE_BASE, "kind", "message"], "native error response");
    if (!NATIVE_ERRORS.has(response.kind) || typeof response.message !== "string"
        || response.message.length === 0 || response.message.length > 320) {
      throw new RustReviewEpisodeError("Protocol", "native error response is invalid");
    }
    if (response.kind === "ResultContract") throw new ReviewEpisodeResultError(response.message);
    throw new RustReviewEpisodeError(response.kind, response.message);
  }
  const operation = request.operation;
  const expectedStatus = operation === "begin" || operation === "transition"
    ? ["applied", "replay"] : operation === "recover" && "transitionId" in request.args
    ? ["committed", "absent"] : ["observed"];
  if (!expectedStatus.includes(response.status)) {
    throw new RustReviewEpisodeError("Protocol", "native response status is invalid");
  }
  const history = operation === "history";
  const result = operation === "validateResult";
  const recovery = operation === "recover" && "transitionId" in request.args;
  exact(response, [...RESPONSE_BASE, "observedRevision", ...(history ? ["history"]
    : result ? ["result"] : recovery ? ["committedRevision", "state"] : ["state"])],
  "native success response");
  if (response.observedRevision !== null && !HEX.test(response.observedRevision)) {
    throw new RustReviewEpisodeError("Protocol", "native observed revision is invalid");
  }
  const state = (value) => {
    if (!value || typeof value !== "object" || Array.isArray(value)
        || !HEX.test(value.revision)) throw new RustReviewEpisodeError("Protocol", "native state is invalid");
    const {revision, ...semantic} = value;
    try {validateState(semantic);} catch {throw new RustReviewEpisodeError("Protocol", "native state is invalid");}
    if (digest(semantic) !== revision) throw new RustReviewEpisodeError("Protocol", "native state revision differs");
    const expectedIdentity = request.args.authority?.identity ?? request.args.identity;
    if (digest(value.identity) !== digest(expectedIdentity)) {
      throw new RustReviewEpisodeError("Protocol", "native state identity differs");
    }
    if (value.authority.grantId !== request.grantId
        || value.authority.source.revision !== request.selectionRevision) {
      throw new RustReviewEpisodeError("Protocol", "native state admission binding differs");
    }
  };
  if (result) {
    try {validateImplementationReviewResult(response.result);} catch {
      throw new RustReviewEpisodeError("Protocol", "native result is invalid");
    }
    if (response.observedRevision === null) throw new RustReviewEpisodeError("Protocol", "native result lacks observed revision");
    if (digest(response.result) !== digest(request.args.result)) {
      throw new RustReviewEpisodeError("Protocol", "native result differs from requested result");
    }
  } else if (history) {
    if (!Array.isArray(response.history)) throw new RustReviewEpisodeError("Protocol", "native history is invalid");
    response.history.forEach(state);
    if ((response.history.at(-1)?.revision ?? null) !== response.observedRevision) {
      throw new RustReviewEpisodeError("Protocol", "native history revision differs");
    }
  } else {
    if (response.state !== null) state(response.state);
    if (["begin", "transition", "resumeInitial"].includes(operation)
        && response.state === null) throw new RustReviewEpisodeError("Protocol", "native state is missing");
    if (operation === "begin" || operation === "transition") {
      const contentDigest = operation === "begin"
        ? digest({action: "begin", unresolvedQuestions: request.args.unresolvedQuestions})
        : digest({action: request.args.action, payload: request.args.payload});
      if (response.state.handledTransitions[request.args.transitionId] !== contentDigest
          || digest(response.state.writer) !== digest(request.args.authority.writer)) {
        throw new RustReviewEpisodeError("Protocol", "native write result differs from admitted command");
      }
    }
    if (operation === "read" && request.args.revision !== null && response.state !== null
        && response.state.revision !== request.args.revision) {
      throw new RustReviewEpisodeError("Protocol", "native historical revision differs");
    }
    if ((operation !== "read" || request.args.revision === null)
        && (response.state?.revision ?? null) !== response.observedRevision) {
      throw new RustReviewEpisodeError("Protocol", "native current revision differs");
    }
    if (recovery) {
      if (response.committedRevision !== null && !HEX.test(response.committedRevision)
          || (response.status === "committed") !== (response.committedRevision !== null)) {
        throw new RustReviewEpisodeError("Protocol", "native recovery status differs");
      }
      const handled = response.state?.handledTransitions?.[request.args.transitionId];
      if (response.status === "committed" && handled !== request.args.contentDigest
          || response.status === "absent" && handled !== undefined) {
        throw new RustReviewEpisodeError("Protocol", "native recovery content differs");
      }
    }
  }
  return response;
}

export function createRustReviewEpisodeService({binaryPath, expectedSha256, root,
  implementationReview, timeoutMs = 250} = {}) {
  const marker = selected(binaryPath, root, expectedSha256);
  if (!implementationReview?.admit) throw new TypeError("native review episode requires implementation-review admission");
  let campaignForIdentity = null;
  const bindCampaignOwner = (resolve) => {
    if (campaignForIdentity || typeof resolve !== "function") throw new TypeError("native campaign owner may be bound once");
    campaignForIdentity = resolve;
  };
  const scope = (identity, authority = null) => {
    validateIdentity(identity);
    if (!campaignForIdentity) throw new RustReviewEpisodeError("Admission", "native campaign owner is unbound");
    const campaign = campaignForIdentity(identity);
    const selection = campaign?.reviewSelection;
    const disposition = selection?.specialists?.find((item) => item.obligationId === identity.reviewObligationId);
    if (!campaign || digest(campaign.identity) !== digest((( {reviewObligationId, reviewEpisodeId, ...value}) => value)(identity))
        || disposition?.selection !== "selected"
        || identity.reviewEpisodeId !== digest({identity: campaign.identity, obligationId: identity.reviewObligationId}).slice(0, 32)) {
      throw new RustReviewEpisodeError("Admission", "native episode is outside selected campaign scope");
    }
    const selectionRevision = digest(selection);
    if (authority) {
      validateAuthority(authority);
      if (digest(authority.identity) !== digest(identity)
          || authority.source.reference !== selection.selectionId
          || authority.source.revision !== selectionRevision
          || authority.source.sha256 !== selectionRevision) {
        throw new RustReviewEpisodeError("Admission", "native authority differs from selected campaign");
      }
    }
    return {selectionRevision, grantId: authority?.grantId ?? `grant:native-review:${digest({identity: campaign.identity, obligationId: identity.reviewObligationId})}`};
  };
  const invoke = (operation, args, {authority = null, observed = null} = {}) => {
    const identity = authority?.identity ?? args.identity;
    const {selectionRevision, grantId} = scope(identity, authority);
    const request = {version: 2, domain: "review-episode", profile: "native-host-v1",
      requestId: randomUUID(), operation, grantId, selectionRevision,
      rootSelectionDigest: marker.selectionDigest, args};
    const payload = args.payload;
    const content = operation === "begin" ? {action: "begin", unresolvedQuestions: args.unresolvedQuestions}
      : operation === "transition" ? {action: args.action, payload} : null;
    const access = ["read", "history", "recover"].includes(operation) ? "read" : "write";
    const descriptor = {schemaVersion: 1, profile: "native-host-v1", root,
      executableSha256: expectedSha256, rootSelectionDigest: marker.selectionDigest,
      requestId: request.requestId, operation, grantId, selectionRevision,
      requestDigest: digest(request), identityKey: identityKey(identity), argsDigest: digest(args),
      authorityDigest: authority ? digest(authority) : null,
      resultDigest: args.result !== undefined ? digest(args.result)
        : payload?.result !== undefined ? digest(payload.result) : null,
      evidenceDigest: payload?.evidenceAdmissions !== undefined ? digest(payload.evidenceAdmissions) : null,
      contentDigest: content === null ? null : digest(content), observedRevision: observed,
      principal: {id: "native-review-host", identityKey: identityKey(identity), access}};
    const possibleCommit = access === "write" && ["begin", "transition"].includes(operation)
      ? {identity: structuredClone(identity), operation, transitionId: args.transitionId,
        contentDigest: descriptor.contentDigest, requestId: request.requestId,
        rootSelectionDigest: marker.selectionDigest} : null;
    const fd = fdForDescriptor(descriptor);
    try {
      const result = spawnSync(binaryPath, ["native-host-v1", "--root", root], {
        input: frame(request), stdio: ["pipe", "pipe", "pipe", fd], timeout: timeoutMs,
        maxBuffer: MAX_RESPONSE + 4, env: process.env,
      });
      if (result.error || result.signal || result.status !== 0) {
        throw new RustReviewEpisodeError("Transport", "native review command did not complete", {
          transport: {code: result.error?.code ?? null, signal: result.signal ?? null,
            exitStatus: result.status ?? null, timedOut: result.error?.code === "ETIMEDOUT"},
          possibleCommit});
      }
      try { return decode(result.stdout, request); }
      catch (error) {
        if (possibleCommit && error instanceof RustReviewEpisodeError
            && ["Transport", "Protocol", "OutcomeUnknown"].includes(error.kind)) {
          error.possibleCommit = freeze(possibleCommit);
        }
        throw error;
      }
    } finally { closeSync(fd); }
  };
  const latest = (identity) => invoke("read", {identity, revision: null}).state;
  const write = (operation, args) => {
    const identity = args.authority.identity;
    const current = latest(identity);
    return freeze(invoke(operation, args, {authority: args.authority, observed: current?.revision ?? null}).state);
  };
  return Object.freeze({
    bindCampaignOwner,
    begin({authority, transitionId, unresolvedQuestions = []}) {
      return write("begin", {authority, transitionId, unresolvedQuestions});
    },
    transition({authority, expectedRevision, transitionId, action, payload}) {
      if (action === "succeed_evidence") {
        throw new RustReviewEpisodeError("Unavailable", "native production-path succession awaits RC admission");
      }
      return write("transition", {authority, expectedRevision, transitionId, action, payload});
    },
    resumeInitial({authority}) {
      return freeze(invoke("resumeInitial", {authority}, {authority, observed: latest(authority.identity)?.revision ?? null}).state);
    },
    validateResult({authority, expectedRevision, result}) {
      implementationReview.admit({result, expectedSubject: result.subject});
      return freeze(invoke("validateResult", {authority, expectedRevision, result},
        {authority, observed: latest(authority.identity)?.revision ?? null}).result);
    },
    recover(identity) { return freeze(invoke("recover", {identity}).state); },
    reconcilePossibleCommit(error) {
      const pending = error instanceof RustReviewEpisodeError ? error.possibleCommit : null;
      if (!pending || pending.rootSelectionDigest !== marker.selectionDigest
          || !["begin", "transition"].includes(pending.operation)
          || typeof pending.transitionId !== "string" || !HEX.test(pending.contentDigest)) {
        throw new RustReviewEpisodeError("Admission", "exact possible-commit context is required");
      }
      const response = invoke("recover", {identity: pending.identity,
        transitionId: pending.transitionId, contentDigest: pending.contentDigest});
      return freeze({status: response.status, committedRevision: response.committedRevision,
        observedRevision: response.observedRevision, state: response.state});
    },
    read({identity, revision = null}) { return freeze(invoke("read", {identity, revision}).state); },
    history({identity}) { return freeze(invoke("history", {identity}).history); },
  });
}
