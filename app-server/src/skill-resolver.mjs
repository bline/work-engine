import { realpath, stat } from "node:fs/promises";
import path from "node:path";

function containedBy(candidate, root) {
  const relative = path.relative(root, candidate);
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative));
}

export class ExactSkillResolver {
  static async create(allowedRoots) {
    if (!Array.isArray(allowedRoots) || allowedRoots.length === 0) {
      throw new TypeError("at least one skill root is required");
    }
    const roots = await Promise.all(allowedRoots.map((root) => realpath(path.resolve(root))));
    return new ExactSkillResolver(roots);
  }

  constructor(canonicalRoots) {
    this.allowedRoots = Object.freeze([...canonicalRoots]);
  }

  async resolve(skills = []) {
    const items = [];
    const names = new Set();
    const paths = new Set();
    for (const skill of skills) {
      if (typeof skill?.name !== "string" || skill.name.trim() === "") {
        throw new TypeError("skill name must be a non-empty string");
      }
      const canonicalPath = await realpath(path.resolve(skill.path));
      const metadata = await stat(canonicalPath);
      if (!metadata.isFile() || path.basename(canonicalPath) !== "SKILL.md") {
        throw new Error(`skill path must name an exact SKILL.md file: ${skill.path}`);
      }
      if (!this.allowedRoots.some((root) => containedBy(canonicalPath, root))) {
        throw new Error(`skill path is outside configured roots: ${skill.path}`);
      }
      if (names.has(skill.name) || paths.has(canonicalPath)) {
        throw new Error(`duplicate skill binding: ${skill.name}`);
      }
      names.add(skill.name);
      paths.add(canonicalPath);
      items.push({ type: "skill", name: skill.name, path: canonicalPath });
    }
    return items;
  }
}
