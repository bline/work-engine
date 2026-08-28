import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const MAX_GIT_OUTPUT = 64 * 1024 * 1024;
const HEX_40 = /^[0-9a-f]{40}$/;

export class GrokReviewPacketError extends Error {}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function requireCommit(value, field) {
  if (typeof value !== "string" || !HEX_40.test(value)) {
    throw new GrokReviewPacketError(`${field} must be a lowercase 40-character Git object id`);
  }
  return value;
}

function requireRelativePath(value, field) {
  if (typeof value !== "string" || value.length === 0 || value.includes("\\") || value.includes("\0")) {
    throw new GrokReviewPacketError(`${field} must be a non-empty repository-relative POSIX path`);
  }
  if (path.posix.isAbsolute(value) || path.posix.normalize(value) !== value) {
    throw new GrokReviewPacketError(`${field} escapes or normalizes outside its declared identity`);
  }
  if (value === ".git" || value.startsWith(".git/")) {
    throw new GrokReviewPacketError(`${field} cannot address Git administrative state`);
  }
  return value;
}

async function git(repositoryRoot, args, { encoding = "utf8" } = {}) {
  try {
    const { stdout } = await execFileAsync("git", args, {
      cwd: repositoryRoot,
      encoding,
      maxBuffer: MAX_GIT_OUTPUT,
      windowsHide: true,
    });
    return stdout;
  } catch (error) {
    throw new GrokReviewPacketError(
      `git ${args[0]} failed: ${error.stderr?.toString().trim() || error.message}`,
    );
  }
}

function lineCount(text) {
  if (text.length === 0) return 0;
  return text.endsWith("\n") ? text.split("\n").length - 1 : text.split("\n").length;
}

async function readCommitFile(repositoryRoot, commit, filePath) {
  const listing = await git(repositoryRoot, ["ls-tree", commit, "--", filePath]);
  const line = listing.trimEnd();
  const tab = line.indexOf("\t");
  const header = tab === -1 ? "" : line.slice(0, tab);
  const listedPath = tab === -1 ? "" : line.slice(tab + 1);
  const [mode, type, object] = header.split(" ");
  if (listedPath !== filePath || type !== "blob" || !["100644", "100755"].includes(mode)) {
    throw new GrokReviewPacketError(`${filePath} is not one regular file at ${commit}`);
  }

  const bytes = await git(repositoryRoot, ["show", `${commit}:${filePath}`], { encoding: "buffer" });
  let text;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new GrokReviewPacketError(`${filePath} is not UTF-8 text`);
  }
  return { mode, object, bytes, text };
}

function normalizedEntries(reviewCase) {
  const roles = [
    ["changed", reviewCase.changed_files],
    ["dependency", reviewCase.dependency_files ?? []],
    ["contract", reviewCase.contract_files ?? []],
  ];
  const seen = new Map();
  for (const [role, values] of roles) {
    if (!Array.isArray(values)) throw new GrokReviewPacketError(`${role}_files must be an array`);
    for (const value of values) {
      const filePath = requireRelativePath(value, `${role}_files entry`);
      const prior = seen.get(filePath);
      if (prior && prior !== role) {
        throw new GrokReviewPacketError(`${filePath} is assigned more than one evidence role`);
      }
      seen.set(filePath, role);
    }
  }
  if (![...seen.values()].includes("changed")) {
    throw new GrokReviewPacketError("at least one changed file is required");
  }
  return [...seen.entries()].map(([filePath, role]) => ({ path: filePath, role }));
}

export async function buildGrokReviewPacket({ repositoryRoot, reviewCase, packetParent } = {}) {
  const commit = requireCommit(reviewCase?.subject?.commit, "subject.commit");
  const baseCommit = requireCommit(reviewCase?.base_commit, "base_commit");
  const entries = normalizedEntries(reviewCase);
  const root = await mkdtemp(path.join(packetParent ?? tmpdir(), "work-engine-grok-review-"));

  try {
    const resolvedCommit = (await git(repositoryRoot, ["rev-parse", "--verify", `${commit}^{commit}`])).trim();
    const resolvedBase = (await git(repositoryRoot, ["rev-parse", "--verify", `${baseCommit}^{commit}`])).trim();
    if (resolvedCommit !== commit || resolvedBase !== baseCommit) {
      throw new GrokReviewPacketError("Git subject resolution did not preserve the declared object ids");
    }
    const tree = (await git(repositoryRoot, ["show", "-s", "--format=%T", commit])).trim();
    if (reviewCase.subject.tree && reviewCase.subject.tree !== tree) {
      throw new GrokReviewPacketError(`subject tree mismatch: expected ${reviewCase.subject.tree}, got ${tree}`);
    }

    const files = [];
    for (const entry of entries) {
      const source = await readCommitFile(repositoryRoot, commit, entry.path);
      const destination = path.join(root, "evidence", ...entry.path.split("/"));
      await mkdir(path.dirname(destination), { recursive: true });
      await writeFile(destination, source.bytes, { mode: 0o444 });
      files.push({
        ...entry,
        git_mode: source.mode,
        git_object: source.object,
        sha256: sha256(source.bytes),
        line_count: lineCount(source.text),
      });
    }

    const changedFiles = entries.filter((entry) => entry.role === "changed").map((entry) => entry.path);
    const patch = await git(repositoryRoot, [
      "diff", "--no-ext-diff", "--binary", baseCommit, commit, "--", ...changedFiles,
    ], { encoding: "buffer" });
    const patchDigest = sha256(patch);
    if (reviewCase.subject.patch_identity && reviewCase.subject.patch_identity !== patchDigest) {
      throw new GrokReviewPacketError(
        `subject patch mismatch: expected ${reviewCase.subject.patch_identity}, got ${patchDigest}`,
      );
    }
    await writeFile(path.join(root, "change.patch"), patch, { mode: 0o444 });

    const manifest = {
      schema_version: 1,
      subject: {
        commit,
        tree,
        patch_identity: reviewCase.subject.patch_identity ?? null,
      },
      base_commit: baseCommit,
      files,
      patch: {
        path: "change.patch",
        sha256: patchDigest,
        byte_length: patch.length,
      },
      review_contract: {
        focus: Array.isArray(reviewCase.focus) ? reviewCase.focus : [],
      },
    };
    await writeFile(path.join(root, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`, { mode: 0o444 });

    return {
      root,
      manifest,
      async dispose() {
        await rm(root, { recursive: true, force: true });
      },
    };
  } catch (error) {
    await rm(root, { recursive: true, force: true });
    throw error;
  }
}

export async function readPacketEvidence(packetRoot, filePath) {
  const safePath = requireRelativePath(filePath, "evidence path");
  return readFile(path.join(packetRoot, "evidence", ...safePath.split("/")), "utf8");
}
