//! Test-only executable exercising the accepted query API directly.
use session_query::{Profile, Query};
use std::env;
fn main() {
    let path = env::args().nth(1).expect("profile path");
    let bytes =
        session_query::read_file(&path, session_query::INPUT_LIMIT, true).expect("profile file");
    let profile: Profile = serde_json::from_slice(&bytes).expect("profile JSON");
    let q = Query::parse("list_sessions", serde_json::json!({})).expect("query");
    let value = match session_query::query(&profile, &q) {
        Ok(v) => serde_json::json!({"owner_result":v}),
        Err(e) => serde_json::json!({"owner_error":{"code":e}}),
    };
    println!("{value}");
}
