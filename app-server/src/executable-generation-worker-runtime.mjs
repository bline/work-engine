const PROTOCOL = "work-engine.executable-generation-ipc";
const PROTOCOL_VERSION = 1;

function text(value, label) {
  if (typeof value !== "string" || value.length === 0) {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function generationRecord(value) {
  return Object.freeze({
    generationId: text(value?.generationId, "generation id"),
    sourceDigest: text(value?.sourceDigest, "generation source digest"),
    environmentFingerprint: text(
      value?.environmentFingerprint,
      "generation environment fingerprint",
    ),
    bootstrapFingerprint: text(
      value?.bootstrapFingerprint,
      "generation bootstrap fingerprint",
    ),
  });
}

function failureType(error) {
  return error instanceof Error ? error.name : typeof error;
}

export function runExecutableGenerationWorker({
  generation,
  validate = async () => ({ valid: true }),
  activate = async () => {},
  dispatch,
  dispose = async () => {},
} = {}) {
  const record = generationRecord(generation);
  if (typeof validate !== "function" || typeof activate !== "function"
      || typeof dispatch !== "function" || typeof dispose !== "function") {
    throw new TypeError("generation worker handlers must be functions");
  }
  let validated = false;
  let active = false;
  let disposing = false;
  let nextEffectId = 1;
  const pendingEffects = new Map();

  // The stable bootstrap owns terminal signals and performs ordered disposal.
  // A lost bootstrap connection still terminates the worker instead of
  // allowing an orphaned executable generation to survive.
  process.on("SIGINT", () => {});
  process.on("SIGTERM", () => {});
  process.on("disconnect", () => process.exit(0));

  const send = (message, callback = undefined) => {
    if (process.send) process.send(message, callback);
  };
  const requestEffect = (requestId, payload) => {
    if (!process.connected) return Promise.reject(new Error("generation bootstrap is unavailable"));
    const effectId = nextEffectId++;
    return new Promise((resolve, reject) => {
      pendingEffects.set(`${requestId}:${effectId}`, { resolve, reject });
      send({
        protocol: PROTOCOL,
        version: PROTOCOL_VERSION,
        kind: "effect_request",
        id: requestId,
        effectId,
        payload: structuredClone(payload),
      }, (error) => {
        if (!error) return;
        pendingEffects.delete(`${requestId}:${effectId}`);
        reject(error);
      });
    });
  };
  process.on("message", async (message) => {
    if (!message || message.protocol !== PROTOCOL || message.version !== PROTOCOL_VERSION
        || !Number.isSafeInteger(message.id)) {
      return;
    }
    if (message.kind === "effect_response") {
      if (!Number.isSafeInteger(message.effectId)) return;
      const key = `${message.id}:${message.effectId}`;
      const pending = pendingEffects.get(key);
      if (!pending) return;
      pendingEffects.delete(key);
      if (message.error) pending.reject(new Error(
        `stable generation effect failed (${message.error.failureType ?? "unknown"})`,
      ));
      else pending.resolve(message.result);
      return;
    }
    if (message.kind !== undefined || typeof message.method !== "string") return;
    const response = { protocol: PROTOCOL, version: PROTOCOL_VERSION, id: message.id };
    try {
      if (disposing && message.method !== "generation.dispose") {
        throw new Error("generation worker is disposing");
      }
      if (message.method === "generation.describe") {
        response.result = record;
      } else if (message.method === "generation.validate") {
        const result = await validate();
        validated = result?.valid === true;
        response.result = { valid: validated };
      } else if (message.method === "generation.activate") {
        if (!validated) throw new Error("generation must validate before activation");
        await activate();
        active = true;
        response.result = { active: true };
      } else if (message.method === "generation.dispatch") {
        if (!active) throw new Error("generation must be active before dispatch");
        response.result = await dispatch(
          text(message.params?.operation, "generation operation"),
          structuredClone(message.params?.payload),
          (payload) => requestEffect(message.id, payload),
        );
      } else if (message.method === "generation.dispose") {
        disposing = true;
        await dispose();
        response.result = { disposed: true };
        send(response, () => process.disconnect?.());
        return;
      } else {
        throw new Error("unknown generation worker method");
      }
    } catch (error) {
      response.error = { failureType: failureType(error) };
    }
    send(response);
  });

  return record;
}
