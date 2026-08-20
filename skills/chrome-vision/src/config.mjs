import crypto from "node:crypto";
import fs from "node:fs/promises";
import path from "node:path";
import { parse } from "yaml";
import { validateAgainstSchema } from "./schema-validation.mjs";

const configSchema = JSON.parse(await fs.readFile(new URL("../schemas/config-v1.schema.json", import.meta.url), "utf8"));

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
  return JSON.stringify(value);
}

export function resolveChromeVisionConfig(value, { baseDirectory, source }) {
  const validation = validateAgainstSchema(value, configSchema);
  if (!validation.valid) throw new Error(`Invalid Chrome Vision configuration from ${source}: ${validation.errors.join("; ")}`);
  const config = structuredClone(value);
  if (config.artifactDirectory) config.artifactDirectory = path.resolve(baseDirectory, config.artifactDirectory);
  return {
    config,
    identity: {
      source,
      schemaVersion: config.version,
      sha256: crypto.createHash("sha256").update(canonical(value)).digest("hex"),
      pathBase: path.resolve(baseDirectory),
    },
  };
}

export async function loadChromeVisionConfig(configPath, { cwd = process.cwd() } = {}) {
  const resolvedPath = path.resolve(cwd, configPath);
  let value;
  try { value = parse(await fs.readFile(resolvedPath, "utf8")); }
  catch (error) { throw new Error(`Cannot read Chrome Vision configuration ${resolvedPath}: ${error.message}`); }
  const result = resolveChromeVisionConfig(value, { baseDirectory: path.dirname(resolvedPath), source: resolvedPath });
  result.identity.kind = "file";
  result.identity.resolvedPath = resolvedPath;
  return result;
}
