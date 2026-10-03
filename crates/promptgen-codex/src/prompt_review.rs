use std::collections::BTreeSet;
use std::fmt;

use promptgen_core::image::MAX_RENDERED_PROMPT_CHARS;
use promptgen_core::image_artifact::sha256::{Sha256, hex};
use promptgen_core::json::JsonValue;

pub(super) const MAX_PROMPT_REFINEMENT_ADDITIONS: usize = 6;
const MAX_PROMPT_REFINEMENT_ITEM_CHARS: usize = 400;
const MAX_PROMPT_REVIEW_SUMMARY_CHARS: usize = 1_000;
const MAX_PROVIDER_LABEL_CHARS: usize = 80;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptRefinement {
    provider: String,
    model: String,
    review_summary: String,
    additions: Vec<String>,
    source_prompt_sha256: String,
    refined_prompt_sha256: String,
    source_prompt_chars: u64,
    refined_prompt_chars: u64,
    prompt: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptRefinementError {
    pub message: String,
}

impl PromptRefinementError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PromptRefinementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PromptRefinementError {}

impl PromptRefinement {
    pub(super) fn from_review(
        source_prompt: &str,
        provider: impl Into<String>,
        model: impl Into<String>,
        review_summary: impl Into<String>,
        additions: Vec<String>,
    ) -> Result<Self, PromptRefinementError> {
        let provider = provider.into();
        let model = model.into();
        let review_summary = review_summary.into();
        validate_label(&provider, "provider")?;
        validate_label(&model, "model")?;
        validate_summary(&review_summary)?;
        validate_additions(&additions)?;
        if source_prompt.trim().is_empty() {
            return Err(PromptRefinementError::new(
                "source prompt must not be blank",
            ));
        }

        let prompt = render_refined_prompt(source_prompt, &additions);
        let refined_prompt_chars = prompt.chars().count();
        if refined_prompt_chars > MAX_RENDERED_PROMPT_CHARS {
            return Err(PromptRefinementError::new(format!(
                "refined prompt exceeds the {MAX_RENDERED_PROMPT_CHARS}-character execution budget"
            )));
        }

        Ok(Self {
            provider,
            model,
            review_summary,
            additions,
            source_prompt_sha256: sha256_hex(source_prompt.as_bytes()),
            refined_prompt_sha256: sha256_hex(prompt.as_bytes()),
            source_prompt_chars: source_prompt.chars().count() as u64,
            refined_prompt_chars: refined_prompt_chars as u64,
            prompt,
        })
    }

    pub fn validate_for(&self, source_prompt: &str) -> Result<(), PromptRefinementError> {
        let expected = Self::from_review(
            source_prompt,
            self.provider.clone(),
            self.model.clone(),
            self.review_summary.clone(),
            self.additions.clone(),
        )?;
        if &expected == self {
            Ok(())
        } else {
            Err(PromptRefinementError::new(
                "prompt refinement is not bound to the supplied canonical prompt",
            ))
        }
    }

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

fn validate_label(value: &str, label: &str) -> Result<(), PromptRefinementError> {
    if value.trim().is_empty()
        || value.len() > MAX_PROVIDER_LABEL_CHARS
        || value.chars().any(char::is_control)
    {
        return Err(PromptRefinementError::new(format!(
            "refinement {label} must be nonblank, bounded, and contain no control characters"
        )));
    }
    Ok(())
}

fn validate_summary(value: &str) -> Result<(), PromptRefinementError> {
    if value.trim().is_empty()
        || value.chars().count() > MAX_PROMPT_REVIEW_SUMMARY_CHARS
        || value.chars().any(char::is_control)
    {
        return Err(PromptRefinementError::new(
            "refinement review summary must be nonblank, bounded, and single-line",
        ));
    }
    Ok(())
}

fn validate_additions(additions: &[String]) -> Result<(), PromptRefinementError> {
    if additions.len() > MAX_PROMPT_REFINEMENT_ADDITIONS {
        return Err(PromptRefinementError::new(format!(
            "refinement may contain at most {MAX_PROMPT_REFINEMENT_ADDITIONS} additions"
        )));
    }
    let mut unique = BTreeSet::new();
    for addition in additions {
        if addition.trim() != addition
            || addition.is_empty()
            || addition.chars().count() > MAX_PROMPT_REFINEMENT_ITEM_CHARS
            || addition.chars().any(char::is_control)
        {
            return Err(PromptRefinementError::new(
                "each refinement addition must be trimmed, single-line, and bounded",
            ));
        }
        if !unique.insert(addition) {
            return Err(PromptRefinementError::new(
                "refinement additions must not contain duplicates",
            ));
        }
    }
    Ok(())
}

fn render_refined_prompt(source_prompt: &str, additions: &[String]) -> String {
    if additions.is_empty() {
        return source_prompt.to_owned();
    }
    let mut output = String::with_capacity(source_prompt.len() + additions.len() * 128);
    output.push_str(source_prompt);
    output.push_str("\n\nAdditional visual clarifications:\n");
    for (index, addition) in additions.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        output.push_str("- ");
        output.push_str(addition);
    }
    output
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refinement_keeps_the_canonical_prompt_verbatim_and_binds_both_hashes() {
        let source = "canonical prompt";
        let refinement = PromptRefinement::from_review(
            source,
            "codex-cli",
            "gpt-5.6-luna",
            "The visual contract is coherent.",
            vec!["Clarify the subject edge against the background.".to_owned()],
        )
        .unwrap();

        assert!(refinement.prompt().starts_with(source));
        assert!(
            refinement
                .prompt()
                .contains("Additional visual clarifications:")
        );
        assert_ne!(
            refinement.source_prompt_sha256(),
            refinement.refined_prompt_sha256()
        );
        assert!(refinement.validate_for(source).is_ok());
        assert!(refinement.validate_for("other canonical prompt").is_err());
    }

    #[test]
    fn an_approved_review_with_no_additions_preserves_the_compiled_prompt_exactly() {
        let source = "canonical prompt";
        let refinement = PromptRefinement::from_review(
            source,
            "codex-cli",
            "gpt-5.6-luna",
            "No change is needed.",
            Vec::new(),
        )
        .unwrap();

        assert_eq!(refinement.prompt(), source);
        assert_eq!(
            refinement.source_prompt_sha256(),
            refinement.refined_prompt_sha256()
        );
        assert!(refinement.validate_for(source).is_ok());
    }

    #[test]
    fn refinement_rejects_unbounded_multiline_and_duplicate_additions() {
        let source = "canonical prompt";
        let invalid = [
            vec!["line one\nline two".to_owned()],
            vec!["   ".to_owned()],
            vec!["same".to_owned(), "same".to_owned()],
            vec!["x".repeat(MAX_PROMPT_REFINEMENT_ITEM_CHARS + 1)],
        ];
        for additions in invalid {
            assert!(
                PromptRefinement::from_review(
                    source,
                    "codex-cli",
                    "gpt-5.6-luna",
                    "Review passed.",
                    additions,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn refinement_rejects_blank_provider_identity() {
        assert!(
            PromptRefinement::from_review(
                "canonical prompt",
                "   ",
                "gpt-5.6-luna",
                "Review passed.",
                Vec::new(),
            )
            .is_err()
        );
    }
}
