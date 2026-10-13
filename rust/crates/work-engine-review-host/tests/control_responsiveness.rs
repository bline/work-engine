#![cfg(all(unix, feature = "test-peer"))]
mod support;
use serde_json::{Value, json};
use std::{
    fs,
    time::{Duration, Instant},
};
use support::*;

fn probe(c: &mut Client, label: &str, samples: usize, terminal: &mut Vec<Value>) -> Vec<u64> {
    let mut times = vec![];
    for i in 0..samples {
        let id = format!("{label}-{i}");
        let sent = Instant::now();
        c.send(json!({"version":1,"requestId":id,"operation":"status"}));
        loop {
            let reply = c.receive();
            if reply["requestId"] == id {
                times.push(sent.elapsed().as_micros() as u64);
                break;
            }
            terminal.push(reply);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    times
}
fn record(label: &str, times: &[u64], extra: Value) {
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    let p99 = sorted[(sorted.len() * 99).div_ceil(100) - 1];
    let max = *sorted.last().unwrap();
    let result = json!({"scenario":label,"sampleCount":times.len(),"samplingIntervalMillis":5,"continuouslyDrainingConsumer":true,"p99Micros":p99,"maxMicros":max,"missedProbes":0,"latencyMicros":times,"details":extra});
    eprintln!("H1 responsiveness {label}: p99={p99}us max={max}us");
    if let Ok(root) = std::env::var("H1_EVIDENCE_ROOT") {
        fs::create_dir_all(&root).unwrap();
        fs::write(
            std::path::Path::new(&root).join(format!("probe-{label}.json")),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
    assert!(p99 <= 50_000 && max <= 250_000, "{result}");
}
#[test]
fn external_control_probes_cover_cold_capture_contention_and_four_requests() {
    let f = Fixture::new(3000, "result");
    let initialized = f.initialize();
    let revision = initialized["campaign"]["revision"].as_str().unwrap();
    let mut c = f.start(&[]);
    let mut observed = vec![];
    let cold = probe(&mut c, "cold", 200, &mut observed);
    record(
        "cold",
        &cold,
        json!({"phase":"startup/reconstruction","memory":"not_observed"}),
    );
    c.ready();
    let started = Instant::now();
    for n in 0..4 {
        c.send(json!({"version":1,"requestId":format!("execute-{n}"),"operation":"execute_initial","obligationId":"review-0","operationId":"h1-initial","expectedRevision":revision}));
    }
    let four = probe(&mut c, "four", 200, &mut observed);
    record("four", &four, json!({"simultaneousRequests":4}));
    let deadline = Instant::now() + Duration::from_secs(20);
    while f.entries() == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let capture = probe(&mut c, "capture", 100, &mut observed);
    record("capture", &capture, json!({"peerEntries":f.entries()}));
    while !observed
        .iter()
        .any(|r| r["result"]["outcome"]["status"] == "committed")
    {
        observed.push(c.receive());
    }
    assert_eq!(
        observed
            .iter()
            .filter(|r| r["error"]["kind"] == "busy")
            .count(),
        3,
        "{observed:?}"
    );
    assert_eq!(f.entries(), 1);
    let operation_ms = started.elapsed().as_millis();
    eprintln!("H1 successful operation including probes {operation_ms}ms");
    if let Ok(root) = std::env::var("H1_EVIDENCE_ROOT") {
        fs::write(
            std::path::Path::new(&root).join("operation-timing.json"),
            serde_json::to_vec_pretty(&json!({"elapsedMillis":operation_ms,
                "successfulOutcomes":1,"busyRefusals":3,"timedOut":0,"unresolved":0,
                "peerEntries":f.entries(),"memory":"not_observed",
                "startupSha256":claim_evidence::sha256_bytes(&fs::read(&f.startup).unwrap())}))
            .unwrap(),
        )
        .unwrap();
    }
    c.finish();

    let f = Fixture::new(0, "result");
    let initialized = f.initialize();
    let mut c = f.start(&[]);
    c.ready();
    let lock = rusqlite::Connection::open(f.document.roots.campaign.join("slice-campaign.sqlite"))
        .unwrap();
    lock.execute_batch("BEGIN IMMEDIATE").unwrap();
    c.domain(
        "execute_initial",
        initialized["campaign"]["revision"].as_str().unwrap(),
    );
    let mut responses = vec![];
    let busy = probe(&mut c, "contention", 200, &mut responses);
    record(
        "contention",
        &busy,
        json!({"lock":"campaign BEGIN IMMEDIATE","release":"after probes"}),
    );
    lock.execute_batch("ROLLBACK").unwrap();
    loop {
        let r = c.receive();
        if r["requestId"] == "execute_initial" {
            assert_eq!(r["result"]["outcome"]["status"], "committed", "{r}");
            break;
        }
    }
    c.finish();
}

#[test]
fn stop_and_child_timeout_preserve_owned_work_and_no_retry() {
    let f = Fixture::new(6000, "result");
    let initialized = f.initialize();
    let mut c = f.start(&[]);
    c.ready();
    c.domain(
        "execute_initial",
        initialized["campaign"]["revision"].as_str().unwrap(),
    );
    c.send(json!({"version":1,"requestId":"stop","operation":"stop_admission"}));
    let stop = c.receive();
    assert_eq!(stop["result"]["status"], "admission_stopped", "{stop}");
    let reply = c.receive();
    assert_eq!(reply["error"]["kind"], "unresolved", "{reply}");
    assert_eq!(f.entries(), 1);
    c.finish();
    let mut recovered = f.start(&[]);
    let ready = recovered.ready();
    recovered.domain(
        "recover_initial",
        ready["result"]["lastObservation"]["revision"]
            .as_str()
            .unwrap(),
    );
    let failure = recovered.receive();
    assert_eq!(failure["result"]["status"], "unresolved", "{failure}");
    assert_eq!(
        failure["result"]["failure"]["failure"]["kind"], "timeout",
        "{failure}"
    );
    assert_eq!(f.entries(), 1);
    recovered.finish();
}

#[test]
fn eof_and_output_backpressure_finish_owned_work_without_resend() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    let f = Fixture::new(100, "result");
    let initial = f.initialize();
    let mut c = f.start(&[]);
    c.ready();
    c.domain(
        "execute_initial",
        initial["campaign"]["revision"].as_str().unwrap(),
    );
    c.finish(); // Close stdin immediately, retain the draining response observer.
    let reply = c.receive();
    assert_eq!(reply["result"]["outcome"]["status"], "committed", "{reply}");
    assert_eq!(f.entries(), 1);

    let f = Fixture::new(500, "result");
    let initial = f.initialize();
    let mut child = Command::new(&f.host)
        .args(["serve", "--startup"])
        .arg(&f.startup)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    loop {
        writeln!(
            input,
            "{}",
            json!({"version":1,"requestId":"ready","operation":"status"})
        )
        .unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let reply: Value = serde_json::from_str(&line).unwrap();
        if reply["result"]["busy"] == false {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    writeln!(input, "{}", json!({"version":1,"requestId":"execute","operation":"execute_initial","obligationId":"review-0","operationId":"h1-initial","expectedRevision":initial["campaign"]["revision"]})).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while f.entries() == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    // Keep stdout open but stop draining. The bounded output queue closes admission;
    // the writer's timeout cannot cancel the already-owned domain operation.
    for n in 0..300 {
        if writeln!(
            input,
            "{}",
            json!({"version":1,"requestId":format!("blocked-{n}"),"operation":"status"})
        )
        .is_err()
        {
            break;
        }
    }
    drop(input);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(
            Instant::now() < deadline,
            "blocked consumer prevented bounded shutdown"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(output);
    let mut recovered = f.start(&[]);
    let ready = recovered.ready();
    recovered.domain(
        "recover_initial",
        ready["result"]["lastObservation"]["revision"]
            .as_str()
            .unwrap(),
    );
    let result = recovered.receive();
    assert_eq!(
        result["result"]["outcome"]["status"], "committed",
        "{result}"
    );
    assert_eq!(f.entries(), 1);
    recovered.finish();
}
