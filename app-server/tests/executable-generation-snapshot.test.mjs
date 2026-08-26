import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import {
  ExecutableGenerationSnapshotError,
  captureExecutableGenerationSnapshot,
} from "../src/index.mjs";

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), "work-engine-generation-snapshot-"));
  const workspaceRoot = path.join(root, "workspace");
  const generationsRoot = path.join(root, "generations");
  await mkdir(path.join(workspaceRoot, "src"), { recursive: true });
  await writeFile(path.join(workspaceRoot, "src", "worker.mjs"), "export const value = 1;\n");
  await writeFile(path.join(workspaceRoot, "runtime.yaml"), "schema_version: 1\n");
  return { root, workspaceRoot, generationsRoot };
}

test("captures a verified content-addressed generation from a closed inventory", async (t) => {
  const current = await fixture();
  t.after(() => rm(current.root, { recursive: true, force: true }));

  const receipt = await captureExecutableGenerationSnapshot({
    workspaceRoot: current.workspaceRoot,
    generationsRoot: current.generationsRoot,
    files: ["runtime.yaml", "src/worker.mjs"],
  });

  assert.equal(receipt.snapshotId, `generation-${receipt.sourceDigest}`);
  assert.deepEqual(receipt.manifest.files.map((entry) => entry.path), [
    "runtime.yaml",
    "src/worker.mjs",
  ]);
  assert.equal(
    await readFile(path.join(receipt.directory, "src", "worker.mjs"), "utf8"),
    "export const value = 1;\n",
  );

  const repeated = await captureExecutableGenerationSnapshot({
    workspaceRoot: current.workspaceRoot,
    generationsRoot: current.generationsRoot,
    files: ["src/worker.mjs", "runtime.yaml"],
  });
  assert.equal(repeated.directory, receipt.directory);
  assert.equal(repeated.sourceDigest, receipt.sourceDigest);
});

test("generated environment projection is content-addressed beside authored sources", async (t) => {
  const current = await fixture();
  t.after(() => rm(current.root, { recursive: true, force: true }));

  const receipt = await captureExecutableGenerationSnapshot({
    workspaceRoot: current.workspaceRoot,
    generationsRoot: current.generationsRoot,
    files: ["src/worker.mjs"],
    generatedFiles: {
      "generated/environment.json": "{\"schemaVersion\":1}\n",
    },
  });

  assert.deepEqual(receipt.manifest.files.map((entry) => entry.path), [
    "generated/environment.json",
    "src/worker.mjs",
  ]);
  assert.equal(
    await readFile(path.join(receipt.directory, "generated", "environment.json"), "utf8"),
    "{\"schemaVersion\":1}\n",
  );
  const changed = await captureExecutableGenerationSnapshot({
    workspaceRoot: current.workspaceRoot,
    generationsRoot: current.generationsRoot,
    files: ["src/worker.mjs"],
    generatedFiles: {
      "generated/environment.json": "{\"schemaVersion\":2}\n",
    },
  });
  assert.notEqual(changed.sourceDigest, receipt.sourceDigest);
});

test("rejects a source tree that changes during snapshot capture", async (t) => {
  const current = await fixture();
  t.after(() => rm(current.root, { recursive: true, force: true }));

  await assert.rejects(
    captureExecutableGenerationSnapshot({
      workspaceRoot: current.workspaceRoot,
      generationsRoot: current.generationsRoot,
      files: ["src/worker.mjs"],
      onPhase: async (phase) => {
        if (phase === "after_copy") {
          await writeFile(
            path.join(current.workspaceRoot, "src", "worker.mjs"),
            "export const value = 2;\n",
          );
        }
      },
    }),
    (error) => error instanceof ExecutableGenerationSnapshotError
      && error.code === "source_changed",
  );
});

test("rejects duplicate, escaping, and symbolic-link inventory entries", async (t) => {
  const current = await fixture();
  t.after(() => rm(current.root, { recursive: true, force: true }));
  await symlink("src/worker.mjs", path.join(current.workspaceRoot, "worker-link.mjs"));

  await assert.rejects(
    captureExecutableGenerationSnapshot({
      workspaceRoot: current.workspaceRoot,
      generationsRoot: current.generationsRoot,
      files: ["runtime.yaml", "runtime.yaml"],
    }),
    /contains duplicates/,
  );
  await assert.rejects(
    captureExecutableGenerationSnapshot({
      workspaceRoot: current.workspaceRoot,
      generationsRoot: current.generationsRoot,
      files: ["../outside.mjs"],
    }),
    /escapes the workspace/,
  );
  await assert.rejects(
    captureExecutableGenerationSnapshot({
      workspaceRoot: current.workspaceRoot,
      generationsRoot: current.generationsRoot,
      files: ["worker-link.mjs"],
    }),
    /non-symlink file/,
  );
  await assert.rejects(
    captureExecutableGenerationSnapshot({
      workspaceRoot: current.workspaceRoot,
      generationsRoot: current.generationsRoot,
      files: ["runtime.yaml"],
      generatedFiles: { "runtime.yaml": "replacement" },
    }),
    /inventories overlap/,
  );
});
