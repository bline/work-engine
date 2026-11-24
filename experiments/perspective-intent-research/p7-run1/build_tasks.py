#!/usr/bin/env python3
"""Build (or verify) the P7 run 1 task texts and run plan from pinned inputs. No model calls.

Each arm run is a fresh tool-less subagent whose task text contains all of its
inputs inline. Documents are rendered byte-exact under neutral ids (DOC-1, ...);
the mapping to original ids lives only in runs/plan.json.

  python3 -I -B experiments/perspective-intent-research/p7-run1/build_tasks.py           # write tasks/ and runs/plan.json
  python3 -I -B experiments/perspective-intent-research/p7-run1/build_tasks.py --check   # verify on-disk artifacts, write nothing

--check re-renders everything in memory and compares it, byte for byte, with
tasks/*.md and runs/plan.json, which records the digests of every asset
(templates, concepts, schema, rubric, comparator, pipeline, agents, leakage
terms, fixtures). Any difference, missing or extra file fails the check.
"""

import hashlib
import json
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPEATS = 3
SEED = 20261124

# original id -> (path relative to this directory, expected SHA-256 or None for fixtures)
INPUTS = {
    "F-043": ("../stage0/case-f/packet/review-r1/043-u4a-held-contract.md", "4bf5ee603ce9854febf29d22e063825cc7e9d795049ca52d965b7939cf7dcd04"),
    "F-041": ("../stage0/case-f/packet/review-r1/041-u4-protection-qualification-PLAN.md", "c1c6090c136ce75504f63bb0ec4a2c8ab60fb7cd9b25e7b8ea9edb54734fd23f"),
    "F-PLAN": ("../stage0/case-f/packet/PLAN.md", "c7511ad79ba57e3de4534576e2fd87ba15abcc9980569aadbc3d0eb852e9b9fd"),
    "F-RE-BRIEF": ("../stage0/case-f/packet/ARCHITECTURE-REESCALATION.md", "d8125989e607696c09e584a5c115dd3ec44621c3a6a9fd0bbca271c9e5988c3b"),
    "F-RE": ("../stage0/case-f/packet/architecture-reescalation-review/ASTRA-ARCHITECTURE.md", "12127e463a3064b0292eb3515681da0bb7699f15660113c352301c32bb2e19d5"),
    "F-AMEND": ("../stage0/case-f/packet/QUALIFICATION-AMENDMENT.md", "93e004dc8b18c900f5495c0926dd354d5abcd1265143e08bd85add399aa8545e"),
    "F-RO-BRIEF": ("../stage0/case-f/packet/RESOLVED-OBJECT-ESCALATION.md", "45994d705b5c2aad795ae9f62c5237c66d0d2e103f415e6c9bee8900f94dec1f"),
    "F-RO": ("../stage0/case-f/packet/resolved-object-review/ASTRA-ARCHITECTURE.md", "e8304cda2ec086702b29a59afc7acea13f84098d1852f043d6f56c7d518d0824"),
    "F-DEC": ("../stage0/case-f/packet/resolved-object-review/OPERATOR-DECISION.md", "d0055826f82250d4e93112d5ba262fab3da588cd870873cc9d705ff350154df9"),
    "B-REV": ("../stage0/case-b/packet/ORG-ASTRA-DESIGN-REVIEW-RESULT.md", "d6d71c5635e476725e400119e230c3ba9c8b13c51f0532d279810dab8b610a9e"),
    "B-DECREC": ("../stage0/case-b/packet/CONTROLLED-BUILDER-ADMISSION-REVISION.md", "2343a8f888bfa80219660532e0d8e573edb37392102e912a3148a3f4c7dcd491"),
    "NC-A": ("fixtures/NC-A-contract.md", None),
    "NC-B-CONTRACT": ("fixtures/NC-B-contract.md", None),
    "NC-B-REVIEW": ("fixtures/NC-B-review.md", None),
    "NC-C-PARENT": ("fixtures/NC-C-parent.md", None),
    "NC-C-DISPOSITION": ("fixtures/NC-C-disposition.md", None),
}
# B-1539: one entry of the Case B session excerpt, text only.
B1539 = {"excerpt": "../stage0/case-b/session-excerpt.json", "manifest": "../stage0/MANIFEST-case-b.json",
         "source_line": 4584,
         "raw_line_sha256": "346b00a670701e3b8348f997077d3bae667e75f24df334cc1be9a864cdff9ff7"}

WITHHELD = [
    "../stage0/case-f/packet/DOCKER-AUTOREMOVE-PROPOSAL.md",
    "../stage0/case-f/packet/DOCKER-AUTOREMOVE-SCOPE-DECISION.md",
    "../stage0/case-f/session-excerpt.json",
    "../stage0/case-f/board-messages.json",
    "../stage0/case-b/session-excerpt.json",
    "../../../design/ideas/pending/perspective-intent-research-note.md",
    "../../../design/ideas/pending/pilots/perspective-intent-pilot.md",
    "../../../planning/obligation-provenance-safeguard-plan-20261123.md",
]

