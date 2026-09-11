#!/usr/bin/env node

import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { realpath, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

import {
  AppServerProtocolProxy,
  ObservableAppServerTransport,
  PINNED_PROTOCOL,
  StdioJsonRpcTransport,
  assertCompatibleCodexCliOutput,
  createExecutableGenerationBootstrap,
  formatAppServerProtocolEvent,
} from "../src/index.mjs";
import { createSupervisorCampaignCapabilityHostRuntime } from "../src/services/slice-campaign/capability-host-runtime.mjs";

const WORKSPACE_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const execFileAsync = promisify(execFile);

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
    semanticProfilePath: null,
    bindingsPath: process.env.WORK_ENGINE_APP_SERVER_BINDINGS
      ?? path.join(os.homedir(), ".local/state/work-engine/app-server-role-bindings.json"),
    trace: false,
    tokenBudget: false,
    generationState: null,
    operationalState: null,
    operationalStateExplicit: false,
    developmentArtifactRoot: WORKSPACE_ROOT,
    codexCommand: process.env.WORK_ENGINE_CODEX ?? "codex",
    codexHome: process.env.WORK_ENGINE_CODEX_HOME
      ? path.resolve(process.env.WORK_ENGINE_CODEX_HOME)
      : null,
    claudeLoginCredentials: null,
    canonicalBranches: [],
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
    else if (argument === "--semantic-profile") {
      options.semanticProfilePath = path.resolve(value());
    }
    else if (argument === "--trace") options.trace = true;
    else if (argument === "--enable-token-budget") options.tokenBudget = true;
    else if (argument === "--generation-state") options.generationState = path.resolve(value());
    else if (argument === "--operational-state") {
      options.operationalState = path.resolve(value());
      options.operationalStateExplicit = true;
    }
    else if (argument === "--development-artifact-root") {
      options.developmentArtifactRoot = path.resolve(value());
    }
    else if (argument === "--codex") options.codexCommand = value();
    else if (argument === "--codex-home") options.codexHome = path.resolve(value());
    else if (argument === "--claude-login-credentials") {
      options.claudeLoginCredentials = path.resolve(value());
    }
    else if (argument === "--canonical-branch") options.canonicalBranches.push(value());
    else throw new Error(`unknown App Server proxy option ${argument}`);
  }
  if (!options.socketPath) throw new Error("--socket PATH is required");
  if (options.canonicalBranches.length === 0) {
    throw new Error("at least one explicit --canonical-branch NAME is required");
  }
  options.semanticProfilePath ??= path.join(
    WORKSPACE_ROOT,
    options.tokenBudget
      ? "app-server/semantic-context-live-profile.yaml"
      : "app-server/semantic-context-profile.yaml",
  );
  options.generationState ??= defaultGenerationState(options.socketPath);
  options.operationalState ??= options.generationState;
  options.codexHome ??= path.join(options.operationalState, "codex-home");
  return options;
}

async function validateExplicitOperationalState(options) {
  if (!options.operationalStateExplicit) return;
  const campaignStatePath = path.join(options.operationalState, "slice-campaign.sqlite3");
  let rootMetadata;
  try {
    rootMetadata = await stat(options.operationalState);
  } catch (error) {
    throw new Error(
      `explicit --operational-state must select an existing supervisor state root: ${options.operationalState}`,
      { cause: error },
    );
  }
  if (!rootMetadata.isDirectory()) {
    throw new Error(
      `explicit --operational-state must select a directory: ${options.operationalState}`,
    );
  }
  let campaignMetadata;
  try {
    campaignMetadata = await stat(campaignStatePath);
  } catch (error) {
    throw new Error(
      `explicit --operational-state must contain an existing ${JSON.stringify("slice-campaign.sqlite3")} file: ${options.operationalState}`,
      { cause: error },
    );
  }
  if (!campaignMetadata.isFile()) {
    throw new Error(
      `explicit --operational-state must contain a regular ${JSON.stringify("slice-campaign.sqlite3")} file: ${options.operationalState}`,
    );
  }
  options.operationalState = await realpath(options.operationalState);
}

async function validateClaudeLoginCredentials(options) {
  if (options.claudeLoginCredentials === null) return;
  let metadata;
  try {
    metadata = await stat(options.claudeLoginCredentials);
  } catch (error) {
    throw new Error(
      `--claude-login-credentials must select an existing credential file: ${options.claudeLoginCredentials}`,
      { cause: error },
    );
  }
  if (!metadata.isFile() || (metadata.mode & 0o077) !== 0) {
    throw new Error("--claude-login-credentials must select an owner-only regular file");
  }
  options.claudeLoginCredentials = await realpath(options.claudeLoginCredentials);
}

