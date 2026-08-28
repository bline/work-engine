#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

import { compileSkill } from "../src/skill-compiler.mjs";

function usage(message) {
  if (message) process.stderr.write(`${message}\n`);
  process.stderr.write("usage: compile-skill.mjs --structure PATH --interface PATH --output PATH [--compare PATH] [--workspace-root PATH]\n");
  process.exit(2);
}

const options = {};
for (let index = 2; index < process.argv.length; index += 2) {
  const flag = process.argv[index];
  const value = process.argv[index + 1];
  if (!value || !["--structure", "--interface", "--output", "--compare", "--workspace-root"].includes(flag)) usage(`unknown or incomplete option ${flag}`);
  options[flag.slice(2)] = value;
}
for (const required of ["structure", "interface", "output"]) if (!options[required]) usage(`missing --${required}`);

const workspaceRoot = path.resolve(options["workspace-root"] ?? process.cwd());
const result = await compileSkill({
  structureSource: await readFile(path.resolve(options.structure), "utf8"),
  interfaceSource: await readFile(path.resolve(options.interface), "utf8"),
  workspaceRoot,
});
await writeFile(path.resolve(options.output), result.output);
if (options.compare) {
  const expected = await readFile(path.resolve(options.compare));
  if (!expected.equals(result.output)) {
    process.stderr.write(`generated output differs from ${options.compare}\n`);
    process.exitCode = 1;
  }
}
process.stdout.write(`${JSON.stringify({
  compiler: result.ir.compiler,
  status: result.ir.status,
  source: result.ir.source,
  input_sha256: result.ir.input_sha256,
  output_sha256: result.ir.output_sha256,
  section_count: result.ir.section_provenance.length,
})}\n`);
