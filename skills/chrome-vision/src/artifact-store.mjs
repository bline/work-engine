import fs from "node:fs/promises"; import path from "node:path"; import crypto from "node:crypto";
export class ArtifactStore {
  constructor(directory, limit = 20) { this.directory = path.resolve(directory); this.limit = limit; this.count = 0; }
  async write(name, bytes, mediaType) {
    if (this.count >= this.limit) return { omitted: true, limitation: `artifact limit ${this.limit} reached` };
    await fs.mkdir(this.directory, { recursive: true }); const safe = path.basename(name); const target = path.join(this.directory, safe);
    await fs.writeFile(target, bytes); this.count += 1;
    return { path: target, mediaType, bytes: bytes.length, sha256: crypto.createHash("sha256").update(bytes).digest("hex") };
  }
}
