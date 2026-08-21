#!/usr/bin/env python3
"""Build, validate, export, and compare evidence-calibrated review bench artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


SCHEMA_VERSION = 1
TRUTH_SCHEMA_VERSIONS = {1, 2}
RESULT_SCHEMA_VERSIONS = {1, 2}
VERDICTS = {"accepted", "rejected", "blocked_unverified", "stale_snapshot"}
SEVERITIES = {"critical", "high", "medium", "low"}
SEVERITY_RANK = {"low": 1, "medium": 2, "high": 3, "critical": 4}
CONFIDENCE = {"high", "medium", "low"}
REVIEW_CLASSES = {
    "placement",
    "implementation",
    "closure",
    "integrity",
    "visual",
    "security",
}
ROUTE_CLASSES = {
    "deterministic_local",
    "runtime_cross_boundary",
    "persistent_state_identity_integrity",
    "novel_architecture_high_consequence",
}
SEMANTIC_DOMAINS = {
    "architecture", "configuration", "identity", "integrity", "lifecycle",
    "performance", "persistence", "runtime", "schema", "security",
    "tests_docs", "ui_visual", "other",
}
TOPOLOGIES = {
    "local", "cross_module", "cross_process", "migration", "generated",
    "concurrency_sensitive", "large_diff",
}
NOVELTY_LEVELS = {"established_pattern", "pattern_extension", "new_ownership"}
EVIDENCE_TYPES = {
    "static_source", "deterministic_test", "runtime_inspection",
    "persisted_artifact", "visual_inspection", "human_judgment",
    "historical_reconstruction",
}
CONSEQUENCES = {
    "cosmetic", "recoverable", "workflow_blocking", "state_corrupting",
    "security_sensitive", "irreversible",
}
REASONING_EFFORTS = {"low", "medium", "high", "xhigh", "max", "ultra", "unknown", "not_configurable"}
AGGREGATION_STRATEGIES = {"none", "self_aggregation", "cross_model_aggregation", "evidence_grounded_aggregation"}
RISKS = {"low", "medium", "high", "critical"}
SNAPSHOT_KINDS = {"git_commit", "checkpoint", "reconstructed"}
RECONSTRUCTION_CONFIDENCE = {"high", "medium", "low", "unknown"}
DISPOSITIONS = {"true_positive", "false_positive", "duplicate", "nonblocking_observation"}


class BenchError(ValueError):
    """Raised when a bench artifact would make comparison ambiguous."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise BenchError(message)


