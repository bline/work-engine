#!/bin/bash
# Hosted analysis run set. Usage: matrix-a.sh <run-root> <subject>
cd /home/bline/code/work-engine/records/dev-vertical-h2-route-20261117
R=$1; SUBJECT=$2; S=/home/bline/.local/state/work-engine/dev-vertical-h2-route-20261117
mkdir -p $R/bin
cp /home/bline/code/work-engine/rust/crates/pi-pressure-context-host/target/release/analysis-qualification \
   /home/bline/code/work-engine/rust/target/debug/examples/pi_hosted_analysis_observer $R/bin/
export ANALYSIS_QUAL=$R/bin/analysis-qualification
[ -d $S/h2-treatment ] || python3 h2_snapshot.py $S/h2-treatment treatment
[ -d $S/h2-baseline ] || python3 h2_snapshot.py $S/h2-baseline baseline
for spec in "ok ok -" "long-answer long-answer -" "read-all-treatment read-all $S/h2-treatment" "read-all-baseline read-all $S/h2-baseline"; do
  set -- $spec
  rm -rf $R/$1
  ANALYSIS_MAX_REQUESTS=24 ANALYSIS_MAX_READS=64 timeout 700 python3 analyze.py $R $1 $2 $3 > $R/$1.summary.json 2> $R/$1.harness.err
  echo "$1 done $?" >> $R/matrix.log
done
python3 check_analysis.py $R $SUBJECT ok long-answer read-all-treatment read-all-baseline > $R/matrix.json
echo "ALL-DONE all_pass=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["all_pass"])' $R/matrix.json)" >> $R/matrix.log