# input set -> (kind, parents, derived). Context-only documents are rendered but not scored.
SETS = {
    "A1-F": ("planner", ["F-043", "F-041", "F-PLAN"], []),
    "A2-F": ("plan", ["F-043", "F-041", "F-PLAN"], []),
    "A3-F": ("review", ["F-043", "F-041", "F-PLAN"], ["F-RE-BRIEF", "F-RE", "F-AMEND", "F-RO-BRIEF", "F-RO", "F-DEC"]),
    "T1": ("plan", ["B-REV", "B-1539"], []),
    "T2": ("plan", ["B-DECREC"], []),
    "NC-A": ("plan", ["NC-A"], []),
    "NC-B": ("review", ["NC-B-CONTRACT"], ["NC-B-REVIEW"]),
    "NC-C": ("review", ["NC-C-PARENT"], ["NC-C-DISPOSITION"]),
}

ASSETS = ["prompts/A1-planner.md", "prompts/M1-plan.md", "prompts/M1-review.md", "prompts/M2-extract.md",
          "prompts/M3-fallback.md", "prompts/scorer.md", "prompts/rubric.json", "prompts/concepts.md",
          "schema/obligation-extract-v2.json", "comparator.py", "pipeline.py", "leakage_terms.txt",
          "agents/p7-arm.md", "agents/p7-scorer.md", "build_tasks.py"]


def read_text(path):
    with open(path) as f:
        return f.read()


def sha256(data):
    return hashlib.sha256(data if isinstance(data, bytes) else data.encode()).hexdigest()


def read(rel):
    with open(os.path.join(HERE, rel), "rb") as f:
        return f.read()


def load_inputs():
    texts, digests = {}, {}
    for doc_id, (rel, expected) in INPUTS.items():
        data = read(rel)
        digest = sha256(data)
        if expected and digest != expected:
            raise SystemExit(f"{doc_id}: expected {expected}, got {digest}")
        texts[doc_id] = data.decode()
        digests[doc_id] = digest
    excerpt_bytes = read(B1539["excerpt"])
    manifest = json.loads(read(B1539["manifest"]))
    if sha256(excerpt_bytes) != manifest["case_b_session_excerpt"]["sha256"]:
        raise SystemExit("B-1539: session excerpt does not match its Stage 0 manifest")
    entries = [e for e in json.loads(excerpt_bytes)["entries"] if e["source_line"] == B1539["source_line"]]
    if len(entries) != 1 or entries[0]["raw_line_sha256"] != B1539["raw_line_sha256"]:
        raise SystemExit("B-1539: excerpt entry coordinate or raw-line digest mismatch")
    texts["B-1539"] = entries[0]["text"]
    digests["B-1539"] = sha256(entries[0]["text"])
    return texts, digests


def render_docs(ids, id_map, texts, label):
    if not ids:
        return f"{label} DOCUMENTS: none"
    parts = []
    for doc_id in ids:
        nid = id_map[doc_id]
        body = texts[doc_id]
        sep = "" if body.endswith("\n") else "\n"
        parts.append(f"=== {label} DOCUMENT {nid} ===\n{body}{sep}=== END {nid} ===")
    return "\n\n".join(parts)


def build(texts):
    t = {n: read(f"prompts/{n}.md").decode() for n in ("A1-planner", "M1-plan", "M1-review", "M2-extract", "concepts")}
    schema = read("schema/obligation-extract-v2.json").decode()
    tasks, task_documents = {}, {}
    for set_id, (kind, parents, derived) in SETS.items():
        order = parents + derived
        id_map = {doc: f"DOC-{i + 1}" for i, doc in enumerate(order)}
        meta = {"kind": kind if kind != "planner" else "plan", "order": [id_map[d] for d in order],
                "parents": [id_map[d] for d in parents], "derived": [id_map[d] for d in derived],
                "documents": {id_map[d]: texts[d] for d in order}, "id_map": id_map}
        source_label = "SOURCE" if kind in ("planner", "plan") else "PARENT"
        if kind == "planner":
            tasks[f"{set_id}.A1"] = t["A1-planner"].replace("{{DOCUMENTS}}", render_docs(parents, id_map, texts, "SOURCE"))
            task_documents[f"{set_id}.A1"] = meta
            continue
        if kind == "plan":
            m1 = t["M1-plan"].replace("{{DOCUMENTS}}", render_docs(parents, id_map, texts, "SOURCE"))
        else:
            m1 = (t["M1-review"].replace("{{PARENTS}}", render_docs(parents, id_map, texts, "PARENT"))
                  .replace("{{DERIVED}}", render_docs(derived, id_map, texts, "DERIVED")))
        m1 = m1.replace("{{CONCEPTS}}", t["concepts"].rstrip("\n"))
        m2 = (t["M2-extract"].replace("{{CONCEPTS}}", t["concepts"].rstrip("\n")).replace("{{SCHEMA}}", schema)
              .replace("{{PARENTS}}", render_docs(parents, id_map, texts, source_label))
              .replace("{{DERIVED}}", render_docs(derived, id_map, texts, "DERIVED")))
        tasks[f"{set_id}.M1"], tasks[f"{set_id}.M2"] = m1, m2
        task_documents[f"{set_id}.M1"] = task_documents[f"{set_id}.M2"] = meta
    for task_id, body in tasks.items():
        if "{{" in body:
            raise SystemExit(f"{task_id}: unfilled placeholder")
    return tasks, task_documents


