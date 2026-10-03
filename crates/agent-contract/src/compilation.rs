use crate::json::{DecodeError, JsonValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageDetail {
    Auto,
    Low,
    Medium,
    High,
}

impl ImageDetail {
    pub fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(DecodeError::new(path, "expected auto/low/medium/high")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMode {
    Auto,
    Opaque,
}

impl BackgroundMode {
    pub fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "opaque" => Ok(Self::Opaque),
            "transparent" => Err(DecodeError::new(
                path,
                "this product publishes opaque PNGs; transparent output is outside its artifact contract",
            )),
            _ => Err(DecodeError::new(path, "expected auto or opaque")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Opaque => "opaque",
        }
    }
}

/// A validated compilation result can enter a sealed prompt artifact only in these states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SuccessfulCompilationStatus {
    Valid,
    ValidWithWarnings,
}
impl CompilationStatus {
    pub const fn successful(self) -> Option<SuccessfulCompilationStatus> {
        match self {
            Self::Valid => Some(SuccessfulCompilationStatus::Valid),
            Self::ValidWithWarnings => Some(SuccessfulCompilationStatus::ValidWithWarnings),
            Self::Invalid => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub path: String,
    pub message: String,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            path: path.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            path: path.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("code", JsonValue::from(self.code.clone())),
            ("message", JsonValue::from(self.message.clone())),
            ("path", JsonValue::from(self.path.clone())),
            ("severity", JsonValue::from(self.severity.as_str())),
        ];
        if let Some(hint) = &self.hint {
            fields.push(("hint", JsonValue::from(hint.clone())));
        }
        JsonValue::object(fields)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CompilationStatus {
    Valid,
    ValidWithWarnings,
    Invalid,
}

impl CompilationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::ValidWithWarnings => "valid_with_warnings",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum PromptKind {
    Image,
}

impl PromptKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CompilationOutcome {
    pub kind: PromptKind,
    pub status: CompilationStatus,
    pub prompt: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
    #[cfg_attr(
        feature = "schema",
        schemars(with = "std::collections::BTreeMap<String, serde_json::Value>")
    )]
    pub metadata: JsonValue,
}

impl CompilationOutcome {
    pub fn new(
        kind: PromptKind,
        prompt: Option<String>,
        diagnostics: Vec<Diagnostic>,
        metadata: JsonValue,
    ) -> Self {
        let status =
            diagnostics
                .iter()
                .fold(CompilationStatus::Valid, |status, diagnostic| {
                    match (status, diagnostic.severity) {
                        (CompilationStatus::Invalid, _) | (_, Severity::Error) => {
                            CompilationStatus::Invalid
                        }
                        (_, Severity::Warning) => CompilationStatus::ValidWithWarnings,
                    }
                });
        Self {
            kind,
            status,
            prompt: if status == CompilationStatus::Invalid {
                None
            } else {
                prompt
            },
            diagnostics,
            metadata,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.status != CompilationStatus::Invalid
    }

    /// Check the producer's status against its diagnostic evidence.
    pub fn validate(&self) -> Result<(), String> {
        let expected =
            self.diagnostics
                .iter()
                .fold(CompilationStatus::Valid, |status, diagnostic| {
                    match (status, diagnostic.severity) {
                        (CompilationStatus::Invalid, _) | (_, Severity::Error) => {
                            CompilationStatus::Invalid
                        }
                        (_, Severity::Warning) => CompilationStatus::ValidWithWarnings,
                    }
                });
        if self.status != expected {
            return Err("compilation status disagrees with its diagnostic evidence".to_owned());
        }
        if self.metadata.as_object().is_none() {
            return Err("compilation metadata must be an object".to_owned());
        }
        if self.status == CompilationStatus::Invalid && self.prompt.is_some() {
            return Err("invalid compilation must not publish a prompt".to_owned());
        }
        Ok(())
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "diagnostics",
                JsonValue::array(self.diagnostics.iter().map(Diagnostic::to_json)),
            ),
            ("kind", JsonValue::from(self.kind.as_str())),
            ("metadata", self.metadata.clone()),
            (
                "prompt",
                self.prompt
                    .as_ref()
                    .map_or(JsonValue::Null, |prompt| JsonValue::from(prompt.clone())),
            ),
            ("status", JsonValue::from(self.status.as_str())),
        ])
    }

    pub fn to_text(&self) -> String {
        let mut output = String::new();
        if let Some(prompt) = &self.prompt {
            output.push_str(prompt);
            if !prompt.ends_with('\n') {
                output.push('\n');
            }
        }
        if !self.diagnostics.is_empty() {
            output.push_str("\n[diagnostics]\n");
            for diagnostic in &self.diagnostics {
                output.push_str(&format!(
                    "- {} {} {}: {}",
                    diagnostic.severity.as_str(),
                    diagnostic.code,
                    diagnostic.path,
                    diagnostic.message
                ));
                if let Some(hint) = &diagnostic.hint {
                    output.push_str(&format!(" (hint: {hint})"));
                }
                output.push('\n');
            }
        }
        output
    }
}

