#!/usr/bin/env node
import {readFile, writeFile} from "node:fs/promises";
import path from "node:path";
import process from "node:process";

import {classifySkillsMigrationIntegrity} from "../src/services/skills-migration-integrity/attribution.mjs";

function parseArgs(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    const name = argv[index];
    const value = argv[index + 1];
    if (!name?.startsWith("--") || value === undefined) throw new Error("arguments must be --name value pairs");
    if (values.has(name)) throw new Error(`duplicate argument ${name}`);
    values.set(name, value);
  }
  for (const name of ["--repository", "--baseline", "--candidate", "--candidate-request"]) {
    if (!values.get(name)) throw new Error(`${name} is required`);
  }
  for (const name of values.keys()) {
    if (!["--repository", "--baseline", "--candidate", "--candidate-request", "--output"].includes(name)) {
      throw new Error(`unknown argument ${name}`);
    }
  }
  return values;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const requestPath = path.resolve(args.get("--candidate-request"));
  const request = JSON.parse(await readFile(requestPath, "utf8"));
  if (!Array.isArray(request?.paths)) throw new Error("candidate request paths are invalid");
  const receipt = classifySkillsMigrationIntegrity({
    repository: path.resolve(args.get("--repository")),
    baselineRevision: args.get("--baseline"),
    candidateRevision: args.get("--candidate"),
    acceptedPaths: request.paths.map(({path: filePath}) => filePath),
  });
  const output = `${JSON.stringify(receipt, null, 2)}\n`;
  if (args.has("--output")) {
    await writeFile(path.resolve(args.get("--output")), output, {flag: "wx", mode: 0o600});
  }
  process.stdout.write(output);
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});
