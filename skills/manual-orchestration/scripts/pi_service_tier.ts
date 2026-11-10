// Per-launch OpenAI service tier for a Pi session started by role_host.py.
//
// Pi 1.4.0 has no service-tier setting; its OpenAI Responses provider sends
// whatever payload the before_provider_request hook returns. This extension
// reads the tier from its own process environment, so concurrent Pi roles on
// the same model can run at different tiers and nothing shared is written.
//
//   WORK_ENGINE_PI_SERVICE_TIER         requested tier (auto|default|flex|priority|scale)
//   WORK_ENGINE_PI_SERVICE_TIER_MODEL   exact "provider/id" the tier applies to
//   WORK_ENGINE_PI_SERVICE_TIER_RECORD  JSONL file for records
//
// Records: "session_start" (the extension loaded), "injected" or "skipped" per
// request, and "observed" with the tier the provider reported on its terminal
// response event. Pi reports a throwing hook and sends the request unchanged,
// so this file never throws; role_host compares injected and observed records
// with the request instead of trusting that the hook ran.
import { appendFileSync } from "node:fs";

const TIERS = new Set(["auto", "default", "flex", "priority", "scale"]);
const TERMINAL = new Set(["response.completed", "response.incomplete", "response.failed"]);

export default function (pi: any) {
	const tier = process.env.WORK_ENGINE_PI_SERVICE_TIER;
	const model = process.env.WORK_ENGINE_PI_SERVICE_TIER_MODEL;
	const recordPath = process.env.WORK_ENGINE_PI_SERVICE_TIER_RECORD;
	if (!recordPath) return;

	const record = (row: Record<string, unknown>) => {
		try {
			appendFileSync(recordPath, `${JSON.stringify({ at: new Date().toISOString(), pid: process.pid, ...row })}\n`);
		} catch (error) {
			console.error(`[work-engine service tier] record failed: ${error instanceof Error ? error.message : String(error)}`);
		}
	};
	const requested = tier ? tier : null;
	const valid = requested === null || (TIERS.has(requested) && typeof model === "string" && model.includes("/"));

	pi.on("session_start", () => {
		record({ event: "session_start", requested_tier: requested, model: model ?? null, valid });
	});

	pi.on("before_provider_request", (event: any, ctx: any) => {
		try {
			const key = ctx?.model ? `${ctx.model.provider}/${ctx.model.id}` : null;
			const api = ctx?.model?.api ?? null;
			let reason: string | null = null;
			if (requested === null) reason = "not_requested";
			else if (!valid) reason = "invalid_configuration";
			else if (key !== model) reason = "model_mismatch";
			else if (api !== "openai-responses") reason = "api_not_supported";
			else if (!event?.payload || typeof event.payload !== "object" || Array.isArray(event.payload)) reason = "payload_not_object";
			if (reason) {
				record({ event: "skipped", model: key, api, reason });
				return undefined;
			}
			record({ event: "injected", model: key, api, tier: requested });
			return { ...event.payload, service_tier: requested };
		} catch (error) {
			record({ event: "skipped", reason: "hook_error", detail: error instanceof Error ? error.message : String(error) });
			return undefined;
		}
	});

	pi.on("provider_stream_event", (event: any) => {
		try {
			const data = event?.data;
			if (!data || typeof data !== "object" || !TERMINAL.has(data.type)) return;
			const response = data.response && typeof data.response === "object" ? data.response : {};
			record({
				event: "observed",
				model: `${event.provider}/${event.model}`,
				api: event.api ?? null,
				response_type: data.type,
				response_id: typeof response.id === "string" ? response.id : null,
				tier: typeof response.service_tier === "string" ? response.service_tier : null,
			});
		} catch {
			// Observation only; a missing record leaves the applied tier unestablished.
		}
	});
}
