import {
  ContextPressureController,
  validateContextPressurePolicy,
} from "./context-pressure-controller.mjs";
import { verifyContextLifecycleEpisode } from "./context-lifecycle-episode.mjs";

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function receipts(episodeStore, logicalRoleInstanceId = null) {
  if (!episodeStore || typeof episodeStore.receipts !== "function") {
    throw new TypeError("pressure recovery requires a lifecycle episode store");
  }
  const values = episodeStore.receipts({ logicalRoleInstanceId });
  if (!Array.isArray(values)) throw new TypeError("lifecycle episode receipts must be an array");
  for (const episode of values) {
    if (!verifyContextLifecycleEpisode(episode)) {
      throw new TypeError("pressure recovery requires integrity-valid lifecycle episodes");
    }
  }
  return values;
}

function maximumSequence(episodes) {
  return episodes.reduce((maximum, episode) =>
    Math.max(maximum, episode.pressure.observationSequence), 0);
}

export function lifecyclePressureSequenceFloor(episodeStore) {
  return maximumSequence(receipts(episodeStore));
}

export function restoreContextPressureController({
  policy,
  episodeStore,
  logicalRoleInstanceId,
}) {
  const role = text(logicalRoleInstanceId, "pressure recovery logical role");
  const normalizedPolicy = validateContextPressurePolicy(policy);
  const episodes = receipts(episodeStore, role);
  const sequences = episodes.map((episode) => episode.pressure.observationSequence);
  if (new Set(sequences).size !== sequences.length) {
    throw new TypeError("pressure recovery found duplicate role observation sequences");
  }
  const minimumSequence = maximumSequence(episodes);
  if (episodes.length === 0) {
    return Object.freeze({
      status: "empty",
      minimumSequence,
      initialDisposition: "comfortable",
      controller: new ContextPressureController({ policy, minimumSequence }),
    });
  }
  const latest = [...episodes].sort((left, right) =>
    left.pressure.observationSequence - right.pressure.observationSequence
    || left.completedAt.localeCompare(right.completedAt)
    || left.episodeId.localeCompare(right.episodeId)
  ).at(-1);
  const compatible = latest.pressure.policyRevision === normalizedPolicy.policyRevision;
  const initialDisposition = compatible ? latest.pressure.disposition : "comfortable";
  return Object.freeze({
    status: compatible ? "restored" : "policy_changed",
    minimumSequence,
    initialDisposition,
    latestEpisodeRevision: latest.episodeRevision,
    controller: new ContextPressureController({
      policy,
      initialDisposition,
      minimumSequence,
    }),
  });
}
