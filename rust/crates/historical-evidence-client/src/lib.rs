//! Passive local client for the bounded historical S5 read operation.
use historical_evidence_wire::{MAX_FRAME, Observation, Reply, Request};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

#[derive(Debug)]
pub enum ReadError {
    InvalidRequest,
    Unavailable,
    Refused,
    InvalidResponse,
}

pub fn inspect(socket: &Path, request: Request) -> Result<Observation, ReadError> {
    request.validate().map_err(|_| ReadError::InvalidRequest)?;
    if !socket.is_absolute() {
        return Err(ReadError::InvalidRequest);
    }
    let mut stream = UnixStream::connect(socket).map_err(|_| ReadError::Unavailable)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(35)))
        .map_err(|_| ReadError::Unavailable)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(8)))
        .map_err(|_| ReadError::Unavailable)?;
    let bytes = serde_json::to_vec(&request).map_err(|_| ReadError::InvalidRequest)?;
    if bytes.len() > MAX_FRAME {
        return Err(ReadError::InvalidRequest);
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|_| ReadError::Unavailable)?;
    stream
        .write_all(&bytes)
        .map_err(|_| ReadError::Unavailable)?;
    let mut len = [0; 4];
    stream
        .read_exact(&mut len)
        .map_err(|_| ReadError::Unavailable)?;
    let len = u32::from_be_bytes(len) as usize;
    if len == 0 || len > MAX_FRAME {
        return Err(ReadError::InvalidResponse);
    }
    let mut response = vec![0; len];
    stream
        .read_exact(&mut response)
        .map_err(|_| ReadError::Unavailable)?;
    match serde_json::from_slice(&response).map_err(|_| ReadError::InvalidResponse)? {
        Reply::Observed { observation } => {
            observation
                .validate()
                .map_err(|_| ReadError::InvalidResponse)?;
            Ok(observation)
        }
        Reply::Refused { .. } => Err(ReadError::Refused),
        Reply::Unavailable { .. } => Err(ReadError::Unavailable),
    }
}

pub fn inspect_v2(
    socket: &Path,
    request: historical_evidence_wire::RequestV2,
) -> Result<historical_evidence_wire::ObservationV2, ReadError> {
    use historical_evidence_wire::ReplyV2;
    request.validate().map_err(|_| ReadError::InvalidRequest)?;
    if !socket.is_absolute() {
        return Err(ReadError::InvalidRequest);
    }
    let mut stream = UnixStream::connect(socket).map_err(|_| ReadError::Unavailable)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(35)))
        .map_err(|_| ReadError::Unavailable)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(8)))
        .map_err(|_| ReadError::Unavailable)?;
    let bytes = serde_json::to_vec(&request).map_err(|_| ReadError::InvalidRequest)?;
    if bytes.len() > MAX_FRAME {
        return Err(ReadError::InvalidRequest);
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .map_err(|_| ReadError::Unavailable)?;
    stream
        .write_all(&bytes)
        .map_err(|_| ReadError::Unavailable)?;
    let mut len = [0; 4];
    stream
        .read_exact(&mut len)
        .map_err(|_| ReadError::Unavailable)?;
    let size = u32::from_be_bytes(len) as usize;
    if size == 0 || size > MAX_FRAME {
        return Err(ReadError::InvalidResponse);
    }
    let mut response = vec![0; size];
    stream
        .read_exact(&mut response)
        .map_err(|_| ReadError::Unavailable)?;
    match serde_json::from_slice(&response).map_err(|_| ReadError::InvalidResponse)? {
        ReplyV2::Observed { observation } => {
            observation
                .validate()
                .map_err(|_| ReadError::InvalidResponse)?;
            Ok(observation)
        }
        ReplyV2::Refused { .. } => Err(ReadError::Refused),
        ReplyV2::Unavailable { .. } => Err(ReadError::Unavailable),
    }
}

#[cfg(test)]
mod v2_nullable_regression_tests {
    use super::*;
    use historical_evidence_wire::RequestV2;
    use std::os::unix::fs::DirBuilderExt;
    use std::os::unix::net::UnixListener;

    fn read_through_client(reply: &serde_json::Value, case: &str) -> Result<(), ReadError> {
        let directory = std::env::temp_dir().join(format!(
            "historical-evidence-client-{}-{case}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let socket = directory.join("read.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let bytes = serde_json::to_vec(reply).unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut length = [0; 4];
            stream.read_exact(&mut length).unwrap();
            let mut request = vec![0; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut request).unwrap();
            stream
                .write_all(&(bytes.len() as u32).to_be_bytes())
                .unwrap();
            stream.write_all(&bytes).unwrap();
        });
        let result = inspect_v2(&socket, RequestV2::all()).map(|_| ());
        server.join().unwrap();
        std::fs::remove_file(&socket).unwrap();
        std::fs::remove_dir(&directory).unwrap();
        result
    }

    // The qualified reply is private owner evidence; the fixture path is supplied
    // during local qualification and its contents never enter the repository.
    #[test]
    #[ignore = "requires S5_QUALIFIED_REPLY pointing at a private qualified reply"]
    fn qualified_reply_requires_explicit_nullable_observations() {
        let path = std::env::var("S5_QUALIFIED_REPLY").expect("qualified reply path");
        let bytes = std::fs::read(path).unwrap();
        let reply: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            reply["observation"]["scheduler"].get("presented_at"),
            Some(&serde_json::Value::Null)
        );
        assert!(read_through_client(&reply, "observed").is_ok());

        let mut omitted = reply.clone();
        omitted["observation"]["scheduler"]
            .as_object_mut()
            .unwrap()
            .remove("presented_at");
        assert!(matches!(
            read_through_client(&omitted, "missing-presentation"),
            Err(ReadError::InvalidResponse)
        ));

        let mut wrong_type = reply.clone();
        wrong_type["observation"]["scheduler"]["presented_at"] = serde_json::json!(false);
        assert!(matches!(
            read_through_client(&wrong_type, "wrong-presentation"),
            Err(ReadError::InvalidResponse)
        ));

        let mut non_null = reply.clone();
        non_null["observation"]["scheduler"]["presented_at"] =
            serde_json::json!("2026-10-16T00:00:00Z");
        assert!(matches!(
            read_through_client(&non_null, "non-null-presentation"),
            Err(ReadError::InvalidResponse)
        ));

        for field in ["metrics", "metrics_unavailable"] {
            let mut omitted = reply.clone();
            omitted["observation"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(matches!(
                read_through_client(&omitted, field),
                Err(ReadError::InvalidResponse)
            ));
        }
        for field in ["model", "api_error_status"] {
            let mut omitted = reply.clone();
            omitted["observation"]["provider"]["attempts"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(matches!(
                read_through_client(&omitted, field),
                Err(ReadError::InvalidResponse)
            ));
        }
    }
}
