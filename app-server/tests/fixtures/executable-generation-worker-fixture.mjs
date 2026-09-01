import { runExecutableGenerationWorker } from "../../src/executable-generation-worker-runtime.mjs";

const generation = JSON.parse(process.env.WORK_ENGINE_TEST_GENERATION);

runExecutableGenerationWorker({
  generation,
  validate: async () => ({ valid: process.env.WORK_ENGINE_TEST_INVALID !== "1" }),
  dispatch: async (operation, payload, effect) => {
    if (operation === "crash") process.exit(17);
    if (operation === "echo") {
      return { disposition: "respond", result: { generationId: generation.generationId, payload } };
    }
    if (["effect", "supervisor_campaign_effect"].includes(operation)) {
      return { disposition: "respond", result: await effect(payload) };
    }
    if (operation === "app_server.server_request"
        && payload?.method === "item/tool/call"
        && payload.params?.tool === "fixture_supervisor_campaign_effect") {
      return { disposition: "respond", result: await effect(payload.params.arguments) };
    }
    if (["app_server.request", "app_server.notification"].includes(operation)) {
      return { disposition: "forward" };
    }
    throw new Error("unsupported fixture operation");
  },
});