def obj(value: Any, path: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{path} must be an object")
    return value


def array(value: Any, path: str) -> list[Any]:
    require(isinstance(value, list), f"{path} must be an array")
    return value


def text(value: Any, path: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{path} must be a nonempty string")
    return value


def boolean(value: Any, path: str) -> bool:
    require(isinstance(value, bool), f"{path} must be boolean")
    return value


def nonnegative_number(value: Any, path: str) -> int | float:
    require(isinstance(value, (int, float)) and not isinstance(value, bool) and value >= 0,
            f"{path} must be nonnegative")
    return value


def positive_integer(value: Any, path: str) -> int:
    require(isinstance(value, int) and not isinstance(value, bool) and value > 0,
            f"{path} must be a positive integer")
    return value


def exact_keys(value: dict[str, Any], required: set[str], optional: set[str], path: str) -> None:
    missing = required - set(value)
    unknown = set(value) - required - optional
    require(not missing, f"{path} missing fields: {', '.join(sorted(missing))}")
    require(not unknown, f"{path} unknown fields: {', '.join(sorted(unknown))}")


def unique(items: Iterable[str], path: str) -> set[str]:
    values = list(items)
    require(len(values) == len(set(values)), f"{path} contains duplicate identifiers")
    return set(values)


def strings(value: Any, path: str) -> list[str]:
    result = array(value, path)
    for index, item in enumerate(result):
        text(item, f"{path}[{index}]")
    return result


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BenchError(f"cannot load {path}: {error}") from error


def emit(value: Any, output: Path | None = None) -> None:
    rendered = json.dumps(value, indent=2, sort_keys=True) + "\n"
    if output:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)


def artifact_header(value: Any, artifact_type: str, path: str) -> dict[str, Any]:
    result = obj(value, path)
    require(result.get("artifact_type") == artifact_type, f"{path}.artifact_type must be {artifact_type}")
    require(result.get("schema_version") == SCHEMA_VERSION, f"{path}.schema_version must be {SCHEMA_VERSION}")
    return result


def validate_snapshot(value: Any, path: str) -> dict[str, Any]:
    snapshot = obj(value, path)
    exact_keys(
        snapshot,
        {"kind", "identity", "digest", "reconstruction_confidence", "limitations"},
        {"repository", "tree_oid"},
        path,
    )
    require(snapshot["kind"] in SNAPSHOT_KINDS, f"{path}.kind is invalid")
    text(snapshot["identity"], f"{path}.identity")
    digest = text(snapshot["digest"], f"{path}.digest")
    require(len(digest) == 64 and all(character in "0123456789abcdef" for character in digest),
            f"{path}.digest must be lowercase sha256")
    require(snapshot["reconstruction_confidence"] in RECONSTRUCTION_CONFIDENCE,
            f"{path}.reconstruction_confidence is invalid")
    strings(snapshot["limitations"], f"{path}.limitations")
    if "repository" in snapshot:
        text(snapshot["repository"], f"{path}.repository")
    if "tree_oid" in snapshot:
        text(snapshot["tree_oid"], f"{path}.tree_oid")
    return snapshot


def validate_case(value: Any, path: str) -> dict[str, Any]:
    case = obj(value, path)
    exact_keys(
        case,
        {
            "case_id", "title", "review_class", "risk", "change_profile", "contract", "snapshot",
            "artifacts", "deterministic_evidence", "limitations",
        },
        set(),
        path,
    )
    text(case["case_id"], f"{path}.case_id")
    text(case["title"], f"{path}.title")
    require(case["review_class"] in REVIEW_CLASSES, f"{path}.review_class is invalid")
    require(case["risk"] in RISKS, f"{path}.risk is invalid")
    profile = obj(case["change_profile"], f"{path}.change_profile")
    exact_keys(
        profile,
        {"route_class", "semantic_domains", "topologies", "novelty", "evidence_types", "consequences"},
        {"changed_files", "changed_lines"},
        f"{path}.change_profile",
    )
    require(profile["route_class"] in ROUTE_CLASSES, f"{path}.change_profile.route_class is invalid")
    for key, vocabulary in (
        ("semantic_domains", SEMANTIC_DOMAINS),
        ("topologies", TOPOLOGIES),
        ("evidence_types", EVIDENCE_TYPES),
        ("consequences", CONSEQUENCES),
    ):
        values = strings(profile[key], f"{path}.change_profile.{key}")
        require(bool(values), f"{path}.change_profile.{key} must not be empty")
        require(set(values) <= vocabulary, f"{path}.change_profile.{key} contains invalid values")
        unique(values, f"{path}.change_profile.{key}")
    require(profile["novelty"] in NOVELTY_LEVELS, f"{path}.change_profile.novelty is invalid")
    for key in ("changed_files", "changed_lines"):
        if key in profile:
            nonnegative_number(profile[key], f"{path}.change_profile.{key}")
    contract = obj(case["contract"], f"{path}.contract")
    exact_keys(contract, {"goal", "acceptance_criteria", "excluded_scope"}, set(), f"{path}.contract")
    text(contract["goal"], f"{path}.contract.goal")
    require(bool(strings(contract["acceptance_criteria"], f"{path}.contract.acceptance_criteria")),
            f"{path}.contract.acceptance_criteria must not be empty")
    strings(contract["excluded_scope"], f"{path}.contract.excluded_scope")
    validate_snapshot(case["snapshot"], f"{path}.snapshot")
    artifacts = array(case["artifacts"], f"{path}.artifacts")
    for index, artifact_value in enumerate(artifacts):
        artifact_path = f"{path}.artifacts[{index}]"
        artifact = obj(artifact_value, artifact_path)
        exact_keys(artifact, {"path", "role"}, {"sha256"}, artifact_path)
        text(artifact["path"], f"{artifact_path}.path")
        text(artifact["role"], f"{artifact_path}.role")
        if "sha256" in artifact:
            digest = text(artifact["sha256"], f"{artifact_path}.sha256")
            require(len(digest) == 64, f"{artifact_path}.sha256 must be 64 characters")
    strings(case["deterministic_evidence"], f"{path}.deterministic_evidence")
    strings(case["limitations"], f"{path}.limitations")
    return case


def validate_corpus(value: Any) -> dict[str, Any]:
    corpus = artifact_header(value, "review_bench_corpus_v1", "corpus")
    exact_keys(corpus, {"artifact_type", "schema_version", "bench_id", "protocol", "cases"}, set(), "corpus")
    text(corpus["bench_id"], "corpus.bench_id")
    protocol = obj(corpus["protocol"], "corpus.protocol")
    exact_keys(protocol, {"name", "version", "prompt", "verdicts", "severities", "confidence_scale"}, set(), "corpus.protocol")
    text(protocol["name"], "corpus.protocol.name")
    text(protocol["version"], "corpus.protocol.version")
    text(protocol["prompt"], "corpus.protocol.prompt")
    require(set(strings(protocol["verdicts"], "corpus.protocol.verdicts")) == VERDICTS,
            "corpus.protocol.verdicts must use the complete vocabulary")
    require(set(strings(protocol["severities"], "corpus.protocol.severities")) == SEVERITIES,
            "corpus.protocol.severities must use the complete vocabulary")
    require(set(strings(protocol["confidence_scale"], "corpus.protocol.confidence_scale")) == CONFIDENCE,
            "corpus.protocol.confidence_scale must use the complete vocabulary")
    cases = array(corpus["cases"], "corpus.cases")
    require(bool(cases), "corpus.cases must not be empty")
    for index, case in enumerate(cases):
        validate_case(case, f"corpus.cases[{index}]")
    unique((case["case_id"] for case in cases), "corpus.cases")
    return corpus


def validate_truth_cases(truth: dict[str, Any], corpus: dict[str, Any]) -> None:
    require(truth["bench_id"] == corpus["bench_id"], "truth.bench_id does not match corpus")
    corpus_ids = {case["case_id"] for case in corpus["cases"]}
    truth_cases = array(truth["cases"], "truth.cases")
    require({case.get("case_id") for case in truth_cases} == corpus_ids,
            "truth.cases must exactly cover corpus cases")
    for index, truth_case_value in enumerate(truth_cases):
        path = f"truth.cases[{index}]"
        truth_case = obj(truth_case_value, path)
        exact_keys(truth_case, {"case_id", "expected_verdict", "findings", "adjudication_notes"}, set(), path)
        text(truth_case["case_id"], f"{path}.case_id")
        require(truth_case["expected_verdict"] in VERDICTS, f"{path}.expected_verdict is invalid")
        findings = array(truth_case["findings"], f"{path}.findings")
        truth_ids: list[str] = []
        for finding_index, finding_value in enumerate(findings):
            finding_path = f"{path}.findings[{finding_index}]"
            finding = obj(finding_value, finding_path)
            exact_keys(
                finding,
                {"truth_id", "severity", "blocking", "category", "claim", "evidence", "adjudication", "tags"},
                set(),
                finding_path,
            )
            truth_ids.append(text(finding["truth_id"], f"{finding_path}.truth_id"))
            require(finding["severity"] in SEVERITIES, f"{finding_path}.severity is invalid")
            boolean(finding["blocking"], f"{finding_path}.blocking")
            text(finding["category"], f"{finding_path}.category")
            text(finding["claim"], f"{finding_path}.claim")
            require(bool(strings(finding["evidence"], f"{finding_path}.evidence")),
                    f"{finding_path}.evidence must not be empty")
            text(finding["adjudication"], f"{finding_path}.adjudication")
            strings(finding["tags"], f"{finding_path}.tags")
        unique(truth_ids, f"{path}.findings")
        strings(truth_case["adjudication_notes"], f"{path}.adjudication_notes")
    strings(truth["limitations"], "truth.limitations")


def truth_target_value(
    truth: dict[str, Any], case_id: str, finding_id: str | None, field: str, path: str
) -> Any:
    truth_case = next((case for case in truth["cases"] if case["case_id"] == case_id), None)
    require(truth_case is not None, f"{path}.case_id is unknown")
    if field == "expected_verdict":
        require(finding_id is None, f"{path}.finding_id must be null for expected_verdict")
        return truth_case[field]
    require(field in {"finding.blocking", "finding.severity", "finding.adjudication"},
            f"{path}.field is invalid")
    require(isinstance(finding_id, str) and bool(finding_id.strip()),
            f"{path}.finding_id is required for a finding field")
    finding = next(
        (item for item in truth_case["findings"] if item["truth_id"] == finding_id), None
    )
    require(finding is not None, f"{path}.finding_id is unknown")
    return finding[field.split(".", 1)[1]]


def validate_truth(value: Any, corpus: dict[str, Any], artifact_path: Path | None = None) -> dict[str, Any]:
    truth = obj(value, "truth")
    schema_version = truth.get("schema_version")
    require(schema_version in TRUTH_SCHEMA_VERSIONS, "truth.schema_version must be 1 or 2")
    require(truth.get("artifact_type") == f"review_bench_truth_v{schema_version}",
            f"truth.artifact_type must be review_bench_truth_v{schema_version}")
    if schema_version == 1:
        exact_keys(truth, {"artifact_type", "schema_version", "bench_id", "cases", "limitations"}, set(), "truth")
        validate_truth_cases(truth, corpus)
        return truth

    exact_keys(
        truth,
        {"artifact_type", "schema_version", "bench_id", "supersedes", "adjudication_amendments", "cases", "limitations"},
        set(),
        "truth",
    )
    validate_truth_cases(truth, corpus)
    supersedes = obj(truth["supersedes"], "truth.supersedes")
    exact_keys(supersedes, {"artifact_type", "path", "sha256"}, set(), "truth.supersedes")
    require(supersedes["artifact_type"] == "review_bench_truth_v1",
            "truth.supersedes.artifact_type must be review_bench_truth_v1")
    relative_path = Path(text(supersedes["path"], "truth.supersedes.path"))
    require(not relative_path.is_absolute() and ".." not in relative_path.parts,
            "truth.supersedes.path must be a contained relative path")
    digest = text(supersedes["sha256"], "truth.supersedes.sha256")
    require(len(digest) == 64 and all(character in "0123456789abcdef" for character in digest),
            "truth.supersedes.sha256 must be lowercase sha256")

    base_truth = None
    if artifact_path is not None:
        base_path = artifact_path.parent / relative_path
        require(base_path.is_file(), f"truth superseded artifact does not exist: {base_path}")
        require(sha256_file(base_path) == digest, "truth superseded artifact sha256 does not match")
        base_truth = validate_truth(load(base_path), corpus)

    amendments = array(truth["adjudication_amendments"], "truth.adjudication_amendments")
    require(bool(amendments), "truth.adjudication_amendments must not be empty")
    amendment_ids: list[str] = []
    changed_targets: set[tuple[str, str | None, str]] = set()
    for index, amendment_value in enumerate(amendments):
        path = f"truth.adjudication_amendments[{index}]"
        amendment = obj(amendment_value, path)
        exact_keys(
            amendment,
            {"amendment_id", "adjudicated_at", "adjudicator", "reason", "evidence", "changes", "limitations"},
            set(),
            path,
        )
        amendment_ids.append(text(amendment["amendment_id"], f"{path}.amendment_id"))
        text(amendment["adjudicated_at"], f"{path}.adjudicated_at")
        text(amendment["adjudicator"], f"{path}.adjudicator")
        text(amendment["reason"], f"{path}.reason")
        require(bool(strings(amendment["evidence"], f"{path}.evidence")), f"{path}.evidence must not be empty")
        strings(amendment["limitations"], f"{path}.limitations")
        changes = array(amendment["changes"], f"{path}.changes")
        require(bool(changes), f"{path}.changes must not be empty")
        for change_index, change_value in enumerate(changes):
            change_path = f"{path}.changes[{change_index}]"
            change = obj(change_value, change_path)
            exact_keys(change, {"case_id", "finding_id", "field", "before", "after"}, set(), change_path)
            case_id = text(change["case_id"], f"{change_path}.case_id")
            finding_id = change["finding_id"]
            require(finding_id is None or isinstance(finding_id, str), f"{change_path}.finding_id must be string or null")
            field = text(change["field"], f"{change_path}.field")
            target = (case_id, finding_id, field)
            require(target not in changed_targets, f"{change_path} duplicates an amended target")
            changed_targets.add(target)
            if base_truth is not None:
                require(truth_target_value(base_truth, case_id, finding_id, field, change_path) == change["before"],
                        f"{change_path}.before does not match superseded truth")
                require(truth_target_value(truth, case_id, finding_id, field, change_path) == change["after"],
                        f"{change_path}.after does not match amended truth")
            require(change["before"] != change["after"], f"{change_path} must change the value")
    unique(amendment_ids, "truth.adjudication_amendments")
    return truth


def validate_result(value: Any, corpus: dict[str, Any]) -> dict[str, Any]:
    result = obj(value, "result")
    schema_version = result.get("schema_version")
    require(schema_version in RESULT_SCHEMA_VERSIONS, "result.schema_version must be 1 or 2")
    require(result.get("artifact_type") == f"review_bench_result_v{schema_version}",
            f"result.artifact_type must be review_bench_result_v{schema_version}")
    version_fields = {"verified_claims", "observations"} if schema_version == 2 else set()
    exact_keys(
        result,
        {
            "artifact_type", "schema_version", "bench_id", "case_id", "result_id",
            "attempt_id", "reviewer", "snapshot_digest", "verdict", "findings",
            "limitations", "timing",
        } | version_fields,
        set(),
        "result",
    )
    require(result["bench_id"] == corpus["bench_id"], "result.bench_id does not match corpus")
    case_by_id = {case["case_id"]: case for case in corpus["cases"]}
    require(result["case_id"] in case_by_id, "result.case_id is not in corpus")
    for key in ("result_id", "attempt_id"):
        text(result[key], f"result.{key}")
    reviewer = obj(result["reviewer"], "result.reviewer")
    exact_keys(
        reviewer,
        {
            "provider", "model", "harness", "inference_family", "fresh_context", "tool_access",
            "reasoning_effort", "review_protocol", "pass_count", "aggregation_strategy",
            "aggregation_members",
        },
        {"session_id"},
        "result.reviewer",
    )
    for key in ("provider", "model", "harness", "inference_family"):
        text(reviewer[key], f"result.reviewer.{key}")
    boolean(reviewer["fresh_context"], "result.reviewer.fresh_context")
    strings(reviewer["tool_access"], "result.reviewer.tool_access")
    if "session_id" in reviewer:
        text(reviewer["session_id"], "result.reviewer.session_id")
    require(reviewer["reasoning_effort"] in REASONING_EFFORTS,
            "result.reviewer.reasoning_effort is invalid")
    text(reviewer["review_protocol"], "result.reviewer.review_protocol")
    positive_integer(reviewer["pass_count"], "result.reviewer.pass_count")
    require(reviewer["aggregation_strategy"] in AGGREGATION_STRATEGIES,
            "result.reviewer.aggregation_strategy is invalid")
    aggregation_members = strings(reviewer["aggregation_members"], "result.reviewer.aggregation_members")
    unique(aggregation_members, "result.reviewer.aggregation_members")
    if reviewer["pass_count"] == 1:
        require(reviewer["aggregation_strategy"] == "none",
                "result.reviewer.aggregation_strategy must be none for one pass")
    if reviewer["aggregation_strategy"] == "none":
        require(not aggregation_members, "result.reviewer.aggregation_members must be empty without aggregation")
    else:
        require(reviewer["pass_count"] > 1, "aggregated results require more than one pass")
        require(bool(aggregation_members), "aggregated results require aggregation_members")
    if reviewer["aggregation_strategy"] == "self_aggregation":
        require(len(aggregation_members) == 1, "self_aggregation requires exactly one member configuration")
    if reviewer["aggregation_strategy"] == "cross_model_aggregation":
        require(len(aggregation_members) >= 2, "cross_model_aggregation requires at least two member configurations")
    require(result["snapshot_digest"] == case_by_id[result["case_id"]]["snapshot"]["digest"],
            "result.snapshot_digest does not match the case")
    require(result["verdict"] in VERDICTS, "result.verdict is invalid")
    finding_ids: list[str] = []
    for index, finding_value in enumerate(array(result["findings"], "result.findings")):
        path = f"result.findings[{index}]"
        finding = obj(finding_value, path)
        exact_keys(finding, {"finding_id", "severity", "blocking", "category", "claim", "evidence", "confidence"}, set(), path)
        finding_ids.append(text(finding["finding_id"], f"{path}.finding_id"))
        require(finding["severity"] in SEVERITIES, f"{path}.severity is invalid")
        boolean(finding["blocking"], f"{path}.blocking")
        text(finding["category"], f"{path}.category")
        text(finding["claim"], f"{path}.claim")
        require(bool(strings(finding["evidence"], f"{path}.evidence")), f"{path}.evidence must not be empty")
        require(finding["confidence"] in CONFIDENCE, f"{path}.confidence is invalid")
    unique(finding_ids, "result.findings")
    if schema_version == 2:
        case_criteria = set(case_by_id[result["case_id"]]["contract"]["acceptance_criteria"])
        verified_ids: list[str] = []
        verified_criteria: set[str] = set()
        for index, claim_value in enumerate(array(result["verified_claims"], "result.verified_claims")):
            path = f"result.verified_claims[{index}]"
            claim = obj(claim_value, path)
            exact_keys(claim, {"claim_id", "claim", "evidence", "confidence", "acceptance_criteria"}, set(), path)
            verified_ids.append(text(claim["claim_id"], f"{path}.claim_id"))
            text(claim["claim"], f"{path}.claim")
            require(bool(strings(claim["evidence"], f"{path}.evidence")), f"{path}.evidence must not be empty")
            require(claim["confidence"] == "high", f"{path}.confidence must be high")
            criteria = strings(claim["acceptance_criteria"], f"{path}.acceptance_criteria")
            require(bool(criteria), f"{path}.acceptance_criteria must not be empty")
            require(set(criteria) <= case_criteria,
                    f"{path}.acceptance_criteria must quote criteria from the case contract")
            verified_criteria.update(criteria)
        unique(verified_ids, "result.verified_claims")

        observation_ids: list[str] = []
        for index, observation_value in enumerate(array(result["observations"], "result.observations")):
            path = f"result.observations[{index}]"
            observation = obj(observation_value, path)
            exact_keys(observation, {"observation_id", "category", "claim", "evidence", "confidence"}, set(), path)
            observation_ids.append(text(observation["observation_id"], f"{path}.observation_id"))
            text(observation["category"], f"{path}.category")
            text(observation["claim"], f"{path}.claim")
            require(bool(strings(observation["evidence"], f"{path}.evidence")),
                    f"{path}.evidence must not be empty")
            require(observation["confidence"] in CONFIDENCE, f"{path}.confidence is invalid")
        unique(observation_ids, "result.observations")
        unique(finding_ids + verified_ids + observation_ids, "result item identifiers")
        if result["verdict"] == "accepted":
            require(not any(finding["blocking"] for finding in result["findings"]),
                    "accepted result v2 cannot contain a blocking finding")
            require(verified_criteria == case_criteria,
                    "accepted result v2 must verify every case acceptance criterion")
        if result["verdict"] == "rejected":
            require(any(finding["blocking"] for finding in result["findings"]),
                    "rejected result v2 requires a blocking finding")
    strings(result["limitations"], "result.limitations")
    timing = obj(result["timing"], "result.timing")
    exact_keys(
        timing,
        {"started_at", "completed_at"},
        {"wall_clock_seconds", "cost_usd", "input_tokens", "output_tokens"},
        "result.timing",
    )
    text(timing["started_at"], "result.timing.started_at")
    text(timing["completed_at"], "result.timing.completed_at")
    for key in ("wall_clock_seconds", "cost_usd", "input_tokens", "output_tokens"):
        if key in timing:
            nonnegative_number(timing[key], f"result.timing.{key}")
    return result


def validate_scoring(value: Any, corpus: dict[str, Any], truth: dict[str, Any], result: dict[str, Any]) -> dict[str, Any]:
    scoring = artifact_header(value, "review_bench_scoring_v1", "scoring")
    exact_keys(
        scoring,
        {
            "artifact_type", "schema_version", "bench_id", "case_id", "result_id",
            "finding_dispositions", "missed_truth_ids", "adjudicator", "limitations",
        },
        set(),
        "scoring",
    )
    require(scoring["bench_id"] == corpus["bench_id"], "scoring.bench_id does not match corpus")
    require(scoring["case_id"] == result["case_id"], "scoring.case_id does not match result")
    require(scoring["result_id"] == result["result_id"], "scoring.result_id does not match result")
    truth_case = next(case for case in truth["cases"] if case["case_id"] == result["case_id"])
    truth_ids = {finding["truth_id"] for finding in truth_case["findings"]}
    result_ids = {finding["finding_id"] for finding in result["findings"]}
    dispositions = array(scoring["finding_dispositions"], "scoring.finding_dispositions")
    observed_result_ids: list[str] = []
    detected_truth_ids: set[str] = set()
    for index, disposition_value in enumerate(dispositions):
        path = f"scoring.finding_dispositions[{index}]"
        disposition = obj(disposition_value, path)
        exact_keys(disposition, {"result_finding_id", "disposition", "truth_finding_ids", "evidence_valid", "notes"}, set(), path)
        result_id = text(disposition["result_finding_id"], f"{path}.result_finding_id")
        observed_result_ids.append(result_id)
        require(result_id in result_ids, f"{path}.result_finding_id is unknown")
        require(disposition["disposition"] in DISPOSITIONS, f"{path}.disposition is invalid")
        mapped = set(strings(disposition["truth_finding_ids"], f"{path}.truth_finding_ids"))
        require(mapped <= truth_ids, f"{path}.truth_finding_ids contains unknown truth")
        if disposition["disposition"] == "true_positive":
            require(bool(mapped), f"{path}.true_positive requires truth_finding_ids")
            detected_truth_ids.update(mapped)
        else:
            require(not mapped or disposition["disposition"] == "duplicate",
                    f"{path}.truth_finding_ids are only valid for true_positive or duplicate")
        boolean(disposition["evidence_valid"], f"{path}.evidence_valid")
        text(disposition["notes"], f"{path}.notes")
    require(set(observed_result_ids) == result_ids and len(observed_result_ids) == len(result_ids),
            "scoring dispositions must classify every result finding exactly once")
    missed = set(strings(scoring["missed_truth_ids"], "scoring.missed_truth_ids"))
    require(missed == truth_ids - detected_truth_ids,
            "scoring.missed_truth_ids must exactly equal undetected truth findings")
    text(scoring["adjudicator"], "scoring.adjudicator")
    strings(scoring["limitations"], "scoring.limitations")
    return scoring


def configuration_key(result: dict[str, Any]) -> str:
    reviewer = result["reviewer"]
    return "|".join((
        reviewer["provider"], reviewer["model"], reviewer["harness"],
        reviewer["reasoning_effort"], reviewer["review_protocol"],
        str(reviewer["pass_count"]), reviewer["aggregation_strategy"],
        ",".join(sorted(reviewer["aggregation_members"])),
        ",".join(sorted(reviewer["tool_access"])),
    ))


def ratio(numerator: int, denominator: int) -> float | None:
    return round(numerator / denominator, 6) if denominator else None


def new_aggregate(result: dict[str, Any]) -> dict[str, Any]:
    reviewer = result["reviewer"]
    return {
        "provider": reviewer["provider"],
        "model": reviewer["model"],
        "harness": reviewer["harness"],
        "inference_family": reviewer["inference_family"],
        "reasoning_effort": reviewer["reasoning_effort"],
        "review_protocol": reviewer["review_protocol"],
        "tool_access": sorted(reviewer["tool_access"]),
        "pass_count": reviewer["pass_count"],
        "aggregation_strategy": reviewer["aggregation_strategy"],
        "aggregation_members": sorted(reviewer["aggregation_members"]),
        "attempts": 0,
        "cases": set(),
        "truth_opportunities": 0,
        "truth_detected": 0,
        "blocking_opportunities": 0,
        "blocking_detected": 0,
        "reported_findings": 0,
        "verified_claims": 0,
        "reviewer_observations": 0,
        "true_positive_findings": 0,
        "false_positive_findings": 0,
        "duplicate_findings": 0,
        "nonblocking_observation_findings": 0,
        "evidence_valid_findings": 0,
        "severity_overstatements": 0,
        "severity_understatements": 0,
        "blocking_label_overstatements": 0,
        "blocking_label_understatements": 0,
        "false_accepts": 0,
        "false_blocks": 0,
        "blocked_unverified": 0,
        "resource_totals": defaultdict(float),
        "resource_observations": defaultdict(int),
    }


def update_aggregate(
    aggregate: dict[str, Any],
    result: dict[str, Any],
    scoring: dict[str, Any],
    truth_findings: dict[str, dict[str, Any]],
    expected_verdict: str,
) -> set[str]:
    detected: set[str] = set()
    aggregate["attempts"] += 1
    aggregate["cases"].add(result["case_id"])
    aggregate["truth_opportunities"] += len(truth_findings)
    blocking_truth = {identifier for identifier, finding in truth_findings.items() if finding["blocking"]}
    aggregate["blocking_opportunities"] += len(blocking_truth)
    aggregate["reported_findings"] += len(result["findings"])
    aggregate["verified_claims"] += len(result.get("verified_claims", []))
    aggregate["reviewer_observations"] += len(result.get("observations", []))
    result_findings = {finding["finding_id"]: finding for finding in result["findings"]}
    for disposition in scoring["finding_dispositions"]:
        if disposition["disposition"] == "true_positive":
            aggregate["true_positive_findings"] += 1
            detected.update(disposition["truth_finding_ids"])
            reported = result_findings[disposition["result_finding_id"]]
            matched = [truth_findings[identifier] for identifier in disposition["truth_finding_ids"]]
            expected_severity = max(matched, key=lambda finding: SEVERITY_RANK[finding["severity"]])["severity"]
            if SEVERITY_RANK[reported["severity"]] > SEVERITY_RANK[expected_severity]:
                aggregate["severity_overstatements"] += 1
            elif SEVERITY_RANK[reported["severity"]] < SEVERITY_RANK[expected_severity]:
                aggregate["severity_understatements"] += 1
            expected_blocking = any(finding["blocking"] for finding in matched)
            if reported["blocking"] and not expected_blocking:
                aggregate["blocking_label_overstatements"] += 1
            elif not reported["blocking"] and expected_blocking:
                aggregate["blocking_label_understatements"] += 1
        elif disposition["disposition"] == "false_positive":
            aggregate["false_positive_findings"] += 1
        elif disposition["disposition"] == "duplicate":
            aggregate["duplicate_findings"] += 1
        elif disposition["disposition"] == "nonblocking_observation":
            aggregate["nonblocking_observation_findings"] += 1
        if disposition["evidence_valid"]:
            aggregate["evidence_valid_findings"] += 1
    aggregate["truth_detected"] += len(detected)
    aggregate["blocking_detected"] += len(detected & blocking_truth)
    if expected_verdict != "accepted" and result["verdict"] == "accepted":
        aggregate["false_accepts"] += 1
    if expected_verdict == "accepted" and result["verdict"] != "accepted":
        aggregate["false_blocks"] += 1
    if result["verdict"] == "blocked_unverified":
        aggregate["blocked_unverified"] += 1
    for resource in ("wall_clock_seconds", "cost_usd", "input_tokens", "output_tokens"):
        if resource in result["timing"]:
            aggregate["resource_totals"][resource] += result["timing"][resource]
            aggregate["resource_observations"][resource] += 1
    return detected


def render_aggregate(aggregate: dict[str, Any]) -> dict[str, Any]:
    rendered = {key: value for key, value in aggregate.items() if key not in {"cases", "resource_totals", "resource_observations"}}
    rendered["case_count"] = len(aggregate["cases"])
    rendered["defect_recall"] = ratio(aggregate["truth_detected"], aggregate["truth_opportunities"])
    rendered["blocking_recall"] = ratio(aggregate["blocking_detected"], aggregate["blocking_opportunities"])
    precision_denominator = aggregate["true_positive_findings"] + aggregate["false_positive_findings"]
    rendered["finding_precision"] = ratio(aggregate["true_positive_findings"], precision_denominator)
    rendered["evidence_validity"] = ratio(aggregate["evidence_valid_findings"], aggregate["reported_findings"])
    rendered["resources"] = {}
    for resource in ("wall_clock_seconds", "cost_usd", "input_tokens", "output_tokens"):
        observations = aggregate["resource_observations"].get(resource, 0)
        total = aggregate["resource_totals"].get(resource, 0)
        rendered["resources"][resource] = {
            "observations": observations,
            "total": total if observations else None,
            "average": round(total / observations, 6) if observations else None,
        }
    detected = aggregate["truth_detected"]
    attempts = aggregate["attempts"]
    rendered["resource_efficiency"] = {
        "cost_usd_per_truth_detected": (
            round(aggregate["resource_totals"]["cost_usd"] / detected, 6)
            if detected and aggregate["resource_observations"]["cost_usd"] == attempts else None
        ),
        "wall_clock_seconds_per_truth_detected": (
            round(aggregate["resource_totals"]["wall_clock_seconds"] / detected, 6)
            if detected and aggregate["resource_observations"]["wall_clock_seconds"] == attempts else None
        ),
        "total_tokens_per_truth_detected": (
            round((aggregate["resource_totals"]["input_tokens"] + aggregate["resource_totals"]["output_tokens"]) / detected, 6)
            if detected
            and aggregate["resource_observations"]["input_tokens"] == attempts
            and aggregate["resource_observations"]["output_tokens"] == attempts
            else None
        ),
    }
    return rendered


def compare(corpus: dict[str, Any], truth: dict[str, Any], results: list[dict[str, Any]], scorings: list[dict[str, Any]]) -> dict[str, Any]:
    scoring_by_result = {scoring["result_id"]: scoring for scoring in scorings}
    require(len(scoring_by_result) == len(scorings), "duplicate scoring result_id")
    require({result["result_id"] for result in results} == set(scoring_by_result),
            "every result must have exactly one scoring artifact")
    truth_by_case = {case["case_id"]: case for case in truth["cases"]}
    corpus_by_case = {case["case_id"]: case for case in corpus["cases"]}
    aggregates: dict[str, dict[str, Any]] = {}
    route_aggregates: dict[str, dict[str, dict[str, Any]]] = defaultdict(dict)
    detection_sets: dict[str, dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))
    false_positive_counts: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))

    for result in results:
        key = configuration_key(result)
        aggregate = aggregates.setdefault(key, new_aggregate(result))
        scoring = scoring_by_result[result["result_id"]]
        truth_case = truth_by_case[result["case_id"]]
        truth_findings = {finding["truth_id"]: finding for finding in truth_case["findings"]}
        detected = update_aggregate(aggregate, result, scoring, truth_findings, truth_case["expected_verdict"])
        route_class = corpus_by_case[result["case_id"]]["change_profile"]["route_class"]
        route_aggregate = route_aggregates[route_class].setdefault(key, new_aggregate(result))
        update_aggregate(route_aggregate, result, scoring, truth_findings, truth_case["expected_verdict"])
        detection_sets[key][result["case_id"]].update(detected)
        false_positive_counts[key][result["case_id"]] += sum(
            disposition["disposition"] == "false_positive"
            for disposition in scoring["finding_dispositions"]
        )

    rendered_aggregates: dict[str, Any] = {}
    for key, aggregate in sorted(aggregates.items()):
        rendered_aggregates[key] = render_aggregate(aggregate)
    rendered_routes = {
        route_class: {key: render_aggregate(aggregate) for key, aggregate in sorted(configurations.items())}
        for route_class, configurations in sorted(route_aggregates.items())
    }

    pairwise: list[dict[str, Any]] = []
    keys = sorted(detection_sets)
    all_truth_by_case = {
        case_id: {finding["truth_id"] for finding in truth_case["findings"]}
        for case_id, truth_case in truth_by_case.items()
    }
    for index, left in enumerate(keys):
        for right in keys[index + 1:]:
            shared_cases = sorted(set(detection_sets[left]) & set(detection_sets[right]))
            left_unique = right_unique = joint = jointly_missed = truth_opportunities = 0
            left_false_positives = right_false_positives = 0
            for case_id in shared_cases:
                left_set = detection_sets[left][case_id]
                right_set = detection_sets[right][case_id]
                truth_ids = all_truth_by_case[case_id]
                truth_opportunities += len(truth_ids)
                left_unique += len(left_set - right_set)
                right_unique += len(right_set - left_set)
                joint += len(left_set & right_set)
                jointly_missed += len(truth_ids - (left_set | right_set))
                left_false_positives += false_positive_counts[left][case_id]
                right_false_positives += false_positive_counts[right][case_id]
            missed_by_left = right_unique + jointly_missed
            missed_by_right = left_unique + jointly_missed
            additional_right_false_positives = max(0, right_false_positives - left_false_positives)
            additional_left_false_positives = max(0, left_false_positives - right_false_positives)
            pairwise.append({
                "left": left,
                "right": right,
                "shared_cases": len(shared_cases),
                "truth_unique_to_left": left_unique,
                "truth_unique_to_right": right_unique,
                "truth_jointly_detected": joint,
                "truth_jointly_missed": jointly_missed,
                "truth_opportunities": truth_opportunities,
                "union_recall": ratio(truth_opportunities - jointly_missed, truth_opportunities),
                "joint_miss_rate": ratio(jointly_missed, truth_opportunities),
                "right_conditional_recall_given_left_miss": ratio(right_unique, missed_by_left),
                "left_conditional_recall_given_right_miss": ratio(left_unique, missed_by_right),
                "left_false_positive_findings": left_false_positives,
                "right_false_positive_findings": right_false_positives,
                "right_incremental_truth_per_additional_false_positive": ratio(right_unique, additional_right_false_positives),
                "left_incremental_truth_per_additional_false_positive": ratio(left_unique, additional_left_false_positives),
                "interpretation": "descriptive union across attempts; adjudicate before policy use",
            })

    return {
        "artifact_type": "review_bench_report_v1",
        "schema_version": SCHEMA_VERSION,
        "bench_id": corpus["bench_id"],
        "truth_artifact_type": truth["artifact_type"],
        "truth_adjudication_amendments": [
            amendment["amendment_id"] for amendment in truth.get("adjudication_amendments", [])
        ],
        "status": "descriptive_only",
        "configurations": rendered_aggregates,
        "by_route_class": rendered_routes,
        "pairwise_complementarity": pairwise,
        "limitations": [
            "Metrics are descriptive and do not establish statistical equivalence.",
            "Repeated attempts within a case are correlated.",
            "Pairwise detection uses the union across attempts for each configuration and case.",
            "Ground truth and finding correspondence depend on recorded adjudication.",
        ],
    }


