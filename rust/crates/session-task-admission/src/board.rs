//! Complete-board eligibility checks; source acquisition remains unbound.
use crate::proof::{Assertion, VerifiedAnswer, reply_matches};
use serde_json::Value;
use std::collections::HashSet;

/// Must come from one complete, integrity-checked authoritative board revision.
/// No public or production constructor exists until the board owner is wired.
#[allow(dead_code)]
pub(crate) struct CompleteBoard {
    pub(crate) revision: String,
    pub(crate) rows: Vec<String>,
}
/// Accept only one closing reply, and only when it is the exact proof verified
/// by this consumer. Any supersession of the request holds. Task/proposal
/// lifecycle is checked separately, not inferred from the board reply.
#[allow(dead_code)]
pub(crate) fn own_close<'a>(
    board: &'a CompleteBoard,
    raw_request: &str,
    raw_answer: &str,
    assertion: &Assertion<'_>,
    verified: &VerifiedAnswer,
) -> Option<&'a str> {
    if board.revision.is_empty() {
        return None;
    }
    let request: Value = serde_json::from_str(raw_request).ok()?;
    let sequence = request.get("sequence")?.as_u64()?;
    if sequence != verified.board_seq || request.get("kind")?.as_str()? != "request" {
        return None;
    }
    let mut seen = HashSet::new();
    let mut previous = 0;
    let mut matched = false;
    let mut closed: Option<&str> = None;
    for raw in &board.rows {
        let row: Value = serde_json::from_str(raw).ok()?;
        let seq = row.get("sequence")?.as_u64()?;
        if seq == 0 || seq <= previous || !seen.insert(seq) {
            return None;
        }
        previous = seq;
        if seq == sequence {
            if raw != raw_request {
                return None;
            }
            matched = true;
        }
        if row.get("supersedes").and_then(Value::as_u64) == Some(sequence) {
            return None;
        }
        if row.get("kind").and_then(Value::as_str) == Some("reply")
            && row.get("in_reply_to").and_then(Value::as_u64) == Some(sequence)
            && matches!(
                row.pointer("/data/disposition").and_then(Value::as_str),
                Some("answered" | "declined")
            )
        {
            if seq <= sequence || closed.is_some() {
                return None;
            }
            closed = Some(raw.as_str());
        }
    }
    let reply = closed.filter(|_| matched)?;
    reply_matches(raw_request, reply, raw_answer, assertion, verified).ok()?;
    Some(reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hash, proof::assertion_digest};
    use serde_json::json;
    use time::macros::datetime;
    #[test]
    fn exact_closing_reply_required_and_supersession_holds() {
        let request =
            json!({"kind":"request","sequence":1,"topic":"t","session_id":"s"}).to_string();
        let answer = "synthetic";
        let a = Assertion {
            credential_id: "c",
            authenticator_data: "d",
            client_data_json: "j",
            signature: "s",
        };
        let v = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "answer".into(),
            answer_digest: hash(answer),
            assertion_digest: assertion_digest(&a),
            board_seq: 1,
            request_digest: hash(&request),
            expires_at: datetime!(2026-11-03 00:00 UTC),
        };
        let reply=json!({"kind":"reply","sequence":2,"in_reply_to":1,"topic":"t","to":["session:s"],"message_id":"answer","data":{"disposition":"answered","decision":"approve","comment":"","answer_id":"answer","signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":answer,"credential_id":"c","authenticator_data":"d","client_data_json":"j","signature":"s"}}}).to_string();
        let mut board = CompleteBoard {
            revision: "r1".into(),
            rows: vec![request.clone(), reply],
        };
        assert!(own_close(&board, &request, answer, &a, &v).is_some());
        board
            .rows
            .push(json!({"kind":"note","sequence":3,"supersedes":1}).to_string());
        assert!(own_close(&board, &request, answer, &a, &v).is_none());
        board.rows.pop();
        board.rows[1] =
            json!({"kind":"reply","sequence":2,"in_reply_to":1,"data":{"disposition":"declined"}})
                .to_string();
        assert!(own_close(&board, &request, answer, &a, &v).is_none());
    }
}

/// A complete-board transfer file must be produced by a separately qualified
/// adapter bound to the current canonical Git-ref board. This loader validates
/// its representation, not its provenance, freshness, or isolation. Until that
/// adapter is bound, no production path may treat this object as authoritative.
#[cfg(unix)]
#[allow(dead_code)]
pub(crate) fn load_snapshot(path: &std::path::Path) -> Option<CompleteBoard> {
    use std::{
        io::Read,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    if !path.is_absolute() {
        return None;
    }
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file()
        || meta.len() > 8 * 1024 * 1024
        || meta.mode() & 0o077 != 0
        || meta.uid() != unsafe { libc::geteuid() }
    {
        return None;
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    let after = file.metadata().ok()?;
    if bytes.len() as u64 != meta.len()
        || after.len() != meta.len()
        || after.mtime_nsec() != meta.mtime_nsec()
        || after.mtime() != meta.mtime()
    {
        return None;
    }
    parse_transfer(&bytes)
}
#[allow(dead_code)]
pub(crate) fn parse_transfer(bytes: &[u8]) -> Option<CompleteBoard> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Transfer {
        revision: String,
        next_sequence: u64,
        rows: Vec<String>,
    }
    let transfer: Transfer = serde_json::from_slice(bytes).ok()?;
    if !matches!(transfer.revision.len(), 40 | 64)
        || !transfer
            .revision
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        || transfer.next_sequence != transfer.rows.len() as u64 + 1
    {
        return None;
    }
    for (index, raw) in transfer.rows.iter().enumerate() {
        #[derive(serde::Deserialize)]
        struct Row {
            sequence: u64,
        }
        let row: Row = serde_json::from_str(raw).ok()?;
        if row.sequence != index as u64 + 1 {
            return None;
        }
    }
    Some(CompleteBoard {
        revision: transfer.revision,
        rows: transfer.rows,
    })
}

#[cfg(all(test, unix))]
mod loader_tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    #[test]
    fn transfer_requires_complete_sequences_and_no_symlink() {
        let dir = std::env::temp_dir().join(format!("task-board-transfer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        let file = dir.join("board.json");
        let transfer = serde_json::json!({"revision":"a".repeat(40),"next_sequence":3,"rows":["{\"sequence\":1}","{\"sequence\":2}"]});
        std::fs::write(&file, transfer.to_string()).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(load_snapshot(&file).is_some());
        let link = dir.join("link");
        symlink(&file, &link).unwrap();
        assert!(load_snapshot(&link).is_none());
        let mut incomplete = transfer;
        incomplete["rows"].as_array_mut().unwrap().remove(0);
        std::fs::write(&file, incomplete.to_string()).unwrap();
        assert!(load_snapshot(&file).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
