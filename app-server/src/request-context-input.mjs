const CONTEXT_PREFIX = "WORK_ENGINE_REQUEST_CONTEXT_V1\n";
const CONTEXT_NAME = /^[A-Za-z0-9][A-Za-z0-9._:-]*$/;

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
}

function requireText(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
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

export const REQUEST_CONTEXT_INPUT_PREFIX = CONTEXT_PREFIX;

export function compileRequestContextInput(requestContext) {
  if (requestContext == null) return null;
  requireRecord(requestContext, "request context");
  const names = Object.keys(requestContext);
  if (names.length === 0) {
    throw new TypeError("request context must contain at least one named entry");
  }

  const entries = {};
  for (const name of names.sort()) {
    requireText(name, "request context entry name");
    if (!CONTEXT_NAME.test(name)) {
      throw new TypeError(`request context entry name is invalid: ${name}`);
    }
    const entry = requestContext[name];
    requireRecord(entry, `request context entry ${name}`);
    const unknown = Object.keys(entry).filter((key) => !["kind", "value"].includes(key));
    if (unknown.length > 0) {
      throw new TypeError(
        `request context entry ${name} contains unsupported fields: ${unknown.sort().join(", ")}`,
      );
    }
    requireText(entry.kind, `request context entry ${name} kind`);
    if (typeof entry.value !== "string") {
      throw new TypeError(`request context entry ${name} value must be a string`);
    }
    entries[name] = { kind: entry.kind, value: entry.value };
  }

  const envelope = {
    schema_version: 1,
    type: "work-engine.request-context",
    entries,
  };
  return Object.freeze({
    type: "text",
    text: `${CONTEXT_PREFIX}${canonicalJson(envelope)}`,
    text_elements: Object.freeze([]),
  });
}
