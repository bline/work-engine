"""Project register.py `anchors` reports onto the fields the Rust historical regression compares.

Usage: python3 -I project.py <ref_py_dir> <out_dir> <targets.txt>
Reads <ref_py_dir>/<commit>/anchors.json for every commit in targets.txt and writes
<out_dir>/<commit>.json. Standard library only; output is deterministic (sorted keys, LF).
"""
import json
import os
import sys

FIELDS = ("decision_id", "class", "error", "owner_review_required", "owner_review_reasons",
          "revision_commit", "before", "after", "candidates", "document_at_target")


def chain(ctx):
    return ctx["section_chain"] if isinstance(ctx, dict) else None


def project(report):
    entries = []
    for e in report["decisions"]:
        row = {k: e.get(k) for k in FIELDS}
        row["context_before.section_chain"] = chain(e.get("context_before"))
        row["context_after.section_chain"] = chain(e.get("context_after"))
        entries.append(row)
    return {"pinned_commit": report["pinned_commit"],
            "register_path": report["register"]["directory"],
            "entries": entries}


def main(ref, out, targets):
    for commit in open(targets).read().split():
        with open(os.path.join(ref, commit, "anchors.json")) as fh:
            report = json.load(fh)
        assert report["pinned_commit"] == commit, commit
        with open(os.path.join(out, f"{commit}.json"), "w", newline="\n") as fh:
            json.dump(project(report), fh, indent=1, sort_keys=True, ensure_ascii=False)
            fh.write("\n")


if __name__ == "__main__":
    main(*sys.argv[1:4])
