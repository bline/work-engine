---
name: chrome-vision
description: Observe and interact with Chrome through its DevTools Protocol using bounded, provenance-bearing evidence packets. Use for inspecting ordinary web pages, taking screenshots, performing targeted interactions, diagnosing Chrome target/session/document lifecycle, or optionally invoking Chrome extension capabilities without rewriting ad hoc CDP clients.
---

# Chrome Vision

Use the repository broker at `scripts/chrome-vision.mjs`. Prefer the least expensive operation that can establish the needed fact.

1. Start with `chrome.health` or `chrome.targets.list`.
2. Select a target with declarative aliases in config; keep application-specific aliases outside this skill.
3. Use `chrome.document.observe` for bounded document facts and `chrome.page.captureScreenshot` only when pixels matter.
4. Use `chrome.dom.click` and `chrome.dom.fill` for ordinary interaction. Record what the operation observed; make interpretation separately.
5. Use `chrome.extensions.openSidePanel` only when extension behavior is actually in scope.
6. Use `cdp.call` as the explicit escape hatch for unsupported CDP mechanisms.

Treat every packet as evidence, not a conclusion. Read `limitations`, `provenance`, and epoch fields before relying on it. Never reuse object, node, execution-context, or document-scoped handles after an epoch changes. If health reports a disconnect or reattach, reacquire observations.

The broker speaks newline-delimited JSON on stdin/stdout. Its CLI accepts YAML or JSON configuration files, validates the parsed value against the versioned JSON Schema in `schemas/`, and resolves `artifactDirectory` relative to the configuration file before broker startup. The authoring syntax does not change the NDJSON transport. Artifact payloads, including oversized `cdp.call` results, are written beneath the configured artifact directory; packets contain paths, hashes, sizes, and truncation state rather than unbounded binary data.

Project recovery commands, product selectors, target meanings, and workflow judgment belong in project adapters. Do not add them to generic config or operations.
