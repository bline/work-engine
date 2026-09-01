import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { AsyncLocalStorage } from "node:async_hooks";

import { runExecutableGenerationWorker } from "./executable-generation-worker-runtime.mjs";
import { createSupervisorCampaignCapabilityDefinitions } from "./services/slice-campaign/capability-contract.mjs";

const ENVIRONMENT_TOOLS = Object.freeze([{
  type: "namespace",
  name: "environment",
  description: "Inspect and replace the executable Work Engine environment at fenced turn boundaries.",
  tools: [{
    type: "function",
    name: "status",
    description: "Read the active executable generation, reload state, and admitted work. This tool has no effects.",
    inputSchema: { type: "object", properties: {}, additionalProperties: false },
  }, {
    type: "function",
    name: "reload",
    description: "Stage the current executable source for validation. Compatible source activates only after the invoking turn and all other admitted work complete; incompatible or invalid source remains inactive.",
    inputSchema: { type: "object", properties: {}, additionalProperties: false },
  }],
}]);

function transparentEnvironmentFingerprint() {
  return `sha256:${createHash("sha256").update(JSON.stringify(ENVIRONMENT_TOOLS)).digest("hex")}`;
}

function withEnvironmentTools(params) {
  const existing = params?.dynamicTools ?? [];
  if (!Array.isArray(existing)) throw new TypeError("dynamicTools must be an array");
  if (existing.some((tool) => tool?.name === "environment")) {
    throw new Error("environment dynamic tool namespace is already declared");
  }
  return { ...params, dynamicTools: [
    ...existing, ...ENVIRONMENT_TOOLS, ...(roleEnvironment?.extensionRegistry?.specs ?? []),
  ] };
}

function forwardedRequest(payload) {
  if (!payload || typeof payload.method !== "string") {
    throw new TypeError("App Server request payload requires a method");
  }
  if (payload.method === "initialize") {
    return {
      ...payload,
      params: {
        ...payload.params,
        capabilities: { ...payload.params?.capabilities, experimentalApi: true },
      },
    };
  }
  if (["thread/start", "thread/resume"].includes(payload.method)) {
    return { ...payload, params: withEnvironmentTools(payload.params) };
  }
  return payload;
}