async function validateCodexHome(options) {
  let homeMetadata;
  let resolvedHome;
  try {
    [homeMetadata, resolvedHome] = await Promise.all([
      stat(options.codexHome),
      realpath(options.codexHome),
    ]);
  } catch (error) {
    throw new Error(
      `dedicated --codex-home must select an existing private directory: ${options.codexHome}`,
      { cause: error },
    );
  }
  if (!homeMetadata.isDirectory() || (homeMetadata.mode & 0o077) !== 0) {
    throw new Error("dedicated --codex-home must be a directory inaccessible to group and other");
  }
  const authPath = path.join(resolvedHome, "auth.json");
  let authMetadata;
  let resolvedAuth;
  try {
    [authMetadata, resolvedAuth] = await Promise.all([stat(authPath), realpath(authPath)]);
  } catch (error) {
    throw new Error(
      "dedicated --codex-home must contain a private auth.json credential projection",
      { cause: error },
    );
  }
  if (path.dirname(resolvedAuth) !== resolvedHome) {
    throw new Error("dedicated --codex-home auth.json must not resolve outside its home");
  }
  if (!authMetadata.isFile() || (authMetadata.mode & 0o077) !== 0) {
    throw new Error("dedicated --codex-home auth.json must be a private regular file");
  }
  options.codexHome = resolvedHome;
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
  await validateExplicitOperationalState(options);
  await validateClaudeLoginCredentials(options);
  let versionOutput;
  try {
    ({ stdout: versionOutput } = await execFileAsync(options.codexCommand, ["--version"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024,
    }));
    assertCompatibleCodexCliOutput(versionOutput);
  } catch (error) {
    throw new Error(
      `selected Codex executable ${JSON.stringify(options.codexCommand)} is unavailable or incompatible; `
      + `select codex-cli ${PINNED_PROTOCOL.codexCliVersion} with --codex PATH: `
      + `${error?.message ?? "version check failed"}`,
      { cause: error },
    );
  }
  await validateCodexHome(options);
  process.stderr.write(
    `[host] generation-state=${options.generationState} operational-state=${options.operationalState} codex-home=${options.codexHome}\n`,
  );
  const delegate = StdioJsonRpcTransport.spawn({
    command: options.codexCommand,
    cwd: options.cwd,
    env: { ...process.env, CODEX_HOME: options.codexHome },
    args: [
      "app-server", "--stdio",
      "--disable", "apps",
      "--disable", "multi_agent",
      "--disable", "plugins",
      "--disable", "remote_plugin",
      "--disable", "skill_mcp_dependency_install",
      "-c", "agents.enabled=false",
      "-c", "project_doc_max_bytes=0",
      ...(options.tokenBudget ? ["--enable", "token_budget"] : []),
    ],
  });
  if (options.trace) {
    delegate.on("stderr", (chunk) => process.stderr.write(`[app-server-child] ${chunk}`));
  }
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
      semanticContextProfilePath: options.semanticProfilePath,
      semanticContextStatePath: path.join(options.generationState, "semantic-context.sqlite3"),
      roleBindingsPath: options.bindingsPath,
      configuredProviderFeatures: options.tokenBudget ? ["token_budget"] : [],
      developmentArtifactRoot: options.developmentArtifactRoot,
      supervisorCampaignHostEffectRuntimeFactory: ({ workspaceRoot }) =>
        createSupervisorCampaignCapabilityHostRuntime({
          workspaceRoot,
          stateRoot: options.operationalState,
          canonicalBranches: options.canonicalBranches,
          reviewerCredentialSourcePath: options.claudeLoginCredentials,
        }),
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
    operatorControl: generationBootstrap.transport.operatorControl(),
    socketPath: options.socketPath,
  });
  proxy.on("protocolError", (error) => {
    process.stderr.write(`App Server proxy protocol error: ${error.message}\n`);
  });
  proxy.on("requestError", ({ method, error }) => {
    process.stderr.write(`App Server proxy request failed (${method}): ${error.stack ?? error.message}\n`);
  });
  proxy.on("controlError", (error) => {
    process.stderr.write(`App Server operator control failed: ${error.message}\n`);
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
    proxy.on("controlRequest", ({ command }) => {
      process.stderr.write(`[operator-control] request command=${command}\n`);
    });
    proxy.on("controlResponse", ({ command, status }) => {
      process.stderr.write(`[operator-control] response command=${command} status=${status}\n`);
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
