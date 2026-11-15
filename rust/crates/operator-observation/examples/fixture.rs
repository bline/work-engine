//! Emit the versioned cross-consumer fixture from the actual Rust producer.
use operator_observation::collect;
use serde_json::json;
fn main() {
    let ids = vec![
        "11111111-1111-4111-8111-111111111111".into(),
        "22222222-2222-4222-8222-222222222222".into(),
    ];
    let mut snapshot=collect(&"a".repeat(64),"Interface qualification",&ids,|args|{
        if args["session_id"]==ids[1]{return Err("offline");}
        Ok(json!({"sessionId":ids[0],"host":"codex","sessionTitle":"Inspect the shared observation",
            "entries":[{"entryId":"100.0","role":"assistant","tool":null,"timestamp":"2026-11-13T00:00:00Z",
                "text":"Source activity observed. Task completion remains unknown.","textTruncated":false,"redactions":0}]}))
    }).unwrap();
    snapshot.observed_at_ms = 1_794_528_000_000;
    for session in &mut snapshot.sessions {
        session.observed_at_ms = snapshot.observed_at_ms;
    }
    println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
}
