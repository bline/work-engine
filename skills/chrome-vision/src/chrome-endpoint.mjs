export class ChromeEndpoint {
  constructor(endpoint, timeoutMs = 5000) { this.endpoint = endpoint.replace(/\/$/, ""); this.timeoutMs = timeoutMs; }
  async fetch(path) {
    const controller = new AbortController(); const timer = setTimeout(() => controller.abort(), this.timeoutMs);
    try { return await fetch(`${this.endpoint}${path}`, { signal: controller.signal }); }
    catch (error) { throw new Error(`Chrome endpoint ${path} failed: ${error.name === "AbortError" ? `timed out after ${this.timeoutMs}ms` : error.message}`); }
    finally { clearTimeout(timer); }
  }
  async listTargets() {
    const response = await this.fetch("/json/list");
    if (!response.ok) throw new Error(`Chrome target list failed: HTTP ${response.status}`);
    return response.json();
  }
  async browserWebSocketUrl() {
    const response = await this.fetch("/json/version");
    if (!response.ok) throw new Error(`Chrome version failed: HTTP ${response.status}`);
    return (await response.json()).webSocketDebuggerUrl;
  }
}

export function selectTarget(targets, matcher = {}) {
  const url = matcher.urlPattern ? new RegExp(matcher.urlPattern) : null;
  const title = matcher.titlePattern ? new RegExp(matcher.titlePattern) : null;
  return targets.find((target) => (!matcher.type || target.type === matcher.type) && (!url || url.test(target.url || "")) && (!title || title.test(target.title || ""))) || null;
}