def render_review_prompt(corpus: dict[str, Any], case: dict[str, Any]) -> str:
    protocol = corpus["protocol"]
    contract = case["contract"]
    lines = [
        f"# Review case: {case['title']}",
        "",
        protocol["prompt"].strip(),
        "",
        "## Exact case identity",
        "",
        f"- Bench: `{corpus['bench_id']}`",
        f"- Case: `{case['case_id']}`",
        f"- Snapshot: `{case['snapshot']['identity']}`",
        f"- Snapshot digest: `{case['snapshot']['digest']}`",
        f"- Review class: `{case['review_class']}`",
        f"- Route class: `{case['change_profile']['route_class']}`",
        f"- Semantic domains: `{', '.join(case['change_profile']['semantic_domains'])}`",
        f"- Topologies: `{', '.join(case['change_profile']['topologies'])}`",
        f"- Novelty: `{case['change_profile']['novelty']}`",
        f"- Evidence types: `{', '.join(case['change_profile']['evidence_types'])}`",
        f"- Consequences: `{', '.join(case['change_profile']['consequences'])}`",
        f"- Risk: `{case['risk']}`",
        "",
        "## Contract",
        "",
        contract["goal"],
        "",
        "Acceptance criteria:",
        "",
    ]
    lines.extend(f"- {criterion}" for criterion in contract["acceptance_criteria"])
    lines.extend(["", "Excluded scope:", ""])
    lines.extend(f"- {item}" for item in contract["excluded_scope"])
    lines.extend(["", "## Available artifacts", ""])
    lines.extend(f"- `{artifact['path']}` — {artifact['role']}" for artifact in case["artifacts"])
    lines.extend(["", "## Deterministic evidence", ""])
    lines.extend(f"- {item}" for item in case["deterministic_evidence"])
    if protocol["version"] == "1.2":
        lines.extend([
            "",
            "Return one `review_bench_result_v2` JSON object with `findings`, `verified_claims`, and `observations` arrays. Do not include ground-truth identifiers.",
            "Put only claimed defects or material risks in `findings`; each may carry severity and blocking status.",
            "Put acceptance-critical conclusions established by evidence in `verified_claims`; identify the supported acceptance criteria and do not attach severity or blocking status.",
            "Put real non-defect improvements or out-of-contract notes in `observations`; do not attach severity or blocking status.",
            "Finding fields are `finding_id`, `severity`, `blocking`, `category`, `claim`, `evidence`, and `confidence`.",
            "Verified-claim fields are `claim_id`, `claim`, `evidence`, `confidence`, and `acceptance_criteria`; quote supported criteria verbatim from this case.",
            "Observation fields are `observation_id`, `category`, `claim`, `evidence`, and `confidence`.",
            "Verified claims require high confidence. An accepted verdict must verify every listed acceptance criterion and contain no blocking finding; otherwise use the evidence-supported non-acceptance verdict.",
            "Do not duplicate one claim across arrays. Use only the configured verdict, severity, and confidence vocabularies.",
            "Every item must cite concrete evidence from the supplied snapshot.",
            "",
        ])
    else:
        lines.extend([
            "",
            "Return one `review_bench_result_v1` JSON object. Do not include ground-truth identifiers.",
            "Use only the configured verdict, severity, and confidence vocabularies.",
            "Every finding must cite concrete evidence from the supplied snapshot.",
            "",
        ])
    return "\n".join(lines)


