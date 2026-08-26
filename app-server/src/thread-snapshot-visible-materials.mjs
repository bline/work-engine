function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${canonicalJson(value[key])}`
    ).join(",")}}`;
  }
  return JSON.stringify(value);
}

const ASSISTANT_ITEMS = new Set(["agentMessage", "plan", "reasoning"]);
const TOOL_ITEMS = new Set([
  "collabAgentToolCall",
  "commandExecution",
  "dynamicToolCall",
  "fileChange",
  "imageGeneration",
  "imageView",
  "mcpToolCall",
  "sleep",
  "subAgentActivity",
  "webSearch",
]);

function classification(type) {
  if (type === "userMessage") {
    return {
      origin: "human",
      trustClass: "human_authority_input",
      instructionApplicability: "contract_defined",
    };
  }
  if (ASSISTANT_ITEMS.has(type)) {
    return {
      origin: "assistant",
      trustClass: "model_output",
      instructionApplicability: "none",
      producer: "codex-app-server",
    };
  }
  if (TOOL_ITEMS.has(type)) {
    return {
      origin: "tool",
      trustClass: "attributed_evidence",
      instructionApplicability: "none",
      producer: "codex-app-server",
    };
  }
  if (type === "contextCompaction") {
    return {
      origin: "application",
      trustClass: "trusted_application_data",
      instructionApplicability: "none",
      producer: "codex-app-server",
    };
  }
  return {
    origin: "application",
    trustClass: "untrusted_data",
    instructionApplicability: "none",
    producer: "codex-app-server",
  };
}

export function projectThreadSnapshotVisibleMaterials(snapshot, {
  excludedTurnIds = [],
} = {}) {
  record(snapshot, "thread context snapshot");
  if (!Array.isArray(snapshot.turns)) {
    throw new TypeError("thread context snapshot turns must be an array");
  }
  if (!Array.isArray(excludedTurnIds)) {
    throw new TypeError("excluded thread turn ids must be an array");
  }
  const excluded = new Set(excludedTurnIds.map((turnId, index) =>
    text(turnId, `excluded thread turn id ${index}`)
  ));
  const materials = [];
  snapshot.turns.forEach((turn, turnIndex) => {
    record(turn, `thread turn ${turnIndex}`);
    const turnId = text(turn.id, `thread turn ${turnIndex} id`);
    if (excluded.has(turnId)) return;
    if (!Array.isArray(turn.items)) {
      throw new TypeError(`thread turn ${turnId} items must be an array`);
    }
    turn.items.forEach((item, itemIndex) => {
      record(item, `thread turn ${turnId} item ${itemIndex}`);
      const type = text(item.type, `thread turn ${turnId} item ${itemIndex} type`);
      const itemIdentity = item.id == null
        ? `index-${itemIndex}`
        : text(item.id, `thread turn ${turnId} item ${itemIndex} id`);
      const identity = `thread-item:${turnId}:${itemIdentity}`;
      materials.push({
        identity,
        ...classification(type),
        contentRef: { kind: "thread-item", reference: identity },
        content: canonicalJson(item),
      });
    });
  });
  return Object.freeze(materials.map(Object.freeze));
}
