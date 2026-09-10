import { createHash, randomUUID } from "node:crypto";
import { mkdir, open, readFile, rename, unlink } from "node:fs/promises";
import path from "node:path";

function sha256(value) { return createHash("sha256").update(value).digest("hex"); }
function digest(value) {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/.test(value)) {
    throw new TypeError("retained turn output digest must be SHA-256");
  }
  return value;
}

export class RetainedTurnOutputStore {
  constructor(root) {
    if (typeof root !== "string" || root.trim() === "") throw new TypeError("output store root is required");
    this.root = path.resolve(root);
  }

  reference(outputDigest) { return `sha256:${digest(outputDigest)}`; }
  #path(outputDigest) { return path.join(this.root, digest(outputDigest)); }

  async put(bytes) {
    const body = Buffer.from(bytes);
    const outputDigest = sha256(body);
    await mkdir(this.root, { recursive: true, mode: 0o700 });
    const destination = this.#path(outputDigest);
    try {
      const existing = await readFile(destination);
      if (sha256(existing) !== outputDigest) throw new Error("retained turn output blob failed integrity verification");
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
      const temporary = `${destination}.${process.pid}.${randomUUID()}.tmp`;
      const handle = await open(temporary, "wx", 0o600);
      try { await handle.writeFile(body); await handle.sync(); } finally { await handle.close(); }
      try { await rename(temporary, destination); }
      catch (renameError) { await unlink(temporary).catch(() => {}); throw renameError; }
    }
    return Object.freeze({ availability: "available", digest: outputDigest,
      byteLength: body.length, reference: this.reference(outputDigest) });
  }

  async read({ digest: outputDigest, byteLength, reference }) {
    digest(outputDigest);
    if (reference !== this.reference(outputDigest)) throw new Error("retained turn output reference mismatch");
    let body;
    try { body = await readFile(this.#path(outputDigest)); }
    catch (error) {
      if (error?.code === "ENOENT") return Object.freeze({ status: "unavailable", reason: "blob_missing" });
      throw error;
    }
    if (body.length !== byteLength || sha256(body) !== outputDigest) {
      return Object.freeze({ status: "integrity_failed", reason: "blob_digest_mismatch" });
    }
    return Object.freeze({ status: "available", bytes: body });
  }
}
