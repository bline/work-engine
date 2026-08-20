#!/usr/bin/env python3
"""Validate and reconcile durable comparative-repository-analysis artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
UNCERTAINTY_STATES = {
    "not_applicable", "not_sought", "not_observed", "provisionally_absent",
    "confirmed_absent", "unknown", "conflicting_evidence",
}
CLAIM_STAGES = {"observation", "interpretation", "comparison_implication"}
CLAIM_STATES = {"supported", "disputed", "unresolved", "superseded"}
RELATIONSHIPS = {
    "depends_on", "enables", "constrains", "conflicts_with",
    "substitutes_for", "shares_state_with",
}
CORRESPONDENCE = (
    "documented", "present", "reachable", "default_enabled",
    "test_demonstrated", "runtime_observed", "production_evidenced",
)
DECISIONS = {"KEEP", "ADOPT", "BORROW", "AVOID", "INVESTIGATE"}
PROFILE_FORBIDDEN = {"decision", "classification", "target_work_engine_boundary"}


class ArtifactError(ValueError):
    """Raised when an artifact would weaken comparison truth or compatibility."""


CONFIG_KEYS = {
    "kind", "configuration_version", "status", "question", "repositories",
    "required_inputs", "selection_bias", "stopping_criteria",
    "ontology_version", "artifact_schema_version", "required_core_dimensions",
    "dimension_definitions", "adaptive_dimensions", "evidence_semantics",
    "minimum_evidence_floor", "work_engine_snapshot_policy",
    "calibration_checks", "migration_policy", "index_policy",
}
INDEX_STATES = {"ready", "coverage_limited", "revision_mismatch", "root_mismatch", "failed", "timed_out", "unsupported_nonterminal_status"}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ArtifactError(message)


def obj(value: Any, path: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    return value


def array(value: Any, path: str) -> list[Any]:
    require(isinstance(value, list), f"{path} must be an array")
    return value


def text(value: Any, path: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{path} must be a nonempty string")
    return value


def exact_keys(value: dict[str, Any], required: set[str], optional: set[str], path: str) -> None:
    missing = required - set(value)
    unknown = set(value) - required - optional
    require(not missing, f"{path} missing fields: {', '.join(sorted(missing))}")
    require(not unknown, f"{path} unknown fields: {', '.join(sorted(unknown))}")


def unique_ids(items: list[Any], path: str) -> set[str]:
    ids: set[str] = set()
    for index, item in enumerate(items):
        identifier = text(obj(item, f"{path}[{index}]").get("id"), f"{path}[{index}].id")
        require(identifier not in ids, f"duplicate {path} id '{identifier}'")
        ids.add(identifier)
    return ids


def compatibility(value: Any, path: str = "compatibility") -> dict[str, Any]:
    result = obj(value, path)
    exact_keys(result, {"run_id", "ontology_version", "artifact_schema_version"}, set(), path)
    text(result["run_id"], f"{path}.run_id")
    text(result["ontology_version"], f"{path}.ontology_version")
    require(result["artifact_schema_version"] == SCHEMA_VERSION,
            f"{path}.artifact_schema_version must be {SCHEMA_VERSION}")
    return result


def validate_contract(contract: Any) -> dict[str, Any]:
    contract = obj(contract, "contract")
    exact_keys(contract, {
        "artifact_type", "compatibility", "question", "repository_corpus",
        "selection_bias", "required_core_dimensions", "dimension_definitions",
        "evidence_semantics", "minimum_evidence_floor", "work_engine_snapshot",
        "calibration_checks", "migration_policy", "index_policy",
    }, {"adaptive_dimensions", "stopping_criteria"}, "contract")
    require(contract["artifact_type"] == "comparison_contract_v1", "contract.artifact_type is incompatible")
    compatibility(contract["compatibility"], "contract.compatibility")
    text(contract["question"], "contract.question")
    corpus = array(contract["repository_corpus"], "contract.repository_corpus")
    require(bool(corpus), "contract.repository_corpus must not be empty")
    for index, repository in enumerate(corpus):
        repository = obj(repository, f"contract.repository_corpus[{index}]")
        exact_keys(repository, {"id", "repository", "inclusion_reason", "commit", "checkout_path",
                                "identity_source", "git_remote", "branch", "detached"}, {"index_project"},
                   f"contract.repository_corpus[{index}]")
        for key in ("id", "repository", "inclusion_reason", "commit", "checkout_path", "identity_source"):
            text(repository[key], f"contract.repository_corpus[{index}].{key}")
        require(repository["identity_source"] in {"origin_remote", "local_path"}, "repository identity_source is invalid")
        require(repository["git_remote"] is None or isinstance(repository["git_remote"], str), "repository git_remote must be string or null")
        require(repository["branch"] is None or isinstance(repository["branch"], str), "repository branch must be string or null")
        require(isinstance(repository["detached"], bool), "repository detached must be boolean")
        if "index_project" in repository:
            text(repository["index_project"], f"contract.repository_corpus[{index}].index_project")
    unique_ids(corpus, "contract.repository_corpus")
    dimensions = array(contract["required_core_dimensions"], "contract.required_core_dimensions")
    require(bool(dimensions) and all(isinstance(item, str) and item for item in dimensions),
            "contract.required_core_dimensions must contain nonempty strings")
    require(len(set(dimensions)) == len(dimensions), "contract.required_core_dimensions contains duplicates")
    adaptive = array(contract.get("adaptive_dimensions", []), "contract.adaptive_dimensions")
    require(all(isinstance(item, str) and item for item in adaptive),
            "contract.adaptive_dimensions must contain nonempty strings")
    require(len(set(adaptive)) == len(adaptive), "contract.adaptive_dimensions contains duplicates")
    require(not set(adaptive) & set(dimensions),
            "contract.adaptive_dimensions must not duplicate required core dimensions")
    definitions = obj(contract["dimension_definitions"], "contract.dimension_definitions")
    require(set(definitions) == set(dimensions), "dimension_definitions must exactly cover required_core_dimensions")
    for name, definition in definitions.items():
        text(definition, f"contract.dimension_definitions.{name}")
    semantics = obj(contract["evidence_semantics"], "contract.evidence_semantics")
    exact_keys(semantics, {"uncertainty_states", "correspondence_states", "confidence_scale"}, set(), "contract.evidence_semantics")
    require(set(array(semantics["uncertainty_states"], "uncertainty_states")) == UNCERTAINTY_STATES,
            "contract uncertainty semantics must use the complete closed vocabulary")
    require(tuple(array(semantics["correspondence_states"], "correspondence_states")) == CORRESPONDENCE,
            "contract correspondence semantics must preserve the defined ordering")
    confidence = array(semantics["confidence_scale"], "confidence_scale")
    require(confidence == ["low", "medium", "high"], "confidence_scale must be low, medium, high")
    for key in ("selection_bias", "minimum_evidence_floor", "work_engine_snapshot", "migration_policy"):
        require(contract[key] not in (None, "", [], {}), f"contract.{key} must record an explicit value")
    array(contract["calibration_checks"], "contract.calibration_checks")
    validate_index_policy(contract["index_policy"])
    return contract


def validate_index_policy(value: Any) -> dict[str, Any]:
    policy = obj(value, "index_policy")
    exact_keys(policy, {"provider", "mode", "persistence", "existing_index", "preparation_context",
                        "agent_deadline_seconds", "status_rechecks", "initial_coverage_scopes",
                        "allowed_fallbacks", "fallback_owner"}, set(), "index_policy")
    require(policy["provider"] == "codebase-memory", "index_policy.provider must be codebase-memory")
    require(policy["mode"] in {"full", "moderate", "fast"}, "index_policy.mode is invalid")
    require(isinstance(policy["persistence"], bool), "index_policy.persistence must be boolean")
    require(policy["existing_index"] in {"reuse_if_exact", "force_reindex"}, "index_policy.existing_index is invalid")
    require(policy["preparation_context"] == "disposable", "index preparation must use a disposable context")
    require(isinstance(policy["agent_deadline_seconds"], int) and not isinstance(policy["agent_deadline_seconds"], bool)
            and policy["agent_deadline_seconds"] > 0, "index_policy.agent_deadline_seconds must be positive")
    require(policy["status_rechecks"] == 0, "status_rechecks must be 0 because Codebase Memory exposes no wait primitive")
    scopes = array(policy["initial_coverage_scopes"], "index_policy.initial_coverage_scopes")
    require(bool(scopes) and all(isinstance(item, str) and item for item in scopes), "initial_coverage_scopes must contain paths")
    fallbacks = array(policy["allowed_fallbacks"], "index_policy.allowed_fallbacks")
    require(set(fallbacks) <= {"targeted_direct_source"}, "index_policy.allowed_fallbacks contains an unsupported fallback")
    require(policy["fallback_owner"] == "comparison_model", "fallback_owner must be comparison_model")
    return policy


def load_yaml_config(path: Path) -> dict[str, Any]:
    try:
        import yaml
    except ImportError as error:
        raise ArtifactError(
            "PyYAML is required for human-authored configuration; install PyYAML and retry"
        ) from error
    source = path.read_text(encoding="utf-8")
    try:
        for token in yaml.scan(source):
            if isinstance(token, (yaml.tokens.AnchorToken, yaml.tokens.AliasToken)):
                raise ArtifactError("YAML anchors and aliases are not allowed in comparison configuration")
        loaded = yaml.safe_load(source)
    except yaml.YAMLError as error:
        raise ArtifactError(f"invalid YAML configuration: {error}") from error
    return obj(loaded, "configuration")


def run_git(path: Path, *arguments: str) -> tuple[int, str]:
    try:
        result = subprocess.run(["git", "-C", str(path), *arguments], capture_output=True, text=True,
                                timeout=10, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        return 1, str(error)[-500:]
    output = (result.stdout.strip() or result.stderr.strip())[-500:]
    return result.returncode, output


def provisional_id(value: str) -> str:
    basename = value.rstrip("/").rsplit("/", 1)[-1]
    if basename.endswith(".git"):
        basename = basename[:-4]
    identifier = re.sub(r"[^a-z0-9]+", "-", basename.lower()).strip("-")
    return identifier or "repository"


def derive_repository(value: Any, index: int) -> tuple[dict[str, Any] | None, list[str]]:
    path_value: Any
    inclusion_reason: Any = None
    explicit_id: Any = None
    pinned_project: Any = None
    expected_revision: Any = None
    if isinstance(value, str):
        path_value = value
    else:
        entry = obj(value, f"configuration.repositories[{index}]")
        exact_keys(entry, set(), {"path", "checkout_path", "inclusion_reason", "id", "index_project", "revision"},
                   f"configuration.repositories[{index}]")
        path_value = entry.get("path", entry.get("checkout_path"))
        inclusion_reason, explicit_id = entry.get("inclusion_reason"), entry.get("id")
        pinned_project, expected_revision = entry.get("index_project"), entry.get("revision")
    if not isinstance(path_value, str) or not path_value.strip():
        return None, [f"repositories[{index}] requires a checkout path"]
    requested = Path(path_value).expanduser()
    if not requested.exists():
        return None, [f"repositories[{index}] path does not exist: {requested}"]
    code, root_output = run_git(requested, "rev-parse", "--show-toplevel")
    if code:
        return None, [f"repositories[{index}] is not a readable Git worktree: {root_output}"]
    root = Path(root_output).resolve()
    code, head = run_git(root, "rev-parse", "HEAD")
    if code:
        return None, [f"repositories[{index}] cannot resolve immutable HEAD: {head}"]
    code, dirty = run_git(root, "status", "--porcelain=v1")
    if code:
        return None, [f"repositories[{index}] cannot inspect working tree: {dirty}"]
    problems: list[str] = []
    if dirty:
        problems.append(f"repositories[{index}] working tree is dirty and does not correspond to HEAD")
    if expected_revision and expected_revision != head:
        problems.append(f"repositories[{index}] configured revision does not match HEAD")
    remote_code, remote = run_git(root, "config", "--get", "remote.origin.url")
    git_remote = None if remote_code or not remote else remote
    identity_source = "origin_remote" if git_remote else "local_path"
    repository = git_remote or f"local:{root}"
    branch_code, branch = run_git(root, "symbolic-ref", "--quiet", "--short", "HEAD")
    detached = branch_code != 0
    identifier = explicit_id if explicit_id else provisional_id(git_remote or str(root))
    if not isinstance(identifier, str) or not identifier.strip():
        problems.append(f"repositories[{index}] id must be a nonempty string")
        identifier = provisional_id(str(root))
    if not isinstance(inclusion_reason, str) or not inclusion_reason.strip():
        problems.append(f"repositories[{index}] requires semantic inclusion_reason")
        inclusion_reason = "REQUIRED_INCLUSION_REASON"
    result = {"id": identifier, "repository": repository, "commit": head, "checkout_path": str(root),
              "inclusion_reason": inclusion_reason, "identity_source": identity_source,
              "git_remote": git_remote, "branch": None if detached else branch, "detached": detached}
    if pinned_project is not None:
        result["index_project"] = text(pinned_project, f"configuration.repositories[{index}].index_project")
    return result, problems


def normalize_yaml_config(config: Any) -> tuple[dict[str, Any] | None, list[str], list[dict[str, Any]]]:
    config = obj(config, "configuration")
    exact_keys(config, CONFIG_KEYS, set(), "configuration")
    require(config["kind"] == "comparative_repository_analysis",
            "configuration.kind must be comparative_repository_analysis")
    require(config["configuration_version"] == 1, "configuration.configuration_version must be 1")
    require(config["status"] in {"draft", "ready"}, "configuration.status must be draft or ready")
    text(config["question"], "configuration.question")
    text(config["ontology_version"], "configuration.ontology_version")
    require(config["artifact_schema_version"] == SCHEMA_VERSION,
            f"configuration.artifact_schema_version must be {SCHEMA_VERSION}")
    repositories = array(config["repositories"], "configuration.repositories")
    array(config["required_inputs"], "configuration.required_inputs")
    missing: list[str] = []
    if config["status"] != "ready":
        missing.append("configuration.status must be ready")
    if not repositories:
        missing.append("at least one repository with an immutable commit or tag")
    normalized_repositories: list[dict[str, Any]] = []
    for index, repository in enumerate(repositories):
        normalized, problems = derive_repository(repository, index)
        missing.extend(problems)
        if normalized:
            normalized_repositories.append(normalized)
    roots = [item["checkout_path"] for item in normalized_repositories]
    ids = [item["id"] for item in normalized_repositories]
    if len(set(roots)) != len(roots):
        missing.append("repository checkout paths must be unique")
    if len(set(ids)) != len(ids):
        missing.append("derived repository ids collide; provide explicit unique ids")
    snapshot = obj(config["work_engine_snapshot_policy"], "configuration.work_engine_snapshot_policy")
    exact_keys(snapshot, {"mode", "revision", "description"}, set(), "configuration.work_engine_snapshot_policy")
    require(snapshot["mode"] == "fixed", "work_engine_snapshot_policy.mode must be fixed")
    text(snapshot["description"], "work_engine_snapshot_policy.description")
    revision = snapshot["revision"]
    if not isinstance(revision, str) or not revision.strip() or revision.startswith("REQUIRED_"):
        missing.append("work_engine_snapshot_policy.revision must be an immutable commit or tag")
    if missing:
        return None, missing, normalized_repositories
    contract_body = {
        "artifact_type": "comparison_contract_v1",
        "question": config["question"],
        "repository_corpus": normalized_repositories,
        "selection_bias": config["selection_bias"],
        "required_core_dimensions": config["required_core_dimensions"],
        "dimension_definitions": config["dimension_definitions"],
        "adaptive_dimensions": config["adaptive_dimensions"],
        "stopping_criteria": config["stopping_criteria"],
        "evidence_semantics": config["evidence_semantics"],
        "minimum_evidence_floor": config["minimum_evidence_floor"],
        "work_engine_snapshot": {
            "mode": snapshot["mode"], "commit": revision, "description": snapshot["description"]
        },
        "calibration_checks": config["calibration_checks"],
        "migration_policy": config["migration_policy"],
        "index_policy": validate_index_policy(config["index_policy"]),
    }
    digest = hashlib.sha256(
        json.dumps(contract_body, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    ).hexdigest()[:20]
    contract = {
        **contract_body,
        "compatibility": {
            "run_id": "comparison-" + digest,
            "ontology_version": config["ontology_version"],
            "artifact_schema_version": config["artifact_schema_version"],
        },
    }
    return validate_contract(contract), [], normalized_repositories


def assert_compatible(expected: dict[str, Any], actual: dict[str, Any], path: str) -> None:
    for key in ("run_id", "ontology_version", "artifact_schema_version"):
        require(actual.get(key) == expected.get(key),
                f"{path}.{key} is incompatible: expected {expected.get(key)!r}, got {actual.get(key)!r}")


def validate_evidence(item: Any, path: str) -> dict[str, Any]:
    item = obj(item, path)
    exact_keys(item, {"id", "mode", "locator", "supports", "limitations"},
               {"symbol_or_section", "range", "commit", "configuration", "coverage"}, path)
    text(item["id"], f"{path}.id")
    require(item["mode"] in {"source", "indexed_structure", "documentation", "test", "runtime", "external"},
            f"{path}.mode is invalid")
    text(item["locator"], f"{path}.locator")
    supports = array(item["supports"], f"{path}.supports")
    require(bool(supports) and all(isinstance(value, str) and value for value in supports),
            f"{path}.supports must contain claim ids")
    require(isinstance(item["limitations"], list), f"{path}.limitations must be an array")
    return item


def validate_claim(item: Any, path: str) -> dict[str, Any]:
    item = obj(item, path)
    exact_keys(item, {"id", "subject_id", "stage", "statement", "state", "confidence", "evidence_ids",
                      "counterevidence_ids", "parent_claim_ids"}, {"alternatives", "uncertainty"}, path)
    for key in ("id", "subject_id", "statement"):
        text(item[key], f"{path}.{key}")
    require(item["stage"] in CLAIM_STAGES, f"{path}.stage is invalid")
    require(item["state"] in CLAIM_STATES, f"{path}.state is invalid")
    require(item["confidence"] in {"low", "medium", "high"}, f"{path}.confidence is invalid")
    for key in ("evidence_ids", "counterevidence_ids", "parent_claim_ids"):
        require(all(isinstance(value, str) and value for value in array(item[key], f"{path}.{key}")),
                f"{path}.{key} contains an invalid id")
    if "uncertainty" in item:
        require(item["uncertainty"] in UNCERTAINTY_STATES, f"{path}.uncertainty is invalid")
    return item


def validate_capability(item: Any, path: str) -> dict[str, Any]:
    item = obj(item, path)
    exact_keys(item, {"id", "name", "mechanism", "enforcement", "correspondence", "applicability",
                      "confidence", "evidence_ids", "counterevidence_ids", "relationships"}, set(), path)
    for key in ("id", "name", "mechanism"):
        text(item[key], f"{path}.{key}")
    require(item["enforcement"] in {"code", "prompt", "config", "mixed"}, f"{path}.enforcement is invalid")
    correspondence = obj(item["correspondence"], f"{path}.correspondence")
    require(set(correspondence) == set(CORRESPONDENCE), f"{path}.correspondence must contain every state")
    for key, value in correspondence.items():
        require(value in {True, False, "unknown"}, f"{path}.correspondence.{key} must be boolean or unknown")
    require(item["applicability"] in UNCERTAINTY_STATES | {"applicable"}, f"{path}.applicability is invalid")
    if item["applicability"] == "confirmed_absent":
        raise ArtifactError(f"{path}.applicability cannot be confirmed_absent for a present capability")
    require(item["confidence"] in {"low", "medium", "high"}, f"{path}.confidence is invalid")
    relationships = obj(item["relationships"], f"{path}.relationships")
    require(set(relationships) == RELATIONSHIPS, f"{path}.relationships must use the complete typed edge vocabulary")
    for key in RELATIONSHIPS:
        require(all(isinstance(value, str) and value for value in array(relationships[key], f"{path}.relationships.{key}")),
                f"{path}.relationships.{key} contains an invalid id")
    return item


def validate_index_receipt(value: Any, contract: dict[str, Any], repository: dict[str, Any]) -> dict[str, Any]:
    receipt = obj(value, "index_receipt")
    exact_keys(receipt, {"artifact_type", "compatibility", "receipt_id", "repository_id", "state", "action",
                         "requested", "observed", "coverage", "provenance", "limitations"}, {"failure"}, "index_receipt")
    require(receipt["artifact_type"] == "index_preparation_receipt_v1", "index_receipt.artifact_type is incompatible")
    assert_compatible(contract["compatibility"], compatibility(receipt["compatibility"], "index_receipt.compatibility"), "index_receipt.compatibility")
    text(receipt["receipt_id"], "index_receipt.receipt_id")
    require(receipt["repository_id"] == repository["id"], "index receipt repository is incompatible")
    require(receipt["state"] in INDEX_STATES, "index_receipt.state is invalid")
    require(receipt["action"] in {"reused", "indexed", "none"}, "index_receipt.action is invalid")
    if receipt["state"] in {"ready", "coverage_limited"}:
        require(receipt["action"] in {"reused", "indexed"}, "ready index receipt requires observed index action")
    requested = obj(receipt["requested"], "index_receipt.requested")
    exact_keys(requested, {"root", "revision", "mode", "persistence", "scopes"}, set(), "index_receipt.requested")
    require(requested["root"] == repository["checkout_path"], "index receipt requested root is incompatible")
    require(requested["revision"] == repository["commit"], "index receipt requested revision is incompatible")
    policy = contract["index_policy"]
    require(requested["mode"] == policy["mode"] and requested["persistence"] == policy["persistence"],
            "index receipt requested policy is incompatible")
    require(requested["scopes"] == policy["initial_coverage_scopes"], "index receipt scopes are incompatible")
    observed = obj(receipt["observed"], "index_receipt.observed")
    observed_fields = {"project", "root", "revision", "generation", "status", "nodes", "edges"}
    if receipt["state"] in {"ready", "coverage_limited"}:
        exact_keys(observed, observed_fields, set(), "index_receipt.observed")
        for key in ("project", "root", "revision", "generation", "status"):
            text(observed[key], f"index_receipt.observed.{key}")
        require(observed["root"] == repository["checkout_path"], "observed index root is incompatible")
        require(observed["revision"] == repository["commit"], "observed index revision is incompatible")
        if repository.get("index_project"):
            require(observed["project"] == repository["index_project"], "observed index project is incompatible")
        require(observed["status"] == "ready", "ready index receipt requires ready observed status")
        require(all(isinstance(observed[key], int) and not isinstance(observed[key], bool) and observed[key] >= 0 for key in ("nodes", "edges")),
                "index receipt graph counts must be nonnegative integers")
    else:
        exact_keys(observed, set(), observed_fields, "index_receipt.observed")
    coverage = obj(receipt["coverage"], "index_receipt.coverage")
    exact_keys(coverage, {"signal", "parse_partial", "skipped", "not_indexed", "limitations"}, set(), "index_receipt.coverage")
    require(coverage["signal"] in {"best_effort", "limited"}, "index receipt coverage signal is invalid")
    for key in ("parse_partial", "skipped", "not_indexed", "limitations"):
        array(coverage[key], f"index_receipt.coverage.{key}")
    has_coverage_gaps = any(coverage[key] for key in ("parse_partial", "skipped", "not_indexed"))
    require((receipt["state"] == "coverage_limited") == has_coverage_gaps,
            "coverage entries and coverage_limited state must correspond")
    require(coverage["signal"] == ("limited" if has_coverage_gaps else "best_effort"),
            "coverage signal does not correspond to recorded coverage state")
    provenance = obj(receipt["provenance"], "index_receipt.provenance")
    exact_keys(provenance, {"provider", "calls"}, set(), "index_receipt.provenance")
    require(provenance["provider"] == "codebase-memory", "index receipt provenance provider is invalid")
    calls = array(provenance["calls"], "index_receipt.provenance.calls")
    if receipt["state"] in {"ready", "coverage_limited"}:
        require(set(calls) >= {"list_projects", "index_status", "check_index_coverage"},
                "ready receipt requires project discovery, status, and coverage call evidence")
        if receipt["action"] == "indexed":
            require("index_repository" in calls, "indexed receipt requires index_repository call evidence")
    array(receipt["limitations"], "index_receipt.limitations")
    if receipt["state"] not in {"ready", "coverage_limited"}:
        failure = obj(receipt.get("failure"), "index_receipt.failure")
        exact_keys(failure, {"kind", "detail"}, set(), "index_receipt.failure")
        text(failure["kind"], "index_receipt.failure.kind")
        text(failure["detail"], "index_receipt.failure.detail")
    require(len(json.dumps(receipt, sort_keys=True, separators=(",", ":"))) <= 12000,
            "index receipt exceeds compact 12000-character boundary")
    validate_compact_content(receipt)
    return receipt


def validate_compact_content(value: Any, path: str = "index_receipt") -> None:
    if isinstance(value, str):
        require(len(value) <= 1000, f"{path} string exceeds compact limit")
    elif isinstance(value, list):
        require(len(value) <= 100, f"{path} array exceeds compact limit")
        for index, child in enumerate(value):
            validate_compact_content(child, f"{path}[{index}]")
    elif isinstance(value, dict):
        for key, child in value.items():
            validate_compact_content(child, f"{path}.{key}")


def receipt_digest(receipt: dict[str, Any]) -> str:
    return hashlib.sha256(json.dumps(receipt, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()


def artifact_digest(value: dict[str, Any]) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()


def validate_ontology_gap(value: Any, contract: dict[str, Any], source_pass: dict[str, Any]) -> dict[str, Any]:
    proposal = obj(value, "ontology_gap_proposal")
    exact_keys(proposal, {"artifact_type", "compatibility", "gap_id", "status", "outside_active_ontology",
                          "repository_id", "source_dimension", "source_pass_sha256", "claim_ids", "evidence_ids",
                          "proposed_dimension", "proposed_definition", "why_ontology_is_insufficient",
                          "decision_relevance", "confidence", "alternatives", "limitations"}, set(), "ontology_gap_proposal")
    require(proposal["artifact_type"] == "ontology_gap_proposal_v1", "ontology gap artifact type is incompatible")
    assert_compatible(contract["compatibility"], compatibility(proposal["compatibility"], "ontology_gap_proposal.compatibility"),
                      "ontology_gap_proposal.compatibility")
    checked_pass = validate_dimension_pass(source_pass, contract)
    require(proposal["repository_id"] == checked_pass["repository_id"] and
            proposal["source_dimension"] == checked_pass["dimension"], "ontology gap source pass identity is incompatible")
    require(proposal["source_pass_sha256"] == artifact_digest(checked_pass), "ontology gap source pass digest is incompatible")
    require(proposal["status"] == "proposed" and proposal["outside_active_ontology"] is True,
            "ontology gap must remain a proposal outside the active ontology")
    for key in ("gap_id", "proposed_dimension", "proposed_definition", "why_ontology_is_insufficient", "decision_relevance"):
        text(proposal[key], f"ontology_gap_proposal.{key}")
    active = set(contract["required_core_dimensions"]) | set(contract.get("adaptive_dimensions", []))
    require(proposal["proposed_dimension"] not in active, "ontology gap proposal already exists in the frozen ontology")
    claim_ids = set(array(proposal["claim_ids"], "ontology_gap_proposal.claim_ids"))
    evidence_ids = set(array(proposal["evidence_ids"], "ontology_gap_proposal.evidence_ids"))
    require(bool(claim_ids) and claim_ids <= {item["id"] for item in checked_pass["claims"]},
            "ontology gap must reference source-pass claims")
    require(bool(evidence_ids) and evidence_ids <= {item["id"] for item in checked_pass["evidence"]},
            "ontology gap must reference source-pass evidence")
    require(proposal["confidence"] in {"low", "medium", "high"}, "ontology gap confidence is invalid")
    array(proposal["alternatives"], "ontology_gap_proposal.alternatives")
    array(proposal["limitations"], "ontology_gap_proposal.limitations")
    return proposal


def validate_index_fallback(value: Any, contract: dict[str, Any], receipt: dict[str, Any]) -> dict[str, Any]:
    decision = obj(value, "index_fallback_decision")
    exact_keys(decision, {"artifact_type", "compatibility", "repository_id", "receipt_id", "receipt_sha256", "decision", "decided_by", "reason"}, set(), "index_fallback_decision")
    require(decision["artifact_type"] == "index_fallback_decision_v1", "index fallback artifact type is incompatible")
    assert_compatible(contract["compatibility"], compatibility(decision["compatibility"], "index_fallback_decision.compatibility"), "index_fallback_decision.compatibility")
    require(decision["repository_id"] == receipt["repository_id"] and decision["receipt_id"] == receipt["receipt_id"],
            "index fallback decision is not tied to this receipt")
    require(decision["receipt_sha256"] == receipt_digest(receipt), "index fallback decision receipt digest is incompatible")
    require(decision["decision"] in contract["index_policy"]["allowed_fallbacks"], "index fallback is not permitted by the contract")
    require(decision["decided_by"] == "comparison_model", "index fallback must be model-owned")
    text(decision["reason"], "index_fallback_decision.reason")
    return decision


def validate_dimension_pass(pass_artifact: Any, contract: dict[str, Any]) -> dict[str, Any]:
    pass_artifact = obj(pass_artifact, "pass")
    exact_keys(pass_artifact, {"artifact_type", "compatibility", "repository_id", "dimension", "dimension_state",
                               "system_boundary", "capabilities", "claims", "evidence", "unknowns", "provenance", "index_receipt"},
               {"negative_search", "index_fallback_decision"}, "pass")
    require(pass_artifact["artifact_type"] == "dimension_pass_v1", "pass.artifact_type is incompatible")
    pass_compat = compatibility(pass_artifact["compatibility"], "pass.compatibility")
    assert_compatible(contract["compatibility"], pass_compat, "pass.compatibility")
    repository_id = text(pass_artifact["repository_id"], "pass.repository_id")
    corpus_ids = {item["id"] for item in contract["repository_corpus"]}
    require(repository_id in corpus_ids, f"pass.repository_id '{repository_id}' is outside the frozen corpus")
    repository = next(item for item in contract["repository_corpus"] if item["id"] == repository_id)
    receipt = validate_index_receipt(pass_artifact["index_receipt"], contract, repository)
    if receipt["state"] == "coverage_limited":
        require(bool(contract["index_policy"]["allowed_fallbacks"]),
                "coverage is limited and the frozen policy permits no fallback")
        require("index_fallback_decision" in pass_artifact, "coverage-limited index requires a separate fallback decision")
        validate_index_fallback(pass_artifact["index_fallback_decision"], contract, receipt)
    else:
        require(receipt["state"] == "ready", f"semantic pass blocked by index state '{receipt['state']}'")
        require("index_fallback_decision" not in pass_artifact, "ready index must not carry a fallback decision")
    dimension = text(pass_artifact["dimension"], "pass.dimension")
    allowed = set(contract["required_core_dimensions"]) | set(contract.get("adaptive_dimensions", []))
    require(dimension in allowed, f"pass.dimension '{dimension}' is outside the frozen ontology")
    require(pass_artifact["dimension_state"] in UNCERTAINTY_STATES | {"analyzed"}, "pass.dimension_state is invalid")
    evidence = [validate_evidence(value, f"pass.evidence[{index}]") for index, value in enumerate(array(pass_artifact["evidence"], "pass.evidence"))]
    claims = [validate_claim(value, f"pass.claims[{index}]") for index, value in enumerate(array(pass_artifact["claims"], "pass.claims"))]
    capabilities = [validate_capability(value, f"pass.capabilities[{index}]") for index, value in enumerate(array(pass_artifact["capabilities"], "pass.capabilities"))]
    obj(pass_artifact["system_boundary"], "pass.system_boundary")
    array(pass_artifact["unknowns"], "pass.unknowns")
    obj(pass_artifact["provenance"], "pass.provenance")
    evidence_ids, claim_ids, capability_ids = unique_ids(evidence, "pass.evidence"), unique_ids(claims, "pass.claims"), unique_ids(capabilities, "pass.capabilities")
    all_subjects = claim_ids | capability_ids
    for evidence_item in evidence:
        require(set(evidence_item["supports"]) <= claim_ids, f"evidence '{evidence_item['id']}' supports an unknown claim")
    stages = {claim["id"]: claim["stage"] for claim in claims}
    for claim in claims:
        require(set(claim["evidence_ids"]) | set(claim["counterevidence_ids"]) <= evidence_ids,
                f"claim '{claim['id']}' references unknown evidence")
        require(set(claim["parent_claim_ids"]) <= claim_ids, f"claim '{claim['id']}' references an unknown parent")
        if claim["stage"] == "observation":
            require(not claim["parent_claim_ids"], f"observation '{claim['id']}' must not depend on an interpretation")
            require(bool(claim["evidence_ids"]), f"observation '{claim['id']}' requires evidence")
        elif claim["stage"] == "interpretation":
            require(any(stages[parent] == "observation" for parent in claim["parent_claim_ids"]),
                    f"interpretation '{claim['id']}' must reference an observation")
        else:
            require(any(stages[parent] == "interpretation" for parent in claim["parent_claim_ids"]),
                    f"comparison implication '{claim['id']}' must reference an interpretation")
        require(claim["subject_id"] in all_subjects or claim["subject_id"] == repository_id,
                f"claim '{claim['id']}' has unknown subject '{claim['subject_id']}'")
    for capability in capabilities:
        referenced = set(capability["evidence_ids"]) | set(capability["counterevidence_ids"])
        require(referenced <= evidence_ids, f"capability '{capability['id']}' references unknown evidence")
        for relation, targets in capability["relationships"].items():
            require(set(targets) <= capability_ids, f"capability '{capability['id']}' {relation} references unknown capability")
    if pass_artifact["dimension_state"] == "confirmed_absent":
        search = obj(pass_artifact.get("negative_search"), "pass.negative_search")
        exact_keys(search, {"scope", "exhaustive", "coverage_limitations"}, set(), "pass.negative_search")
        require(search["exhaustive"] is True, "confirmed_absent requires an exhaustive bounded search")
        text(search["scope"], "pass.negative_search.scope")
        require(isinstance(search["coverage_limitations"], list), "coverage_limitations must be an array")
    return pass_artifact


def find_forbidden(value: Any, path: str = "profile") -> list[str]:
    found: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            if key in PROFILE_FORBIDDEN:
                found.append(f"{path}.{key}")
            found.extend(find_forbidden(child, f"{path}.{key}"))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            found.extend(find_forbidden(child, f"{path}[{index}]"))
    return found


def reconcile(contract: dict[str, Any], passes: list[dict[str, Any]], repository_id: str) -> dict[str, Any]:
    validate_contract(contract)
    require(bool(passes), "reconciliation requires at least one dimension pass")
    checked = [validate_dimension_pass(value, contract) for value in passes]
    require(all(value["repository_id"] == repository_id for value in checked),
            "all passes must describe the selected repository")
    receipt_digests = {receipt_digest(value["index_receipt"]) for value in checked}
    require(len(receipt_digests) == 1, "all reconciled passes must use the same complete index receipt")
    index_receipt = checked[0]["index_receipt"]
    dimensions: dict[str, str] = {}
    dimension_values: dict[str, set[str]] = {}
    capabilities: dict[str, dict[str, Any]] = {}
    claims: dict[str, dict[str, Any]] = {}
    evidence: dict[str, dict[str, Any]] = {}
    conflicts: list[dict[str, Any]] = []
    unknowns: list[Any] = []
    provenance: list[Any] = []
    boundary: list[Any] = []
    for artifact in checked:
        dimension = artifact["dimension"]
        values = dimension_values.setdefault(dimension, set())
        values.add(artifact["dimension_state"])
        if len(values) > 1:
            dimensions[dimension] = "conflicting_evidence"
            conflicts.append({"kind": "dimension_state", "subject_id": dimension,
                              "values": sorted(values)})
        else:
            dimensions[dimension] = artifact["dimension_state"]
        boundary.append(artifact["system_boundary"])
        unknowns.extend(artifact["unknowns"])
        provenance.append(artifact["provenance"])
        for item in artifact["evidence"]:
            existing = evidence.get(item["id"])
            require(existing is None or existing == item, f"evidence id '{item['id']}' has conflicting definitions")
            evidence[item["id"]] = item
        for item in artifact["claims"]:
            existing = claims.get(item["id"])
            require(existing is None or existing == item, f"claim id '{item['id']}' has conflicting definitions")
            claims[item["id"]] = item
        for item in artifact["capabilities"]:
            existing = capabilities.get(item["id"])
            if existing is None:
                capabilities[item["id"]] = json.loads(json.dumps(item))
                continue
            immutable = ("name", "mechanism", "enforcement", "applicability")
            if not all(existing[key] == item[key] for key in immutable):
                conflicts.append({
                    "kind": "capability_identity", "subject_id": item["id"],
                    "definitions": [
                        {key: existing[key] for key in immutable},
                        {key: item[key] for key in immutable},
                    ],
                })
            for key in RELATIONSHIPS:
                existing["relationships"][key] = sorted(set(existing["relationships"][key]) | set(item["relationships"][key]))
            for key in ("evidence_ids", "counterevidence_ids"):
                existing[key] = sorted(set(existing[key]) | set(item[key]))
            for key in CORRESPONDENCE:
                if existing["correspondence"][key] != item["correspondence"][key]:
                    existing["correspondence"][key] = "unknown"
                    conflicts.append({"kind": "correspondence", "subject_id": item["id"], "field": key})
    for required_dimension in contract["required_core_dimensions"]:
        dimensions.setdefault(required_dimension, "not_sought")
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for claim in claims.values():
        grouped.setdefault((claim["subject_id"], claim["stage"]), []).append(claim)
    for (subject_id, stage), values in grouped.items():
        statements = {value["statement"] for value in values if value["state"] != "superseded"}
        if len(statements) > 1:
            conflicts.append({"kind": "claim_disagreement", "subject_id": subject_id,
                              "stage": stage, "claim_ids": sorted(value["id"] for value in values)})
    profile_provenance = {"dimension_passes": provenance, "index_receipt": index_receipt,
                          "index_receipt_sha256": receipt_digest(index_receipt)}
    if index_receipt["state"] == "coverage_limited":
        decisions = [value["index_fallback_decision"] for value in checked]
        require(all(value == decisions[0] for value in decisions),
                "all coverage-limited passes must use the same fallback decision")
        profile_provenance["index_fallback_decision"] = decisions[0]
    profile = {
        "artifact_type": "repository_profile_v1",
        "compatibility": contract["compatibility"],
        "repository_id": repository_id,
        "dimension_states": dimensions,
        "system_boundary_observations": boundary,
        "capabilities": sorted(capabilities.values(), key=lambda value: value["id"]),
        "claims": sorted(claims.values(), key=lambda value: value["id"]),
        "evidence": sorted(evidence.values(), key=lambda value: value["id"]),
        "reconciliation": {"conflicts": conflicts, "conflict_count": len(conflicts)},
        "unknowns": unknowns,
        "provenance": profile_provenance,
    }
    require(not find_forbidden(profile), "repository profile contains decision vocabulary")
    return profile


def validate_profile(profile: Any, contract: dict[str, Any]) -> dict[str, Any]:
    profile = obj(profile, "profile")
    forbidden = find_forbidden(profile)
    require(not forbidden, f"descriptive profile contains decision fields at {', '.join(forbidden)}")
    exact_keys(profile, {
        "artifact_type", "compatibility", "repository_id", "dimension_states",
        "system_boundary_observations", "capabilities", "claims", "evidence",
        "reconciliation", "unknowns", "provenance",
    }, set(), "profile")
    require(profile.get("artifact_type") == "repository_profile_v1", "profile.artifact_type is incompatible")
    assert_compatible(contract["compatibility"], compatibility(profile.get("compatibility"), "profile.compatibility"),
                      "profile.compatibility")
    text(profile["repository_id"], "profile.repository_id")
    dimension_states = obj(profile["dimension_states"], "profile.dimension_states")
    require(all(value in UNCERTAINTY_STATES | {"analyzed"} for value in dimension_states.values()),
            "profile.dimension_states contains an invalid state")
    require(set(dimension_states) >= set(contract["required_core_dimensions"]),
            "profile omits required core dimensions")
    allowed_dimensions = set(contract["required_core_dimensions"]) | set(contract.get("adaptive_dimensions", []))
    require(set(dimension_states) <= allowed_dimensions,
            "profile contains dimensions outside the frozen ontology")
    claims = [validate_claim(value, f"profile.claims[{index}]")
              for index, value in enumerate(array(profile["claims"], "profile.claims"))]
    evidence = [validate_evidence(value, f"profile.evidence[{index}]")
                for index, value in enumerate(array(profile["evidence"], "profile.evidence"))]
    capabilities = [validate_capability(value, f"profile.capabilities[{index}]")
                    for index, value in enumerate(array(profile["capabilities"], "profile.capabilities"))]
    claim_ids = unique_ids(claims, "profile.claims")
    evidence_ids = unique_ids(evidence, "profile.evidence")
    capability_ids = unique_ids(capabilities, "profile.capabilities")
    stages = {claim["id"]: claim["stage"] for claim in claims}
    for evidence_item in evidence:
        require(set(evidence_item["supports"]) <= claim_ids,
                f"profile evidence '{evidence_item['id']}' supports an unknown claim")
    for claim in claims:
        require(set(claim["evidence_ids"]) | set(claim["counterevidence_ids"]) <= evidence_ids,
                f"profile claim '{claim['id']}' references unknown evidence")
        require(set(claim["parent_claim_ids"]) <= claim_ids,
                f"profile claim '{claim['id']}' references an unknown parent")
        if claim["stage"] == "observation":
            require(not claim["parent_claim_ids"] and bool(claim["evidence_ids"]),
                    f"profile observation '{claim['id']}' requires evidence and no parent")
        elif claim["stage"] == "interpretation":
            require(any(stages[parent] == "observation" for parent in claim["parent_claim_ids"]),
                    f"profile interpretation '{claim['id']}' must reference an observation")
        else:
            require(any(stages[parent] == "interpretation" for parent in claim["parent_claim_ids"]),
                    f"profile comparison implication '{claim['id']}' must reference an interpretation")
        require(claim["subject_id"] in claim_ids | capability_ids or claim["subject_id"] == profile["repository_id"],
                f"profile claim '{claim['id']}' has an unknown subject")
    for capability in capabilities:
        require(set(capability["evidence_ids"]) | set(capability["counterevidence_ids"]) <= evidence_ids,
                f"profile capability '{capability['id']}' references unknown evidence")
        for relation, targets in capability["relationships"].items():
            require(set(targets) <= capability_ids,
                    f"profile capability '{capability['id']}' {relation} references unknown capability")
    array(profile["system_boundary_observations"], "profile.system_boundary_observations")
    obj(profile["reconciliation"], "profile.reconciliation")
    array(profile["unknowns"], "profile.unknowns")
    provenance = obj(profile["provenance"], "profile.provenance")
    exact_keys(provenance, {"dimension_passes", "index_receipt", "index_receipt_sha256"}, {"index_fallback_decision"}, "profile.provenance")
    array(provenance["dimension_passes"], "profile.provenance.dimension_passes")
    repository = next((item for item in contract["repository_corpus"] if item["id"] == profile["repository_id"]), None)
    require(repository is not None, "profile repository is outside the frozen corpus")
    validated_receipt = validate_index_receipt(provenance["index_receipt"], contract, repository)
    require(provenance["index_receipt_sha256"] == receipt_digest(validated_receipt),
            "profile index receipt digest is incompatible")
    if validated_receipt["state"] == "coverage_limited":
        require("index_fallback_decision" in provenance, "coverage-limited profile requires fallback decision")
        validate_index_fallback(provenance["index_fallback_decision"], contract, validated_receipt)
    else:
        require("index_fallback_decision" not in provenance, "ready profile must not contain fallback decision")
    return profile


def validate_decision(decision: Any, contract: dict[str, Any], profiles: list[dict[str, Any]]) -> dict[str, Any]:
    decision = obj(decision, "decision")
    exact_keys(decision, {"artifact_type", "compatibility", "classification", "target_work_engine_boundary",
                          "profile_claim_refs", "expected_value", "architectural_fit", "evidence_strength",
                          "integration_cost", "migration_cost", "maintenance_burden", "constraints",
                          "reversibility_and_lock_in", "alternatives_considered", "assumptions",
                          "invalidation_triggers", "unresolved_questions"}, set(), "decision")
    require(decision["artifact_type"] == "comparison_decision_v1", "decision.artifact_type is incompatible")
    assert_compatible(contract["compatibility"], compatibility(decision["compatibility"], "decision.compatibility"),
                      "decision.compatibility")
    require(decision["classification"] in DECISIONS, "decision.classification is invalid")
    text(decision["target_work_engine_boundary"], "decision.target_work_engine_boundary")
    checked_profiles = [validate_profile(profile, contract) for profile in profiles]
    claims = {f"{profile['repository_id']}:{claim['id']}" for profile in checked_profiles for claim in profile["claims"]}
    refs = array(decision["profile_claim_refs"], "decision.profile_claim_refs")
    require(bool(refs) and set(refs) <= claims, "decision.profile_claim_refs contains an unknown or unqualified claim")
    for key in ("alternatives_considered", "assumptions", "invalidation_triggers", "unresolved_questions"):
        array(decision[key], f"decision.{key}")
    return decision


def load(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def emit(value: Any, output: Path | None = None) -> None:
    rendered = json.dumps(value, indent=2, sort_keys=True) + "\n"
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    validate_contract_parser = sub.add_parser("validate-contract")
    validate_contract_parser.add_argument("contract", type=Path)
    freeze = sub.add_parser("freeze-contract")
    freeze.add_argument("contract", type=Path)
    freeze.add_argument("output", type=Path)
    validate_pass_parser = sub.add_parser("validate-pass")
    validate_pass_parser.add_argument("contract", type=Path)
    validate_pass_parser.add_argument("pass_artifact", type=Path)
    reconcile_parser = sub.add_parser("reconcile")
    reconcile_parser.add_argument("contract", type=Path)
    reconcile_parser.add_argument("repository_id")
    reconcile_parser.add_argument("passes", nargs="+", type=Path)
    reconcile_parser.add_argument("--output", type=Path)
    compatible_parser = sub.add_parser("check-compatible")
    compatible_parser.add_argument("contract", type=Path)
    compatible_parser.add_argument("profiles", nargs="+", type=Path)
    decision_parser = sub.add_parser("validate-decision")
    decision_parser.add_argument("contract", type=Path)
    decision_parser.add_argument("decision", type=Path)
    decision_parser.add_argument("profiles", nargs="+", type=Path)
    gap_parser = sub.add_parser("validate-ontology-gap")
    gap_parser.add_argument("contract", type=Path)
    gap_parser.add_argument("proposal", type=Path)
    gap_parser.add_argument("source_pass", type=Path)
    pilot_parser = sub.add_parser("pilot")
    pilot_parser.add_argument("--fixture-dir", required=True, type=Path)
    pilot_parser.add_argument("--output", type=Path)
    prepare_parser = sub.add_parser("prepare-run")
    prepare_parser.add_argument("--config", required=True, type=Path)
    prepare_parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        if args.command == "prepare-run":
            contract, missing, derived = normalize_yaml_config(load_yaml_config(args.config))
            if missing:
                emit({"status": "not_ready", "missing_inputs": missing, "derived_repositories": derived})
                return 3
            require(not args.output.exists(), "prepared contract output already exists")
            emit(contract, args.output)
            emit({"status": "prepared", "output": str(args.output),
                  "run_id": contract["compatibility"]["run_id"]})
        elif args.command in {"validate-contract", "freeze-contract"}:
            result = validate_contract(load(args.contract))
            if args.command == "freeze-contract":
                require(not args.output.exists(), "frozen contract output already exists")
                emit(result, args.output)
            else:
                emit({"status": "valid", "run_id": result["compatibility"]["run_id"]})
        elif args.command == "validate-pass":
            result = validate_dimension_pass(load(args.pass_artifact), validate_contract(load(args.contract)))
            emit({"status": "valid", "repository_id": result["repository_id"], "dimension": result["dimension"]})
        elif args.command == "validate-ontology-gap":
            result = validate_ontology_gap(load(args.proposal), validate_contract(load(args.contract)), load(args.source_pass))
            emit({"status": "valid", "gap_id": result["gap_id"], "outside_active_ontology": True})
        elif args.command == "reconcile":
            result = reconcile(validate_contract(load(args.contract)), [load(path) for path in args.passes], args.repository_id)
            emit(result, args.output)
        elif args.command == "check-compatible":
            contract = validate_contract(load(args.contract))
            profiles = [validate_profile(load(path), contract) for path in args.profiles]
            emit({"status": "compatible", "run_id": contract["compatibility"]["run_id"], "profile_count": len(profiles)})
        elif args.command == "validate-decision":
            contract = validate_contract(load(args.contract))
            result = validate_decision(load(args.decision), contract, [load(path) for path in args.profiles])
            emit({"status": "valid", "classification": result["classification"]})
        else:
            fixture = args.fixture_dir
            contract = validate_contract(load(fixture / "contract.json"))
            passes = sorted(fixture.glob("pass-*.json"))
            profile = reconcile(contract, [load(path) for path in passes], "sample")
            validate_decision(load(fixture / "decision.json"), contract, [profile])
            result = {"status": "pilot-complete", "profile": profile, "decision_separate": True}
            emit(result, args.output)
    except (ArtifactError, OSError, json.JSONDecodeError) as error:
        print(json.dumps({"status": "invalid", "error": str(error)}, sort_keys=True), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
