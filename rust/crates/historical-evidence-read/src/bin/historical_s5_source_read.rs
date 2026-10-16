//! Bounded owner-contract reads for one admitted historical S5 case.
#[path = "../source_read/mod.rs"]
mod source_read;
use source_read::{Result, grant};
use std::path::Path;

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 6
        || args[0] != "--grant"
        || args[2] != "--grant-sha256"
        || args[4] != "--section"
    {
        return Err(
            "use --grant PATH --grant-sha256 SHA256 --section provider|scheduler|metrics".into(),
        );
    }
    let admitted = grant(Path::new(&args[1]), &args[3])?;
    let observed = match args[5].as_str() {
        "provider" => source_read::provider_capture::inspect(&admitted)?,
        "scheduler" => source_read::scheduler_item::inspect(&admitted)?,
        "metrics" => source_read::frozen_metrics::inspect(&admitted)?,
        _ => return Err("unsupported source section".into()),
    };
    let encoded = serde_json::to_string(&observed).map_err(|e| e.to_string())?;
    if encoded.len() > historical_evidence_wire::MAX_FRAME {
        return Err("source projection too large".into());
    }
    println!("{encoded}");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("historical-s5-source-read: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::source_read::{decimal_scaled, decimal_sum};
    #[test]
    fn literal_decimal_cost_has_no_f64_rounding() {
        let costs = [
            "2.1413145",
            "1.6940937",
            "2.0389467",
            "1.7264928",
            "2.245674",
            "1.107375",
            "2.0792646",
            "2.7213881999999994",
        ];
        assert_eq!(
            decimal_sum(&costs.map(str::to_string)).unwrap(),
            "15.7545494999999994"
        );
        assert!(decimal_scaled("1e10").is_err());
    }
}
