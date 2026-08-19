export class CdpSession {
  constructor(webSocketUrl, { eventLimit = 100 } = {}) {
    this.webSocketUrl = webSocketUrl; this.eventLimit = eventLimit; this.sequence = 0;
    this.epochs = { target: 1, session: 0, document: 0 }; this.events = []; this.pending = new Map();
    this.lastDisconnect = null; this.connected = false; this.attach();
  }
  attach() {
    const socket = new WebSocket(this.webSocketUrl); this.socket = socket; this.epochs.session += 1;
    this.ready = new Promise((resolve, reject) => {
      socket.addEventListener("open", () => { if (socket === this.socket) this.connected = true; resolve(); }, { once: true });
      socket.addEventListener("error", reject, { once: true });
    });
    socket.addEventListener("message", (event) => { if (socket === this.socket) this.#message(JSON.parse(event.data)); });
    const disconnected = () => { if (socket === this.socket) this.#disconnect(); };
    socket.addEventListener("close", disconnected, { once: true });
    socket.addEventListener("error", disconnected, { once: true });
  }
  #message(message) {
    if (message.id) {
      const operation = this.pending.get(message.id); if (!operation) return;
      this.pending.delete(message.id); clearTimeout(operation.timer);
      return message.error ? operation.reject(new Error(`${operation.method}: ${message.error.message}`)) : operation.resolve(message.result || {});
    }
    if (message.method === "Page.frameNavigated" || message.method === "Runtime.executionContextsCleared") this.epochs.document += 1;
    this.events.push({ method: message.method, params: message.params || {}, documentEpoch: this.epochs.document });
    if (this.events.length > this.eventLimit) this.events.splice(0, this.events.length - this.eventLimit);
  }
  #disconnect() {
    if (!this.connected && this.lastDisconnect) return;
    this.connected = false; this.lastDisconnect = new Date().toISOString();
    for (const operation of this.pending.values()) { clearTimeout(operation.timer); operation.reject(new Error(`${operation.method}: Chrome debugging connection closed`)); }
    this.pending.clear();
  }
  async call(method, params = {}, timeout = 20000) {
    await this.ready; const id = ++this.sequence;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`${method}: timed out after ${timeout}ms`)); }, timeout);
      this.pending.set(id, { method, resolve, reject, timer });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression, timeout = 60000) {
    const result = await this.call("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true, userGesture: true }, timeout);
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description || result.exceptionDetails.text || "Browser evaluation failed");
    return result.result?.value;
  }
  async reconnect() { try { this.socket.close(); } catch {} this.attach(); await this.ready; }
  markTargetReplacement(webSocketUrl) { this.webSocketUrl = webSocketUrl; this.epochs.target += 1; this.epochs.document = 0; }
  health() { return { connected: this.connected, pendingCalls: this.pending.size, queuedEvents: this.events.length, eventLimit: this.eventLimit, lastDisconnect: this.lastDisconnect, epochs: { ...this.epochs } }; }
  close() { this.socket.close(); }
}
