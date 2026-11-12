//! Production startup intentionally has no usable authority installation.
use research_register::{
    card_admission::Installation,
    card_host::{self, CardRequest, WorkerInput},
    card_wire::{self, Envelope},
    read_host,
};
use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
static CANCEL: AtomicBool = AtomicBool::new(false);
extern "C" fn cancel(_: i32) {
    CANCEL.store(true, Ordering::SeqCst);
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--private-worker-v1"] {
        let mut bytes = vec![];
        let input = std::io::stdin()
            .take(16385)
            .read_to_end(&mut bytes)
            .ok()
            .filter(|_| bytes.len() <= 16384)
            .and_then(|_| serde_json::from_slice::<WorkerInput>(&bytes).ok());
        let value = if let Some(input) = input {
            let producer = input.producer.clone();
            match card_host::worker(input) {
                Ok(v) => v,
                Err(reason) => {
                    serde_json::json!({"producer":producer,"outcome":"refused","reason":reason})
                }
            }
        } else {
            serde_json::json!({"outcome":"refused","reason":"bad_request"})
        };
        if read_host::frame(&value)
            .and_then(|b| std::io::stdout().write_all(&b).map_err(|e| e.to_string()))
            .is_err()
        {
            std::process::exit(4);
        }
        return;
    }
    let mut q = None;
    let result = (|| -> Result<Vec<u8>, String> {
        if args.len() != 2 {
            return Err("bad_request".into());
        }
        let mut profile_bytes = vec![];
        std::fs::File::open(&args[1])
            .map_err(|_| "scope_unavailable")?
            .take(8193)
            .read_to_end(&mut profile_bytes)
            .map_err(|_| "scope_unavailable")?;
        if profile_bytes.len() > 8192 {
            return Err("resource_exceeded".into());
        }
        research_register::parse::document("profile", &profile_bytes, &mut vec![])
            .ok_or("bad_request")?;
        let profile: read_host::HostProfile =
            serde_json::from_slice(&profile_bytes).map_err(|_| "bad_request")?;
        profile.validate().map_err(|_| "resource_exceeded")?;
        #[cfg(target_os = "linux")]
        unsafe {
            libc::signal(libc::SIGINT, cancel as libc::sighandler_t);
            libc::signal(libc::SIGTERM, cancel as libc::sighandler_t);
        }
        let start = std::time::Instant::now();
        let bytes =
            read_host::acquire_control_input(&CANCEL, start, profile.wall_millis).map_err(|e| {
                if e == "Cancelled" {
                    "cancelled"
                } else if e == "ControlsUnavailable" {
                    "controls_unavailable"
                } else {
                    "resource_exceeded"
                }
            })?;
        research_register::parse::document("request", &bytes, &mut vec![]).ok_or("bad_request")?;
        let request: CardRequest = serde_json::from_slice(&bytes).map_err(|_| "bad_request")?;
        request.validate_scalars()?;
        q = Some(request.current.clone());
        request.validate()?;
        let exe = std::env::current_exe().map_err(|_| "controls_unavailable")?;
        let product = card_host::run(
            &exe,
            Path::new(&args[0]),
            request,
            profile,
            &Installation::unavailable(),
            &CANCEL,
            start,
        )?;
        Ok(product.envelope_utf8.into_bytes())
    })();
    let (bytes, code) = match result {
        Ok(b) => (b, 0),
        Err(e) => {
            let e = if card_wire::OUTCOMES.contains(&e.as_str()) {
                e
            } else {
                "unreadable".into()
            };
            let env = Envelope::refusal(q.clone(), &e);
            (env.bytes().unwrap_or_else(|_| b"{\"card\":null,\"codec\":\"research_a4_json_v1\",\"detail\":\"bad_request\",\"outcome\":\"bad_request\",\"request\":null,\"schema\":\"research_a4_current_v1\"}\n".to_vec()), 4)
        }
    };
    if std::io::stdout().write_all(&bytes).is_err() {
        std::process::exit(4);
    }
    std::process::exit(code);
}
