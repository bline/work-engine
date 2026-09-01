import { DynamicToolBridge } from "./dynamic-tool-bridge.mjs";
import { extensionDigest } from "./run-extension-bundle-contract.mjs";

export function createRunExtensionRegistry(attachment, hostAdapters = new Map()) {
  if (!attachment || attachment.sha256 !== extensionDigest((({ sha256, canonical_json, ...value }) => value)(attachment))) {
    throw new Error("extension attachment digest is stale");
  }
  const registrations = attachment.registry.map((entry) => {
    const adapter = hostAdapters.get(entry.adapter);
    if (!adapter) throw new Error(`extension host adapter is unavailable: ${entry.adapter}`);
    return {
      namespace: attachment.run.namespace, name: entry.name,
      description: entry.description, inputSchema: entry.input_schema,
      handler: adapter,
    };
  });
  const bridge = new DynamicToolBridge(registrations);
  return Object.freeze({ attachment_sha256: attachment.sha256, bridge, specs: bridge.specs() });
}
