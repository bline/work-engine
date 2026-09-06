#!/usr/bin/env node

import http from "node:http";
import path from "node:path";

const CONTROL_PATH = "/work-engine/control";
const RESPONSE_BODY_LIMIT = 64 * 1024;

function usage() {
  return [
    "Usage:",
    "  npm run app-server:control -- --socket PATH status",
    "  npm run app-server:control -- --socket PATH interrupt [--thread ID] [--turn ID]",
    "  npm run app-server:control -- --socket PATH disconnect",
  ].join("\n");
}

function parseArguments(argv) {
  let socketPath = null;
  let command = null;
  const target = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const value = () => {
      index += 1;
      if (index >= argv.length) throw new Error(`${argument} requires a value`);
      return argv[index];
    };
    if (argument === "--socket") socketPath = path.resolve(value());
    else if (argument === "--thread") target.threadId = value();
    else if (argument === "--turn") target.turnId = value();
    else if (!argument.startsWith("-") && command === null) command = argument;
    else throw new Error(`unknown App Server control option ${argument}`);
  }
  if (!socketPath) throw new Error("--socket PATH is required");
  if (!["status", "interrupt", "disconnect"].includes(command)) {
    throw new Error("command must be status, interrupt, or disconnect");
  }
  if (command !== "interrupt" && Object.keys(target).length > 0) {
    throw new Error(`${command} does not accept --thread or --turn`);
  }
  return { socketPath, command, ...(command === "interrupt" ? { target } : {}) };
}

function requestControl({ socketPath, command, target }) {
  const body = Buffer.from(JSON.stringify({
    command,
    ...(target ? { target } : {}),
  }), "utf8");
  return new Promise((resolve, reject) => {
    const request = http.request({
      socketPath,
      path: CONTROL_PATH,
      method: "POST",
      headers: {
        "Content-Length": String(body.length),
        "Content-Type": "application/json",
      },
    }, (response) => {
      const chunks = [];
      let length = 0;
      response.on("data", (chunk) => {
        length += chunk.length;
        if (length > RESPONSE_BODY_LIMIT) {
          response.destroy(new Error("App Server control response exceeds 64 KiB"));
          return;
        }
        chunks.push(chunk);
      });
      response.once("error", reject);
      response.once("end", () => {
        let payload;
        try {
          payload = JSON.parse(Buffer.concat(chunks).toString("utf8"));
        } catch {
          reject(new Error("App Server control returned invalid JSON"));
          return;
        }
        if ((response.statusCode ?? 500) < 200 || response.statusCode >= 300 || !payload?.ok) {
          const error = new Error(payload?.error?.message ?? `control failed with HTTP ${response.statusCode}`);
          error.code = payload?.error?.code ?? "operator_control_failed";
          reject(error);
          return;
        }
        resolve(payload.result);
      });
    });
    request.setTimeout(10_000, () => request.destroy(new Error("App Server control timed out")));
    request.once("error", reject);
    request.end(body);
  });
}

async function main() {
  const options = parseArguments(process.argv.slice(2));
  const result = await requestControl(options);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

main().catch((error) => {
  process.stderr.write(`App Server control failed${error?.code ? ` (${error.code})` : ""}: ${error?.message ?? error}\n`);
  process.stderr.write(`${usage()}\n`);
  process.exitCode = 1;
});
