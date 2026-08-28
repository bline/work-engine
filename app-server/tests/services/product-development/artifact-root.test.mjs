import assert from "node:assert/strict";
import { access, mkdtemp, mkdir, rm, symlink } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { ProductDevelopmentArtifactRoot } from "../../../src/services/product-development/artifact-root.mjs";

test("artifact root publishes create-only bundles idempotently and refuses replacement", async () => {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-artifacts-"));
  try {
    const root = path.join(repository, "fixture-artifacts");
    await mkdir(root);
    const artifacts = new ProductDevelopmentArtifactRoot({ repositoryRoot: repository, artifactRoot: root });
    const request = {
      operationId: "op-1", artifactKind: "fixture", artifactId: "item-1",
      destination: ["items", "item-1"], files: [{ path: "record.json", content: "{}\n" }],
    };
    const created = await artifacts.publishCreateOnly(request);
    assert.equal(created.publication.state, "created");
    assert.equal(created.publication.expected_state, "absent");
    assert.equal(created.non_authorization.implementation_authorized, false);

    const replayed = await artifacts.publishCreateOnly(request);
    assert.equal(replayed.publication.state, "idempotent");
    assert.equal(replayed.publication.result_digest, created.publication.result_digest);

    const refused = await artifacts.publishCreateOnly({
      ...request, operationId: "op-2", files: [{ path: "record.json", content: "{\"changed\":true}\n" }],
    });
    assert.equal(refused.outcome, "refused");
    assert.equal(refused.publication.state, "refused");
  } finally {
    await rm(repository, { recursive: true, force: true });
  }
});

test("artifact root rejects escape paths", async () => {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-artifacts-"));
  try {
    const outside = path.resolve(repository, "..");
    assert.throws(
      () => new ProductDevelopmentArtifactRoot({ repositoryRoot: repository, artifactRoot: outside }),
      /inside the live repository/,
    );
  } finally {
    await rm(repository, { recursive: true, force: true });
  }
});

test("artifact root rejects configured and destination symlinks before outside writes", async () => {
  const container = await mkdtemp(path.join(os.tmpdir(), "work-engine-artifacts-"));
  try {
    const repository = path.join(container, "repository");
    const outside = path.join(container, "outside");
    const root = path.join(repository, "fixture-artifacts");
    await mkdir(repository);
    await mkdir(outside);
    await symlink(outside, root, "dir");
    assert.throws(
      () => new ProductDevelopmentArtifactRoot({ repositoryRoot: repository, artifactRoot: root }),
      /cannot contain symbolic links/,
    );
    await rm(root);
    await mkdir(root);
    await symlink(outside, path.join(root, "items"), "dir");
    const artifacts = new ProductDevelopmentArtifactRoot({ repositoryRoot: repository, artifactRoot: root });
    await assert.rejects(
      artifacts.publishCreateOnly({
        operationId: "op-symlink", artifactKind: "fixture", artifactId: "item-1",
        destination: ["items", "item-1"],
        files: [{ path: "record.json", content: "{}\n" }],
      }),
      /only real directories/,
    );
    await assert.rejects(access(path.join(outside, "item-1", "record.json")));
  } finally {
    await rm(container, { recursive: true, force: true });
  }
});
