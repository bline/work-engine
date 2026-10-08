use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
    thread,
    time::Duration,
};

use lifecycle_client::{
    ClientError, ObservationTransport, SubjectRef, UnixSocketTransport, WaitHintV1,
};
use lifecycle_wire::{CursorV1, ExecutionOutcomeV1, WaitTargetV1};
use tempfile::tempdir;

#[test]
fn concrete_socket_reads_v1_snapshot_with_bounded_frame() {
    let root = tempdir().unwrap();
    let path = root.path().join("read.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let fixture = serde_json::to_vec(
        &serde_json::from_slice::<serde_json::Value>(
            &fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../lifecycle-wire/tests/fixtures/failed-unsettled-v1.json"),
            )
            .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        assert!(request.contains("\"op\":\"snapshot\""));
        stream.write_all(&fixture).unwrap();
        stream.write_all(b"\n").unwrap();
    });
    let transport = UnixSocketTransport::new(&path);
    let snapshot = transport
        .snapshot(
            &SubjectRef::parse("subject-1").unwrap(),
            None,
            Duration::from_secs(1),
        )
        .unwrap();
    assert_eq!(snapshot.subject_id, "subject-1");
    server.join().unwrap();
}

#[test]
fn concrete_socket_honors_read_budget_when_wait_hint_is_lost() {
    let root = tempdir().unwrap();
    let path = root.path().join("wait.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        assert!(request.contains("\"op\":\"wait\""));
        thread::sleep(Duration::from_millis(100));
    });
    let transport = UnixSocketTransport::new(&path);
    let result = transport.wait_hint(
        &SubjectRef::parse("subject-1").unwrap(),
        &WaitTargetV1::DeliveryOutcome {
            delivery_id: "delivery-1".into(),
            outcome: ExecutionOutcomeV1::Completed,
        },
        &CursorV1 {
            store_id: "store-1".into(),
            stream_id: "stream-1".into(),
            commit_sequence: "0".into(),
        },
        Duration::from_millis(20),
    );
    assert!(
        matches!(result, Err(ClientError::DeadlineExpired)),
        "{result:?}"
    );
    server.join().unwrap();
}

#[test]
fn concrete_socket_exposes_cursor_gap_as_hint_only() {
    let root = tempdir().unwrap();
    let path = root.path().join("gap.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut request)
            .unwrap();
        stream.write_all(b"{\"hint\":\"cursor_gap\"}\n").unwrap();
    });
    let transport = UnixSocketTransport::new(&path);
    let result = transport
        .wait_hint(
            &SubjectRef::parse("subject-1").unwrap(),
            &WaitTargetV1::DeliveryOutcome {
                delivery_id: "delivery-1".into(),
                outcome: ExecutionOutcomeV1::Completed,
            },
            &CursorV1 {
                store_id: "store-1".into(),
                stream_id: "stream-1".into(),
                commit_sequence: "0".into(),
            },
            Duration::from_secs(1),
        )
        .unwrap();
    assert_eq!(result, WaitHintV1::CursorGap);
    server.join().unwrap();
}