def export_case(corpus: dict[str, Any], case_id: str, output_dir: Path, repository_root: Path | None, archive_git: bool) -> dict[str, Any]:
    case = next((item for item in corpus["cases"] if item["case_id"] == case_id), None)
    require(case is not None, f"case '{case_id}' is not in corpus")
    output_dir.mkdir(parents=True, exist_ok=True)
    emit({
        "artifact_type": "review_bench_case_packet_v1",
        "schema_version": SCHEMA_VERSION,
        "bench_id": corpus["bench_id"],
        "protocol": corpus["protocol"],
        "case": case,
    }, output_dir / "case.json")
    (output_dir / "review-prompt.md").write_text(render_review_prompt(corpus, case), encoding="utf-8")
    archive_path = None
    if archive_git:
        require(repository_root is not None, "--archive-git requires --repository-root")
        archive_path = output_dir / "snapshot.tar.gz"
        completed = subprocess.run(
            ["git", "-C", str(repository_root), "archive", "--format=tar.gz", "--output", str(archive_path), case["snapshot"]["identity"]],
            capture_output=True,
            text=True,
        )
        require(completed.returncode == 0, f"git archive failed: {completed.stderr.strip()}")
        require(sha256_file(archive_path) == case["snapshot"]["digest"],
                "exported archive digest does not match the case snapshot")
    receipt = {
        "status": "exported",
        "bench_id": corpus["bench_id"],
        "case_id": case_id,
        "packet": str((output_dir / "case.json").resolve()),
        "prompt": str((output_dir / "review-prompt.md").resolve()),
        "archive": str(archive_path.resolve()) if archive_path else None,
    }
    emit(receipt, output_dir / "export-receipt.json")
    return receipt


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    try:
        with path.open(encoding="utf-8") as handle:
            for line_number, line in enumerate(handle, 1):
                if not line.strip():
                    continue
                value = json.loads(line)
                require(isinstance(value, dict), f"{path}:{line_number} must contain an object")
                rows.append(value)
    except (OSError, json.JSONDecodeError) as error:
        raise BenchError(f"cannot read {path}: {error}") from error
    return rows


