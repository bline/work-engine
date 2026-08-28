import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";
import test from "node:test";

import { ProductDevelopmentArtifactRoot } from "../../../src/services/product-development/artifact-root.mjs";
import { createIntakeDelivery } from "../../../src/services/product-development/intake-delivery.mjs";

const run = promisify(execFile);
const digest = (value) => `sha256:${createHash("sha256").update(value).digest("hex")}`;

test("intake delivery reads an exact Git source and publishes validator-derived projection", async () => {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-intake-"));
  try {
    await run("git", ["init", "-q", repository]);
    await run("git", ["-C", repository, "config", "user.email", "fixture@example.invalid"]);
    await run("git", ["-C", repository, "config", "user.name", "Fixture"]);
    const source = "An exact fixture idea.\n";
    await writeFile(path.join(repository, "idea.md"), source);
    await run("git", ["-C", repository, "add", "idea.md"]);
    await run("git", ["-C", repository, "commit", "-qm", "fixture"]);
    const { stdout: revision } = await run("git", ["-C", repository, "rev-parse", "HEAD"]);

    const snapshot = path.join(repository, "snapshot");
    await mkdir(path.join(snapshot, "skills/idea-intake/scripts"), { recursive: true });
    await writeFile(path.join(snapshot, "skills/idea-intake/scripts/idea_intake.py"), "# fixture\n");
    const artifacts = new ProductDevelopmentArtifactRoot({ repositoryRoot: repository, artifactRoot: repository });
    const invokeProcess = async (command, args) => {
      if (command === "git") return (await run(command, args, { encoding: "utf8" })).stdout;
      return JSON.stringify({ status: "valid", projection: { schema_version: 1, idea_id: "fixture-idea", source: { repository_revision: revision.trim(), repository_path: "idea.md", sha256: digest(source) }, candidates: [] } });
    };
    const delivery = createIntakeDelivery({ repositoryRoot: repository, snapshotRoot: snapshot, artifacts, invokeProcess });
    const loaded = await delivery.readSource({ repository_revision: revision.trim(), repository_path: "idea.md", start_line: 1, end_line: 1, sha256: digest(source) });
    assert.equal(loaded.content, source);
    const receipt = await delivery.publish({
      operation_id: "intake-op", idea_id: "fixture-idea",
      files: [{ path: "record.json", content: "{\"idea_id\":\"fixture-idea\"}\n" }, { path: "assessment.md", content: "Fixture\n" }],
    });
    assert.equal(receipt.publication.state, "created");
    assert.equal(receipt.projection.idea_id, "fixture-idea");
    assert.equal(receipt.validator.name, "idea-intake");
    assert.equal(receipt.validator.projection_sha256, digest(JSON.stringify(receipt.projection)));
  } finally {
    await rm(repository, { recursive: true, force: true });
  }
});
