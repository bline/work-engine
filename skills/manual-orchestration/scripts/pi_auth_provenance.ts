// Install as ~/.pi/agent/extensions/pi-auth-provenance.ts for Pi auto-discovery.
// This observes configuration, not the provider invoice or the final wire request.
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

type Snapshot = {
  billing_class: "oauth" | "api_key" | "bearer" | "unknown";
  source: string | null;
  captured_at: string;
};

// Keep only known source labels, never auth objects, headers, or resolver errors.
const SOURCES = new Set([
  "OAuth", "stored credential", "ANTHROPIC_API_KEY", "ANTHROPIC_OAUTH_TOKEN",
  "ANTHROPIC_AUTH_TOKEN", "workload identity federation",
]);

export default function (pi: ExtensionAPI) {
  let turn: { requests: number; snapshot?: Snapshot } | undefined;
  pi.on("turn_start", () => { turn = { requests: 0 }; });
  pi.on("turn_end", () => { turn = undefined; });
  pi.on("agent_end", () => { turn = undefined; });

  pi.on("before_provider_headers", async (event, ctx) => {
    const current = turn;
    // The hook exposes no provider or request id. If a second request (e.g.
    // cache warming) fires in this turn, do not guess which snapshot belongs.
    if (!current || ++current.requests !== 1) return;
    let billing_class: Snapshot["billing_class"] = "unknown";
    let source: string | null = null;
    try {
      const resolution = await ctx.modelRegistry.getProviderAuth("anthropic");
      if (resolution && SOURCES.has(resolution.source)) source = resolution.source;
      const key = resolution?.auth.apiKey;
      const authorization = Object.entries(event.headers).find(([name]) => name.toLowerCase() === "authorization")?.[1];
      const keyHeader = Object.entries(event.headers).find(([name]) => name.toLowerCase() === "x-api-key")?.[1];
      if (typeof authorization === "string" && /^Bearer\s+\S/i.test(authorization)) {
        billing_class = keyHeader ? "unknown" : "bearer";
      } else if (!authorization && key && (!keyHeader || keyHeader === key)) {
        // Match Pi's isOAuthToken test, not isUsingOAuth (stored login type).
        billing_class = key.includes("sk-ant-oat") ? "oauth" : "api_key";
      }
    } catch {
      // Leave attribution unknown without reporting any credential-bearing error.
    }
    current.snapshot = { billing_class, source, captured_at: new Date().toISOString() };
  });

  pi.on("message_end", ({ message }) => {
    if (message.role !== "assistant") return;
    const current = turn;
    turn = undefined;
    if (message.provider !== "anthropic" || !message.responseId ||
        current?.requests !== 1 || !current.snapshot) return;
    pi.appendEntry("auth-provenance", {
      responseId: message.responseId,
      provider: message.provider,
      model: message.responseModel || message.model,
      confidence: "configuration_snapshot",
      ...current.snapshot,
    });
  });
}
