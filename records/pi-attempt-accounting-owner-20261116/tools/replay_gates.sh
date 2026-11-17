#!/usr/bin/env bash
# Replay the predecessor's nine accepted gates against SUBJECT with TARGET cargo dir.
# Usage: replay_gates.sh SUBJECT TARGET OUTDIR
set -u
S=$(realpath "$1"); T=$(realpath "$2"); O=$(realpath -m "$3"); mkdir -p "$O/tmp"
(cd "$S/tooling/spec-check" && CARGO_TARGET_DIR="$T" cargo test --no-run --offline >"$O/build.log" 2>&1) || { echo BUILD_FAIL; exit 2; }
SPEC="$T/debug/pi-accounting-spec-check"
FIN=$(ls -t "$T"/debug/deps/independent_raw_axes-* | grep -v '\.d$' | head -1)
export R9_QUALIFICATION_SUBJECT="$S" CARGO_TARGET_DIR="$T" PYTHONDONTWRITEBYTECODE=1 PYTHONWARNINGS=ignore::ResourceWarning \
  R10_EVIDENCE_EXECUTABLE="$SPEC" R10_FINANCIAL_EXECUTABLE="$FIN" TMPDIR="$O/tmp" SPEC_CHECK_BINARY="$SPEC"
cd "$S"
run() { local n=$1; shift; timeout 1200 "$@" >"$O/$n.out" 2>&1; local rc=$?; echo "$n rc=$rc"; echo "{\"gate\":\"$n\",\"rc\":$rc}" >>"$O/results.jsonl"; }
: >"$O/results.jsonl"
run python-controls python3 -m unittest discover -s validator -p test_r10_qualification.py -v
run rust-positive "$FIN" r9_ --nocapture
run rust-r10-controls "$FIN" r10_ --nocapture
run python-main-pilot python3 validator/validate_corpus.py check "$S/corpus" --pilot
run generated "$SPEC" check "$S" --generated
run impact "$SPEC" impact "$S" LOGICAL-PRIMARY DEFINITE-LINK WHOLE-COVERAGE F-PROJECTION SCHEMA-ACCOUNTING
run schema_agreement python3 "$S/tooling/spec-check/tests/schema_agreement.py" "$S" "$SPEC"
run wire_corpus python3 "$S/tooling/spec-check/tests/wire_corpus.py" "$S" "$SPEC"
run evidence_controls python3 "$S/tooling/spec-check/tests/evidence_controls.py" "$S" "$SPEC"
# Added after spec-r1 review P3-5: full crate tests and slice-1b controls when present.
run cargo-test bash -c "cd '$S/tooling/spec-check' && CARGO_TARGET_DIR='$T' cargo test --offline"
for t in c_owner_cli.py nano_domain.py numeric_domains.py no_panic_mutations.py; do
  [ -f "$S/tooling/spec-check/tests/$t" ] && run "$t" python3 "$S/tooling/spec-check/tests/$t" "$S" "$SPEC"
done
