#!/usr/bin/env node
import { readFile, open, lstat, chmod, rename, unlink } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import path from "node:path";
import process from "node:process";

import { compileSkill } from "../src/skill-compiler.mjs";

function strictUtf8(bytes, label) {
  try { return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes); }
  catch { throw new TypeError(`${label} must be UTF-8`); }
}

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

const controller = new AbortController();
const onSignal = (name) => controller.abort(new Error(`compiler interrupted by ${name}`));
const onInt = () => onSignal("SIGINT");
const onTerm = () => onSignal("SIGTERM");
process.on("SIGINT", onInt);
process.on("SIGTERM", onTerm);

async function destinationMode(output) {
  try {
    const stat = await lstat(output);
    if (!stat.isFile()) throw new Error("output destination must be a regular file; symlinks are refused");
    return stat.mode & 0o777;
  } catch (error) {
    if (error.code === "ENOENT") return 0o666 & ~process.umask();
    throw error;
  }
}
async function publish(output, bytes) {
  const mode = await destinationMode(output);
  const temp = path.join(path.dirname(output), `.${path.basename(output)}.${randomUUID()}.tmp`);
  let published = false;
  try {
    controller.signal.throwIfAborted();
    const file = await open(temp, "wx", mode);
    try { await file.writeFile(bytes); await file.sync(); } finally { await file.close(); }
    await chmod(temp, mode);
    controller.signal.throwIfAborted();
    const pause = Number(process.env.WORK_ENGINE_C2_TEST_BEFORE_RENAME_PAUSE_MS ?? 0);
    if (Number.isSafeInteger(pause) && pause > 0 && pause <= 500) await new Promise((resolve) => setTimeout(resolve, pause));
    await destinationMode(output);
    controller.signal.throwIfAborted();
    await rename(temp, output);
    published = true;
  } finally {
    if (!published) await unlink(temp).catch((error) => { if (error.code !== "ENOENT") throw error; });
  }
}
try {
  const workspaceRoot = path.resolve(options["workspace-root"] ?? process.cwd());
  const result = await compileSkill({
    structureSource: strictUtf8(await readFile(path.resolve(options.structure)), "structure source"),
    interfaceSource: strictUtf8(await readFile(path.resolve(options.interface)), "interface source"),
    workspaceRoot,
    signal: controller.signal,
  });
  const outputPath = path.resolve(options.output);
  await publish(outputPath, result.output);
  let compareMatch = null;
  if (options.compare) {
    const expected = await readFile(path.resolve(options.compare));
    compareMatch = expected.equals(result.output);
    if (!compareMatch) {
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
    ...(options.compare ? { compare_match: compareMatch } : {}),
  })}\n`);
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
} finally {
  process.off("SIGINT", onInt);
  process.off("SIGTERM", onTerm);
}
