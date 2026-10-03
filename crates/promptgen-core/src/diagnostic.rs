use crate::json::JsonValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub path: String,
    pub message: String,
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
pub struct CompilationOutcome {
    pub kind: PromptKind,
    pub status: CompilationStatus,
    pub prompt: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
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
