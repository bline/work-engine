//! The closed, named, versioned rule set for private or secret material (operator decision
//! D3/OQ-2, 2026-11-17). Every snapshot member is scanned line by line, and its path is checked;
//! any hit refuses with the rule id, member path and line number (0 for a path hit). Matched
//! content is never echoed.
//!
//! The rules are deliberately broad and have no allow-list: a false positive refuses the job,
//! which is the safe direction for material leaving to a provider. A changed rule is a new
//! [`RULESET_ID`]. `v2` (H1 review P2-1 (a)–(e)) adds typed, unquoted, whitespace, call/header and
//! bearer forms to `secret-assignment`, more token prefixes, `/var/lib/work-engine` and a
//! word-boundary end to `private-path`, and the rules `sensitive-member-path` and `binary-patch`.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const RULESET_ID: &str = "work-engine.review-hosted-secret-rules.v2";

/// Rule ids, in evaluation order.
pub const RULE_IDS: [&str; 7] = [
    PEM_PRIVATE_KEY,
    CREDENTIAL_TOKEN_PREFIX,
    SECRET_ASSIGNMENT,
    PRIVATE_PATH,
    EMAIL_ADDRESS,
    SENSITIVE_MEMBER_PATH,
    BINARY_PATCH,
];
/// A PEM private key block header (`-----BEGIN … PRIVATE KEY-----`).
pub const PEM_PRIVATE_KEY: &str = "pem-private-key";
/// A credential token prefix with a token-shaped tail: `sk-` (incl. `sk-ant-`, `sk-proj-`), GitHub
/// `ghp_`/`gho_`/`ghu_`/`ghs_`/`ghr_`/`github_pat_`, Slack `xox?-`/`xoxe`/`xapp-`, AWS
/// `AKIA`/`ASIA`, Google `AIza`, GitLab `glpat-`, npm `npm_`, Hugging Face `hf_`, Stripe
/// `sk_live_`/`rk_live_`, a JWT (`eyJ….eyJ`), `AGE-SECRET-KEY-1`, `PuTTY-User-Key-File-`.
pub const CREDENTIAL_TOKEN_PREFIX: &str = "credential-token-prefix";
/// A `password|passwd|secret|token|api[_-]?key` name given a value: a quoted literal (optionally
/// after a type annotation, whitespace allowed inside) or an unquoted token-shaped value to the end
/// of the line; a key/value call or header form (`"x-api-key", "…"`); or `Bearer <token>`.
/// Literals must be non-trivial ([`non_trivial`]); unquoted values must also hold a letter and a
/// digit and be at least 16 characters.
pub const SECRET_ASSIGNMENT: &str = "secret-assignment";
/// A path under `~/.pi`, `~/.codex*`, `~/.claude*`, `~/.local/state/work-engine` (as `~`,
/// `$HOME`, `/root` or `/home/<user>`), `/etc/work-engine` or `/var/lib/work-engine`.
pub const PRIVATE_PATH: &str = "private-path";
/// An e-mail address.
pub const EMAIL_ADDRESS: &str = "email-address";
/// A member whose path names credential material: `.env*`, `*.pem`, `*.key`, `*.p12`, `id_rsa*`,
/// `id_ed25519*`, `.netrc`, `.npmrc`, `auth.json`, `credentials*` (basename, case-insensitive).
pub const SENSITIVE_MEMBER_PATH: &str = "sensitive-member-path";
/// A `GIT binary patch` line: base85 content no other rule can read (e.g. a deleted binary file's
/// old bytes inside `.review/patch.diff`).
pub const BINARY_PATCH: &str = "binary-patch";

/// One hit: rule, member and 1-based line (0 for a path hit). No content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretHit {
    pub rule: String,
    pub path: String,
    pub line: usize,
}

struct Rule {
    id: &'static str,
    pattern: Regex,
    /// Capture group 1 must pass this predicate for the match to count.
    value_filter: Option<fn(&str) -> bool>,
}

const HOME: &str = r#"(?:~|\$HOME|\$\{HOME\}|/root|/home/[^/\s"'`]+)"#;
const NAME: &str = r#"(?i)(?:password|passwd|secret|token|api[_-]?key)[A-Za-z0-9_.-]*"#;

