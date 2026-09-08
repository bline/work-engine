const INSTANCE_ID = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const INPUT_FIELDS = new Set(["instance_id", "client_user_message_id", "text"]);

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function rejectUnknownFields(value) {
  const unknown = Object.keys(value).filter((field) => !INPUT_FIELDS.has(field)).sort();
  if (unknown.length > 0) {
    throw new TypeError(`managed builder turn contains unsupported fields: ${unknown.join(", ")}`);
  }
}

export function createManagedBuilderTurnHandler(deliverTurn) {
  if (typeof deliverTurn !== "function") {
    throw new TypeError("managed builder turn requires a delivery function");
  }
  return async (input) => {
    requireRecord(input, "managed builder turn");
    rejectUnknownFields(input);
    const instanceId = requireText(input.instance_id, "managed builder instance id");
    if (!INSTANCE_ID.test(instanceId)) {
      throw new TypeError("managed builder instance id contains unsupported characters");
    }
    return deliverTurn({
      roleId: "slice-builder",
      instanceId,
      clientUserMessageId: requireText(
        input.client_user_message_id,
        "managed builder client user message id",
      ),
      text: requireText(input.text, "managed builder turn text"),
    });
  };
}
