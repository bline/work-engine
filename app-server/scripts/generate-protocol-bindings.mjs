#!/usr/bin/env node

import { execFile } from "node:child_process";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const HERE = path.dirname(fileURLToPath(import.meta.url));
const APP_SERVER_ROOT = path.resolve(HERE, "..");
const lockPath = path.join(APP_SERVER_ROOT, "protocol-bindings.lock.json");

async function commandOutput(command, args) {
  const { stdout } = await execFileAsync(command, args, {
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
  });
  return stdout.trim();
}

function importedTypes(source, relativePath) {
  const directory = path.posix.dirname(relativePath);
  return [...source.matchAll(/from "([^"]+)"/g)].map((match) =>
    path.posix.normalize(path.posix.join(directory, `${match[1]}.ts`)),
  );
}

async function dependencyClosure(sourceRoot, roots) {
  const pending = [...roots];
  const selected = new Set();
  while (pending.length > 0) {
    const relativePath = pending.pop();
    if (selected.has(relativePath)) continue;
    if (relativePath.startsWith("../") || path.posix.isAbsolute(relativePath)) {
      throw new Error(`generated import escaped protocol root: ${relativePath}`);
    }
    const source = await readFile(path.join(sourceRoot, relativePath), "utf8");
    selected.add(relativePath);
    pending.push(...importedTypes(source, relativePath));
  }
  return [...selected].sort();
}

async function main() {
  const lock = JSON.parse(await readFile(lockPath, "utf8"));
  if (lock.schemaVersion !== 1 || !Array.isArray(lock.roots)) {
    throw new Error("unsupported protocol binding lock");
  }

  const actualVersion = (await commandOutput("codex", ["--version"])).replace(/^codex-cli\s+/, "");
  if (actualVersion !== lock.codexCliVersion) {
    throw new Error(
      `codex CLI ${actualVersion} does not match pinned ${lock.codexCliVersion}`,
    );
  }

  const temporaryRoot = await mkdtemp(path.join(os.tmpdir(), "work-engine-protocol."));
  try {
    const generatedRoot = path.join(temporaryRoot, "all");
    await mkdir(generatedRoot);
    const args = ["app-server", "generate-ts", "--out", generatedRoot];
    if (lock.experimental) args.push("--experimental");
    await execFileAsync("codex", args, { maxBuffer: 64 * 1024 * 1024 });

    const selected = await dependencyClosure(generatedRoot, lock.roots);
    const targetRoot = path.join(
      APP_SERVER_ROOT,
      "generated",
      `codex-cli-${lock.codexCliVersion}`,
    );
    if (!targetRoot.startsWith(path.join(APP_SERVER_ROOT, "generated") + path.sep)) {
      throw new Error("refusing to replace an unsafe generated target");
    }
    await rm(targetRoot, { recursive: true, force: true });
    for (const relativePath of selected) {
      const target = path.join(targetRoot, relativePath);
      await mkdir(path.dirname(target), { recursive: true });
      await cp(path.join(generatedRoot, relativePath), target);
    }
    const index = lock.roots
      .map((relativePath) => {
        const exportPath = `./${relativePath.replace(/\.ts$/, "")}`;
        return `export type * from "${exportPath}";`;
      })
      .join("\n");
    await writeFile(path.join(targetRoot, "index.ts"), `${index}\n`, "utf8");
    process.stdout.write(
      `generated ${selected.length} protocol files for codex-cli ${actualVersion}\n`,
    );
  } finally {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});
