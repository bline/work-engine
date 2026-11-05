//! Out-of-band operator CLI. Reads nothing from the environment; trusted server
//! and read-profile paths and digests come only from explicit arguments.
//!
//! repository-claims-handoff emit --selected-by T --basis T --ref CLAIM,REVISION,SHA[,EVIDENCE_SHA...]...
//! repository-claims-handoff extract (CHECKPOINT_FILE | --continuity-result FILE)
//! repository-claims-handoff verify (--checkpoint FILE | --continuity-result FILE | --handoff FILE) --server PATH
//!     --server-sha256 H --profile PATH --profile-sha256 H [--page-bytes N] [--deadline-seconds N]
use repository_claims_handoff::{
    ExactRef, Handoff, MARKER, MAX_CHECKPOINT_BYTES, MAX_HANDOFF_BYTES, Options, TrustedServer,
    checkpoint_from_continuity_result, extract_from_checkpoint, resolve_handoff,
};
use std::{
    fs, io::Read, os::unix::fs::OpenOptionsExt, path::PathBuf, process::ExitCode, time::Duration,
};

fn refuse(code: &str, detail: &str) -> ExitCode {
    println!(
        "{}",
        serde_json::json!({"status":"refused","code":code,"detail":detail})
    );
    ExitCode::from(2)
}
/// Linux `O_NONBLOCK`: opening a FIFO must not wait for a writer.
const O_NONBLOCK: i32 = 0o4000;
/// Read a regular file of at most `max` bytes. Non-regular inputs (FIFOs,
/// devices, directories) are refused without being read, and ingestion itself
/// stops at `max + 1` bytes, so a file that grows after the size check is still
/// refused before it is held in memory.
fn read_bounded(path: &str, max: usize) -> Result<String, ExitCode> {
    let unreadable = || refuse("invalid_input", "file unreadable");
    let regular = |m: &fs::Metadata| m.is_file();
    if !fs::metadata(path).map_err(|_| unreadable())?.is_file() {
        return Err(refuse("invalid_input", "input must be a regular file"));
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(path)
        .map_err(|_| unreadable())?;
    let meta = file.metadata().map_err(|_| unreadable())?;
    if !regular(&meta) {
        return Err(refuse("invalid_input", "input must be a regular file"));
    }
    if meta.len() > max as u64 {
        return Err(refuse("invalid_input", "file exceeds bound"));
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unreadable())?;
    if bytes.len() > max {
        return Err(refuse("invalid_input", "file exceeds bound"));
    }
    String::from_utf8(bytes).map_err(|_| refuse("invalid_input", "file is not UTF-8"))
}
enum Source<'a> {
    Checkpoint(&'a str),
    ContinuityResult(&'a str),
    HandoffFile(&'a str),
}
fn recover(source: Source) -> Result<Handoff, ExitCode> {
    let fail = |e: repository_claims_handoff::Error| refuse(e.code(), e.detail());
    match source {
        Source::Checkpoint(p) => {
            extract_from_checkpoint(&read_bounded(p, MAX_CHECKPOINT_BYTES)?).map_err(fail)
        }
        Source::ContinuityResult(p) => {
            let checkpoint =
                checkpoint_from_continuity_result(&read_bounded(p, 4 * MAX_CHECKPOINT_BYTES)?)
                    .map_err(fail)?;
            extract_from_checkpoint(&checkpoint).map_err(fail)
        }
        Source::HandoffFile(p) => {
            let t = read_bounded(p, MAX_HANDOFF_BYTES + 32)?;
            let t = t.trim_end_matches('\n');
            Handoff::parse(t.strip_prefix(MARKER).unwrap_or(t)).map_err(fail)
        }
    }
}
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut flags: Vec<(String, String)> = Vec::new();
    let mut positional = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if let Some(name) = args[i].strip_prefix("--") {
            let Some(value) = args.get(i + 1) else {
                return refuse("invalid_input", "flag without value");
            };
            flags.push((name.into(), value.clone()));
            i += 2;
        } else {
            positional.push(args[i].clone());
            i += 1;
        }
    }
    let one = |name: &str| -> Option<&String> {
        let mut it = flags.iter().filter(|(k, _)| k == name).map(|(_, v)| v);
        let first = it.next();
        if it.next().is_some() { None } else { first }
    };
    let Some(command) = args.first() else {
        return refuse("invalid_input", "command required: emit|extract|verify");
    };
    match command.as_str() {
        "emit" => {
            let (Some(by), Some(basis)) = (one("selected-by"), one("basis")) else {
                return refuse("invalid_input", "--selected-by and --basis required");
            };
            let mut refs = Vec::new();
            for (_, spec) in flags.iter().filter(|(k, _)| k == "ref") {
                let parts: Vec<&str> = spec.split(',').collect();
                if parts.len() < 3 {
                    return refuse("invalid_input", "--ref CLAIM,REVISION,SHA[,EVIDENCE...]");
                }
                refs.push(ExactRef {
                    claim_id: parts[0].into(),
                    revision_id: parts[1].into(),
                    revision_sha256: parts[2].into(),
                    evidence_sha256: parts[3..].iter().map(|s| s.to_string()).collect(),
                });
            }
            match Handoff::new(by, basis, refs).and_then(|h| h.line()) {
                Ok(line) => {
                    println!("{line}");
                    ExitCode::SUCCESS
                }
                Err(e) => refuse(e.code(), e.detail()),
            }
        }
        "extract" => {
            let source = match (positional.first(), one("continuity-result")) {
                (Some(p), None) => Source::Checkpoint(p),
                (None, Some(p)) => Source::ContinuityResult(p),
                _ => {
                    return refuse(
                        "invalid_input",
                        "exactly one checkpoint file or --continuity-result required",
                    );
                }
            };
            match recover(source)
                .and_then(|h| h.canonical().map_err(|e| refuse(e.code(), e.detail())))
            {
                Ok(body) => {
                    println!("{MARKER}{body}");
                    ExitCode::SUCCESS
                }
                Err(c) => c,
            }
        }
        "verify" => {
            let handoff = match (one("checkpoint"), one("continuity-result"), one("handoff")) {
                (Some(p), None, None) => recover(Source::Checkpoint(p)),
                (None, Some(p), None) => recover(Source::ContinuityResult(p)),
                (None, None, Some(p)) => recover(Source::HandoffFile(p)),
                _ => {
                    return refuse(
                        "invalid_input",
                        "exactly one of --checkpoint, --continuity-result or --handoff required",
                    );
                }
            };
            let handoff = match handoff {
                Ok(h) => h,
                Err(c) => return c,
            };
            let (Some(server), Some(server_sha), Some(profile), Some(profile_sha)) = (
                one("server"),
                one("server-sha256"),
                one("profile"),
                one("profile-sha256"),
            ) else {
                return refuse(
                    "invalid_input",
                    "--server, --server-sha256, --profile, --profile-sha256 required",
                );
            };
            let mut options = Options::default();
            if let Some(n) = one("page-bytes") {
                match n.parse() {
                    Ok(n) => options.page_bytes = n,
                    Err(_) => return refuse("invalid_input", "--page-bytes"),
                }
            }
            if let Some(n) = one("deadline-seconds") {
                match n.parse() {
                    Ok(n) => options.deadline = Duration::from_secs(n),
                    Err(_) => return refuse("invalid_input", "--deadline-seconds"),
                }
            }
            let trusted = TrustedServer {
                executable: PathBuf::from(server),
                executable_sha256: server_sha.clone(),
                read_profile: PathBuf::from(profile),
                read_profile_sha256: profile_sha.clone(),
            };
            match resolve_handoff(&handoff, &trusted, &options) {
                Ok(r) => {
                    println!("{}", r.report());
                    ExitCode::SUCCESS
                }
                Err(e) => refuse(e.code(), e.detail()),
            }
        }
        _ => refuse("invalid_input", "command required: emit|extract|verify"),
    }
}
