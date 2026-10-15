use serde::{Deserialize, Serialize};

/// First controlled synthetic profile. A caller may choose smaller bounds, but
/// an oversized input is explicit ineligibility rather than silent truncation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SemanticPolicy {
    pub id: String,
    pub schema_version: u32,
    pub max_source_items: usize,
    pub max_source_item_bytes: usize,
    pub max_source_total_bytes: usize,
    pub max_control_input_bytes: usize,
    pub max_control_output_bytes: usize,
    pub max_domain_prompt_bytes: usize,
    pub require_isolated_verifier: bool,
    pub allow_retained_verifier_for_successor: bool,
}

impl Default for SemanticPolicy {
    fn default() -> Self {
        Self {
            id: "s6-strict-synthetic-v1".into(),
            schema_version: 1,
            max_source_items: 64,
            max_source_item_bytes: 16 * 1024,
            max_source_total_bytes: 64 * 1024,
            max_control_input_bytes: 64 * 1024,
            max_control_output_bytes: 64 * 1024,
            max_domain_prompt_bytes: 2048,
            require_isolated_verifier: true,
            allow_retained_verifier_for_successor: true,
        }
    }
}

impl SemanticPolicy {
    /// The bounded ordinary retained-task profile admits the full task input
    /// envelope while keeping the S6 synthetic policy unchanged.
    pub fn retained_repository_analysis() -> Self {
        Self {
            id: "retained-repository-analysis-v1".into(),
            max_domain_prompt_bytes: 8192,
            ..Self::default()
        }
    }

    pub fn within_frozen_limits(&self) -> bool {
        self.schema_version == 1
            && matches!(
                self.id.as_str(),
                "s6-strict-synthetic-v1" | "retained-repository-analysis-v1"
            )
            && self.max_source_items > 0
            && self.max_source_items <= 64
            && self.max_source_item_bytes > 0
            && self.max_source_item_bytes <= 16 * 1024
            && self.max_source_total_bytes > 0
            && self.max_source_total_bytes <= 64 * 1024
            && self.max_control_input_bytes > 0
            && self.max_control_input_bytes <= 64 * 1024
            && self.max_control_output_bytes > 0
            && self.max_control_output_bytes <= 64 * 1024
            && self.max_domain_prompt_bytes > 0
            && self.max_domain_prompt_bytes
                <= if self.id == "retained-repository-analysis-v1" {
                    8192
                } else {
                    2048
                }
            && self.require_isolated_verifier
    }
}