const roleEnvironment = process.env.WORK_ENGINE_EXECUTABLE_ROLE_ENVIRONMENT === "1"
  ? await (async () => {
    const { DynamicToolBridge } = await import("./dynamic-tool-bridge.mjs");
    const { createRunExtensionRegistry } = await import("./run-extension-registry.mjs");
    const { ProductDevelopmentArtifactRoot } = await import(
      "./services/product-development/artifact-root.mjs"
    );
    const { createIntakeDelivery, intakeCapabilityDefinitions } = await import(
      "./services/product-development/intake-delivery.mjs"
    );
    const { createProposalDelivery, proposalCapabilityDefinitions } = await import(
      "./services/product-development/proposal-delivery.mjs"
    );
    const { createExecutableGenerationRoleEnvironment } = await import(
      "./executable-generation-role-environment.mjs"
    );
    const repositoryRoot = process.env.WORK_ENGINE_LIVE_REPOSITORY_ROOT;
    const artifactRoot = process.env.WORK_ENGINE_DEVELOPMENT_ARTIFACT_ROOT;
    const snapshotRoot = process.env.WORK_ENGINE_EXECUTABLE_SNAPSHOT_ROOT;
    const artifacts = new ProductDevelopmentArtifactRoot({ repositoryRoot, artifactRoot });
    const intake = createIntakeDelivery({ repositoryRoot, snapshotRoot, artifacts });
    const proposal = createProposalDelivery({ repositoryRoot, artifactRoot, snapshotRoot, artifacts });
    const supervisorEffects = new AsyncLocalStorage();
    const supervisor = createSupervisorCampaignCapabilityDefinitions(async (request) => {
      const effect = supervisorEffects.getStore();
      if (typeof effect !== "function") {
        throw new Error("supervisor capability call is outside an admitted generation dispatch");
      }
      return effect(request);
    });
    const catalog = new Map([
      ...Object.entries(intakeCapabilityDefinitions).map(([id, definition]) => [id, {
        ...definition,
        handler: definition.name === "read_source" ? intake.readSource : intake.publish,
      }]),
      ...Object.entries(proposalCapabilityDefinitions).map(([id, definition]) => [id, {
        ...definition,
        handler: definition.name === "read_intake" ? proposal.readIntake : proposal.publish,
      }]),
      ...supervisor,
    ]);
    const attachmentPath = process.env.WORK_ENGINE_RUN_EXTENSION_ATTACHMENT_PATH;
    const extensionAttachment = attachmentPath
      ? JSON.parse(await readFile(attachmentPath, "utf8")) : null;
    const extensionRegistry = extensionAttachment
      ? createRunExtensionRegistry(extensionAttachment, new Map(
        [...catalog.entries()].map(([id, definition]) => [id, definition.handler]),
      )) : null;
    const roleToolBridgeResolver = (capabilityIds) => {
      if (!Array.isArray(capabilityIds)) throw new TypeError("role capabilities must be an array");
      const registrations = capabilityIds.map((id) => {
        const registration = catalog.get(id);
        if (!registration) throw new Error(`runtime role declares unknown capability ${id}`);
        return registration;
      });
      return registrations.length === 0 ? null : new DynamicToolBridge(registrations);
    };
    const environment = await createExecutableGenerationRoleEnvironment({
      snapshotRoot: process.env.WORK_ENGINE_EXECUTABLE_SNAPSHOT_ROOT,
      bindingsPath: process.env.WORK_ENGINE_ROLE_BINDINGS_PATH,
      attachmentPath: process.env.WORK_ENGINE_SWITCHBOARD_ATTACHMENT_PATH,
      semanticContextStatePath: process.env.WORK_ENGINE_SEMANTIC_CONTEXT_STATE_PATH,
      configuredProviderFeatures: JSON.parse(
        process.env.WORK_ENGINE_CONFIGURED_PROVIDER_FEATURES ?? "[]",
      ),
      dynamicTools: ENVIRONMENT_TOOLS,
      roleToolBridgeResolver,
      productDevelopmentEnvironmentIdentity: artifacts.environmentIdentity(),
      extensionRegistryIdentity: extensionRegistry?.attachment_sha256 ?? null,
    });
    return Object.freeze({ ...environment, extensionRegistry, supervisorEffects });
  })()
  : null;

const generation = {
  ...JSON.parse(process.env.WORK_ENGINE_EXECUTABLE_GENERATION),
  environmentFingerprint: roleEnvironment
    ? roleEnvironment.environmentFingerprint()
    : transparentEnvironmentFingerprint(),
};

runExecutableGenerationWorker({
  generation,
  dispose: async () => roleEnvironment?.close(),
  dispatch: async (operation, payload, effect) => {
    if (operation === "app_server.request") {
      const forwarded = forwardedRequest(payload);
      if (roleEnvironment) return roleEnvironment.handleRequest(forwarded, effect);
      return { disposition: "forward", payload: forwarded };
    }
    if (operation === "app_server.server_request") {
      if (payload?.method === "item/tool/call" && payload.params?.namespace === "environment") {
        if (payload.params.tool === "status") {
          return { disposition: "control", control: "environment.status" };
        }
        if (payload.params.tool === "reload") {
          return { disposition: "control", control: "environment.reload" };
        }
      }
      if (roleEnvironment?.extensionRegistry
          && payload?.method === "item/tool/call"
          && payload.params?.namespace === roleEnvironment.extensionRegistry.specs[0]?.name) {
        return {
          disposition: "respond",
          result: await roleEnvironment.extensionRegistry.bridge.dispatch(payload.params),
        };
      }
      if (roleEnvironment) {
        return roleEnvironment.supervisorEffects.run(
          effect,
          () => roleEnvironment.handleServerRequest(payload),
        );
      }
      return { disposition: "forward" };
    }
    if (operation === "app_server.notification") {
      return { disposition: "forward" };
    }
    if (operation === "app_server.backend_notification") {
      if (roleEnvironment) return roleEnvironment.handleNotification(payload, effect);
      return { disposition: "forward" };
    }
    throw new Error("unsupported executable generation operation");
  },
});
