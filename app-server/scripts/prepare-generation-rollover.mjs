#!/usr/bin/env node

import { createHash } from "node:crypto";
import { constants as fsConstants } from "node:fs";
import { chmod, copyFile, lstat, mkdir, readFile, readdir, rm } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SEMANTIC_STATE = "semantic-context.sqlite3";

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function socketIsLive(socketPath) {
  return new Promise((resolve, reject) => {
    const socket = net.createConnection(socketPath);
    let settled = false;
    const finish = (value, error = null) => {
      if (settled) return;
      settled = true;
      socket.destroy();
      if (error) reject(error); else resolve(value);
    };
    socket.once("connect", () => finish(true));
    socket.once("error", (error) => {
      if (["ENOENT", "ECONNREFUSED"].includes(error?.code)) finish(false);
      else finish(false, error);
    });
    socket.setTimeout(500, () => finish(false,
      new Error(`could not establish that App Server socket is stopped: ${socketPath}`)));
  });
}

export async function prepareGenerationRollover({from, to, socket}) {
  const sourceRoot = path.resolve(from);
  const targetRoot = path.resolve(to);
  const socketPath = path.resolve(socket);
  if (sourceRoot === targetRoot) throw new Error("source and target generation roots must differ");
  if (await socketIsLive(socketPath)) {
    throw new Error(`App Server socket is active; stop the proxy before rollover: ${socketPath}`);
  }
  const sourceStat = await lstat(sourceRoot);
  if (!sourceStat.isDirectory()) throw new Error("source generation root must be a directory");
  const sourceState = path.join(sourceRoot, SEMANTIC_STATE);
  const sourceStateStat = await lstat(sourceState);
  if (!sourceStateStat.isFile()) throw new Error("source semantic-context state must be a regular file");
  try {
    await lstat(targetRoot);
    throw new Error("target generation root already exists");
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }

  const sourceBefore = await readFile(sourceState);
  const sourceDigest = sha256(sourceBefore);
  let created = false;
  try {
    await mkdir(targetRoot, {mode: 0o700});
    created = true;
    const targetState = path.join(targetRoot, SEMANTIC_STATE);
    await copyFile(sourceState, targetState, fsConstants.COPYFILE_EXCL);
    await chmod(targetState, 0o600);
    const [sourceAfter, targetBytes, entries] = await Promise.all([
      readFile(sourceState), readFile(targetState), readdir(targetRoot),
    ]);
    if (sha256(sourceAfter) !== sourceDigest || sha256(targetBytes) !== sourceDigest) {
      throw new Error("semantic-context state changed or failed integrity verification during rollover");
    }
    if (entries.length !== 1 || entries[0] !== SEMANTIC_STATE) {
      throw new Error("target generation root contains unexpected state");
    }
    await chmod(targetRoot, 0o700);
    return Object.freeze({schemaVersion: 1, sourceGenerationState: sourceRoot,
      targetGenerationState: targetRoot, semanticContextState: SEMANTIC_STATE,
      semanticContextSha256: sourceDigest, copiedGenerationIdentity: false,
      copiedRoleBindings: false});
  } catch (error) {
    if (created) await rm(targetRoot, {recursive: true, force: true});
    throw error;
  }
}

function parse(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 1) {
    const name = argv[index];
    if (!["--from", "--to", "--socket"].includes(name) || index + 1 >= argv.length) {
      throw new Error("usage: prepare-generation-rollover --from <old-root> --to <new-root> --socket <path>");
    }
    options[name.slice(2)] = argv[index += 1];
  }
  if (!options.from || !options.to || !options.socket) {
    throw new Error("usage: prepare-generation-rollover --from <old-root> --to <new-root> --socket <path>");
  }
  return options;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.stdout.write(`${JSON.stringify(await prepareGenerationRollover(parse(process.argv.slice(2))), null, 2)}\n`);
  } catch (error) {
    process.stderr.write(`Generation rollover failed: ${error.message}\n`);
    process.exitCode = 1;
  }
}
