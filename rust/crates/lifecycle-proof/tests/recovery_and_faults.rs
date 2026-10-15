use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use lifecycle_proof::provider::{Reply, ScriptedProvider, Step};

#[test]
fn provider_rejects_credentialed_request_without_entering_script() {
    let root = tempfile::tempdir().unwrap();
    let provider = ScriptedProvider::start(
        vec![Step {
            required: Vec::new(),
            forbidden: Vec::new(),
            reply: Reply::Text("should not run".into()),
            release_path: None,
        }],
        root.path().join("ledger.jsonl"),
    )
    .unwrap();
    let address = provider
        .base_url()
        .strip_prefix("http://")
        .unwrap()
        .strip_suffix("/v1")
        .unwrap()
        .to_owned();
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .write_all(b"POST /v1/responses HTTP/1.1\r\nAuthorization: Bearer forbidden\r\nContent-Length: 2\r\n\r\n{}")
        .unwrap();
    let mut bytes = Vec::new();
    let _ = stream.read_to_end(&mut bytes);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(provider.entry_count(), 0);
    assert!(provider.finish().is_err());
}
