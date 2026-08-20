#!/usr/bin/env python3
"""Assemble authoritative builder telemetry into one semantic audit receipt."""

from __future__ import annotations

import argparse
import copy
import importlib.util
import json
from pathlib import Path
from typing import Any

import jsonschema


APPEND_PATH = Path(__file__).with_name("append_metrics.py")
APPEND_SPEC = importlib.util.spec_from_file_location(
    "work_engine_append_metrics", APPEND_PATH
)
if not APPEND_SPEC or not APPEND_SPEC.loader:
    raise RuntimeError(f"cannot load receipt validator: {APPEND_PATH}")
APPEND_METRICS = importlib.util.module_from_spec(APPEND_SPEC)
APPEND_SPEC.loader.exec_module(APPEND_METRICS)
INGRESS_SCHEMA_PATH = Path(__file__).parents[1] / "references" / "telemetry-ingress.schema.json"
with INGRESS_SCHEMA_PATH.open(encoding="utf-8") as ingress_schema_source:
    INGRESS_SCHEMA = json.load(ingress_schema_source)
jsonschema.Draft202012Validator.check_schema(INGRESS_SCHEMA)
INGRESS_VALIDATOR = jsonschema.Draft202012Validator(INGRESS_SCHEMA)


class ReceiptAssemblyError(ValueError):
    """An input cannot establish one valid authoritative receipt."""


RUNTIME_FIELD_MAP = {
    "input_tokens": "builder_input_tokens",
    "output_tokens": "builder_output_tokens",
    "peak_context_tokens": "builder_context_usage",
    "turn_count": "builder_turn_count",
    "wall_clock_seconds": "builder_wall_clock_seconds",
}


def fail(message: str) -> None:
    raise ReceiptAssemblyError(message)


def validate_ingress(value: Any) -> dict[str, Any]:
    errors = sorted(INGRESS_VALIDATOR.iter_errors(value), key=lambda error: list(error.path))
    if errors:
        error = errors[0]
        location = ".".join(str(part) for part in error.absolute_path)
        prefix = f"telemetry_ingress.{location}" if location else "telemetry_ingress"
        fail(f"{prefix}: {error.message}")
    return value


def validate_preflight(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail("campaign_preflight must be an object")
    expected = {"campaignPath", "engineConfig", "campaignSource", "resolvedCapabilities"}
    if set(value) != expected:
        fail("campaign_preflight fields must exactly match preflight output")
    if not isinstance(value["engineConfig"], dict):
        fail("campaign_preflight.engineConfig must be an object")
    if not isinstance(value["resolvedCapabilities"], dict):
        fail("campaign_preflight.resolvedCapabilities must be an object")
    source = value["campaignSource"]
    source_fields = {
        "source", "authoredReference", "resolvedPath", "pathBase", "sha256", "schemaVersion"
    }
    if not isinstance(source, dict) or set(source) != source_fields:
        fail("campaign_preflight.campaignSource fields are invalid")
    for key in source_fields - {"schemaVersion"}:
        if not isinstance(source[key], str) or not source[key]:
            fail(f"campaign_preflight.campaignSource.{key} must be a nonempty string")
    if source["source"] != "file" or source["schemaVersion"] != 1:
        fail("campaign_preflight.campaignSource identity is invalid")
    digest = source["sha256"]
    if len(digest) != 64 or any(character not in "0123456789abcdef" for character in digest):
        fail("campaign_preflight.campaignSource.sha256 must be lowercase SHA-256")
    if value["campaignPath"] != source["resolvedPath"]:
        fail("campaign_preflight campaignPath does not match campaignSource.resolvedPath")
    if value["engineConfig"].get("source") != source["authoredReference"]:
        fail("campaign_preflight engineConfig.source does not match campaignSource")
    return value


def assemble(
    semantic_receipt: Any, telemetry_ingress: Any, campaign_preflight: Any
) -> dict[str, Any]:
    try:
        APPEND_METRICS.validate(semantic_receipt)
    except ValueError as error:
        raise ReceiptAssemblyError(f"semantic receipt is invalid: {error}") from error
    if semantic_receipt["schema_version"] != 4:
        fail("semantic receipt schema_version must be 4")
    ingress = validate_ingress(telemetry_ingress)
    preflight = validate_preflight(campaign_preflight)
    if ingress["run_id"] != semantic_receipt["run_id"]:
        fail("telemetry ingress run_id does not match semantic receipt")
    if ingress["slice_id"] != str(semantic_receipt["slice_number"]):
        fail("telemetry ingress slice_id does not match semantic receipt slice_number")
    if "telemetry_ingress" in semantic_receipt["producer_metrics"]:
        fail("semantic receipt already contains authoritative telemetry_ingress")
    if "campaign_source" in semantic_receipt["producer_metrics"]:
        fail("semantic receipt already contains authoritative campaign_source")

    assembled = copy.deepcopy(semantic_receipt)
    assembled["engine_config"] = copy.deepcopy(preflight["engineConfig"])
    runtime = ingress["builder_runtime"]
    measurements = runtime["measurements"]
    for ingress_name, receipt_name in RUNTIME_FIELD_MAP.items():
        assembled["worker_metrics"][receipt_name] = measurements[ingress_name]["value"]

    assembled["producer_metrics"]["telemetry_ingress"] = {
        "schema_version": ingress["schema_version"],
        "source_kind": runtime["source_kind"],
        "source_identity": copy.deepcopy(runtime["source_identity"]),
        "artifacts": copy.deepcopy(runtime["artifacts"]),
        "binding_provenance": copy.deepcopy(runtime["binding_provenance"]),
        "measurements": copy.deepcopy(measurements),
    }
    assembled["producer_metrics"]["campaign_source"] = copy.deepcopy(
        preflight["campaignSource"]
    )
    try:
        return APPEND_METRICS.validate(assembled)
    except ValueError as error:
        raise ReceiptAssemblyError(f"assembled receipt is invalid: {error}") from error


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--semantic-receipt-json", required=True)
    parser.add_argument("--telemetry-ingress-json", required=True)
    parser.add_argument("--campaign-preflight-json", required=True)
    args = parser.parse_args()
    try:
        semantic = json.loads(args.semantic_receipt_json)
        telemetry = json.loads(args.telemetry_ingress_json)
        preflight = json.loads(args.campaign_preflight_json)
        result = assemble(semantic, telemetry, preflight)
    except (json.JSONDecodeError, ReceiptAssemblyError) as error:
        parser.error(str(error))
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))


if __name__ == "__main__":
    main()
