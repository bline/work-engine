import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  createLocalSemanticShadowHost,
  loadSemanticContextRuntimeProfile,
  projectRuntimeManifest,
} from "../src/index.mjs";

class ShadowAdapter {
  constructor() {
    this.handlers = new Set();
    this.inferenceCalls = 0;
  }

  onNotification(handler) {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  async deliverTurn({ role }) {
    const delivery = {
      logicalRoleInstanceId: role.logicalRoleInstanceId,
      threadId: "shadow-thread-1",
      turnId: "shadow-turn-1",
      createdThread: true,
      replayedDelivery: false,
      binding: { bindingRevision: 1 },
    };
    for (const handler of this.handlers) handler({
      method: "thread/tokenUsage/updated",
      params: {
        threadId: delivery.threadId,
        turnId: delivery.turnId,
        tokenUsage: {
          last: {
            inputTokens: 80,
            cachedInputTokens: 0,
            outputTokens: 20,
            reasoningOutputTokens: 0,
            totalTokens: 100,
          },
          total: {
            inputTokens: 80,
            cachedInputTokens: 0,
            outputTokens: 20,
            reasoningOutputTokens: 0,
            totalTokens: 100,
          },
          modelContextWindow: 1_000,
        },
      },
    });
    return delivery;
  }

  async waitForTurnCompletion() {
    return { status: "completed", outputText: "shadow role response" };
  }

  async runEphemeralTurn() {
    this.inferenceCalls += 1;
    throw new Error("comfortable pressure must not invoke inference");
  }
}

test("local semantic host records one durable comfortable episode without inference", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "work-engine-local-shadow."));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const skillPath = path.join(directory, "skills", "test-role", "SKILL.md");
  await mkdir(path.dirname(skillPath), { recursive: true });
  await writeFile(skillPath, "# Test role\n", "utf8");
  const manifest = projectRuntimeManifest({
    schema_version: 1,
    manifest_id: "local.shadow.test",
    roles: {
      "test-role": {
        contract: "skills/test-role/SKILL.md",
        developer_instructions: "Test role.",
        skills: [{ name: "test-role", path: "skills/test-role/SKILL.md" }],
      },
    },
  }, { baseDirectory: directory });
  const adapter = new ShadowAdapter();
  const profile = await loadSemanticContextRuntimeProfile(
    new URL("../semantic-context-profile.yaml", import.meta.url).pathname,
  );
  const host = await createLocalSemanticShadowHost({
    adapter,
    manifest,
    stateFilePath: path.join(directory, "state.sqlite3"),
    profile,
  });
  t.after(host.close);

  const result = await host.runtime.deliverTurn({
    roleId: "test-role",
    instanceId: "main",
    clientUserMessageId: "local-shadow-message-1",
    text: "Observe this completed operator turn.",
  });

  assert.equal(result.completion.outputText, "shadow role response");
  assert.equal(result.shadow.status, "recorded");
  assert.equal(result.shadow.episode.pressure.disposition, "comfortable");
  assert.equal(result.shadow.episode.inference.status, "not_scheduled");
  assert.equal(adapter.inferenceCalls, 0);
  const durable = host.episodeStore.get(result.shadow.episode.episodeId);
  assert.equal(durable.episodeRevision, result.shadow.episode.episodeRevision);
  assert.match(host.signingKeyId, /^local-shadow:[a-f0-9]{64}$/);
});
