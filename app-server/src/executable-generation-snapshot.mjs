import { createHash, randomUUID } from "node:crypto";
import {
  chmod,
  copyFile,
  lstat,
  mkdir,
  readFile,
  realpath,
  rename,
  rm,
  writeFile,
} from "node:fs/promises";
import path from "node:path";

function freeze(value) {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    for (const item of Object.values(value)) freeze(item);
    Object.freeze(value);
  }
  return value;
}

function normalizeRelativePath(value) {
  if (typeof value !== "string" || value.length === 0 || path.isAbsolute(value)) {
    throw new TypeError("generation source paths must be non-empty relative paths");
  }
  const normalized = path.normalize(value);
  if (normalized === "." || normalized === ".." || normalized.startsWith(`..${path.sep}`)) {
    throw new TypeError(`generation source path escapes the workspace: ${value}`);
  }
  return normalized.split(path.sep).join("/");
}

function digest(value) {
  return createHash("sha256").update(value).digest("hex");
}

function manifestDigest(files) {
  return digest(JSON.stringify({ schemaVersion: 1, files }));
}

function generatedEntry(relativePath, content) {
  const bytes = Buffer.isBuffer(content) ? content : Buffer.from(content, "utf8");
  return freeze({
    path: relativePath,
    sha256: digest(bytes),
    size: bytes.length,
    executable: false,
  });
}

async function fileEntry(root, relativePath) {
  const pathname = path.join(root, ...relativePath.split("/"));
  const metadata = await lstat(pathname);
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    throw new TypeError(`generation source must be a regular non-symlink file: ${relativePath}`);
  }
  const content = await readFile(pathname);
  return freeze({
    path: relativePath,
    sha256: digest(content),
    size: content.length,
    executable: Boolean(metadata.mode & 0o111),
  });
}

async function createManifest(root, files) {
  const entries = [];
  for (const relativePath of files) entries.push(await fileEntry(root, relativePath));
  return freeze({
    schemaVersion: 1,
    files: entries,
    digest: manifestDigest(entries),
  });
}

function combinedManifest(sourceManifest, generatedEntries) {
  const entries = [...sourceManifest.files, ...generatedEntries]
    .sort((left, right) => left.path.localeCompare(right.path));
  return freeze({ schemaVersion: 1, files: entries, digest: manifestDigest(entries) });
}

function sameManifest(left, right) {
  return left.digest === right.digest && JSON.stringify(left.files) === JSON.stringify(right.files);
}

async function copyManifestFiles(sourceRoot, targetRoot, manifest) {
  for (const entry of manifest.files) {
    const source = path.join(sourceRoot, ...entry.path.split("/"));
    const target = path.join(targetRoot, ...entry.path.split("/"));
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(source, target);
    await chmod(target, entry.executable ? 0o700 : 0o600);
  }
}


async function writeGeneratedFiles(targetRoot, generatedFiles) {
  for (const [relativePath, content] of generatedFiles) {
    const target = path.join(targetRoot, ...relativePath.split("/"));
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, content, { mode: 0o600 });
  }
}

export class ExecutableGenerationSnapshotError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "ExecutableGenerationSnapshotError";
    this.code = code;
  }
}

export async function captureExecutableGenerationSnapshot({
  workspaceRoot,
  generationsRoot,
  files,
  generatedFiles = {},
  onPhase = null,
} = {}) {
  if (!Array.isArray(files) || files.length === 0) {
    throw new TypeError("generation snapshot requires a closed non-empty file inventory");
  }
  if (onPhase !== null && typeof onPhase !== "function") {
    throw new TypeError("generation snapshot phase observer must be a function or null");
  }
  const sourceRoot = await realpath(path.resolve(workspaceRoot));
  const targetRoot = path.resolve(generationsRoot);
  const normalizedFiles = [...new Set(files.map(normalizeRelativePath))]
    .sort((left, right) => left.localeCompare(right));
  if (normalizedFiles.length !== files.length) {
    throw new TypeError("generation snapshot file inventory contains duplicates");
  }
  if (!generatedFiles || typeof generatedFiles !== "object" || Array.isArray(generatedFiles)) {
    throw new TypeError("generation snapshot generated files must be an object");
  }
  const normalizedGenerated = Object.entries(generatedFiles).map(([relativePath, content]) => {
    const normalized = normalizeRelativePath(relativePath);
    if (typeof content !== "string" && !Buffer.isBuffer(content)) {
      throw new TypeError(`generated generation source must be text or bytes: ${relativePath}`);
    }
    return [normalized, content];
  }).sort(([left], [right]) => left.localeCompare(right));
  const generatedPaths = new Set(normalizedGenerated.map(([relativePath]) => relativePath));
  if (generatedPaths.size !== normalizedGenerated.length
      || normalizedFiles.some((relativePath) => generatedPaths.has(relativePath))) {
    throw new TypeError("generation snapshot source and generated inventories overlap");
  }
  const generatedEntries = normalizedGenerated.map(([relativePath, content]) =>
    generatedEntry(relativePath, content)
  );
  await mkdir(targetRoot, { recursive: true });
  const temporary = path.join(targetRoot, `.snapshot-${process.pid}-${randomUUID()}`);
  await mkdir(temporary, { mode: 0o700 });
  try {
    const beforeSource = await createManifest(sourceRoot, normalizedFiles);
    const before = combinedManifest(beforeSource, generatedEntries);
    await onPhase?.("before_copy", before);
    await copyManifestFiles(sourceRoot, temporary, beforeSource);
    await writeGeneratedFiles(temporary, normalizedGenerated);
    const copied = await createManifest(
      temporary,
      [...normalizedFiles, ...generatedEntries.map((entry) => entry.path)]
        .sort((left, right) => left.localeCompare(right)),
    );
    await onPhase?.("after_copy", copied);
    const after = combinedManifest(await createManifest(sourceRoot, normalizedFiles), generatedEntries);
    if (!sameManifest(before, copied) || !sameManifest(before, after)) {
      throw new ExecutableGenerationSnapshotError(
        "source_changed",
        "generation source changed while the immutable snapshot was captured",
      );
    }
    const snapshotId = `generation-${before.digest}`;
    const destination = path.join(targetRoot, snapshotId);
    const receipt = freeze({
      schemaVersion: 1,
      snapshotId,
      sourceRoot,
      sourceDigest: before.digest,
      manifest: before,
    });
    await writeFile(
      path.join(temporary, "generation-manifest.json"),
      `${JSON.stringify(receipt, null, 2)}\n`,
      { encoding: "utf8", mode: 0o600 },
    );
    try {
      await rename(temporary, destination);
    } catch (error) {
      if (error?.code !== "EEXIST" && error?.code !== "ENOTEMPTY") throw error;
      const existing = JSON.parse(await readFile(
        path.join(destination, "generation-manifest.json"),
        "utf8",
      ));
      if (existing.sourceDigest !== receipt.sourceDigest
          || JSON.stringify(existing.manifest) !== JSON.stringify(receipt.manifest)) {
        throw new ExecutableGenerationSnapshotError(
          "snapshot_collision",
          "content-addressed generation directory does not match its source digest",
        );
      }
      await rm(temporary, { recursive: true, force: true });
    }
    return freeze({ ...receipt, directory: destination });
  } catch (error) {
    await rm(temporary, { recursive: true, force: true });
    throw error;
  }
}
