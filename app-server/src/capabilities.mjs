export const PINNED_PROTOCOL = Object.freeze({
  codexCliVersion: "0.149.1",
  capabilities: Object.freeze({
    thread_start: "stable",
    thread_resume: "stable",
    thread_read: "stable",
    thread_turns_list: "experimental",
    turn_start: "stable",
    exact_skill_input: "stable",
    thread_scoped_dynamic_tools: "stable",
    client_message_id: "stable",
  }),
});

export const FOUNDATION_CAPABILITIES = Object.freeze([
  "thread_start",
  "thread_resume",
  "thread_read",
  "thread_turns_list",
  "turn_start",
  "exact_skill_input",
  "thread_scoped_dynamic_tools",
  "client_message_id",
]);

export const MODEL_CONTEXT_REPLACEMENT_CAPABILITY = "model_context_replacement";

export const PINNED_PROVIDER_RUNTIME = Object.freeze({
  codexCliVersion: "0.149.1",
  capabilities: Object.freeze({
    [MODEL_CONTEXT_REPLACEMENT_CAPABILITY]: Object.freeze({
      mechanism: "new_context",
      invocation: "target_model",
      requiredFeature: "token_budget",
      configurationEvidence: "host_declared",
      transitionEvidenceRequired: true,
    }),
  }),
});

export class CapabilityError extends Error {}

function normalizedNames(values, label) {
  if (!Array.isArray(values)) throw new CapabilityError(`${label} must be an array`);
  const names = new Set();
  for (const value of values) {
    if (typeof value !== "string" || value.trim() === "") {
      throw new CapabilityError(`${label} must contain non-empty strings`);
    }
    if (names.has(value)) throw new CapabilityError(`${label} contains duplicate ${value}`);
    names.add(value);
  }
  return [...names].sort();
}

export function negotiateProviderCapabilities({
  required = [],
  configuredFeatures = [],
  profile = PINNED_PROVIDER_RUNTIME,
  protocol = PINNED_PROTOCOL,
} = {}) {
  if (profile.codexCliVersion !== protocol.codexCliVersion) {
    throw new CapabilityError(
      `provider runtime ${profile.codexCliVersion ?? "unknown"} does not match pinned ${protocol.codexCliVersion}`,
    );
  }
  const requiredNames = normalizedNames(required, "required provider capabilities");
  const features = normalizedNames(configuredFeatures, "configured provider features");
  const configured = new Set(features);
  const selected = {};
  for (const name of requiredNames) {
    const capability = profile.capabilities?.[name];
    if (!capability) throw new CapabilityError(`unsupported provider capability: ${name}`);
    if (name === MODEL_CONTEXT_REPLACEMENT_CAPABILITY
        && (capability.mechanism !== "new_context"
          || capability.invocation !== "target_model"
          || capability.transitionEvidenceRequired !== true)) {
      throw new CapabilityError("model context replacement profile is not fail-closed");
    }
    if (!configured.has(capability.requiredFeature)) {
      throw new CapabilityError(
        `provider capability ${name} requires configured feature ${capability.requiredFeature}`,
      );
    }
    selected[name] = capability;
  }
  return Object.freeze({
    protocolVersion: profile.codexCliVersion,
    configuredFeatures: Object.freeze(features),
    selected: Object.freeze(selected),
  });
}

export function negotiateCapabilities(
  required = FOUNDATION_CAPABILITIES,
  protocol = PINNED_PROTOCOL,
) {
  const selected = {};
  let experimentalApi = false;
  for (const name of required) {
    const stability = protocol.capabilities[name];
    if (!stability) throw new CapabilityError(`unsupported App Server capability: ${name}`);
    if (stability === "experimental") experimentalApi = true;
    selected[name] = stability;
  }
  return Object.freeze({
    protocolVersion: protocol.codexCliVersion,
    selected: Object.freeze(selected),
    initializeCapabilities: Object.freeze({
      experimentalApi,
      requestAttestation: false,
    }),
  });
}

export function serverVersionFromUserAgent(userAgent) {
  const match = /^\S+\/([^\s]+)/.exec(userAgent ?? "");
  return match?.[1] ?? null;
}

export function assertCompatibleServer(initializeResponse, protocol = PINNED_PROTOCOL) {
  const actualVersion = serverVersionFromUserAgent(initializeResponse?.userAgent);
  if (actualVersion !== protocol.codexCliVersion) {
    throw new CapabilityError(
      `App Server ${actualVersion ?? "unknown"} does not match pinned ${protocol.codexCliVersion}`,
    );
  }
  return actualVersion;
}