pub fn compilation_schema() -> JsonValue {
    JsonValue::object([
        (
            "$schema",
            JsonValue::from("https://json-schema.org/draft/2020-12/schema"),
        ),
        ("title", JsonValue::from("CompilationOutcome")),
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                (
                    "diagnostics",
                    JsonValue::object([
                        ("type", JsonValue::from("array")),
                        (
                            "items",
                            JsonValue::object([
                                ("additionalProperties", JsonValue::from(false)),
                                (
                                    "properties",
                                    JsonValue::object([
                                        (
                                            "code",
                                            JsonValue::object([(
                                                "type",
                                                JsonValue::from("string"),
                                            )]),
                                        ),
                                        (
                                            "hint",
                                            JsonValue::object([(
                                                "type",
                                                JsonValue::from("string"),
                                            )]),
                                        ),
                                        (
                                            "message",
                                            JsonValue::object([(
                                                "type",
                                                JsonValue::from("string"),
                                            )]),
                                        ),
                                        (
                                            "path",
                                            JsonValue::object([(
                                                "type",
                                                JsonValue::from("string"),
                                            )]),
                                        ),
                                        (
                                            "severity",
                                            JsonValue::object([(
                                                "enum",
                                                JsonValue::array([
                                                    JsonValue::from("error"),
                                                    JsonValue::from("warning"),
                                                ]),
                                            )]),
                                        ),
                                    ]),
                                ),
                                (
                                    "required",
                                    JsonValue::array([
                                        JsonValue::from("code"),
                                        JsonValue::from("message"),
                                        JsonValue::from("path"),
                                        JsonValue::from("severity"),
                                    ]),
                                ),
                                ("type", JsonValue::from("object")),
                            ]),
                        ),
                    ]),
                ),
                (
                    "kind",
                    JsonValue::object([("enum", JsonValue::array([JsonValue::from("image")]))]),
                ),
                (
                    "metadata",
                    JsonValue::object([("type", JsonValue::from("object"))]),
                ),
                (
                    "prompt",
                    JsonValue::object([(
                        "anyOf",
                        JsonValue::array([
                            JsonValue::object([("type", JsonValue::from("string"))]),
                            JsonValue::object([("type", JsonValue::from("null"))]),
                        ]),
                    )]),
                ),
                (
                    "status",
                    JsonValue::object([(
                        "enum",
                        JsonValue::array([
                            JsonValue::from("valid"),
                            JsonValue::from("valid_with_warnings"),
                            JsonValue::from("invalid"),
                        ]),
                    )]),
                ),
            ]),
        ),
        (
            "required",
            JsonValue::array([
                JsonValue::from("diagnostics"),
                JsonValue::from("kind"),
                JsonValue::from("metadata"),
                JsonValue::from("prompt"),
                JsonValue::from("status"),
            ]),
        ),
        ("type", JsonValue::from("object")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compilation_schema_declares_draft_2020_12() {
        let schema = compilation_schema();
        assert_eq!(
            schema
                .as_object()
                .and_then(|root| root.get("$schema"))
                .and_then(JsonValue::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
    }
}
