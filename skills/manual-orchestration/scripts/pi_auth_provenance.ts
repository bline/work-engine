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
  "ANTHROPIC_AUTH_TOKEN", "workload identity federation", "OPENAI_API_KEY",
]);

const PROVIDERS = ["anthropic", "openai"] as const;
type Provider = typeof PROVIDERS[number];

export default function (pi: ExtensionAPI) {
  let turn: { requests: number; snapshots?: Map<Provider, Snapshot> } | undefined;
  pi.on("turn_start", () => { turn = { requests: 0 }; });
  pi.on("turn_end", () => { turn = undefined; });
  pi.on("agent_end", () => { turn = undefined; });

  pi.on("before_provider_headers", async (event, ctx) => {
    const current = turn;
    // The hook exposes no provider or request id. If a second request (e.g.
    // cache warming) fires in this turn, do not guess which snapshot belongs.
    if (!current || ++current.requests !== 1) return;
    const authorization = Object.entries(event.headers).find(([name]) => name.toLowerCase() === "authorization")?.[1];
    const keyHeader = Object.entries(event.headers).find(([name]) => name.toLowerCase() === "x-api-key")?.[1];
    current.snapshots = new Map(await Promise.all(PROVIDERS.map(async (provider) => {
      let billing_class: Snapshot["billing_class"] = "unknown";
      let source: string | null = null;
      try {
        const resolution = await ctx.modelRegistry.getProviderAuth(provider);
        if (typeof resolution?.source === "string" && SOURCES.has(resolution.source)) source = resolution.source;
        const key = resolution?.auth.apiKey;
        if (provider === "openai") {
          // Sign in with ChatGPT also populates apiKey with its access token.
          // Pi's resolver distinguishes it by source, not the token format.
          if (key && !keyHeader && (!authorization || authorization === `Bearer ${key}`)) {
            if (source === "OAuth") billing_class = "oauth";
            if (source === "OPENAI_API_KEY" || source === "stored credential") billing_class = "api_key";
          }
        } else if (typeof authorization === "string" && /^Bearer\s+\S/i.test(authorization)) {
          billing_class = keyHeader ? "unknown" : "bearer";
        } else if (!authorization && key && (!keyHeader || keyHeader === key)) {
          billing_class = key.includes("sk-ant-oat") ? "oauth" : "api_key";
        }
      } catch {
        // Leave attribution unknown without reporting any credential-bearing error.
      }
      return [provider, { billing_class, source, captured_at: new Date().toISOString() }] as const;
    })));
  });

  pi.on("message_end", ({ message }) => {
    if (message.role !== "assistant") return;
    const current = turn;
    turn = undefined;
    if ((message.provider !== "anthropic" && message.provider !== "openai") || !message.responseId ||
        current?.requests !== 1) return;
    const snapshot = current.snapshots?.get(message.provider);
    if (!snapshot) return;
    pi.appendEntry("auth-provenance", {
      responseId: message.responseId,
      provider: message.provider,
      model: message.responseModel || message.model,
      confidence: "configuration_snapshot",
      ...snapshot,
    });
  });
}