def session_candidates(session_root: Path, run_ids: set[str]) -> dict[str, list[dict[str, str]]]:
    matches: dict[str, list[dict[str, str]]] = defaultdict(list)
    for path in session_root.rglob("*.jsonl"):
        try:
            content = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        agent_path = ""
        for line in content.splitlines():
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("type") != "session_meta":
                continue
            source = event.get("payload", {}).get("source")
            if isinstance(source, dict):
                agent_path = (
                    source.get("subagent", {})
                    .get("thread_spawn", {})
                    .get("agent_path", "")
                )
            break
        for run_id in run_ids:
            run_prefix = run_id.split("-", 1)[0]
            if run_id in content or (agent_path and run_prefix in agent_path):
                matches[run_id].append({"path": str(path.resolve()), "agent_path": agent_path})
    return matches


def summarize_findings(value: Any) -> bool:
    if not isinstance(value, dict):
        return False
    for nested in value.values():
        if isinstance(nested, bool):
            continue
        if isinstance(nested, (int, float)) and nested > 0:
            return True
        if isinstance(nested, dict) and summarize_findings(nested):
            return True
    return False


def inventory_history(metric_paths: list[Path], session_root: Path | None) -> dict[str, Any]:
    rows = [row for path in metric_paths for row in read_jsonl(path)]
    run_ids = {str(row.get("run_id")) for row in rows if row.get("run_id")}
    sessions = session_candidates(session_root, run_ids) if session_root else {}
    candidates: list[dict[str, Any]] = []
    for row in rows:
        worker = row.get("worker_metrics") if isinstance(row.get("worker_metrics"), dict) else {}
        review_calls = worker.get("review_gate_calls", 0)
        fix_iterations = worker.get("review_fix_iterations", row.get("review_fix_iterations", 0))
        findings = row.get("review_findings")
        owned_files = row.get("task_owned_files") or worker.get("task_owned_files") or []
        workspace = worker.get("additional_metrics", {}).get("workspace", {}) if isinstance(worker.get("additional_metrics"), dict) else {}
        snapshot_head = workspace.get("head") if isinstance(workspace, dict) else None
        run_session_records = sessions.get(str(row.get("run_id")), [])
        slice_number = row.get("slice_number")
        slice_pattern = re.compile(rf"(?:^|_)slice{re.escape(str(slice_number))}(?:_|$)")
        exact_sessions = [
            item["path"] for item in run_session_records
            if item["agent_path"] and slice_pattern.search(item["agent_path"])
        ]
        run_sessions = exact_sessions or [item["path"] for item in run_session_records]
        session_match_quality = "run_and_slice" if exact_sessions else ("run_only" if run_sessions else "none")
        is_candidate = bool(review_calls or fix_iterations or summarize_findings(findings))
        if not is_candidate:
            continue
        if snapshot_head and owned_files:
            reconstruction = "partial"
            missing = ["immutable dirty-worktree tree or checkpoint"]
        elif run_sessions and owned_files:
            reconstruction = "forensic_only"
            missing = ["immutable snapshot identity", "verified patch reconstruction"]
        else:
            reconstruction = "insufficient"
            missing = ["immutable snapshot identity", "complete task-owned manifest"]
        candidates.append({
            "run_id": row.get("run_id"),
            "slice_number": row.get("slice_number"),
            "slice_title": row.get("slice_title"),
            "status": row.get("status"),
            "review_gate_calls": review_calls or 0,
            "review_fix_iterations": fix_iterations or 0,
            "has_finding_summary": findings is not None,
            "has_nonzero_findings": summarize_findings(findings),
            "task_owned_files": owned_files,
            "snapshot_head": snapshot_head,
            "codex_session_candidates": run_sessions,
            "session_match_quality": session_match_quality,
            "reconstruction_state": reconstruction,
            "missing_for_exact_case": missing,
        })
    counts = defaultdict(int)
    for candidate in candidates:
        counts[candidate["reconstruction_state"]] += 1
    return {
        "artifact_type": "review_bench_inventory_v1",
        "schema_version": SCHEMA_VERSION,
        "status": "provisional_inventory",
        "source_receipts": len(rows),
        "candidate_cases": len(candidates),
        "reconstruction_states": dict(sorted(counts.items())),
        "candidates": candidates,
        "limitations": [
            "Provider attempt counts may combine placement and implementation-review activity in historical receipts.",
            "A Git HEAD does not identify uncommitted reviewed content.",
            "Session matches are candidates and require prompt/result extraction before use.",
        ],
    }


