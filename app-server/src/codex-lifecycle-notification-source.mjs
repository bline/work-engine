import { PINNED_PROTOCOL } from "./capabilities.mjs";
import {
  CONTEXT_LIFECYCLE_EVIDENCE_SCHEMA_VERSION,
  normalizeLifecycleObservation,
} from "./context-lifecycle-evidence.mjs";

function source(method, protocolVersion) {
  return {
    provider: "codex",
    transport: "app-server",
    protocolVersion,
    method,
  };
}

function baseObservation(notification, protocolVersion, observationType, details) {
  return normalizeLifecycleObservation({
    schemaVersion: CONTEXT_LIFECYCLE_EVIDENCE_SCHEMA_VERSION,
    observationType,
    source: source(notification.method, protocolVersion),
    threadId: notification.params?.threadId,
    turnId: notification.params?.turnId,
    details,
  });
}

export function normalizeCodexLifecycleNotification(notification, {
  protocolVersion = PINNED_PROTOCOL.codexCliVersion,
} = {}) {
  if (!notification || typeof notification !== "object" || Array.isArray(notification)) {
    throw new TypeError("App Server notification must be an object");
  }
  if (notification.method === "thread/tokenUsage/updated") {
    const usage = notification.params?.tokenUsage;
    return baseObservation(notification, protocolVersion, "token_usage", {
      last: usage?.last,
      total: usage?.total,
      modelContextWindow: usage?.modelContextWindow ?? null,
    });
  }
  if (notification.method === "thread/compacted") {
    return baseObservation(
      notification,
      protocolVersion,
      "context_transition_signal",
      {
        signal: "context_compaction",
        phase: "reported",
        classification: "unclassified",
        providerItemId: null,
      },
    );
  }
  if (notification.method === "item/started" || notification.method === "item/completed") {
    if (notification.params?.item?.type !== "contextCompaction") return null;
    return baseObservation(
      notification,
      protocolVersion,
      "context_transition_signal",
      {
        signal: "context_compaction",
        phase: notification.method === "item/started" ? "started" : "completed",
        classification: "unclassified",
        providerItemId: notification.params.item.id,
      },
    );
  }
  return null;
}

export function attachCodexLifecycleEvidence({
  adapter,
  collector,
  protocolVersion = PINNED_PROTOCOL.codexCliVersion,
  onError = null,
}) {
  if (!adapter || typeof adapter.onNotification !== "function") {
    throw new TypeError("Codex lifecycle evidence requires a notification source");
  }
  if (!collector || typeof collector.record !== "function") {
    throw new TypeError("Codex lifecycle evidence requires a collector");
  }
  if (onError !== null && typeof onError !== "function") {
    throw new TypeError("Codex lifecycle evidence onError must be a function or null");
  }
  return adapter.onNotification((notification) => {
    try {
      const observation = normalizeCodexLifecycleNotification(notification, {
        protocolVersion,
      });
      if (observation) collector.record(observation);
    } catch (error) {
      if (onError) onError(error, notification);
      else throw error;
    }
  });
}
