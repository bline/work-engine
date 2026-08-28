import { createHash, randomUUID } from "node:crypto";
import { lstatSync, realpathSync } from "node:fs";
import {
  lstat, mkdir, readFile, readdir, realpath, rename, rm, writeFile,
} from "node:fs/promises";
import path from "node:path";

const SEGMENT = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const MAX_FILES = 64;
const MAX_FILE_BYTES = 1024 * 1024;

function digest(value) {
  return `sha256:${createHash("sha256").update(value).digest("hex")}`;
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`,
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

function safeRelative(value, label) {
  if (typeof value !== "string" || value === "" || path.isAbsolute(value)) {
    throw new TypeError(`${label} must be a relative path`);
  }
  const normalized = value.split(path.sep).join("/");
  if (normalized.split("/").some((segment) => !SEGMENT.test(segment))) {
    throw new TypeError(`${label} contains an invalid path segment`);
  }
  return normalized;
}

function safeSegments(segments, label) {
  if (!Array.isArray(segments) || segments.length === 0
      || segments.some((segment) => typeof segment !== "string" || !SEGMENT.test(segment))) {
    throw new TypeError(`${label} contains an invalid path segment`);
  }
  return segments;
}

function isWithin(parent, candidate) {
  const relative = path.relative(parent, candidate);
  return relative === "" || (!relative.startsWith(`..${path.sep}`)
    && relative !== ".." && !path.isAbsolute(relative));
}

async function filesIn(directory, prefix = "") {
  const entries = await readdir(directory, { withFileTypes: true });
  const result = [];
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isSymbolicLink()) throw new Error("artifact bundles cannot contain symbolic links");
    if (entry.isDirectory()) result.push(...await filesIn(path.join(directory, entry.name), relative));
    else if (entry.isFile()) result.push(relative);
    else throw new Error("artifact bundles can contain only regular files and directories");
  }
  return result;
}

async function describeBundle(directory) {
  const artifacts = [];
  for (const relativePath of await filesIn(directory)) {
    const content = await readFile(path.join(directory, ...relativePath.split("/")));
    artifacts.push({ path: relativePath, sha256: digest(content), size_bytes: content.length });
  }
  return {
    artifacts,
    digest: digest(canonicalJson(artifacts)),
  };
}

export class ProductDevelopmentArtifactRoot {
  constructor({ repositoryRoot, artifactRoot }) {
    this.configuredRepositoryRoot = path.resolve(repositoryRoot);
    this.configuredArtifactRoot = path.resolve(artifactRoot);
    const relative = path.relative(this.configuredRepositoryRoot, this.configuredArtifactRoot);
    if (relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
      throw new TypeError("development artifact root must be inside the live repository");
    }
    let rootState;
    try {
      rootState = lstatSync(this.configuredArtifactRoot);
      this.repositoryRoot = realpathSync(this.configuredRepositoryRoot);
      this.artifactRoot = realpathSync(this.configuredArtifactRoot);
    } catch {
      throw new TypeError("development artifact root must be an existing directory");
    }
    if (rootState.isSymbolicLink()
        || this.artifactRoot !== path.join(this.repositoryRoot, relative)) {
      throw new TypeError("development artifact root cannot contain symbolic links");
    }
    if (!rootState.isDirectory()) {
      throw new TypeError("development artifact root must be an existing directory");
    }
    if (!isWithin(this.repositoryRoot, this.artifactRoot)) {
      throw new TypeError("development artifact root resolves outside the live repository");
    }
  }

  environmentIdentity() {
    return digest(canonicalJson({
      repositoryRoot: this.repositoryRoot,
      artifactRoot: this.artifactRoot,
    }));
  }

  async resolveExistingFile({ destination, file }) {
    const destinationSegments = safeSegments(destination, "artifact destination");
    const relativeFile = safeRelative(file, "artifact file path");
    const fileSegments = relativeFile.split("/");
    const parent = await this.#resolveDirectory(
      [...destinationSegments, ...fileSegments.slice(0, -1)],
      { create: false },
    );
    const candidate = path.join(parent, fileSegments.at(-1));
    const state = await lstat(candidate);
    if (state.isSymbolicLink() || !state.isFile()) {
      throw new Error("artifact file must be a regular file without symbolic links");
    }
    const resolved = await realpath(candidate);
    if (!isWithin(this.artifactRoot, resolved)) {
      throw new Error("artifact file resolves outside the development artifact root");
    }
    return resolved;
  }

  async publishCreateOnly({ operationId, artifactKind, artifactId, destination, files, prepare }) {
    if (typeof operationId !== "string" || !SEGMENT.test(operationId)) {
      throw new TypeError("artifact operation id is invalid");
    }
    if (typeof artifactKind !== "string" || !SEGMENT.test(artifactKind)
        || typeof artifactId !== "string" || !SEGMENT.test(artifactId)) {
      throw new TypeError("artifact identity is invalid");
    }
    const targetSegments = safeSegments(destination, "artifact destination");
    const targetName = targetSegments.at(-1);
    const parentSegments = targetSegments.slice(0, -1);
    const parent = await this.#resolveDirectory(parentSegments, { create: true });
    const target = path.join(parent, targetName);
    await this.#rejectSymlink(target, { allowMissing: true });
    const stage = path.join(parent, `.${path.basename(target)}.${operationId}.${randomUUID()}.stage`);
    await mkdir(stage, { recursive: false });
    try {
      if (!Array.isArray(files) || files.length === 0 || files.length > MAX_FILES) {
        throw new TypeError("artifact publication requires files");
      }
      const names = new Set();
      for (const file of files) {
        const relativePath = safeRelative(file?.path, "artifact file path");
        if (names.has(relativePath)) throw new TypeError("artifact file paths must be unique");
        names.add(relativePath);
        if (typeof file.content !== "string") throw new TypeError("artifact file content must be text");
        if (Buffer.byteLength(file.content, "utf8") > MAX_FILE_BYTES) {
          throw new TypeError("artifact file content exceeds the host limit");
        }
        const output = path.join(stage, ...relativePath.split("/"));
        await mkdir(path.dirname(output), { recursive: true });
        await writeFile(output, file.content, { encoding: "utf8", flag: "wx" });
      }
      const prepared = await prepare?.(stage);
      const bundle = await describeBundle(stage);
      try {
        const reboundParent = await this.#resolveDirectory(parentSegments, { create: false });
        if (reboundParent !== parent) {
          throw new Error("artifact destination changed during publication");
        }
        const reboundTarget = path.join(reboundParent, targetName);
        await this.#rejectSymlink(reboundTarget, { allowMissing: true });
        await rename(stage, reboundTarget);
        await this.#resolveDirectory(targetSegments, { create: false });
        return this.#receipt({
          operationId, artifactKind, artifactId, state: "created", bundle, prepared,
        });
      } catch (error) {
        if (error?.code !== "EEXIST" && error?.code !== "ENOTEMPTY") throw error;
        const existingTarget = await this.#resolveDirectory(targetSegments, { create: false });
        const existing = await describeBundle(existingTarget);
        if (existing.digest !== bundle.digest) {
          return this.#receipt({
            operationId,
            artifactKind,
            artifactId,
            state: "refused",
            bundle: existing,
            expectedDigest: bundle.digest,
            prepared,
          });
        }
        return this.#receipt({
          operationId, artifactKind, artifactId, state: "idempotent", bundle: existing, prepared,
        });
      }
    } finally {
      await rm(stage, { recursive: true, force: true });
    }
  }

  async #authenticateRoot() {
    const repository = await realpath(this.configuredRepositoryRoot);
    const rootState = await lstat(this.configuredArtifactRoot);
    const root = await realpath(this.configuredArtifactRoot);
    if (repository !== this.repositoryRoot || root !== this.artifactRoot
        || rootState.isSymbolicLink() || !rootState.isDirectory()
        || !isWithin(repository, root)) {
      throw new Error("development artifact root confinement changed");
    }
    return root;
  }

  async #resolveDirectory(segments, { create }) {
    if (!Array.isArray(segments)
        || segments.some((segment) => typeof segment !== "string" || !SEGMENT.test(segment))) {
      throw new TypeError("artifact destination contains an invalid path segment");
    }
    let current = await this.#authenticateRoot();
    for (const segment of segments) {
      const candidate = path.join(current, segment);
      let state;
      try {
        state = await lstat(candidate);
      } catch (error) {
        if (!create || error?.code !== "ENOENT") throw error;
        await mkdir(candidate, { recursive: false });
        state = await lstat(candidate);
      }
      if (state.isSymbolicLink() || !state.isDirectory()) {
        throw new Error("artifact destination must contain only real directories");
      }
      const resolved = await realpath(candidate);
      if (!isWithin(this.artifactRoot, resolved)) {
        throw new Error("artifact destination resolves outside the development artifact root");
      }
      current = resolved;
    }
    return current;
  }

  async #rejectSymlink(candidate, { allowMissing }) {
    try {
      const state = await lstat(candidate);
      if (state.isSymbolicLink()) {
        throw new Error("artifact destination cannot be a symbolic link");
      }
    } catch (error) {
      if (allowMissing && error?.code === "ENOENT") return;
      throw error;
    }
  }

  #receipt({ operationId, artifactKind, artifactId, state, bundle, expectedDigest = null, prepared }) {
    return {
      schema_version: 1,
      operation_id: operationId,
      outcome: state === "refused" ? "refused" : "succeeded",
      artifact_kind: artifactKind,
      artifact_id: artifactId,
      source_binding: prepared?.source_binding ?? null,
      validator: prepared?.validator ?? null,
      publication: {
        state,
        expected_state: "absent",
        prior_digest: state === "created" ? null : bundle.digest,
        result_digest: bundle.digest,
        expected_digest: expectedDigest,
      },
      artifacts: bundle.artifacts,
      non_authorization: {
        proposal_accepted: false,
        roadmap_priority_changed: false,
        implementation_authorized: false,
        cleanup_authorized: false,
      },
      ...(prepared?.projection ? { projection: prepared.projection } : {}),
    };
  }
}
