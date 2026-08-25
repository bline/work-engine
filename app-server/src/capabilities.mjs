export const PINNED_PROTOCOL = Object.freeze({
  codexCliVersion: "0.149.1",
  capabilities: Object.freeze({
    thread_start: "stable",
    thread_resume: "stable",
    turn_start: "stable",
    exact_skill_input: "stable",
    thread_scoped_dynamic_tools: "stable",
    client_message_id: "stable",
  }),
});

export const FOUNDATION_CAPABILITIES = Object.freeze([
  "thread_start",
  "thread_resume",
  "turn_start",
  "exact_skill_input",
  "thread_scoped_dynamic_tools",
  "client_message_id",
]);

export class CapabilityError extends Error {}

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