def load_results(directory: Path, corpus: dict[str, Any]) -> list[dict[str, Any]]:
    paths = sorted(directory.glob("*.json"))
    require(bool(paths), f"no JSON results found in {directory}")
    results = [validate_result(load(path), corpus) for path in paths]
    unique((result["result_id"] for result in results), "results")
    return results


def load_scorings(directory: Path, corpus: dict[str, Any], truth: dict[str, Any], results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    result_by_id = {result["result_id"]: result for result in results}
    paths = sorted(directory.glob("*.json"))
    require(bool(paths), f"no JSON scoring artifacts found in {directory}")
    scorings: list[dict[str, Any]] = []
    for path in paths:
        value = load(path)
        result_id = value.get("result_id") if isinstance(value, dict) else None
        require(result_id in result_by_id, f"{path} names unknown result_id")
        scorings.append(validate_scoring(value, corpus, truth, result_by_id[result_id]))
    unique((scoring["result_id"] for scoring in scorings), "scorings")
    return scorings


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)

    validate_command = commands.add_parser("validate", help="validate corpus and optional result artifacts")
    validate_command.add_argument("--corpus", type=Path, required=True)
    validate_command.add_argument("--truth", type=Path)
    validate_command.add_argument("--result", type=Path)
    validate_command.add_argument("--scoring", type=Path)

    compare_command = commands.add_parser("compare", help="compare adjudicated reviewer results")
    compare_command.add_argument("--corpus", type=Path, required=True)
    compare_command.add_argument("--truth", type=Path, required=True)
    compare_command.add_argument("--results-dir", type=Path, required=True)
    compare_command.add_argument("--scoring-dir", type=Path, required=True)
    compare_command.add_argument("--output", type=Path)

    export_command = commands.add_parser("export-case", help="export a blinded manual-review packet")
    export_command.add_argument("--corpus", type=Path, required=True)
    export_command.add_argument("--case-id", required=True)
    export_command.add_argument("--output-dir", type=Path, required=True)
    export_command.add_argument("--repository-root", type=Path)
    export_command.add_argument("--archive-git", action="store_true")

    inventory_command = commands.add_parser("inventory-history", help="inventory historical receipt and session candidates")
    inventory_command.add_argument("--metrics", type=Path, nargs="+", required=True)
    inventory_command.add_argument("--codex-sessions", type=Path)
    inventory_command.add_argument("--output", type=Path)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "inventory-history":
            emit(inventory_history(args.metrics, args.codex_sessions), args.output)
            return 0

        corpus = validate_corpus(load(args.corpus))
        if args.command == "export-case":
            emit(export_case(corpus, args.case_id, args.output_dir, args.repository_root, args.archive_git))
            return 0

        truth_path = getattr(args, "truth", None)
        result_path = getattr(args, "result", None)
        scoring_path = getattr(args, "scoring", None)
        truth = validate_truth(load(truth_path), corpus, truth_path) if truth_path else None
        review_result = validate_result(load(result_path), corpus) if result_path else None
        if scoring_path:
            require(truth is not None and review_result is not None, "--scoring requires --truth and --result")
            validate_scoring(load(scoring_path), corpus, truth, review_result)
        if args.command == "validate":
            emit({"status": "valid", "bench_id": corpus["bench_id"]})
            return 0
        require(truth is not None, "compare requires truth")
        results = load_results(args.results_dir, corpus)
        scorings = load_scorings(args.scoring_dir, corpus, truth, results)
        emit(compare(corpus, truth, results, scorings), args.output)
        return 0
    except BenchError as error:
        print(f"review-bench: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
