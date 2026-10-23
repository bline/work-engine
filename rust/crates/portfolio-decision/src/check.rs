//! `check --kind portfolio-decision` (contract 4.2): the parse, then every rule in the order of
//! contract 4.2. Checking only reads; it writes nothing and mutates no bound file.

use crate::cross;
use crate::diag::{CheckReport, Failure};
use crate::structure::{parse_portfolio, validate};
use crate::upstream::resolve_all;

/// The `kind` printed in a portfolio-decision report.
pub const KIND_PORTFOLIO: &str = "portfolio-decision";

/// Check the bytes of a portfolio decision. Not JSON is a [`Failure`] (exit 2); every violated
/// rule of a parseable document is listed in the report (exit 1 when any).
pub fn check_portfolio_bytes(bytes: &[u8]) -> Result<CheckReport, Failure> {
    let record = match parse_portfolio(bytes)? {
        Err(errors) => return Ok(CheckReport::new(KIND_PORTFOLIO, errors, Vec::new())),
        Ok(record) => record,
    };
    let mut errors = validate(&record);
    let up = resolve_all(&record);
    errors.extend(up.errors);
    errors.extend(cross::check(&record, &up.packets));
    Ok(CheckReport::new(KIND_PORTFOLIO, errors, Vec::new()))
}
