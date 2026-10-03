//! Semantic image edit evidence. Approval and artifact authority remain with the host.
use crate::json::JsonValue;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SemanticEditReceipt {
    pub backend: String,
    pub authentication: String,
    pub promptgen_version: String,
    pub agent_model: String,
    pub observed_image_model: Option<String>,
    pub task_mode: String,
    pub backend_profile: String,
    pub requested_detail: String,
    pub reference_sha256: Option<String>,
    pub reference_bytes: Option<u64>,
    pub codex_version: String,
    pub codex_binary_sha256: Option<String>,
    pub event_contract: String,
    pub thread_id: String,
    pub image_call_id: String,
    pub source_artifact: PathBuf,
    pub output_artifact: PathBuf,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub dimensions_normalized: bool,
    pub started_unix_ms: u64,
    pub finished_unix_ms: u64,
    pub process_exit_code: i32,
    pub compiled_prompt_sha256: String,
    pub compiled_prompt_chars: u64,
    pub executed_prompt_sha256: String,
    pub executed_prompt_chars: u64,
    pub prompt_refinement: PromptRefinementEvidence,
    pub fidelity_checks: Vec<FidelityCheck>,
    pub control_notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SubjectCountCheck {
    pub id: String,
    pub expected: u16,
    pub observed: u16,
    pub evidence: String,
    pub instances: Vec<VisibleInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VisibleInstance {
    pub x_percent: u8,
    pub y_percent: u8,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TextFidelityCheck {
    pub id: String,
    pub exact: bool,
    pub observed_text: String,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TextSeparatorCheck {
    pub id: String,
    pub expected_per_line: u8,
    pub observed_per_line: Vec<u8>,
    pub proven: bool,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FidelityCheck {
    pub candidate_sha256: String,
    pub pass: bool,
    pub summary: String,
    pub violations: Vec<String>,
    pub repair_instruction: String,
    pub subject_counts: Vec<SubjectCountCheck>,
    pub text_checks: Vec<TextFidelityCheck>,
    pub text_separator_checks: Vec<TextSeparatorCheck>,
}

impl FidelityCheck {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "candidate_sha256",
                JsonValue::from(self.candidate_sha256.clone()),
            ),
            ("pass", JsonValue::from(self.pass)),
            (
                "text_separator_checks",
                JsonValue::array(self.text_separator_checks.iter().map(|check| {
                    JsonValue::object([
                        ("id", JsonValue::from(check.id.clone())),
                        (
                            "expected_per_line",
                            JsonValue::from(u64::from(check.expected_per_line)),
                        ),
                        (
                            "observed_per_line",
                            JsonValue::array(
                                check
                                    .observed_per_line
                                    .iter()
                                    .map(|count| JsonValue::from(u64::from(*count))),
                            ),
                        ),
                        ("proven", JsonValue::from(check.proven)),
                        ("evidence", JsonValue::from(check.evidence.clone())),
                    ])
                })),
            ),
            (
                "repair_instruction",
                JsonValue::from(self.repair_instruction.clone()),
            ),
            (
                "subject_counts",
                JsonValue::array(self.subject_counts.iter().map(|check| {
                    JsonValue::object([
                        ("evidence", JsonValue::from(check.evidence.clone())),
                        ("expected", JsonValue::from(u64::from(check.expected))),
                        ("id", JsonValue::from(check.id.clone())),
                        (
                            "instances",
                            JsonValue::array(check.instances.iter().map(|instance| {
                                JsonValue::object([
                                    ("evidence", JsonValue::from(instance.evidence.clone())),
                                    ("x_percent", JsonValue::from(u64::from(instance.x_percent))),
                                    ("y_percent", JsonValue::from(u64::from(instance.y_percent))),
                                ])
                            })),
                        ),
                        ("observed", JsonValue::from(u64::from(check.observed))),
                    ])
                })),
            ),
            ("summary", JsonValue::from(self.summary.clone())),
            (
                "text_checks",
                JsonValue::array(self.text_checks.iter().map(|check| {
                    JsonValue::object([
                        ("evidence", JsonValue::from(check.evidence.clone())),
                        ("exact", JsonValue::from(check.exact)),
                        ("id", JsonValue::from(check.id.clone())),
                        (
                            "observed_text",
                            JsonValue::from(check.observed_text.clone()),
                        ),
                    ])
                })),
            ),
            ("violations", JsonValue::strings(&self.violations)),
        ])
    }
}

impl SemanticEditReceipt {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("backend", JsonValue::from(self.backend.clone())),
            (
                "authentication",
                JsonValue::from(self.authentication.clone()),
            ),
            (
                "promptgen_version",
                JsonValue::from(self.promptgen_version.clone()),
            ),
            ("agent_model", JsonValue::from(self.agent_model.clone())),
            ("task_mode", JsonValue::from(self.task_mode.clone())),
            (
                "backend_profile",
                JsonValue::from(self.backend_profile.clone()),
            ),
            (
                "requested_detail",
                JsonValue::from(self.requested_detail.clone()),
            ),
            (
                "observed_image_model",
                self.observed_image_model
                    .clone()
                    .map(JsonValue::from)
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "reference_sha256",
                self.reference_sha256
                    .clone()
                    .map(JsonValue::from)
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "reference_bytes",
                self.reference_bytes
                    .map(JsonValue::from)
                    .unwrap_or(JsonValue::Null),
            ),
            ("codex_version", JsonValue::from(self.codex_version.clone())),
            (
                "codex_binary_sha256",
                self.codex_binary_sha256
                    .clone()
                    .map(JsonValue::from)
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "compiled_prompt_chars",
                JsonValue::from(self.compiled_prompt_chars),
            ),
            (
                "compiled_prompt_sha256",
                JsonValue::from(self.compiled_prompt_sha256.clone()),
            ),
            (
                "executed_prompt_chars",
                JsonValue::from(self.executed_prompt_chars),
            ),
            (
                "executed_prompt_sha256",
                JsonValue::from(self.executed_prompt_sha256.clone()),
            ),
            ("control_notes", JsonValue::strings(&self.control_notes)),
            (
                "event_contract",
                JsonValue::from(self.event_contract.clone()),
            ),
            (
                "dimensions_normalized",
                JsonValue::from(self.dimensions_normalized),
            ),
            (
                "fidelity_checks",
                JsonValue::array(self.fidelity_checks.iter().map(FidelityCheck::to_json)),
            ),
            ("finished_unix_ms", JsonValue::from(self.finished_unix_ms)),
            ("height", JsonValue::from(u64::from(self.height))),
            ("image_call_id", JsonValue::from(self.image_call_id.clone())),
            (
                "output_artifact",
                JsonValue::from(self.output_artifact.display().to_string()),
            ),
            (
                "process_exit_code",
                JsonValue::from(i64::from(self.process_exit_code)),
            ),
            ("prompt_refinement", self.prompt_refinement.to_json()),
            ("sha256", JsonValue::from(self.sha256.clone())),
            (
                "source_artifact",
                JsonValue::from(self.source_artifact.display().to_string()),
            ),
            (
                "source_height",
                JsonValue::from(u64::from(self.source_height)),
            ),
            (
                "source_width",
                JsonValue::from(u64::from(self.source_width)),
            ),
            ("started_unix_ms", JsonValue::from(self.started_unix_ms)),
            ("thread_id", JsonValue::from(self.thread_id.clone())),
            ("width", JsonValue::from(u64::from(self.width))),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptRefinementEvidence {
    pub provider: String,
    pub model: String,
    pub review_summary: String,
    pub additions: Vec<String>,
    pub source_prompt_sha256: String,
    pub refined_prompt_sha256: String,
    pub source_prompt_chars: u64,
    pub refined_prompt_chars: u64,
    pub prompt: String,
}

impl PromptRefinementEvidence {
    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn review_summary(&self) -> &str {
        &self.review_summary
    }

    pub fn additions(&self) -> &[String] {
        &self.additions
    }

    pub fn source_prompt_sha256(&self) -> &str {
        &self.source_prompt_sha256
    }

    pub fn refined_prompt_sha256(&self) -> &str {
        &self.refined_prompt_sha256
    }

    pub const fn source_prompt_chars(&self) -> u64 {
        self.source_prompt_chars
    }

    pub const fn refined_prompt_chars(&self) -> u64 {
        self.refined_prompt_chars
    }

    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("additions", JsonValue::strings(&self.additions)),
            ("model", JsonValue::from(self.model.clone())),
            ("provider", JsonValue::from(self.provider.clone())),
            ("prompt", JsonValue::from(self.prompt.clone())),
            (
                "refined_prompt_chars",
                JsonValue::from(self.refined_prompt_chars),
            ),
            (
                "refined_prompt_sha256",
                JsonValue::from(self.refined_prompt_sha256.clone()),
            ),
            (
                "review_summary",
                JsonValue::from(self.review_summary.clone()),
            ),
            (
                "source_prompt_chars",
                JsonValue::from(self.source_prompt_chars),
            ),
            (
                "source_prompt_sha256",
                JsonValue::from(self.source_prompt_sha256.clone()),
            ),
        ])
    }
}
