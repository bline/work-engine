//! Incomplete experimental input. These stubs are the tasks, not product code.
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub struct LineSummary {
    pub bytes: usize,
    pub scalars: usize,
    pub lines: usize,
}

pub fn summarize_lines(_input: &str) -> LineSummary {
    todo!("P03: implement the public task contract")
}

pub fn is_portable_relative_path(_input: &str) -> bool {
    todo!("P04: implement the public task contract")
}

pub fn tag_counts(_input: &[&str]) -> BTreeMap<String, u64> {
    todo!("P05 stage 1: implement the public task contract")
}

#[derive(Debug, PartialEq, Eq)]
pub struct CountOverflow;

pub fn merge_counts(
    _left: &BTreeMap<String, u64>,
    _right: &BTreeMap<String, u64>,
) -> Result<BTreeMap<String, u64>, CountOverflow> {
    todo!("P05 stage 2: implement the public task contract")
}
