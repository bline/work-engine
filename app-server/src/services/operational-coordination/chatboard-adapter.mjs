import { execFile } from "node:child_process";
import path from "node:path";
import { promisify } from "node:util";

const executeFile = promisify(execFile);

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} is invalid`);
  }
  return value;
}

function revision(value) {
  if (value !== null && (typeof value !== "string" || !value.trim())) {
    throw new Error("chatboard result revision is invalid");
  }
}

function validateResult(operation, input, value) {
  record(value, "chatboard result");
  revision(value.revision);
  if (operation === "read") {
    if (!Array.isArray(value.messages) || !value.claims || typeof value.claims !== "object"
        || Array.isArray(value.claims)) throw new Error("chatboard read result is invalid");
  } else if (operation === "claim") {
    record(value.claim, "chatboard claim result");
    if (value.claim.resource !== input.resource || value.claim.claim_id !== input.claim_id
        || value.claim.session_id !== input.session_id
        || value.claim.authority !== "advisory_coordination_only") {
      throw new Error("chatboard claim result identity is invalid");
    }
  } else if (operation === "post") {
    record(value.message, "chatboard message result");
    if (value.message.message_id !== input.message_id
        || value.message.session_id !== input.session_id) {
      throw new Error("chatboard message result identity is invalid");
    }
  } else if (value.resource !== input.resource || typeof value.released !== "boolean") {
    throw new Error("chatboard release result identity is invalid");
  }
  return Object.freeze(structuredClone(value));
}

function argumentsFor(operation, input) {
  if (operation === "read") return ["read", "--since", String(input.since), "--limit", String(input.limit)];
  if (operation === "claim") return [
    "claim", "--resource", input.resource, "--author", input.author,
    "--session-id", input.session_id, "--claim-id", input.claim_id,
    "--ttl-seconds", String(input.ttl_seconds), "--note", input.note,
  ];
  if (operation === "post") return [
    "post", "--author", input.author, "--session-id", input.session_id,
    "--topic", input.topic, "--body", input.body, "--message-id", input.message_id,
    ...input.references.flatMap((reference) => ["--ref", reference]),
  ];
  return [
    "release", "--resource", input.resource, "--session-id", input.session_id,
    "--claim-id", input.claim_id,
  ];
}

async function recoverReplay({runFile, pythonExecutable, script, repository, operation, input}) {
  if (!["claim", "post"].includes(operation)) return null;
  let board;
  try {
    const output = await runFile(pythonExecutable,
      [script, "--repository", repository, "read", "--since", "0", "--limit", "500"],
      {encoding: "utf8", maxBuffer: 4 * 1024 * 1024});
    board = JSON.parse(output.stdout);
  } catch { return null; }
  if (operation === "claim") {
    const claim = board.claims?.[input.resource];
    const duration = claim ? (Date.parse(claim.expires_at) - Date.parse(claim.claimed_at)) / 1000 : null;
    if (claim?.claim_id === input.claim_id && claim.author === input.author
        && claim.session_id === input.session_id && claim.note === input.note
        && claim.authority === "advisory_coordination_only" && duration === input.ttl_seconds) {
      return {revision: board.revision, claim};
    }
    return null;
  }
  let since = 0;
  while (true) {
    if (!Array.isArray(board.messages)) return null;
    let previous = since;
    for (const message of board.messages) {
      if (!Number.isSafeInteger(message.sequence) || message.sequence <= previous) return null;
      previous = message.sequence;
      if (message.message_id === input.message_id) {
        if (message.author === input.author && message.session_id === input.session_id
            && message.topic === input.topic && message.body === input.body
            && JSON.stringify(message.references) === JSON.stringify(input.references)) {
          return {revision: board.revision, message};
        }
        return null;
      }
    }
    if (board.messages.length < 500) return null;
    if (previous <= since) return null;
    since = previous;
    try {
      const output = await runFile(pythonExecutable,
        [script, "--repository", repository, "read", "--since", String(since), "--limit", "500"],
        {encoding: "utf8", maxBuffer: 4 * 1024 * 1024});
      board = JSON.parse(output.stdout);
    } catch { return null; }
  }
}

export function createChatboardAdapter({
  workspaceRoot,
  pythonExecutable = "python3",
  runFile = executeFile,
} = {}) {
  if (typeof workspaceRoot !== "string" || !workspaceRoot.trim()) {
    throw new TypeError("chatboard adapter requires a workspace root");
  }
  if (typeof pythonExecutable !== "string" || !pythonExecutable.trim()) {
    throw new TypeError("chatboard adapter requires a Python executable");
  }
  if (typeof runFile !== "function") throw new TypeError("chatboard adapter requires an exec-file boundary");
  const repository = path.resolve(workspaceRoot);
  const script = path.join(repository, "skills/durable-state/scripts/codex_chatboard.py");
  return Object.freeze({
    async execute(operation, input) {
      if (!["read", "claim", "post", "release"].includes(operation)) {
        throw new Error("chatboard operation is unsupported");
      }
      let output;
      try {
        output = await runFile(pythonExecutable, [script, "--repository", repository,
          ...argumentsFor(operation, input)], {encoding: "utf8", maxBuffer: 4 * 1024 * 1024});
      } catch {
        const replay = await recoverReplay({runFile, pythonExecutable, script, repository,
          operation, input});
        if (replay) return validateResult(operation, input, replay);
        throw new Error(`canonical chatboard ${operation} operation failed`);
      }
      let value;
      try { value = JSON.parse(output.stdout); }
      catch { throw new Error("canonical chatboard returned malformed output"); }
      return validateResult(operation, input, value);
    },
    identity: Object.freeze({
      schema_version: 1,
      owner: "skills/durable-state/scripts/codex_chatboard.py",
      authority: "advisory_coordination_only",
      repository,
    }),
  });
}
