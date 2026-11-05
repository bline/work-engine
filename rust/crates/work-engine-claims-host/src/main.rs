use claim_evidence::{
    RepositoryApplication, RepositoryReader, WriteError,
    codec::{JsValue, canonical_json, exact_fields, field, parse_json},
    repository_locator_value,
};
use std::{
    io::{self, Read},
    path::Path,
};
use work_engine_claims_host::TrustedAdmission;
fn run() -> Result<JsValue, String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 || !["initialize", "execute", "inspect", "read"].contains(&args[0].as_str())
    {
        return Err("usage: work-engine-claims-host initialize|execute|inspect|read TRUSTED_STARTUP EXPECTED_STARTUP_SHA256".into());
    }
    let port = TrustedAdmission::load(Path::new(&args[1]), &args[2]).map_err(|e| e.message)?;
    let startup = port.startup().producer().map_err(|e| e.message)?;
    if args[0] == "initialize" {
        RepositoryApplication::initialize(startup, port).map_err(|e| e.message)?;
        return Ok(JsValue::object([("status", JsValue::text("initialized"))]));
    }
    let mut bytes = Vec::new();
    io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1024 * 1024 {
        return Err("request exceeds capacity".into());
    }
    let request = parse_json(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
        .map_err(|e| e.message)?;
    if args[0] == "read" {
        let reader = RepositoryReader::open(startup.read).map_err(|e| e.message)?;
        let action = field(&request, "action")
            .and_then(|v| v.as_text())
            .and_then(|v| v.to_string_checked())
            .map_err(|e| e.message)?;
        return match action.as_str() {
            "resolve" => {
                exact_fields(
                    &request,
                    &["action", "claim_id", "revision_id", "revision_sha256"],
                    "resolve",
                )
                .map_err(|e| e.message)?;
                let text = |key| {
                    field(&request, key)
                        .and_then(|v| v.as_text())
                        .and_then(|v| v.to_string_checked())
                        .map_err(|e| e.message)
                };
                reader
                    .resolve(
                        &text("claim_id")?,
                        &text("revision_id")?,
                        &text("revision_sha256")?,
                    )
                    .map_err(|e| e.message)
            }
            "discover" => {
                exact_fields(
                    &request,
                    &["action", "query", "limit", "cursor"],
                    "discover",
                )
                .map_err(|e| e.message)?;
                let query = field(&request, "query")
                    .and_then(|v| v.as_text())
                    .and_then(|v| v.to_string_checked())
                    .map_err(|e| e.message)?;
                let limit = match field(&request, "limit").map_err(|e| e.message)? {
                    JsValue::Number(n) if *n >= 1.0 && *n <= 100.0 && n.fract() == 0.0 => {
                        *n as usize
                    }
                    _ => return Err("invalid limit".into()),
                };
                let cursor = field(&request, "cursor").map_err(|e| e.message)?;
                reader
                    .discover(
                        &query,
                        limit,
                        if cursor == &JsValue::Null {
                            None
                        } else {
                            Some(cursor)
                        },
                    )
                    .map_err(|e| e.message)
            }
            _ => Err("unsupported read action".into()),
        };
    }
    let mut app = RepositoryApplication::open(startup, port).map_err(|e| e.message)?;
    if args[0] == "inspect" {
        return app.inspect(&request).map_err(|e| e.message);
    }
    let locator = app.locator(&request).ok();
    match app.execute(&request) {
        Ok(result) => Ok(result),
        Err(WriteError::NoEffect(e)) => Ok(JsValue::object([
            ("status", JsValue::text("no_effect")),
            ("error", JsValue::text(&e.message)),
            (
                "locator",
                locator
                    .as_ref()
                    .map_or(JsValue::Null, repository_locator_value),
            ),
        ])),
        Err(WriteError::OutcomeUnknown(locator)) => Ok(JsValue::object([
            ("status", JsValue::text("outcome_unknown")),
            ("locator", repository_locator_value(&locator)),
        ])),
    }
}
fn main() {
    match run() {
        Ok(value) => match canonical_json(&value) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("{}", e.message);
                std::process::exit(1)
            }
        },
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    }
}
