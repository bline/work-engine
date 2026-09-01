export const SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL =
  "work-engine.supervisor-capability-effect.v1";

const ENVELOPE_KEYS = new Set(["capability", "input", "operation", "protocol"]);

function requireText(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function requireRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function exactKeys(value, allowed, label) {
  const unsupported = Object.keys(value).find((key) => !allowed.has(key));
  if (unsupported) throw new TypeError(`${label} contains unsupported field ${unsupported}`);
}

function key(capability, operation) { return `${capability}\u0000${operation}`; }

export function isSupervisorCampaignHostEffectCandidate(value) {
  return Boolean(value && typeof value === "object" && !Array.isArray(value)
    && (Object.hasOwn(value, "protocol")
      || Object.hasOwn(value, "capability")
      || Object.hasOwn(value, "operation")));
}

export function createSupervisorCampaignHostEffectRuntime({ registrations = [] } = {}) {
  if (!Array.isArray(registrations)) {
    throw new TypeError("supervisor campaign host-effect registrations must be an array");
  }
  const handlers = new Map();
  for (const registration of registrations) {
    requireRecord(registration, "supervisor campaign host-effect registration");
    exactKeys(registration, new Set([
      "capability", "operation", "validateInput", "validateOutput", "handler",
    ]), "supervisor campaign host-effect registration");
    const capability = requireText(registration.capability, "supervisor capability");
    const operation = requireText(registration.operation, "supervisor capability operation");
    for (const field of ["validateInput", "validateOutput", "handler"]) {
      if (typeof registration[field] !== "function") {
        throw new TypeError(`supervisor campaign host-effect registration requires ${field}`);
      }
    }
    const registrationKey = key(capability, operation);
    if (handlers.has(registrationKey)) {
      throw new Error(`duplicate supervisor campaign host-effect registration ${capability}/${operation}`);
    }
    handlers.set(registrationKey, Object.freeze({ ...registration }));
  }
  let closed = false;
  return Object.freeze({
    async dispatch({ generationId, effect } = {}) {
      if (closed) throw new Error("supervisor campaign host-effect runtime is closed");
      requireText(generationId, "host-observed executable generation identity");
      requireRecord(effect, "supervisor campaign host-effect envelope");
      exactKeys(effect, ENVELOPE_KEYS, "supervisor campaign host-effect envelope");
      if (effect.protocol !== SUPERVISOR_CAMPAIGN_HOST_EFFECT_PROTOCOL) {
        throw new TypeError("supervisor campaign host-effect protocol is unsupported");
      }
      const capability = requireText(effect.capability, "supervisor capability");
      const operation = requireText(effect.operation, "supervisor capability operation");
      const registration = handlers.get(key(capability, operation));
      if (!registration) {
        throw new Error(`supervisor campaign host effect is unavailable for ${capability}/${operation}`);
      }
      const input = registration.validateInput(structuredClone(effect.input));
      const output = await registration.handler(Object.freeze({
        generationId,
        input: structuredClone(input),
      }));
      return structuredClone(registration.validateOutput(structuredClone(output)));
    },
    close() { closed = true; },
  });
}
