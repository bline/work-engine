import { BUILD_VERSION } from "./contract.mjs";
import { buildProjection } from "./projections.mjs";

export const LEGACY_BUILD_VERSION = "claim-evidence-python-v1";
export const NATIVE_BUILD_VERSION = BUILD_VERSION;

export function buildLegacyProjection(store) {
  return buildProjection(store, { buildVersion: LEGACY_BUILD_VERSION });
}

