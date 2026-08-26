#!/usr/bin/env node

import { createInterface } from "node:readline";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  CodexAppServerAdapter,
  ExactSkillResolver,
  FileRoleBindingRegistry,
  MODEL_CONTEXT_REPLACEMENT_CAPABILITY,
  ManifestRoleRuntime,
  ObservableAppServerTransport,
  OperatorSwitchboard,
  renderOperatorSwitchboardResult,
  StdioJsonRpcTransport,
  createLocalSemanticShadowHost,
  formatAppServerProtocolEvent,
  loadRuntimeManifest,
  loadSemanticContextRuntimeProfile,
} from "../src/index.mjs";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, "../..");

function parseArguments(argv) {
  const options = {
    manifestPath: path.join(ROOT, "app-server/runtime-manifest.yaml"),
    bindingsPath: process.env.WORK_ENGINE_APP_SERVER_BINDINGS
      ?? path.join(os.homedir(), ".local/state/work-engine/app-server-role-bindings.json"),
    cwd: ROOT,
    skillRoots: [path.join(ROOT, "skills")],
    trace: false,
    tokenBudget: false,
    semanticShadow: false,
    semanticProfilePath: path.join(ROOT, "app-server/semantic-context-profile.yaml"),
    semanticStatePath: process.env.WORK_ENGINE_APP_SERVER_STATE
      ?? path.join(os.homedir(), ".local/state/work-engine/app-server-state.sqlite3"),
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const value = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`${argument} requires a value`);
      return argv[index];
    };
    if (argument === "--manifest") options.manifestPath = path.resolve(value());
    else if (argument === "--bindings") options.bindingsPath = path.resolve(value());
    else if (argument === "--cwd") options.cwd = path.resolve(value());
    else if (argument === "--skill-root") options.skillRoots.push(path.resolve(value()));
    else if (argument === "--trace") options.trace = true;
    else if (argument === "--enable-token-budget") options.tokenBudget = true;
    else if (argument === "--semantic-shadow") options.semanticShadow = true;
    else if (argument === "--semantic-profile") {
      options.semanticProfilePath = path.resolve(value());
    } else if (argument === "--semantic-state") {
      options.semanticStatePath = path.resolve(value());
    }
    else throw new Error(`unknown switchboard option ${argument}`);
  }
  return options;
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  const delegate = StdioJsonRpcTransport.spawn({
    cwd: options.cwd,
    ...(options.tokenBudget
      ? { args: ["app-server", "--stdio", "--enable", "token_budget"] }
      : {}),
  });
  const transport = new ObservableAppServerTransport({
    transport: delegate,
    ...(options.trace ? {
      onEvent: (event) => process.stderr.write(
        `[app-server] ${formatAppServerProtocolEvent(event)}\n`,
      ),
    } : {}),
  });
  const registry = new FileRoleBindingRegistry(options.bindingsPath);
  const adapter = new CodexAppServerAdapter({
    transport,
    registry,
    skillResolver: await ExactSkillResolver.create(options.skillRoots),
    configuredProviderFeatures: options.tokenBudget ? ["token_budget"] : [],
  });
  await adapter.initialize({
    requiredProviderCapabilities: options.tokenBudget
      ? [MODEL_CONTEXT_REPLACEMENT_CAPABILITY]
      : [],
  });
  const manifest = await loadRuntimeManifest(options.manifestPath);
  let semanticHost = null;
  let runtime;
  if (options.semanticShadow) {
    const profile = await loadSemanticContextRuntimeProfile(options.semanticProfilePath);
    semanticHost = await createLocalSemanticShadowHost({
      adapter,
      manifest,
      stateFilePath: options.semanticStatePath,
      profile,
      inferenceThreadOptions: {
        cwd: options.cwd,
        approvalPolicy: "never",
        sandbox: "read-only",
      },
      onLifecycleEvidenceError: (error) => {
        process.stderr.write(`lifecycle evidence error: ${error.message}\n`);
      },
    });
    runtime = semanticHost.runtime;
  } else {
    runtime = new ManifestRoleRuntime({ adapter, manifest });
  }
  const switchboard = new OperatorSwitchboard({
    manifest,
    runtime,
    registry,
    observer: transport,
    completionWaiter: options.semanticShadow
      ? null
      : (delivery) => adapter.waitForTurnCompletion(delivery),
  });
  const terminal = Boolean(process.stdin.isTTY && process.stdout.isTTY);
  const input = createInterface({ input: process.stdin, output: process.stdout, terminal });
  process.stdout.write(
    `Work Engine switchboard ready (${manifest.manifestId}; ${semanticHost?.mode ?? "direct"}). Use ::agents and ::attach role:instance.\n`,
  );
  const iterator = input[Symbol.asyncIterator]();
  try {
    while (true) {
      if (terminal) process.stdout.write("work-engine> ");
      const { value, done } = await iterator.next();
      if (done) break;
      try {
        process.stdout.write(`${renderOperatorSwitchboardResult(
          await switchboard.handleLine(value),
        )}\n`);
      } catch (error) {
        process.stderr.write(`switchboard error: ${error.message}\n`);
      }
    }
  } finally {
    input.close();
    semanticHost?.close();
    transport.close();
  }
}

main().catch((error) => {
  process.stderr.write(`switchboard startup failed: ${error.stack ?? error.message}\n`);
  process.exitCode = 1;
});
