import assert from "node:assert/strict";
import test from "node:test";

import { projectThreadSnapshotVisibleMaterials } from "../src/index.mjs";

test("thread snapshot projection accounts for every non-excluded item with conservative trust", () => {
  const materials = projectThreadSnapshotVisibleMaterials({
    turns: [{
      id: "turn-domain",
      items: [
        { type: "userMessage", id: "user-1", text: "Proceed." },
        { type: "agentMessage", id: "agent-1", text: "Working." },
        { type: "mcpToolCall", id: "tool-1", server: "evidence" },
        { type: "hookPrompt", id: "hook-1", text: "Unclassified hook content." },
        { type: "contextCompaction", id: "compact-1" },
      ],
    }, {
      id: "turn-attestation",
      items: [{ type: "agentMessage", id: "agent-2", text: "Sterile receipt." }],
    }],
  }, { excludedTurnIds: ["turn-attestation"] });
  assert.equal(materials.length, 5);
  assert.deepEqual(materials.map(({ origin, trustClass, instructionApplicability }) => ({
    origin,
    trustClass,
    instructionApplicability,
  })), [{
    origin: "human",
    trustClass: "human_authority_input",
    instructionApplicability: "contract_defined",
  }, {
    origin: "assistant",
    trustClass: "model_output",
    instructionApplicability: "none",
  }, {
    origin: "tool",
    trustClass: "attributed_evidence",
    instructionApplicability: "none",
  }, {
    origin: "application",
    trustClass: "untrusted_data",
    instructionApplicability: "none",
  }, {
    origin: "application",
    trustClass: "trusted_application_data",
    instructionApplicability: "none",
  }]);
  assert.equal(materials.every(({ content }) => typeof content === "string"), true);
});

test("thread snapshot projection fails closed on malformed items", () => {
  assert.throws(
    () => projectThreadSnapshotVisibleMaterials({
      turns: [{ id: "turn-1", items: [{ id: "missing-type" }] }],
    }),
    /type must be a non-empty string/,
  );
});
