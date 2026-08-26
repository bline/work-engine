#!/usr/bin/env python3
"""Validate and evaluate linguistic-register profile-separability artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path
from typing import Any

import yaml


SCHEMA_VERSION = 1
ROLE_STATUSES = {"awaiting_human_acceptance", "frozen"}
BEHAVIOR_STATUSES = {"encoded", "unencoded", "ambiguous"}
GENRES = {"formal", "responsive_or_informal"}
RIGHTS_STATUSES = {"open_license", "permission", "public_readable_reuse_unconfirmed"}
LAYERS = {"surface", "discourse"}
DISPOSITIONS = {"realization_only", "semantic_duplicate", "semantic_addition", "uncertain"}


class PilotError(ValueError):
    """Raised when an artifact would weaken pilot compatibility or evidence."""


class UniqueKeyLoader(yaml.SafeLoader):
    pass


def _construct_mapping(loader: UniqueKeyLoader, node: yaml.MappingNode, deep: bool = False) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in result:
            raise PilotError(f"duplicate YAML key: {key}")
        result[key] = loader.construct_object(value_node, deep=deep)
    return result


UniqueKeyLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _construct_mapping)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PilotError(message)


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


def identifier(value: Any, path: str) -> str:
    result = text(value, path)
    require(result.replace("-", "").replace("_", "").isalnum(),
            f"{path} must contain only letters, digits, hyphens, and underscores")
    return result


def sha256(value: Any, path: str) -> str:
    result = text(value, path)
    require(len(result) == 64 and all(character in "0123456789abcdef" for character in result),
            f"{path} must be lowercase sha256")
    return result


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def finite_nonnegative(value: Any, path: str) -> float:
    require(isinstance(value, (int, float)) and not isinstance(value, bool), f"{path} must be numeric")
    result = float(value)
    require(math.isfinite(result) and result >= 0, f"{path} must be finite and nonnegative")
    return result


def positive_integer(value: Any, path: str) -> int:
    require(isinstance(value, int) and not isinstance(value, bool) and value > 0,
            f"{path} must be a positive integer")
    return value


def load_yaml(path: Path) -> dict[str, Any]:
    try:
        source = path.read_text(encoding="utf-8")
        for token in yaml.scan(source):
            if isinstance(token, (yaml.tokens.AnchorToken, yaml.tokens.AliasToken)):
                raise PilotError(f"YAML anchors and aliases are not allowed: {path}")
        return obj(yaml.load(source, Loader=UniqueKeyLoader), str(path))
    except (OSError, yaml.YAMLError) as error:
        raise PilotError(f"cannot load {path}: {error}") from error


def artifact_digest(path: Path) -> str:
    return sha256_file(path)


def validate_role(value: Any) -> dict[str, Any]:
    role = obj(value, "role")
    exact_keys(role, {
        "artifact_type", "schema_version", "status", "role_id", "source",
        "semantic_units", "behavior_classifications", "classification_owner",
        "limitations",
    }, set(), "role")
    require(role["artifact_type"] == "linguistic_register_role_v1", "role.artifact_type is incompatible")
    require(role["schema_version"] == SCHEMA_VERSION, f"role.schema_version must be {SCHEMA_VERSION}")
    require(role["status"] in ROLE_STATUSES, "role.status is invalid")
    identifier(role["role_id"], "role.role_id")
    source = obj(role["source"], "role.source")
    exact_keys(source, {"repository", "commit", "files"}, set(), "role.source")
    for key in ("repository", "commit"):
        text(source[key], f"role.source.{key}")
    files = array(source["files"], "role.source.files")
    require(bool(files), "role.source.files must not be empty")
    source_paths: set[str] = set()
    for index, file_value in enumerate(files):
        path = f"role.source.files[{index}]"
        source_file = obj(file_value, path)
        exact_keys(source_file, {"path", "sha256", "role"}, set(), path)
        source_path = text(source_file["path"], f"{path}.path")
        pure_path = Path(source_path)
        require(not pure_path.is_absolute() and ".." not in pure_path.parts,
                f"{path}.path must stay within the recorded repository")
        require(source_path not in source_paths, f"duplicate role source path: {source_path}")
        source_paths.add(source_path)
        sha256(source_file["sha256"], f"{path}.sha256")
        text(source_file["role"], f"{path}.role")

    units = array(role["semantic_units"], "role.semantic_units")
    require(bool(units), "role.semantic_units must not be empty")
    unit_ids: set[str] = set()
    for index, unit_value in enumerate(units):
        path = f"role.semantic_units[{index}]"
        unit = obj(unit_value, path)
        exact_keys(unit, {"id", "meaning", "source_locator"}, set(), path)
        unit_id = identifier(unit["id"], f"{path}.id")
        require(unit_id not in unit_ids, f"duplicate semantic unit id: {unit_id}")
        unit_ids.add(unit_id)
        text(unit["meaning"], f"{path}.meaning")
        text(unit["source_locator"], f"{path}.source_locator")

    behaviors = array(role["behavior_classifications"], "role.behavior_classifications")
    require(bool(behaviors), "role.behavior_classifications must not be empty")
    behavior_ids: set[str] = set()
    for index, behavior_value in enumerate(behaviors):
        path = f"role.behavior_classifications[{index}]"
        behavior = obj(behavior_value, path)
        exact_keys(behavior, {"id", "status", "semantic_unit_ids", "rationale"}, set(), path)
        behavior_id = identifier(behavior["id"], f"{path}.id")
        require(behavior_id not in behavior_ids, f"duplicate behavior id: {behavior_id}")
        behavior_ids.add(behavior_id)
        require(behavior["status"] in BEHAVIOR_STATUSES, f"{path}.status is invalid")
        references = array(behavior["semantic_unit_ids"], f"{path}.semantic_unit_ids")
        require(all(isinstance(item, str) for item in references), f"{path}.semantic_unit_ids must contain strings")
        require(len(references) == len(set(references)), f"{path}.semantic_unit_ids contains duplicates")
        require(set(references) <= unit_ids, f"{path}.semantic_unit_ids contains unknown units")
        require(behavior["status"] != "encoded" or bool(references),
                f"{path}.semantic_unit_ids must identify evidence for encoded behavior")
        text(behavior["rationale"], f"{path}.rationale")
    text(role["classification_owner"], "role.classification_owner")
    limitations = array(role["limitations"], "role.limitations")
    for index, limitation in enumerate(limitations):
        text(limitation, f"role.limitations[{index}]")
    return role


def verify_role_source(role: dict[str, Any]) -> None:
    source = role["source"]
    repository = Path(source["repository"])
    require(repository.is_absolute(), "role.source.repository must be an absolute local path")
    for source_file in source["files"]:
        source_path = repository / source_file["path"]
        require(source_path.is_file(), f"role source is not a readable file: {source_path}")
        require(sha256_file(source_path) == source_file["sha256"],
                f"role source sha256 does not match recorded bytes: {source_file['path']}")
        try:
            committed = subprocess.run(
                ["git", "-C", str(repository), "show", f"{source['commit']}:{source_file['path']}"],
                check=False, capture_output=True, timeout=10,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise PilotError(f"cannot read committed role source: {error}") from error
        require(committed.returncode == 0,
                f"role source is not present at recorded commit: {source_file['path']}")
        committed_digest = hashlib.sha256(committed.stdout).hexdigest()
        require(committed_digest == source_file["sha256"],
                f"role source sha256 does not match recorded commit: {source_file['path']}")


def validate_corpus(value: Any) -> dict[str, Any]:
    corpus = obj(value, "corpus")
    exact_keys(corpus, {
        "artifact_type", "schema_version", "candidate_id", "target_practice",
        "selection_basis", "sources", "limitations",
    }, set(), "corpus")
    require(corpus["artifact_type"] == "linguistic_register_mini_corpus_v1",
            "corpus.artifact_type is incompatible")
    require(corpus["schema_version"] == SCHEMA_VERSION, f"corpus.schema_version must be {SCHEMA_VERSION}")
    identifier(corpus["candidate_id"], "corpus.candidate_id")
    text(corpus["target_practice"], "corpus.target_practice")
    text(corpus["selection_basis"], "corpus.selection_basis")
    sources = array(corpus["sources"], "corpus.sources")
    require(len(sources) == 2, "corpus.sources must contain exactly two works for the separability slice")
    source_ids: set[str] = set()
    genres: set[str] = set()
    for index, source_value in enumerate(sources):
        path = f"corpus.sources[{index}]"
        source = obj(source_value, path)
        exact_keys(source, {
            "id", "title", "authors", "authorship_basis", "genre", "public_url",
            "access_basis", "rights_status", "content_sha256",
        }, set(), path)
        source_id = identifier(source["id"], f"{path}.id")
        require(source_id not in source_ids, f"duplicate corpus source id: {source_id}")
        source_ids.add(source_id)
        text(source["title"], f"{path}.title")
        authors = array(source["authors"], f"{path}.authors")
        require(bool(authors), f"{path}.authors must not be empty")
        for author_index, author in enumerate(authors):
            text(author, f"{path}.authors[{author_index}]")
        text(source["authorship_basis"], f"{path}.authorship_basis")
        require(source["genre"] in GENRES, f"{path}.genre is invalid")
        genres.add(source["genre"])
        url = text(source["public_url"], f"{path}.public_url")
        require(url.startswith("https://"), f"{path}.public_url must use https")
        text(source["access_basis"], f"{path}.access_basis")
        require(source["rights_status"] in RIGHTS_STATUSES, f"{path}.rights_status is invalid")
        sha256(source["content_sha256"], f"{path}.content_sha256")
    require(genres == GENRES, "corpus.sources must include one formal and one responsive_or_informal work")
    for index, limitation in enumerate(array(corpus["limitations"], "corpus.limitations")):
        text(limitation, f"corpus.limitations[{index}]")
    return corpus


def validate_profile(value: Any, role: dict[str, Any], corpus: dict[str, Any], role_digest: str,
                     corpus_digest: str) -> dict[str, Any]:
    profile = obj(value, "profile")
    exact_keys(profile, {
        "artifact_type", "schema_version", "candidate_id", "role_id", "role_artifact_sha256",
        "corpus_artifact_sha256", "features", "content_screening", "thresholds",
        "judgment_provenance", "limitations",
    }, set(), "profile")
    require(profile["artifact_type"] == "linguistic_register_profile_v1",
            "profile.artifact_type is incompatible")
    require(profile["schema_version"] == SCHEMA_VERSION, f"profile.schema_version must be {SCHEMA_VERSION}")
    require(profile["candidate_id"] == corpus["candidate_id"], "profile.candidate_id does not match corpus")
    require(profile["role_id"] == role["role_id"], "profile.role_id does not match role")
    require(sha256(profile["role_artifact_sha256"], "profile.role_artifact_sha256") == role_digest,
            "profile.role_artifact_sha256 does not match role artifact")
    require(sha256(profile["corpus_artifact_sha256"], "profile.corpus_artifact_sha256") == corpus_digest,
            "profile.corpus_artifact_sha256 does not match corpus artifact")

    source_genres = {source["id"]: source["genre"] for source in corpus["sources"]}
    features = array(profile["features"], "profile.features")
    require(bool(features), "profile.features must not be empty")
    feature_ids: set[str] = set()
    for index, feature_value in enumerate(features):
        path = f"profile.features[{index}]"
        feature = obj(feature_value, path)
        exact_keys(feature, {
            "id", "layer", "category", "description", "distinctiveness_weight", "evidence",
            "cross_genre", "disposition", "disposition_rationale",
        }, set(), path)
        feature_id = identifier(feature["id"], f"{path}.id")
        require(feature_id not in feature_ids, f"duplicate feature id: {feature_id}")
        feature_ids.add(feature_id)
        require(feature["layer"] in LAYERS, f"{path}.layer is invalid")
        identifier(feature["category"], f"{path}.category")
        text(feature["description"], f"{path}.description")
        finite_nonnegative(feature["distinctiveness_weight"], f"{path}.distinctiveness_weight")
        evidence = array(feature["evidence"], f"{path}.evidence")
        require(bool(evidence), f"{path}.evidence must not be empty")
        evidence_sources: set[str] = set()
        for evidence_index, evidence_value in enumerate(evidence):
            evidence_path = f"{path}.evidence[{evidence_index}]"
            item = obj(evidence_value, evidence_path)
            exact_keys(item, {"source_id", "locator", "observation"}, set(), evidence_path)
            source_id = text(item["source_id"], f"{evidence_path}.source_id")
            require(source_id in source_genres, f"{evidence_path}.source_id is unknown")
            evidence_sources.add(source_id)
            text(item["locator"], f"{evidence_path}.locator")
            text(item["observation"], f"{evidence_path}.observation")
        require(isinstance(feature["cross_genre"], bool), f"{path}.cross_genre must be boolean")
        observed_genres = {source_genres[source_id] for source_id in evidence_sources}
        require(not feature["cross_genre"] or observed_genres == GENRES,
                f"{path}.cross_genre requires evidence from both genres")
        require(feature["disposition"] in DISPOSITIONS, f"{path}.disposition is invalid")
        text(feature["disposition_rationale"], f"{path}.disposition_rationale")

    screening = obj(profile["content_screening"], "profile.content_screening")
    exact_keys(screening, {
        "names_removed", "copied_language_removed", "domain_terms_removed", "named_methods_removed",
        "reviewer", "rationale",
    }, set(), "profile.content_screening")
    for key in ("names_removed", "copied_language_removed", "domain_terms_removed", "named_methods_removed"):
        require(screening[key] is True, f"profile.content_screening.{key} must be true")
    text(screening["reviewer"], "profile.content_screening.reviewer")
    text(screening["rationale"], "profile.content_screening.rationale")

    thresholds = obj(profile["thresholds"], "profile.thresholds")
    exact_keys(thresholds, {
        "minimum_retained_features", "minimum_weighted_retention", "minimum_cross_genre_retained_features",
    }, set(), "profile.thresholds")
    positive_integer(thresholds["minimum_retained_features"], "profile.thresholds.minimum_retained_features")
    weighted = finite_nonnegative(thresholds["minimum_weighted_retention"],
                                  "profile.thresholds.minimum_weighted_retention")
    require(weighted <= 1, "profile.thresholds.minimum_weighted_retention must be at most 1")
    positive_integer(thresholds["minimum_cross_genre_retained_features"],
                     "profile.thresholds.minimum_cross_genre_retained_features")
    provenance = obj(profile["judgment_provenance"], "profile.judgment_provenance")
    exact_keys(provenance, {
        "extractor", "semantic_classifier", "weighting_basis",
        "weights_assigned_before_semantic_classification", "performed_before_outcomes",
    },
               set(), "profile.judgment_provenance")
    for key in ("extractor", "semantic_classifier", "weighting_basis"):
        text(provenance[key], f"profile.judgment_provenance.{key}")
    require(provenance["weights_assigned_before_semantic_classification"] is True,
            "profile.judgment_provenance.weights_assigned_before_semantic_classification must be true")
    require(provenance["performed_before_outcomes"] is True,
            "profile.judgment_provenance.performed_before_outcomes must be true")
    for index, limitation in enumerate(array(profile["limitations"], "profile.limitations")):
        text(limitation, f"profile.limitations[{index}]")
    return profile


def evaluate(role: dict[str, Any], corpus: dict[str, Any], profile: dict[str, Any], role_digest: str,
             corpus_digest: str, profile_digest: str) -> dict[str, Any]:
    require(role["status"] == "frozen", "role.status must be frozen before evaluation")
    features = profile["features"]
    counts = {disposition: 0 for disposition in sorted(DISPOSITIONS)}
    weights = {disposition: 0.0 for disposition in sorted(DISPOSITIONS)}
    for feature in features:
        disposition = feature["disposition"]
        counts[disposition] += 1
        weights[disposition] += float(feature["distinctiveness_weight"])
    total_weight = sum(weights.values())
    retained_count = counts["realization_only"]
    retained_weight = weights["realization_only"]
    weighted_retention = retained_weight / total_weight if total_weight else 0.0
    cross_genre_retained = sum(
        1 for feature in features
        if feature["disposition"] == "realization_only" and feature["cross_genre"]
    )
    thresholds = profile["thresholds"]
    checks = {
        "retained_feature_count": retained_count >= thresholds["minimum_retained_features"],
        "weighted_retention": weighted_retention >= thresholds["minimum_weighted_retention"],
        "cross_genre_retained_features": (
            cross_genre_retained >= thresholds["minimum_cross_genre_retained_features"]
        ),
    }
    if retained_count == 0:
        disposition = "candidate_not_separable"
    elif all(checks.values()):
        disposition = "candidate_viable"
    else:
        disposition = "candidate_attenuated"
    return {
        "artifact_type": "linguistic_register_separability_report_v1",
        "schema_version": SCHEMA_VERSION,
        "candidate_id": corpus["candidate_id"],
        "role_id": role["role_id"],
        "bindings": {
            "role_artifact_sha256": role_digest,
            "corpus_artifact_sha256": corpus_digest,
            "profile_artifact_sha256": profile_digest,
        },
        "metrics": {
            "extracted_feature_count": len(features),
            "counts_by_disposition": counts,
            "weights_by_disposition": {key: round(value, 10) for key, value in weights.items()},
            "total_distinctiveness_weight": round(total_weight, 10),
            "retained_feature_count": retained_count,
            "weighted_retention": round(weighted_retention, 10),
            "cross_genre_retained_feature_count": cross_genre_retained,
        },
        "thresholds": thresholds,
        "checks": checks,
        "disposition": disposition,
        "authority": "mechanical gate state only; human acceptance is required before rendering",
    }


def emit(value: Any, output: Path | None = None) -> None:
    rendered = json.dumps(value, indent=2, sort_keys=True) + "\n"
    if output is None:
        sys.stdout.write(rendered)
        return
    require(not output.exists(), f"refusing to overwrite existing output: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(rendered, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("validate-role", "validate-corpus"):
        subparser = subparsers.add_parser(command)
        subparser.add_argument("artifact", type=Path)
    profile_parser = subparsers.add_parser("validate-profile")
    profile_parser.add_argument("--role", required=True, type=Path)
    profile_parser.add_argument("--corpus", required=True, type=Path)
    profile_parser.add_argument("profile", type=Path)
    evaluate_parser = subparsers.add_parser("evaluate")
    evaluate_parser.add_argument("--role", required=True, type=Path)
    evaluate_parser.add_argument("--corpus", required=True, type=Path)
    evaluate_parser.add_argument("--profile", required=True, type=Path)
    evaluate_parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()

    if arguments.command == "validate-role":
        role = validate_role(load_yaml(arguments.artifact))
        verify_role_source(role)
        emit({"status": "valid", "artifact_type": "linguistic_register_role_v1"})
    elif arguments.command == "validate-corpus":
        validate_corpus(load_yaml(arguments.artifact))
        emit({"status": "valid", "artifact_type": "linguistic_register_mini_corpus_v1"})
    else:
        role = validate_role(load_yaml(arguments.role))
        verify_role_source(role)
        corpus = validate_corpus(load_yaml(arguments.corpus))
        profile = validate_profile(
            load_yaml(arguments.profile), role, corpus,
            artifact_digest(arguments.role), artifact_digest(arguments.corpus),
        )
        if arguments.command == "validate-profile":
            emit({"status": "valid", "artifact_type": "linguistic_register_profile_v1"})
        else:
            emit(evaluate(
                role, corpus, profile, artifact_digest(arguments.role),
                artifact_digest(arguments.corpus), artifact_digest(arguments.profile),
            ), arguments.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except PilotError as error:
        sys.stderr.write(f"error: {error}\n")
        raise SystemExit(2)
