#!/usr/bin/env node

import { createHash } from "node:crypto";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  AppServerProtocolProxy,
  ObservableAppServerTransport,
  StdioJsonRpcTransport,
  createExecutableGenerationBootstrap,
  formatAppServerProtocolEvent,
} from "../src/index.mjs";

const WORKSPACE_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

function defaultGenerationState(socketPath) {
  const socketIdentity = createHash("sha256").update(socketPath).digest("hex").slice(0, 16);
  return path.join(
    os.homedir(),
    ".local",
    "state",
    "work-engine",
    "app-server-proxy",
    socketIdentity,
  );
}

function parseArguments(argv) {
  const options = {
    socketPath: null,
    cwd: process.cwd(),
    manifestPath: path.join(WORKSPACE_ROOT, "app-server/runtime-manifest.yaml"),
    bindingsPath: process.env.WORK_ENGINE_APP_SERVER_BINDINGS
      ?? path.join(os.homedir(), ".local/state/work-engine/app-server-role-bindings.json"),
    trace: false,
    tokenBudget: false,
    generationState: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const value = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`${argument} requires a value`);
      return argv[index];
    };
    if (argument === "--socket") options.socketPath = path.resolve(value());
    else if (argument === "--cwd") options.cwd = path.resolve(value());
    else if (argument === "--manifest") options.manifestPath = path.resolve(value());
    else if (argument === "--bindings") options.bindingsPath = path.resolve(value());
    else if (argument === "--trace") options.trace = true;
    else if (argument === "--enable-token-budget") options.tokenBudget = true;
    else if (argument === "--generation-state") options.generationState = path.resolve(value());
    else throw new Error(`unknown App Server proxy option ${argument}`);
  }
  if (!options.socketPath) throw new Error("--socket PATH is required");
  options.generationState ??= defaultGenerationState(options.socketPath);
  return options;
}

function formatStartupFailure(error) {
  const code = typeof error?.code === "string" ? ` code=${error.code}` : "";
  const details = error?.details && typeof error.details === "object"
    ? ` details=${JSON.stringify(error.details)}`
    : "";
  return `${error?.message ?? String(error)}${code}${details}`;
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  process.stderr.write(`[host] state=${options.generationState}\n`);
  const delegate = StdioJsonRpcTransport.spawn({
    cwd: options.cwd,
    ...(options.tokenBudget
      ? { args: ["app-server", "--stdio", "--enable", "token_budget"] }
      : {}),
  });
  process.stderr.write("[startup] app-server-child=starting\n");
  await delegate.ready();
  process.stderr.write("[startup] app-server-child=spawned generation=validating\n");
  const transport = new ObservableAppServerTransport({
    transport: delegate,
    ...(options.trace ? {
      onEvent: (event) => process.stderr.write(
        `[app-server] ${formatAppServerProtocolEvent(event)}\n`,
      ),
    } : {}),
  });
  let stopping = false;
  transport.onClosed((error) => {
    if (stopping) return;
    process.stderr.write(`App Server transport closed: ${error.message}\n`);
    process.exitCode = 1;
  });
  let generationBootstrap;
  try {
    generationBootstrap = await createExecutableGenerationBootstrap({
      workspaceRoot: WORKSPACE_ROOT,
      stateRoot: options.generationState,
      workerCwd: options.cwd,
      transport,
      runtimeManifestPath: options.manifestPath,
      roleBindingsPath: options.bindingsPath,
      configuredProviderFeatures: options.tokenBudget ? ["token_budget"] : [],
    });
  } catch (error) {
    transport.close();
    throw error;
  }
  const selection = generationBootstrap.startupSelection;
  process.stderr.write(
    `[host] workspace=${selection.workspaceGenerationId} selected=${selection.selectedGenerationId} outcome=${selection.outcome}${selection.reconciliationId ? ` reconciliation=${selection.reconciliationId}` : ""}\n`,
  );
  process.stderr.write("[startup] generation=active proxy-socket=opening\n");
  const proxy = new AppServerProtocolProxy({
    transport: generationBootstrap.transport,
    socketPath: options.socketPath,
  });
  proxy.on("protocolError", (error) => {
    process.stderr.write(`App Server proxy protocol error: ${error.message}\n`);
  });
  proxy.on("requestError", ({ method, error }) => {
    process.stderr.write(`App Server proxy request failed (${method}): ${error.stack ?? error.message}\n`);
  });
  if (options.trace) {
    proxy.on("clientRequest", ({ id, method }) => {
      process.stderr.write(`[proxy-client] request id=${id} method=${method}\n`);
    });
    proxy.on("clientResponse", ({ id, method, threadId, turnId, turnStatus }) => {
      const subject = [
        threadId ? `thread=${threadId}` : null,
        turnId ? `turn=${turnId}` : null,
        turnStatus ? `status=${turnStatus}` : null,
      ].filter(Boolean).join(" ");
      process.stderr.write(`[proxy-client] response id=${id} method=${method}${subject ? ` ${subject}` : ""}\n`);
    });
  }
  try {
    await proxy.listen();
  } catch (error) {
    await generationBootstrap.close({ abandonActiveWork: true });
    transport.close();
    throw error;
  }
  process.stdout.write(`Work Engine App Server proxy listening at unix://${options.socketPath}\n`);

  const stop = async () => {
    if (stopping) return;
    stopping = true;
    await proxy.close();
    await generationBootstrap.close({ abandonActiveWork: true });
  };
  process.once("SIGINT", () => stop().catch((error) => {
    process.stderr.write(`${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  }));
  process.once("SIGTERM", () => stop().catch((error) => {
    process.stderr.write(`${error.stack ?? error.message}\n`);
    process.exitCode = 1;
  }));
}

main().catch((error) => {
  process.stderr.write(`App Server host startup failed: ${formatStartupFailure(error)}\n`);
  if (process.env.WORK_ENGINE_STARTUP_STACK === "1" && error?.stack) {
    process.stderr.write(`${error.stack}\n`);
  }
  process.exitCode = 1;
});
