use std::env;
#[cfg(feature = "test-faults")]
use std::io::Write;
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::time::Duration;

use review_episode::admission::FixtureAdmission;
use review_episode::admission::HostAdmissionPort;
use review_episode::host_admission::NativeHostAdmission;
use review_episode::protocol::{
    DEFAULT_REQUEST_LIMIT, DEFAULT_RESPONSE_LIMIT, Request, error_reply, error_reply_for_request,
    read_frame, write_frame,
};
use review_episode::{AppError, Application};
use review_episode_store::{
    EpisodeStore, StoreOptions, init_native_root, init_offline_root, read_native_selection,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("review-episode command failed: {}", error.kind());
        std::process::exit(2);
    }
}

fn run() -> Result<(), AppError> {
    validate_fault_configuration()?;
    let mut args = env::args_os().skip(1);
    let mode = args
        .next()
        .ok_or_else(|| AppError::Framing("offline mode is required".into()))?;
    if mode == "init-offline" || mode == "init-native" {
        if args.next().as_deref() != Some(std::ffi::OsStr::new("--root")) {
            return Err(AppError::Framing("expected --root".into()));
        }
        let root = PathBuf::from(
            args.next()
                .ok_or_else(|| AppError::Framing("root missing".into()))?,
        );
        if args.next().is_some() {
            return Err(AppError::Framing("unexpected launch argument".into()));
        }
        if mode == "init-native" {
            init_native_root(&root)?;
        } else {
            init_offline_root(&root)?;
        }
        return Ok(());
    }
    let native = mode == "native-host-v1";
    if mode != "offline" && !native {
        return Err(AppError::Framing(
            "unsupported review-episode launch profile".into(),
        ));
    }
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--root")) {
        return Err(AppError::Framing("expected --root".into()));
    }
    let root = PathBuf::from(
        args.next()
            .ok_or_else(|| AppError::Framing("root missing".into()))?,
    );
    let fixture = if native {
        None
    } else {
        if args.next().as_deref() != Some(std::ffi::OsStr::new("--fixture")) {
            return Err(AppError::Framing("expected --fixture".into()));
        }
        Some(PathBuf::from(args.next().ok_or_else(|| {
            AppError::Framing("fixture missing".into())
        })?))
    };
    let mut request_limit = DEFAULT_REQUEST_LIMIT;
    let mut response_limit = DEFAULT_RESPONSE_LIMIT;
    let mut busy_timeout_ms = if native { 50u64 } else { 5_000u64 };
    while let Some(option) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| AppError::Framing("launch option value missing".into()))?;
        let value = value
            .to_str()
            .ok_or_else(|| AppError::Framing("launch option must be ASCII".into()))?
            .parse::<u64>()
            .map_err(|_| AppError::Framing("launch option must be a positive integer".into()))?;
        match option.to_str() {
            Some("--max-request-bytes") if value > 0 && value <= DEFAULT_REQUEST_LIMIT as u64 => {
                request_limit = value as usize
            }
            Some("--max-response-bytes")
                if value >= 512 && value <= DEFAULT_RESPONSE_LIMIT as u64 =>
            {
                response_limit = value as usize
            }
            Some("--busy-timeout-ms") if value > 0 && value <= if native { 50 } else { 5_000 } => {
                busy_timeout_ms = value
            }
            _ => {
                return Err(AppError::Framing(
                    "unsupported or out-of-range launch option".into(),
                ));
            }
        }
    }
    let mut input = BufReader::new(io::stdin().lock());
    let bytes = match read_frame(&mut input, request_limit) {
        Ok(bytes) => bytes,
        Err(error) => {
            let response = error_reply(None, error.kind(), &error.public_message());
            write_frame(&mut io::stdout().lock(), &response, response_limit)?;
            return Ok(());
        }
    };
    let request = match Request::parse(&bytes) {
        Ok(request) => request,
        Err(error) => {
            let response = error_reply(None, error.kind(), &error.public_message());
            write_frame(&mut io::stdout().lock(), &response, response_limit)?;
            return Ok(());
        }
    };
    let response = match (|| {
        if native != (request.version() == 2) {
            return Err(AppError::Framing(
                "request and launch profiles differ".into(),
            ));
        }
        let options = StoreOptions {
            busy_timeout: Duration::from_millis(busy_timeout_ms),
        };
        if native {
            let selection = read_native_selection(&root)?;
            let port = NativeHostAdmission::from_inherited_fd(selection)?;
            port.precheck(&request)?;
            let (store, _) = EpisodeStore::open_native(&root, options)?;
            let mut app = Application::new(store, port, response_limit);
            app.execute(&request)
        } else {
            let port = FixtureAdmission::load(
                fixture.as_ref().expect("offline launch has fixture"),
                &root,
            )?;
            let store = EpisodeStore::open(&root, options)?;
            let mut app = Application::new(store, port, response_limit);
            app.execute(&request)
        }
    })() {
        Ok(response) => response,
        Err(error) => error_reply_for_request(&request, error.kind(), &error.public_message()),
    };
    review_episode_store::test_checkpoint("before_reply");
    #[cfg(feature = "test-faults")]
    if std::env::var("REVIEW_EPISODE_FAULT_CUT").ok().as_deref() == Some("after_partial_reply")
        && std::env::var("REVIEW_EPISODE_FAULT_OPERATION")
            .ok()
            .is_none_or(|operation| operation == request.operation())
    {
        let mut stdout = io::stdout().lock();
        let midpoint = response.len() / 2;
        stdout
            .write_all(&(response.len() as u32).to_be_bytes())
            .map_err(|e| AppError::Io(e.to_string()))?;
        stdout
            .write_all(&response.as_bytes()[..midpoint])
            .map_err(|e| AppError::Io(e.to_string()))?;
        stdout.flush().map_err(|e| AppError::Io(e.to_string()))?;
        review_episode_store::test_checkpoint("after_partial_reply");
        stdout
            .write_all(&response.as_bytes()[midpoint..])
            .map_err(|e| AppError::Io(e.to_string()))?;
        stdout.flush().map_err(|e| AppError::Io(e.to_string()))?;
        return Ok(());
    }
    write_frame(&mut io::stdout().lock(), &response, response_limit)?;
    Ok(())
}

fn validate_fault_configuration() -> Result<(), AppError> {
    let cut = std::env::var("REVIEW_EPISODE_FAULT_CUT").ok();
    let directory = std::env::var("REVIEW_EPISODE_FAULT_DIR").ok();
    #[cfg(not(feature = "test-faults"))]
    if cut.is_some() || directory.is_some() {
        return Err(AppError::Framing(
            "fault controls unavailable in default executable".into(),
        ));
    }
    #[cfg(feature = "test-faults")]
    if cut.is_some() || directory.is_some() {
        let valid = matches!(
            cut.as_deref(),
            Some(
                "after_admission"
                    | "after_begin_immediate"
                    | "after_history_insert"
                    | "after_current_update"
                    | "before_commit"
                    | "after_commit"
                    | "after_commit_unknown"
                    | "before_reply"
                    | "after_partial_reply"
            )
        );
        if !valid || directory.as_deref().is_none_or(|d| d.is_empty()) {
            return Err(AppError::Framing("invalid test fault configuration".into()));
        }
    }
    Ok(())
}
