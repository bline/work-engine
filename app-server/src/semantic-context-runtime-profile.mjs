import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";

import { parseDocument } from "yaml";

import { validateContextPressurePolicy } from "./context-pressure-controller.mjs";
import { validateShadowContextLifecycleSchedule } from "./shadow-context-lifecycle-coordinator.mjs";
import { validateTokenUsagePressureProfile } from "./token-usage-pressure-projection.mjs";

const TOP_LEVEL_FIELDS = new Set([
  "schema_version",
  "profile_id",
  "mode",
  "pressure_profile",
  "pressure_policy",
  "shadow_schedule",
  "live_schedule",
]);
const PRESSURE_PROFILE_FIELDS = new Set([
  "usage_field",
  "window_field",
  "rounding",
  "saturation",
]);
const PRESSURE_POLICY_FIELDS = new Set([
  "unit",
  "approaching",
  "replacement_candidate",
  "critical",
]);
const BAND_FIELDS = new Set(["enter", "exit"]);
const SCHEDULE_FIELDS = new Set(["inspect_at", "publish_accepted_checkpoint"]);
const LIVE_SCHEDULE_FIELDS = new Set(["transition_at"]);
const IDENTIFIER = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const SHA256 = /^[a-f0-9]{64}$/;

function record(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

function rejectUnknown(value, allowed, label) {
  const unknown = Object.keys(value).filter((field) => !allowed.has(field)).sort();
  if (unknown.length > 0) {
    throw new TypeError(`${label} contains unsupported fields: ${unknown.join(", ")}`);
  }
}

function text(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new TypeError(`${label} must be a non-empty string`);
  }
  return value;
}

function band(value, label) {
  record(value, label);
  rejectUnknown(value, BAND_FIELDS, label);
  return { enter: value.enter, exit: value.exit };
}

function freeze(value) {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) freeze(child);
  return Object.freeze(value);
}

export function projectSemanticContextRuntimeProfile(document, source = {}) {
  record(document, "semantic context runtime profile");
  rejectUnknown(document, TOP_LEVEL_FIELDS, "semantic context runtime profile");
  if (document.schema_version !== 1) {
    throw new TypeError("semantic context runtime profile schema_version must be 1");
  }
  const profileId = text(document.profile_id, "semantic context runtime profile id");
  if (!IDENTIFIER.test(profileId)) {
    throw new TypeError("semantic context runtime profile id is invalid");
  }
  const mode = document.mode ?? "shadow";
  if (!["shadow", "live"].includes(mode)) {
    throw new TypeError("semantic context runtime profile mode must be shadow or live");
  }
  const sourceSha256 = text(source.sha256, "semantic context runtime profile source digest");
  if (!SHA256.test(sourceSha256)) {
    throw new TypeError("semantic context runtime profile source digest must be SHA-256 hex");
  }
  const authoredPressureProfile = record(document.pressure_profile, "pressure_profile");
  rejectUnknown(authoredPressureProfile, PRESSURE_PROFILE_FIELDS, "pressure_profile");
  const pressureProfile = validateTokenUsagePressureProfile({
    schemaVersion: 1,
    usageField: authoredPressureProfile.usage_field,
    windowField: authoredPressureProfile.window_field,
    rounding: authoredPressureProfile.rounding,
    saturation: authoredPressureProfile.saturation,
  });
  const authoredPolicy = record(document.pressure_policy, "pressure_policy");
  rejectUnknown(authoredPolicy, PRESSURE_POLICY_FIELDS, "pressure_policy");
  const pressurePolicy = validateContextPressurePolicy({
    schemaVersion: 1,
    unit: authoredPolicy.unit,
    approaching: band(authoredPolicy.approaching, "pressure_policy.approaching"),
    replacementCandidate: band(
      authoredPolicy.replacement_candidate,
      "pressure_policy.replacement_candidate",
    ),
    critical: band(authoredPolicy.critical, "pressure_policy.critical"),
  });
  let shadowSchedule = null;
  let liveSchedule = null;
  if (mode === "shadow") {
    if (document.live_schedule !== undefined) {
      throw new TypeError("shadow semantic context profile cannot declare live_schedule");
    }
    const authoredSchedule = record(document.shadow_schedule, "shadow_schedule");
    rejectUnknown(authoredSchedule, SCHEDULE_FIELDS, "shadow_schedule");
    shadowSchedule = validateShadowContextLifecycleSchedule({
      schemaVersion: 1,
      inspectAt: authoredSchedule.inspect_at,
      publishAcceptedCheckpoint: authoredSchedule.publish_accepted_checkpoint,
    });
    if (shadowSchedule.publishAcceptedCheckpoint) {
      throw new TypeError("local semantic shadow profile cannot publish checkpoints");
    }
  } else {
    if (document.shadow_schedule !== undefined) {
      throw new TypeError("live semantic context profile cannot declare shadow_schedule");
    }
    const authoredSchedule = record(document.live_schedule, "live_schedule");
    rejectUnknown(authoredSchedule, LIVE_SCHEDULE_FIELDS, "live_schedule");
    const transitionAt = authoredSchedule.transition_at;
    if (!Array.isArray(transitionAt) || transitionAt.length === 0
        || new Set(transitionAt).size !== transitionAt.length
        || transitionAt.some((value) => ![
          "comfortable", "approaching", "replacement_candidate", "critical",
        ].includes(value))) {
      throw new TypeError("live_schedule.transition_at is invalid");
    }
    liveSchedule = freeze({ schemaVersion: 1, transitionAt: [...transitionAt] });
  }
  return freeze({
    schemaVersion: 1,
    profileId,
    mode,
    source: {
      path: source.path ? path.resolve(source.path) : null,
      sha256: sourceSha256,
    },
    pressureProfile,
    pressurePolicy,
    shadowSchedule,
    liveSchedule,
  });
}

export async function loadSemanticContextRuntimeProfile(profilePath) {
  const resolvedPath = path.resolve(text(profilePath, "semantic context runtime profile path"));
  const content = await readFile(resolvedPath, "utf8");
  const parsed = parseDocument(content, { uniqueKeys: true });
  if (parsed.errors.length > 0) {
    throw new TypeError(`invalid semantic context runtime profile YAML: ${parsed.errors[0].message}`);
  }
  return projectSemanticContextRuntimeProfile(parsed.toJS({ maxAliasCount: 0 }), {
    path: resolvedPath,
    sha256: createHash("sha256").update(content).digest("hex"),
  });
}