def make_runs(tasks):
    runs = []
    for set_id, (kind, _, _) in SETS.items():
        for method in (["A1"] if kind == "planner" else ["M1", "M2", "M3"]):
            task_id = f"{set_id}.{'M2' if method == 'M3' else method}"
            for r in range(1, REPEATS + 1):
                runs.append({"run_id": f"{set_id}.{method}.r{r}", "input_set": set_id, "method": method, "repeat": r,
                             "task_id": task_id, "task_sha256": sha256(tasks[task_id]),
                             "agent": "p7-arm", "model": "claude-sdk/claude-opus-5-5", "thinking": "high"})
    order = [r["run_id"] for r in runs]
    random.Random(SEED).shuffle(order)
    return runs, order


def check_leakage(tasks):
    terms = [x.strip() for x in read("leakage_terms.txt").decode().splitlines() if x.strip() and not x.startswith("#")]
    withheld = "\n".join(read(p).decode(errors="replace") for p in WITHHELD).lower()
    problems = []
    for term in terms:
        low = term.lower()
        hits = [tid for tid, body in tasks.items() if low in body.lower()]
        if hits:
            problems.append(f"leakage term {term!r} occurs in task text {hits}")
        if low not in withheld:
            problems.append(f"leakage term {term!r} occurs in no withheld source")
    if problems:
        raise SystemExit("\n".join(problems))
    return terms


def expected_artifacts():
    texts, digests = load_inputs()
    tasks, task_documents = build(texts)
    runs, order = make_runs(tasks)
    terms = check_leakage(tasks)
    if len(runs) != 66:
        raise SystemExit(f"expected 66 runs, got {len(runs)}")
    plan = {"protocol": "p7-run1", "seed": SEED, "input_digests": digests,
            "asset_digests": {a: sha256(read(a)) for a in ASSETS},
            "fixture_digests": {k: v for k, v in digests.items() if k.startswith("NC-")},
            "task_documents": task_documents, "runs": runs, "execution_order": order,
            "leakage_terms": terms}
    return tasks, plan


def verify(tasks, plan, disk_tasks, disk_plan):
    """Pure comparison of expected artifacts with what is on disk. Returns a list of problems."""
    problems = []
    for tid in sorted(set(tasks) | set(disk_tasks)):
        if tid not in disk_tasks:
            problems.append(f"missing task file {tid}")
        elif tid not in tasks:
            problems.append(f"unexpected task file {tid}")
        elif tasks[tid] != disk_tasks[tid]:
            problems.append(f"task {tid} differs from its render")
    if disk_plan != plan:
        diff = sorted(k for k in set(plan) | set(disk_plan or {}) if (disk_plan or {}).get(k) != plan.get(k))
        problems.append(f"plan.json differs in {diff}")
    return problems


def read_disk():
    tdir = os.path.join(HERE, "tasks")
    disk_tasks = {}
    if os.path.isdir(tdir):
        for name in os.listdir(tdir):
            if name.endswith(".md"):
                with open(os.path.join(tdir, name)) as f:
                    disk_tasks[name[:-3]] = f.read()
    ppath = os.path.join(HERE, "runs", "plan.json")
    disk_plan = json.loads(read_text(ppath)) if os.path.exists(ppath) else None
    return disk_tasks, disk_plan


def main(argv):
    tasks, plan = expected_artifacts()
    if "--check" in argv:
        problems = verify(tasks, plan, *read_disk())
        if problems:
            print("\n".join(problems))
            return 21
        print(json.dumps({"ok": True, "runs": len(plan["runs"]), "unique_tasks": len(tasks),
                          "leakage_terms": len(plan["leakage_terms"])}))
        return 0
    tdir = os.path.join(HERE, "tasks")
    os.makedirs(tdir, exist_ok=True)
    for name in os.listdir(tdir):
        if name.endswith(".md") and name[:-3] not in tasks:
            os.remove(os.path.join(tdir, name))
    for tid, body in tasks.items():
        with open(os.path.join(tdir, f"{tid}.md"), "w") as f:
            f.write(body)
    os.makedirs(os.path.join(HERE, "runs"), exist_ok=True)
    with open(os.path.join(HERE, "runs", "plan.json"), "w") as f:
        json.dump(plan, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(json.dumps({"written": True, "runs": len(plan["runs"]), "unique_tasks": len(tasks)}))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