fn rules() -> &'static [Rule] {
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        let re = |p: &str| Regex::new(p).expect("rule pattern compiles");
        let rule = |id, p: &str, value_filter| Rule {
            id,
            pattern: re(p),
            value_filter,
        };
        vec![
            rule(
                PEM_PRIVATE_KEY,
                r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----",
                None,
            ),
            rule(
                CREDENTIAL_TOKEN_PREFIX,
                concat!(
                    r"\b(?:sk-(?:ant-|proj-)?[A-Za-z0-9_-]{20,}",
                    r"|gh[pousr]_[A-Za-z0-9]{30,}",
                    r"|github_pat_[A-Za-z0-9_]{30,}",
                    r"|xox[abprs]-[A-Za-z0-9-]{10,}",
                    r"|xoxe[.-][A-Za-z0-9.-]{10,}",
                    r"|xapp-[A-Za-z0-9-]{10,}",
                    r"|AKIA[0-9A-Z]{16}",
                    r"|ASIA[0-9A-Z]{16}",
                    r"|AIza[0-9A-Za-z_-]{35}",
                    r"|glpat-[A-Za-z0-9_-]{20,}",
                    r"|npm_[A-Za-z0-9]{36}",
                    r"|hf_[A-Za-z0-9]{30,}",
                    r"|[sr]k_live_[A-Za-z0-9]{10,}",
                    r"|eyJ[A-Za-z0-9_-]{10,}\.eyJ",
                    r"|AGE-SECRET-KEY-1[0-9A-Z]{20,})",
                    r"|PuTTY-User-Key-File-"
                ),
                None,
            ),
            // (a) quoted literal, optionally after a type annotation (`NAME: &str = "…"`).
            rule(
                SECRET_ASSIGNMENT,
                &format!(r#"{NAME}["']?\s*(?::[^=:"\n]{{1,60}})?[:=]\s*["']([^"'\n]{{8,}})["']"#),
                Some(non_trivial),
            ),
            // (a) unquoted token-shaped value to the end of the line (`.env`, `export`, YAML).
            rule(
                SECRET_ASSIGNMENT,
                &format!(r#"{NAME}\s*[:=]\s*([A-Za-z0-9+/=_.-]{{16,}})\s*$"#),
                Some(token_shaped),
            ),
            // (a) key/value call or header form: `"x-api-key", "…"`, `("authorization", "…")`.
            rule(
                SECRET_ASSIGNMENT,
                r#"(?i)(?:x-api-key|api[_-]?key|authorization|x-auth-token|token|secret|password)["']\s*,\s*["']([^"'\n]{8,})["']"#,
                Some(non_trivial),
            ),
            // (a) bearer credential.
            rule(SECRET_ASSIGNMENT, r"Bearer\s+[A-Za-z0-9._~+/-]{20,}", None),
            rule(
                PRIVATE_PATH,
                &format!(
                    r#"{HOME}/\.(?:pi|codex|claude)(?:[^A-Za-z0-9_]|$)|{HOME}/\.local/state/work-engine|/etc/work-engine|/var/lib/work-engine"#
                ),
                None,
            ),
            rule(
                EMAIL_ADDRESS,
                r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}",
                None,
            ),
            rule(BINARY_PATCH, r"^GIT binary patch\s*$", None),
        ]
    })
}

/// A literal is trivial when it is a placeholder, an environment-variable name, a template
/// reference or has fewer than four distinct characters.
pub fn non_trivial(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let placeholder = [
        "example",
        "placeholder",
        "changeme",
        "redacted",
        "dummy",
        "your_",
        "your-",
        "xxxx",
        "****",
        "<",
        "${",
        "{{",
    ]
    .iter()
    .any(|p| lower.contains(p));
    let env_name = value
        .bytes()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
    let distinct = value
        .chars()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    !(placeholder || env_name || value.starts_with('$') || distinct < 4)
}

/// Unquoted values: non-trivial and holding at least one letter and one digit.
pub fn token_shaped(value: &str) -> bool {
    non_trivial(value)
        && value.bytes().any(|b| b.is_ascii_alphabetic())
        && value.bytes().any(|b| b.is_ascii_digit())
}

/// (c) Whether a member path names credential material.
pub fn sensitive_path(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    base.starts_with(".env")
        || [".pem", ".key", ".p12"].iter().any(|s| base.ends_with(s))
        || base.starts_with("id_rsa")
        || base.starts_with("id_ed25519")
        || matches!(base.as_str(), ".netrc" | ".npmrc" | "auth.json")
        || base.starts_with("credentials")
}

/// Scans one member: its path, then every line; each (rule, line) is reported once.
pub fn scan(path: &str, text: &str) -> Vec<SecretHit> {
    let mut hits = Vec::new();
    if sensitive_path(path) {
        hits.push(SecretHit {
            rule: SENSITIVE_MEMBER_PATH.to_owned(),
            path: path.to_owned(),
            line: 0,
        });
    }
    for (index, line) in text.lines().enumerate() {
        for rule in rules() {
            let hit = match rule.value_filter {
                None => rule.pattern.is_match(line),
                Some(filter) => rule
                    .pattern
                    .captures_iter(line)
                    .any(|c| c.get(1).is_some_and(|m| filter(m.as_str().trim()))),
            };
            let seen = hits
                .last()
                .is_some_and(|h: &SecretHit| h.rule == rule.id && h.line == index + 1);
            if hit && !seen {
                hits.push(SecretHit {
                    rule: rule.id.to_owned(),
                    path: path.to_owned(),
                    line: index + 1,
                });
            }
        }
    }
    hits
}
