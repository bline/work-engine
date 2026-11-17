#!/bin/bash
# Slice 1 (D4) run set: the r8 scenarios plus long-answer. Usage: matrix-s1.sh <run-root> <subject>
# Binaries are copied into <run-root>/bin before the runs; check.py pins them.
cd /home/bline/code/work-engine/records/dev-vertical-h2-route-20261117
R=$1
SUBJECT=$2
mkdir -p $R/bin
cp /home/bline/code/work-engine/rust/crates/pi-pressure-context-host/target/release/coding-qualification \
   /home/bline/code/work-engine/rust/target/debug/examples/pi_hosted_coding_observer $R/bin/
for spec in "ok ok none" "long-answer long-answer@2 none" "late-close-short late-close-short none" "late-close-long late-close-long none" "trailing trailing none" "refuse-all refuse-all none" "primary-500 primary-500 none" "primary-429-echo primary-429-echo none" "primary-429-truncated primary-429-truncated none" "truncate truncate none" "midstream-failed midstream-failed none" "trailing-data trailing-data none" "echo-body echo-body none" "echo-trailing echo-trailing none" "stall-head stall-head none" "stall-body stall-body none" "truncate-3 truncate@3 none" "crash stall-body crash" "cancel stall-body cancel" "cancel-head stall-head cancel"; do
  set -- $spec
  rm -rf $R/$1
  timeout 700 python3 qualify.py $R $1 $2 $3 > $R/$1.summary.json 2> $R/$1.harness.err
  echo "$1 done $?" >> $R/matrix.log
done
python3 check.py $R $SUBJECT > $R/matrix.json
echo "ALL-DONE all_pass=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["all_pass"])' $R/matrix.json)" >> $R/matrix.log
