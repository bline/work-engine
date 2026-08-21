#!/usr/bin/env python3
"""Validate, analyze, and render Work Engine Agent Environment Graph inputs."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

import yaml


class GraphError(ValueError):
    """Raised when graph inputs or judgments violate their deterministic contract."""


SEMANTIC_CATEGORIES = {
    "invariant_status", "causal_parent", "ownership", "authority",
    "conditionality", "equivalence",
}
TOP_LEVEL_KEYS = {
    "schema_version", "document_id", "status", "verified_on", "scope",
    "sources", "semantics", "entities", "roles", "analysis_queries",
}
ROLE_KEYS = {
    "label", "objective", "context_lifetime", "bound_by", "must_require",
    "may_invoke", "may_observe", "observation_limits", "may_mutate", "owns",
    "consumes", "emits", "mediated_transitions", "forbidden_from", "independence",
}
RELATION_LISTS = ("bound_by", "may_invoke", "may_observe", "owns", "consumes", "emits")
RELATION_LABELS = {
    "bound_by": "BOUND_BY", "may_invoke": "MAY_INVOKE", "owns": "OWNS",
    "may_observe": "MAY_OBSERVE", "consumes": "CONSUMES", "emits": "EMITS",
}


class UniqueKeyLoader(yaml.SafeLoader):
    pass


def _construct_mapping(loader: UniqueKeyLoader, node: yaml.MappingNode, deep: bool = False) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in result:
            raise GraphError(f"duplicate YAML key: {key}")
        result[key] = loader.construct_object(value_node, deep=deep)
    return result


UniqueKeyLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _construct_mapping)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise GraphError(message)


def load_yaml(path: Path) -> dict[str, Any]:
    try:
        value = yaml.load(path.read_text(encoding="utf-8"), Loader=UniqueKeyLoader)
    except yaml.YAMLError as error:
        raise GraphError(f"invalid YAML: {error}") from error
    require(isinstance(value, dict), "environment document must be a mapping")
    return value


def _split_row(line: str) -> list[str]:
    """Split a Markdown table row while preserving escaped pipes as cell text."""
    value = line.strip()
    if value.startswith("|"):
        value = value[1:]
    if value.endswith("|") and not value.endswith(r"\|"):
        value = value[:-1]
    cells: list[str] = []
    cell: list[str] = []
    index = 0
    while index < len(value):
        character = value[index]
        if character == "\\" and index + 1 < len(value) and value[index + 1] == "|":
            cell.append("|")
            index += 2
            continue
        if character == "|":
            cells.append("".join(cell).strip())
            cell = []
        else:
            cell.append(character)
        index += 1
    cells.append("".join(cell).strip())
    return cells


def _table_rows(source: str, heading: str) -> list[list[str]]:
    start = source.find(heading)
    require(start >= 0, f"missing section: {heading}")
    tail = source[start + len(heading):]
    boundary = re.search(r"\n##(?:#)? ", tail)
    section = tail[:boundary.start()] if boundary else tail
    rows = []
    for line in section.splitlines():
        if not line.startswith("|") or re.match(r"^\|[ -]+\|", line):
            continue
        cells = _split_row(line)
        if cells and cells[0] not in {"ID", "Field"}:
            rows.append(cells)
    return rows


def parse_invariants(path: Path) -> dict[str, Any]:
    source = path.read_text(encoding="utf-8")
    invariant_rows: list[list[str]] = []
    catalog_start = source.find("## Invariant catalog")
    machinery_start = source.find("## Current machinery catalog")
    require(catalog_start >= 0 and machinery_start > catalog_start, "missing invariant or machinery catalog")
    for line in source[catalog_start:machinery_start].splitlines():
        if line.startswith("|"):
            cells = _split_row(line)
            if cells and re.fullmatch(r"`INV-[0-9]+`", cells[0]):
                invariant_rows.append(cells)
    mechanism_rows = _table_rows(source, "## Current machinery catalog")
    invariants: dict[str, dict[str, Any]] = {}
    for row in invariant_rows:
        require(len(row) == 10, f"malformed invariant row: {row[0]}")
        identifier = row[0].strip("`")
        require(identifier not in invariants, f"duplicate invariant id: {identifier}")
        invariants[identifier] = {
            "owner": row[1], "applies": row[2], "class": row[3],
            "condition": row[4], "causal_parent": row[5], "enforcement": row[6],
            "mechanisms": re.findall(r"`(MECH-[A-Z0-9-]+)`", row[7]),
            "relations": re.findall(r"`(INV-[0-9]+)`", row[8]), "sources": row[9],
        }
    mechanisms: dict[str, dict[str, str]] = {}
    for row in mechanism_rows:
        if not row or not re.fullmatch(r"`MECH-[A-Z0-9-]+`", row[0]):
            continue
        require(len(row) == 4, f"malformed mechanism row: {row[0]}")
        identifier = row[0].strip("`")
        require(identifier not in mechanisms, f"duplicate mechanism id: {identifier}")
        mechanisms[identifier] = {"component": row[1], "affordance": row[2], "edges": row[3]}
    require(bool(invariants), "invariant catalog is empty")
    require(bool(mechanisms), "machinery catalog is empty")
    for identifier, item in invariants.items():
        for target in item["relations"]:
            require(target in invariants, f"{identifier} references unknown invariant {target}")
        for mechanism in item["mechanisms"]:
            require(mechanism in mechanisms, f"{identifier} references unknown mechanism {mechanism}")
    return {"invariants": invariants, "mechanisms": mechanisms, "source": source}


def _unique_strings(values: Any, path: str) -> list[str]:
    require(isinstance(values, list), f"{path} must be a list")
    require(all(isinstance(value, str) and value for value in values), f"{path} must contain nonempty strings")
    require(len(values) == len(set(values)), f"{path} contains duplicate values")
    return values


def validate_environment(data: dict[str, Any], catalog: dict[str, Any]) -> dict[str, Any]:
    unknown = set(data) - TOP_LEVEL_KEYS
    require(not unknown, f"unknown environment fields: {', '.join(sorted(unknown))}")
    require(data.get("schema_version") == 1, "schema_version must be 1")
    entities = data.get("entities")
    roles = data.get("roles")
    require(isinstance(entities, dict) and isinstance(roles, dict) and roles, "entities and roles are required mappings")
    require(set(entities) == {"states", "artifacts", "capabilities"}, "entities must contain states, artifacts, capabilities")
    entity_ids: set[str] = set()
    for kind, mapping in entities.items():
        require(isinstance(mapping, dict), f"entities.{kind} must be a mapping")
        for identifier, item in mapping.items():
            require(identifier not in entity_ids, f"duplicate entity id: {identifier}")
            require(isinstance(item, dict) and isinstance(item.get("label"), str), f"{identifier} requires a label")
            allowed = {"label", "mechanism", "configured", "configured_or_risk_selected"}
            require(not set(item) - allowed, f"{identifier} has unknown fields")
            if "mechanism" in item:
                require(item["mechanism"] in catalog["mechanisms"], f"{identifier} references unknown mechanism {item['mechanism']}")
            entity_ids.add(identifier)
    role_ids = set(roles)
    for role_id, role in roles.items():
        require(role_id.startswith("role.") and isinstance(role, dict), f"invalid role: {role_id}")
        require(not set(role) - ROLE_KEYS, f"{role_id} has unknown fields: {', '.join(sorted(set(role) - ROLE_KEYS))}")
        for required in ("label", "objective", "context_lifetime"):
            require(isinstance(role.get(required), str) and role[required].strip(), f"{role_id}.{required} is required")
        for field in RELATION_LISTS:
            for target in _unique_strings(role.get(field, []), f"{role_id}.{field}"):
                if field == "bound_by":
                    require(target in catalog["invariants"], f"{role_id}.{field} references unknown invariant {target}")
                else:
                    require(target in entity_ids, f"{role_id}.{field} references unknown entity {target}")
        for target_role, invariant_ids in role.get("must_require", {}).items():
            require(target_role in role_ids, f"{role_id}.must_require references unknown role {target_role}")
            for invariant_id in _unique_strings(invariant_ids, f"{role_id}.must_require.{target_role}"):
                require(invariant_id in catalog["invariants"], f"{role_id}.must_require references unknown invariant {invariant_id}")
        for index, mutation in enumerate(role.get("may_mutate", [])):
            require(isinstance(mutation, dict) and set(mutation) == {"target", "boundary"}, f"{role_id}.may_mutate[{index}] has invalid shape")
            require(mutation["target"] in entity_ids and isinstance(mutation["boundary"], str), f"{role_id}.may_mutate[{index}] is invalid")
        for index, transition in enumerate(role.get("mediated_transitions", [])):
            require(isinstance(transition, dict) and set(transition) == {"transition", "mediated_by"}, f"{role_id}.mediated_transitions[{index}] has invalid shape")
            require(transition["mediated_by"] in entity_ids | role_ids, f"{role_id}.mediated_transitions[{index}] has unknown mediator")
        _unique_strings(role.get("forbidden_from", []), f"{role_id}.forbidden_from")
    queries = data.get("analysis_queries")
    require(isinstance(queries, dict) and queries, "analysis_queries must be a nonempty mapping")
    require(all(isinstance(value, str) and value.strip() for value in queries.values()), "analysis query descriptions must be nonempty")
    return data


def load_model(invariants: Path, environments: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    catalog = parse_invariants(invariants)
    environment = validate_environment(load_yaml(environments), catalog)
    return catalog, environment


def candidates(catalog: dict[str, Any], environment: dict[str, Any]) -> dict[str, Any]:
    return {
        "status": "candidates_only",
        "semantic_authority": "AI judgment; human approval required for genuine contract changes",
        "invariant_candidates": [
            {"id": key, "owner_text": value["owner"], "condition_text": value["condition"],
             "causal_parent_text": value["causal_parent"]}
            for key, value in sorted(catalog["invariants"].items())
        ],
        "role_relation_candidates": [
            {"role": role_id, "relation": field, "target": target}
            for role_id, role in sorted(environment["roles"].items())
            for field in RELATION_LISTS for target in role.get(field, [])
        ],
        "analysis_intents": environment["analysis_queries"],
    }


def analyze(catalog: dict[str, Any], environment: dict[str, Any]) -> dict[str, Any]:
    owners = {target for role in environment["roles"].values() for target in role.get("owns", [])}
    consumers = {target for role in environment["roles"].values() for target in role.get("consumes", [])}
    emitted = {target for role in environment["roles"].values() for target in role.get("emits", [])}
    invoked = {target for role in environment["roles"].values() for target in role.get("may_invoke", [])}
    capability_ids = set(environment["entities"]["capabilities"])
    findings = []
    for target in sorted(emitted - owners - consumers):
        findings.append({"kind": "emitted_without_declared_owner_or_consumer", "target": target})
    for target in sorted(capability_ids - invoked):
        findings.append({"kind": "capability_not_invoked_by_baseline_role", "target": target})
    return {
        "status": "candidate_findings_only",
        "counts": {"invariants": len(catalog["invariants"]), "mechanisms": len(catalog["mechanisms"]),
                   "roles": len(environment["roles"]), "entities": sum(len(v) for v in environment["entities"].values())},
        "candidate_findings": findings,
        "semantic_queries": environment["analysis_queries"],
    }


def _node_id(value: str) -> str:
    return "N_" + re.sub(r"[^A-Za-z0-9_]", "_", value)


def _label(value: str) -> str:
    return value.replace('"', "'").replace("[", "(").replace("]", ")")


def _markdown_cell(value: str) -> str:
    return " ".join(value.split()).replace("|", r"\|")


def _relation_rows(role: dict[str, Any], entity_labels: dict[str, str]) -> list[tuple[str, str, str]]:
    rows: list[tuple[str, str, str]] = []
    for field, relation in RELATION_LABELS.items():
        for target in role.get(field, []):
            rows.append((relation, target, entity_labels.get(target, target)))
    for mutation in role.get("may_mutate", []):
        detail = f"{entity_labels.get(mutation['target'], mutation['target'])} (boundary: {mutation['boundary']})"
        rows.append(("MAY_MUTATE", mutation["target"], detail))
    for index, transition in enumerate(role.get("mediated_transitions", []), 1):
        detail = f"{transition['transition']} (via {transition['mediated_by']})"
        rows.append(("MEDIATED", f"transition-{index}", detail))
    for index, denial in enumerate(role.get("forbidden_from", []), 1):
        rows.append(("FORBIDDEN_FROM", f"deny-{index}", denial))
    return rows


def _role_table_lines(role: dict[str, Any], entity_labels: dict[str, str]) -> list[str]:
    rows = _relation_rows(role, entity_labels)
    lines = [
        "| Relation | Target | Label |",
        "| --- | --- | --- |",
    ]
    lines.extend(
        f"| `{relation}` | `{_markdown_cell(target)}` | {_markdown_cell(detail)} |"
        for relation, target, detail in rows
    )
    return lines


def _matrix_lines(environment: dict[str, Any], entity_labels: dict[str, str]) -> list[str]:
    role_ids = sorted(environment["roles"])
    targets = sorted({
        target
        for role in environment["roles"].values()
        for field in RELATION_LABELS
        for target in role.get(field, [])
    })
    if not targets:
        return []
    role_labels = [_markdown_cell(environment["roles"][role_id]["label"]) for role_id in role_ids]
    lines = [
        "| Target | " + " | ".join(role_labels) + " |",
        "| --- | " + " | ".join("---" for _ in role_ids) + " |",
    ]
    for target in targets:
        cells = []
        for role_id in role_ids:
            role = environment["roles"][role_id]
            relations = [
                relation
                for field, relation in RELATION_LABELS.items()
                if target in role.get(field, [])
            ]
            cells.append(", ".join(relations))
        lines.append(
            f"| {_markdown_cell(entity_labels.get(target, target))} | "
            + " | ".join(cells) + " |"
        )
    return lines


def render(catalog: dict[str, Any], environment: dict[str, Any], invariant_path: Path, environment_path: Path) -> str:
    inv_digest = hashlib.sha256(invariant_path.read_bytes()).hexdigest()
    env_digest = hashlib.sha256(environment_path.read_bytes()).hexdigest()
    lines = [
        "# Agent Environment Graphs", "",
        "<!-- Generated by skills/agent-environment-graph/scripts/agent_environment_graph.py. -->",
        f"<!-- invariant-sha256: {inv_digest} -->", f"<!-- environment-sha256: {env_digest} -->", "",
        "This is a deterministic view of [`agent-environments.yaml`](agent-environments.yaml).",
        "Invariant and machinery truth remains owned by [`workflow-invariants.md`](workflow-invariants.md);",
        "role projection truth remains owned by the YAML input. Edit those owners and regenerate this view.", "",
        "Analysis produced by the CLI is candidate evidence only. Semantic classification and genuine contract changes",
        "remain AI-judged and human-authorized respectively.", "",
    ]
    entity_labels = {identifier: item["label"] for group in environment["entities"].values() for identifier, item in group.items()}
    for role_id, role in environment["roles"].items():
        lines += [f"## {role['label']}", "", "```mermaid", "flowchart TB", f"  R[\"{_label(role['label'])}\"]"]
        groups = [
            ("Pinned structure", "BOUND_BY", [(item, item) for item in role.get("bound_by", [])]),
            ("Capabilities", "MAY_INVOKE", [(item, entity_labels[item]) for item in role.get("may_invoke", [])]),
            ("Owned state and artifacts", "OWNS", [(item, entity_labels[item]) for item in role.get("owns", [])]),
            ("Observable inputs", "MAY_OBSERVE", [(item, entity_labels[item]) for item in role.get("may_observe", [])]),
            ("Explicit non-authority", "FORBIDDEN_FROM", [(f"deny-{index}", value) for index, value in enumerate(role.get("forbidden_from", []), 1)]),
        ]
        for number, (title, relation, items) in enumerate(groups, 1):
            if not items:
                continue
            lines.append(f"  subgraph G{number}[\"{title}\"]")
            for identifier, label in items:
                node = _node_id(f"{role_id}-{number}-{identifier}")
                lines.append(f"    {node}[\"{_label(label)}\"]")
            lines.append("  end")
            for identifier, _ in items:
                node = _node_id(f"{role_id}-{number}-{identifier}")
                lines.append(f"  R -->|{relation}| {node}")
        for index, transition in enumerate(role.get("mediated_transitions", []), 1):
            node = _node_id(role_id + '-transition-' + str(index))
            lines += [f"  {node}[\"{_label(transition['transition'])}\"]", f"  R -.->|MEDIATED_BY {transition['mediated_by']}| {node}"]
        lines += ["```", "", role["objective"].strip(), ""]
        table = _role_table_lines(role, entity_labels)
        if table:
            lines += ["#### Relation table", ""] + table + [""]
    matrix = _matrix_lines(environment, entity_labels)
    if matrix:
        lines += [
            "## Role × relation matrix", "",
            "Every target this environment references, and which relation each role holds to it.", "",
        ]
        lines += matrix + [""]
    summary = analyze(catalog, environment)
    lines += ["## Deterministic baseline summary", "", f"- Invariants: {summary['counts']['invariants']}",
              f"- Mechanisms: {summary['counts']['mechanisms']}", f"- Roles: {summary['counts']['roles']}",
              f"- States, artifacts, and capabilities: {summary['counts']['entities']}",
              f"- Candidate structural findings: {len(summary['candidate_findings'])}", ""]
    return "\n".join(lines)


def validate_judgments(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(data, dict) and set(data) == {"schema_version", "producer", "judgments"}, "judgment artifact has invalid fields")
    require(data["schema_version"] == 1, "judgment schema_version must be 1")
    require(isinstance(data["producer"], str) and data["producer"].strip(), "judgment producer is required")
    judgments = data["judgments"]
    require(isinstance(judgments, list), "judgments must be a list")
    ids = set()
    required = {"id", "category", "subject", "conclusion", "evidence", "changes_contract", "human_approval"}
    for index, item in enumerate(judgments):
        require(isinstance(item, dict) and set(item) == required, f"judgments[{index}] has invalid fields")
        require(isinstance(item["id"], str) and item["id"] not in ids, f"judgments[{index}].id is missing or duplicate")
        ids.add(item["id"])
        require(item["category"] in SEMANTIC_CATEGORIES, f"judgments[{index}].category is unsupported")
        for field in ("subject", "conclusion"):
            require(isinstance(item[field], str) and item[field].strip(), f"judgments[{index}].{field} is required")
        _unique_strings(item["evidence"], f"judgments[{index}].evidence")
        require(bool(item["evidence"]), f"judgments[{index}].evidence must not be empty")
        require(isinstance(item["changes_contract"], bool), f"judgments[{index}].changes_contract must be boolean")
        if item["changes_contract"]:
            approval = item["human_approval"]
            require(isinstance(approval, dict) and set(approval) == {"approver", "reference"}, f"judgments[{index}] contract change requires human approval")
            require(all(isinstance(value, str) and value.strip() for value in approval.values()), f"judgments[{index}] approval is incomplete")
        else:
            require(item["human_approval"] is None, f"judgments[{index}] non-contract judgment must not claim approval")
    return {"status": "valid", "judgments": len(judgments)}


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    sub = result.add_subparsers(dest="command", required=True)
    for name in ("validate", "candidates", "analyze", "render", "check"):
        command = sub.add_parser(name)
        command.add_argument("--invariants", type=Path, required=True)
        command.add_argument("--environments", type=Path, required=True)
        if name == "render": command.add_argument("--output", type=Path, required=True)
        if name == "check": command.add_argument("--rendered", type=Path, required=True)
    judgments = sub.add_parser("validate-judgments")
    judgments.add_argument("path", type=Path)
    return result


def main() -> int:
    arguments = parser().parse_args()
    try:
        if arguments.command == "validate-judgments":
            result = validate_judgments(arguments.path)
        else:
            catalog, environment = load_model(arguments.invariants, arguments.environments)
            if arguments.command == "validate": result = {"status": "valid", **analyze(catalog, environment)["counts"]}
            elif arguments.command == "candidates": result = candidates(catalog, environment)
            elif arguments.command == "analyze": result = analyze(catalog, environment)
            else:
                output = render(catalog, environment, arguments.invariants, arguments.environments)
                if arguments.command == "render":
                    arguments.output.write_text(output, encoding="utf-8")
                    result = {"status": "rendered", "output": str(arguments.output)}
                else:
                    require(arguments.rendered.read_text(encoding="utf-8") == output, f"rendered view is stale: {arguments.rendered}")
                    result = {"status": "current", "rendered": str(arguments.rendered)}
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0
    except (GraphError, OSError, json.JSONDecodeError) as error:
        print(json.dumps({"status": "invalid", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
