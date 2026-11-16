#!/bin/bash
# Two synthetic routes. Usage: OBS1_FAULT=.. OBS1_EXPECT=n OBS2_FAULT=.. OBS2_EXPECT=n run2.sh <run-dir>
set -u
R=$1; mkdir -p $R; cd $R || exit 2
E=/home/bline/code/work-engine/rust/target/debug/examples
T=$(ls -t /home/bline/code/work-engine/rust/target/debug/deps/session_task_admission-* | grep -v '\.d$' | head -1)
JOB=$(printf 'pi-native-offline-repair-v1' | sha256sum | cut -d' ' -f1); EXE=$(sha256sum $E/pi_native_tools_probe | cut -d' ' -f1)
sha256sum $E/pi_native_tools_probe $E/pi_native_observer $E/pi_native_read $T /home/bline/code/work-engine/rust/crates/work-engine-manual-host/receiver/pi-native-tools.mjs > inputs.sha256
PI_FIXTURE_EXPORT=$R/fixture PI_FIXTURE_JOB_SHA=$JOB PI_FIXTURE_EXECUTOR_SHA=$EXE PI_NATIVE_CHECKED_REGISTRY=$R/store/registry PI_FIXTURE_LIFETIME_SECONDS=7200 timeout 180 $T --ignored --exact pi_pilot::tests::export_signed_pi_fixture --nocapture > fixture.log 2>&1 || { echo fixture_failed; exit 3; }
rmdir store 2>/dev/null
PIN=$(python3 -c 'import json;print(json.load(open("fixture/descriptor-pin.json"))["sha256"])')
PI_OBSERVER_FAULT=${OBS1_FAULT:-} PI_OBSERVER_EXPECT_RECEIPTS=${OBS1_EXPECT:-1} timeout 120 $E/pi_native_observer > observer1.jsonl 2> observer1.err & O1=$!
PI_OBSERVER_FAULT=${OBS2_FAULT:-} PI_OBSERVER_EXPECT_RECEIPTS=${OBS2_EXPECT:-1} timeout 120 $E/pi_native_observer > observer2.jsonl 2> observer2.err & O2=$!
for i in $(seq 50); do [ -s observer1.jsonl ] && [ -s observer2.jsonl ] && break; sleep 0.1; done
A1=$(head -1 observer1.jsonl | python3 -c 'import json,sys;print(json.load(sys.stdin)["address"])')
A2=$(head -1 observer2.jsonl | python3 -c 'import json,sys;print(json.load(sys.stdin)["address"])')
timeout 210 $E/pi_native_tools_probe $R/store $R/fixture/descriptor.json $PIN $A1,$A2 > probe.out 2> probe.err; echo "probe_rc=$?"
sleep 0.5; kill $O1 $O2 2>/dev/null; wait $O1; echo "observer1_rc=$?"; wait $O2; echo "observer2_rc=$?"
cat probe.err
echo "observer1 receipts=$(grep -c '"receipt"' observer1.jsonl) observer2 receipts=$(grep -c '"receipt"' observer2.jsonl)"
LEFT=$(docker ps -a --filter label=work-engine.owner=01a1db17-native -q)
echo "owned_containers=$(echo -n "$LEFT" | grep -c .)"
if [ -n "$LEFT" ]; then echo "$LEFT" > leftover-containers.txt; docker rm -f $LEFT >/dev/null && echo "leftover_removed=$(wc -l < leftover-containers.txt)"; fi
if [ -f store/control-route-000.json ]; then /home/bline/code/work-engine/rust/target/debug/examples/pi_route_recover $R/store > recovery.json 2> recovery.err; echo "recover_rc=$?"; fi
REG=$(ls store/registry/*.sqlite 2>/dev/null | sort | tail -1); [ -n "$REG" ] && sqlite3 "$REG" "select key,state from pi_effect" 2>/dev/null > effects.txt || sqlite3 "$REG" "select * from pi_effect" | cut -d'|' -f2,3 > effects.txt; echo "effects=$(tr '\n' ' ' < effects.txt)"
