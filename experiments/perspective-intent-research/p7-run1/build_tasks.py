#!/usr/bin/env python3
"""Build the P7 run 1 task texts and run plan from pinned inputs. No model calls.

Each arm run is a fresh tool-less subagent whose task text contains all of its
inputs inline; this script produces those texts deterministically.

- verifies every input against its Stage 0 or fixture SHA-256;
- renders tasks/<task_id>.md from the versioned prompt templates;
- writes runs/plan.json: 66 arm runs (task id, task SHA-256, arm, method, repeat);
- checks that no leakage term (leakage_terms.txt) occurs in any task text, and
  that every leakage term occurs in at least one withheld source, so the
  post-run scan is meaningful.

Usage (from the repository root):
  python3 -I experiments/perspective-intent-research/p7-run1/build_tasks.py [--check]
--check builds into memory and verifies without writing.
"""

import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
STAGE0 = os.path.join(HERE, "..", "stage0")
REPO = os.path.abspath(os.path.join(HERE, "../../.."))
REPEATS = 3

# id -> (path relative to this directory, expected SHA-256)
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
# B-1539: one entry of the Case B session excerpt (text only).
B1539 = {"excerpt": "../stage0/case-b/session-excerpt.json", "source_line": 4584,
         "raw_line_sha256": "346b00a670701e3b8348f997077d3bae667e75f24df334cc1be9a864cdff9ff7"}

# Withheld sources for the leakage-term check (must contain each term somewhere).
WITHHELD = [
    "../stage0/case-f/packet/DOCKER-AUTOREMOVE-PROPOSAL.md",
    "../stage0/case-f/packet/DOCKER-AUTOREMOVE-SCOPE-DECISION.md",
    "../stage0/case-f/session-excerpt.json",
    "../stage0/case-f/board-messages.json",
    "../stage0/case-b/session-excerpt.json",
    "../../../design/ideas/pending/perspective-intent-research-note.md",
    "../../../design/ideas/pending/pilots/perspective-intent-pilot.md",
    "../../../planning/obligation-provenance-safeguard-plan-20261123.md",
    "../../../skills/manual-orchestration/references/architectural-escalation.md",
]

# input set id -> (kind, parents, derived)
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


def sha256(data):
    return hashlib.sha256(data).hexdigest()


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
    excerpt = json.loads(read(B1539["excerpt"]))
    entry = next(e for e in excerpt["entries"] if e["source_line"] == B1539["source_line"])
    if entry["raw_line_sha256"] != B1539["raw_line_sha256"]:
        raise SystemExit("B-1539: raw line digest mismatch")
    texts["B-1539"] = entry["text"]
    digests["B-1539"] = sha256(entry["text"].encode())
    return texts, digests


def render_docs(ids, texts, label):
    if not ids:
        return f"{label} DOCUMENTS: none"
    parts = []
    for doc_id in ids:
        parts.append(f"=== {label} DOCUMENT {doc_id} ===\n{texts[doc_id].rstrip()}\n=== END {doc_id} ===")
    return "\n\n".join(parts)


def build(texts):
    templates = {name: read(f"prompts/{name}.md").decode() for name in
                 ("A1-planner", "M1-plan", "M1-review", "M2-extract")}
    schema = read("schema/p7-obligation-extract-v1.json").decode()
    tasks = {}
    for set_id, (kind, parents, derived) in SETS.items():
        if kind == "planner":
            body = templates["A1-planner"].replace("{{DOCUMENTS}}", render_docs(parents, texts, "SOURCE"))
            tasks[f"{set_id}.A1"] = body
            continue
        if kind == "plan":
            m1 = templates["M1-plan"].replace("{{DOCUMENTS}}", render_docs(parents, texts, "SOURCE"))
        else:
            m1 = (templates["M1-review"].replace("{{PARENTS}}", render_docs(parents, texts, "PARENT"))
                  .replace("{{DERIVED}}", render_docs(derived, texts, "DERIVED")))
        m2 = (templates["M2-extract"].replace("{{SCHEMA}}", schema)
              .replace("{{PARENTS}}", render_docs(parents, texts, "PARENT"))
              .replace("{{DERIVED}}", render_docs(derived, texts, "DERIVED")))
        tasks[f"{set_id}.M1"] = m1
        tasks[f"{set_id}.M2"] = m2  # M3 uses the same extraction task, then fallback calls
    for task_id, body in tasks.items():
        if "{{" in body:
            raise SystemExit(f"{task_id}: unfilled placeholder")
    return tasks


def plan(tasks):
    runs = []
    for set_id, (kind, _, _) in SETS.items():
        methods = ["A1"] if kind == "planner" else ["M1", "M2", "M3"]
        for method in methods:
            task_id = f"{set_id}.{'M2' if method == 'M3' else method}"
            for r in range(1, REPEATS + 1):
                runs.append({"run_id": f"{set_id}.{method}.r{r}", "input_set": set_id, "method": method,
                             "repeat": r, "task_id": task_id,
                             "task_sha256": sha256(tasks[task_id].encode()),
                             "model": "claude-sdk/claude-opus-5-5:high", "tools": "none"})
    return runs


def check_leakage(tasks):
    terms = [t.strip() for t in read("leakage_terms.txt").decode().splitlines()
             if t.strip() and not t.startswith("#")]
    withheld = "\n".join(read(p).decode(errors="replace") for p in WITHHELD).lower()
    problems = []
    for term in terms:
        low = term.lower()
        hits = [tid for tid, body in tasks.items() if low in body.lower()]
        if hits:
            problems.append(f"leakage term {term!r} occurs in task text {hits}")
        if low not in withheld:
            problems.append(f"leakage term {term!r} occurs in no withheld source; it cannot detect leakage")
    if problems:
        raise SystemExit("\n".join(problems))
    return terms


def main(argv):
    texts, digests = load_inputs()
    tasks = build(texts)
    runs = plan(tasks)
    terms = check_leakage(tasks)
    if len(runs) != 66:
        raise SystemExit(f"expected 66 runs, got {len(runs)}")
    summary = {"runs": len(runs), "unique_tasks": len(tasks), "leakage_terms": len(terms)}
    if "--check" not in argv:
        os.makedirs(os.path.join(HERE, "tasks"), exist_ok=True)
        os.makedirs(os.path.join(HERE, "runs"), exist_ok=True)
        for task_id, body in tasks.items():
            with open(os.path.join(HERE, "tasks", f"{task_id}.md"), "w") as f:
                f.write(body)
        fixture_digests = {k: v for k, v in digests.items() if k.startswith("NC-")}
        with open(os.path.join(HERE, "runs", "plan.json"), "w") as f:
            json.dump({"protocol": "p7-run1", "input_digests": digests,
                       "fixture_digests": fixture_digests,
                       "template_digests": {n: sha256(read(f"prompts/{n}.md")) for n in
                                            ("A1-planner", "M1-plan", "M1-review", "M2-extract", "M3-fallback", "scorer")},
                       "schema_digest": sha256(read("schema/p7-obligation-extract-v1.json")),
                       "rubric_digest": sha256(read("prompts/rubric.json")),
                       "comparator_digest": sha256(read("comparator.py")),
                       "runs": runs}, f, indent=2)
            f.write("\n")
    print(json.dumps(summary))


if __name__ == "__main__":
    main(sys.argv)
