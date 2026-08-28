import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { ProductDevelopmentArtifactRoot } from "../../../src/services/product-development/artifact-root.mjs";
import { createProposalDelivery } from "../../../src/services/product-development/proposal-delivery.mjs";

test("proposal delivery validates a staged complete packet before create-only publication", async () => {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-proposal-"));
  try {
    const snapshot = path.join(repository, "snapshot");
    await mkdir(path.join(snapshot, "skills/idea-intake/scripts"), { recursive: true });
    await mkdir(path.join(snapshot, "skills/proposal-packets/scripts"), { recursive: true });
    await writeFile(path.join(snapshot, "skills/idea-intake/scripts/idea_intake.py"), "# fixture\n");
    await writeFile(path.join(snapshot, "skills/proposal-packets/scripts/proposal_packets.py"), "# fixture\n");
    const artifacts = new ProductDevelopmentArtifactRoot({ repositoryRoot: repository, artifactRoot: repository });
    const record = "{\"idea_id\":\"fixture\"}\n";
    await mkdir(path.join(repository, "ideas/intake/fixture"), { recursive: true });
    await writeFile(path.join(repository, "ideas/intake/fixture/record.json"), record);
    const recordSha256 = `sha256:${createHash("sha256").update(record).digest("hex")}`;
    const invokeProcess = async (_command, args) => args.includes("project")
      ? JSON.stringify({ status: "valid", projection: { schema_version: 1, idea_id: "fixture" } })
      : JSON.stringify({ status: "valid", packet_count: 1, proposal_ids: ["proposal-1"] });
    const delivery = createProposalDelivery({ repositoryRoot: repository, artifactRoot: repository, snapshotRoot: snapshot, artifacts, invokeProcess });
    const receipt = await delivery.publish({
      operation_id: "proposal-op", idea_id: "fixture", intake_record_sha256: recordSha256,
      family_id: "family-1", proposal_id: "proposal-1",
      files: [{ path: "packet.json", content: "{\"proposal_id\":\"proposal-1\",\"family_id\":\"family-1\"}\n" }, { path: "proposal.md", content: "Fixture proposal\n" }],
    });
    assert.equal(receipt.publication.state, "created");
    assert.equal(receipt.validator.name, "proposal-packets");
    assert.deepEqual(receipt.validator.proposal_ids, ["proposal-1"]);
    assert.equal(receipt.source_binding.record_sha256, recordSha256);
    assert.equal(receipt.non_authorization.proposal_accepted, false);
  } finally {
    await rm(repository, { recursive: true, force: true });
  }
});

test("proposal intake reads reject path syntax and symlinks before reads, validation, or publication", async () => {
  const repository = await mkdtemp(path.join(os.tmpdir(), "work-engine-proposal-"));
  try {
    const artifactRoot = path.join(repository, "artifacts");
    const outside = path.join(repository, "outside");
    const snapshot = path.join(repository, "snapshot");
    await mkdir(artifactRoot);
    await mkdir(outside);
    await mkdir(path.join(snapshot, "skills/idea-intake/scripts"), { recursive: true });
    const record = "{\"idea_id\":\"outside\"}\n";
    await writeFile(path.join(outside, "record.json"), record);
    const recordSha256 = `sha256:${createHash("sha256").update(record).digest("hex")}`;
    let validatorCalls = 0;
    let publicationCalls = 0;
    const artifacts = new ProductDevelopmentArtifactRoot({ repositoryRoot: repository, artifactRoot });
    const delivery = createProposalDelivery({
      repositoryRoot: repository,
      artifactRoot,
      snapshotRoot: snapshot,
      artifacts: new Proxy(artifacts, {
        get(target, property, receiver) {
          if (property === "publishCreateOnly") {
            return (...args) => {
              publicationCalls += 1;
              return target.publishCreateOnly(...args);
            };
          }
          const value = Reflect.get(target, property, receiver);
          return typeof value === "function" ? value.bind(target) : value;
        },
      }),
      invokeProcess: async () => {
        validatorCalls += 1;
        return JSON.stringify({ status: "valid", projection: { idea_id: "outside" } });
      },
    });
    await assert.rejects(
      delivery.readIntake({ idea_id: "../../../outside", record_sha256: recordSha256 }),
      /invalid path segment/,
    );
    await assert.rejects(
      delivery.publish({
        operation_id: "proposal-traversal", idea_id: "../../../outside",
        intake_record_sha256: recordSha256, family_id: "family-1", proposal_id: "proposal-1",
        files: [{ path: "packet.json", content: "{}\n" }],
      }),
      /invalid path segment/,
    );
    assert.equal(validatorCalls, 0);
    assert.equal(publicationCalls, 0);

    await mkdir(path.join(artifactRoot, "ideas", "intake"), { recursive: true });
    await symlink(outside, path.join(artifactRoot, "ideas", "intake", "linked"), "dir");
    await assert.rejects(
      delivery.readIntake({ idea_id: "linked", record_sha256: recordSha256 }),
      /only real directories/,
    );
    assert.equal(validatorCalls, 0);
  } finally {
    await rm(repository, { recursive: true, force: true });
  }
});
