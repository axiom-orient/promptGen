use std::collections::BTreeMap;

use crate::diagnostic::{CompilationOutcome, CompilationStatus, PromptKind};
use crate::image::ImagePromptRequest;
use crate::json::{
    DecodeError, JsonValue, into_object, reject_unknown, take_optional, take_string,
};

mod image;

const IMAGE_ANSWER_KEYS: &[&str] = &[
    "image.adult_editorial_confirm",
    "image.app_icon_concept",
    "image.aspect_ratio",
    "image.brand_applications",
    "image.campaign_plan",
    "image.card_plan",
    "image.category",
    "image.character_sheet",
    "image.collage_plan",
    "image.composition",
    "image.deck_plan",
    "image.exclusions",
    "image.face",
    "image.formulation",
    "image.hair",
    "image.icon_set",
    "image.information_plan",
    "image.keyart_plan",
    "image.lighting",
    "image.lighting_detail",
    "image.lut",
    "image.medium",
    "image.meta_ui_plan",
    "image.occlusion_plan",
    "image.palette",
    "image.panel_plan",
    "image.product_guide",
    "image.profile",
    "image.detail",
    "image.scene",
    "image.series_plan",
    "image.stage_plan",
    "image.storyboard_plan",
    "image.subject",
    "image.surface",
    "image.text",
    "image.text_mode",
    "image.text_position",
    "image.text_style",
    "image.variants",
    "image.visual_controls",
    "image.task_mode",
    "image.reference_description",
    "image.preserve",
    "image.change_only",
    "image.wardrobe",
    "language",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterviewRequest {
    pub kind: PromptKind,
    pub brief: String,
    pub answers: BTreeMap<String, String>,
}

impl InterviewRequest {
    pub fn from_json(value: JsonValue) -> Result<Self, DecodeError> {
        let path = "$";
        let mut fields = into_object(value, path)?;
        let kind_value = take_string(&mut fields, "kind", path)?;
        let kind = match kind_value.as_str() {
            "image" => PromptKind::Image,
            _ => {
                return Err(DecodeError::new("$.kind", "expected image"));
            }
        };
        let brief = take_string(&mut fields, "brief", path)?;
        if brief.trim().is_empty() {
            return Err(DecodeError::new("$.brief", "brief must not be empty"));
        }
        let answers = match take_optional(&mut fields, "answers") {
            None | Some(JsonValue::Null) => BTreeMap::new(),
            Some(value) => {
                let values = into_object(value, "$.answers")?;
                let mut answers = BTreeMap::new();
                for (key, value) in values {
                    if key.is_empty() || key.trim() != key {
                        return Err(DecodeError::new(
                            "$.answers",
                            format!(
                                "answer key {key:?} must be non-empty and must not contain surrounding whitespace"
                            ),
                        ));
                    }
                    match value {
                        JsonValue::String(value) => {
                            answers.insert(key, value);
                        }
                        _ => {
                            return Err(DecodeError::new(
                                format!("$.answers.{key}"),
                                "interview answers must be strings",
                            ));
                        }
                    }
                }
                if let Some(key) = answers
                    .keys()
                    .find(|key| !IMAGE_ANSWER_KEYS.contains(&key.as_str()))
                {
                    return Err(DecodeError::new(
                        format!("$.answers.{key}"),
                        "unknown image interview answer",
                    ));
                }
                answers
            }
        };
        reject_unknown(fields, path)?;
        Ok(Self {
            kind,
            brief,
            answers,
        })
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "answers",
                JsonValue::Object(
                    self.answers
                        .iter()
                        .map(|(key, value)| (key.clone(), JsonValue::from(value.clone())))
                        .collect(),
                ),
            ),
            ("brief", JsonValue::from(self.brief.clone())),
            ("kind", JsonValue::from(self.kind.as_str())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterviewOption {
    pub value: String,
    pub label: String,
    pub description: String,
}

impl InterviewOption {
    pub fn new(value: &str, label: &str, description: &str) -> Self {
        Self {
            value: value.to_owned(),
            label: label.to_owned(),
            description: description.to_owned(),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("description", JsonValue::from(self.description.clone())),
            ("label", JsonValue::from(self.label.clone())),
            ("value", JsonValue::from(self.value.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterviewQuestion {
    pub id: String,
    pub label: String,
    pub help: String,
    pub control: String,
    pub placeholder: String,
    pub required: bool,
    pub options: Vec<InterviewOption>,
    pub validation_error: Option<String>,
}

impl InterviewQuestion {
    pub fn text(id: &str, label: &str, help: &str, placeholder: &str) -> Self {
        Self {
            id: id.to_owned(),
            label: label.to_owned(),
            help: help.to_owned(),
            control: "text".to_owned(),
            placeholder: placeholder.to_owned(),
            required: true,
            options: Vec::new(),
            validation_error: None,
        }
    }

    pub fn multiline(id: &str, label: &str, help: &str, placeholder: &str) -> Self {
        Self {
            control: "multiline".to_owned(),
            ..Self::text(id, label, help, placeholder)
        }
    }

    pub fn choice(id: &str, label: &str, help: &str, options: Vec<InterviewOption>) -> Self {
        Self {
            id: id.to_owned(),
            label: label.to_owned(),
            help: help.to_owned(),
            control: "choice".to_owned(),
            placeholder: String::new(),
            required: true,
            options,
            validation_error: None,
        }
    }

    pub fn multi_choice(id: &str, label: &str, help: &str, options: Vec<InterviewOption>) -> Self {
        Self {
            id: id.to_owned(),
            label: label.to_owned(),
            help: help.to_owned(),
            control: "multi_choice".to_owned(),
            placeholder: String::new(),
            required: true,
            options,
            validation_error: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_error(mut self, message: impl Into<String>) -> Self {
        self.validation_error = Some(message.into());
        self
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("control", JsonValue::from(self.control.clone())),
            ("help", JsonValue::from(self.help.clone())),
            ("id", JsonValue::from(self.id.clone())),
            ("label", JsonValue::from(self.label.clone())),
            (
                "options",
                JsonValue::array(self.options.iter().map(InterviewOption::to_json)),
            ),
            ("placeholder", JsonValue::from(self.placeholder.clone())),
            ("required", JsonValue::from(self.required)),
            (
                "validation_error",
                self.validation_error
                    .as_ref()
                    .map_or(JsonValue::Null, |value| JsonValue::from(value.clone())),
            ),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterviewStatus {
    NeedsInput,
    Ready,
    Invalid,
}

impl InterviewStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NeedsInput => "needs_input",
            Self::Ready => "ready",
            Self::Invalid => "invalid",
        }
    }
}

/// The typed request an interview produced, kept alongside its serialization.
///
/// The interview must decode the request it built in order to compile it. Dropping
/// the decoded value forced every caller that needed the typed form — the Studio's
/// generation route, for one — to serialize and decode it a second time, and to
/// handle a "ready but nothing to run" case that cannot occur.
#[derive(Clone, Debug, PartialEq)]
pub enum InterviewRequestValue {
    Image(Box<ImagePromptRequest>),
}

/// What a completed interview carries. Borrowed as a set so the three values
/// cannot be observed apart.
#[derive(Clone, Copy, Debug)]
pub struct InterviewReady<'a> {
    pub request_json: &'a JsonValue,
    pub request: &'a InterviewRequestValue,
    pub compilation: &'a CompilationOutcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InterviewOutcome {
    pub status: InterviewStatus,
    pub kind: PromptKind,
    pub normalized_answers: BTreeMap<String, String>,
    pub inferences: Vec<String>,
    pub questions: Vec<InterviewQuestion>,
    pub request: Option<JsonValue>,
    pub request_value: Option<InterviewRequestValue>,
    pub compilation: Option<CompilationOutcome>,
}

impl InterviewOutcome {
    pub fn needs_input(
        kind: PromptKind,
        normalized_answers: BTreeMap<String, String>,
        inferences: Vec<String>,
        questions: Vec<InterviewQuestion>,
    ) -> Self {
        Self {
            status: InterviewStatus::NeedsInput,
            kind,
            normalized_answers,
            inferences,
            questions,
            request: None,
            request_value: None,
            compilation: None,
        }
    }

    pub fn completed(
        kind: PromptKind,
        normalized_answers: BTreeMap<String, String>,
        inferences: Vec<String>,
        request: JsonValue,
        request_value: Option<InterviewRequestValue>,
        compilation: CompilationOutcome,
    ) -> Self {
        let status = if compilation.status == CompilationStatus::Invalid {
            InterviewStatus::Invalid
        } else {
            InterviewStatus::Ready
        };
        Self {
            status,
            kind,
            normalized_answers,
            inferences,
            questions: Vec::new(),
            request: Some(request),
            request_value,
            compilation: Some(compilation),
        }
    }

    /// The compiled result, or `None` while the interview still needs input.
    ///
    /// Callers that need the request and its compilation get them together or not
    /// at all, which is what removes the "ready but empty" branches.
    pub fn ready(&self) -> Option<InterviewReady<'_>> {
        if self.status != InterviewStatus::Ready {
            return None;
        }
        Some(InterviewReady {
            request_json: self.request.as_ref()?,
            request: self.request_value.as_ref()?,
            compilation: self.compilation.as_ref()?,
        })
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "compilation",
                self.compilation
                    .as_ref()
                    .map_or(JsonValue::Null, CompilationOutcome::to_json),
            ),
            ("inferences", JsonValue::strings(&self.inferences)),
            ("kind", JsonValue::from(self.kind.as_str())),
            (
                "normalized_answers",
                JsonValue::Object(
                    self.normalized_answers
                        .iter()
                        .map(|(key, value)| (key.clone(), JsonValue::from(value.clone())))
                        .collect(),
                ),
            ),
            (
                "questions",
                JsonValue::array(self.questions.iter().map(InterviewQuestion::to_json)),
            ),
            ("request", self.request.clone().unwrap_or(JsonValue::Null)),
            ("status", JsonValue::from(self.status.as_str())),
        ])
    }
}

pub fn run_interview(request: &InterviewRequest) -> InterviewOutcome {
    match request.kind {
        PromptKind::Image => image::run(request),
    }
}

pub(crate) fn normalized_answers(request: &InterviewRequest) -> BTreeMap<String, String> {
    request
        .answers
        .iter()
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

pub(crate) fn answer<'a>(answers: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    answers
        .get(key)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn interview_schema() -> JsonValue {
    let answer_schema = JsonValue::object([
        (
            "additionalProperties",
            JsonValue::object([("type", JsonValue::from("string"))]),
        ),
        (
            "propertyNames",
            JsonValue::object([("enum", JsonValue::strings(IMAGE_ANSWER_KEYS))]),
        ),
        ("type", JsonValue::from("object")),
    ]);
    JsonValue::object([
        (
            "$schema",
            JsonValue::from("https://json-schema.org/draft/2020-12/schema"),
        ),
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object([
                ("answers", answer_schema),
                (
                    "brief",
                    JsonValue::object([
                        ("minLength", JsonValue::from(1_u64)),
                        ("pattern", JsonValue::from("\\S")),
                        ("type", JsonValue::from("string")),
                    ]),
                ),
                (
                    "kind",
                    JsonValue::object([
                        ("enum", JsonValue::array([JsonValue::from("image")])),
                        ("type", JsonValue::from("string")),
                    ]),
                ),
            ]),
        ),
        (
            "required",
            JsonValue::array(["kind", "brief"].into_iter().map(JsonValue::from)),
        ),
        (
            "title",
            JsonValue::from("promptGen guided interview request"),
        ),
        ("type", JsonValue::from("object")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn request_rejects_empty_brief_and_noncanonical_answer_keys() {
        for source in [
            r#"{"kind":"image","brief":"   ","answers":{}}"#,
            r#"{"kind":"image","brief":"x","answers":{" image.category ":"C1"}}"#,
            r#"{"kind":"image","brief":"x","answers":{"":"C1"}}"#,
        ] {
            let value = parse(source).expect("JSON");
            assert!(InterviewRequest::from_json(value).is_err(), "{source}");
        }
    }

    #[test]
    fn strict_request_rejects_non_string_answer() {
        let value =
            parse(r#"{"kind":"image","brief":"x","answers":{"image.category":1}}"#).unwrap();
        assert!(InterviewRequest::from_json(value).is_err());
    }

    #[test]
    fn strict_request_rejects_unknown_answer_key() {
        let value =
            parse(r#"{"kind":"image","brief":"x","answers":{"unsupported.answer":"value"}}"#)
                .unwrap();
        let error = InterviewRequest::from_json(value).unwrap_err();
        assert!(error.to_string().contains("unknown image interview answer"));
    }

    #[test]
    fn schema_is_valid_json() {
        let rendered = interview_schema().to_pretty_string();
        assert!(crate::json::parse(&rendered).is_ok());
    }

    #[test]
    fn schema_declares_the_exact_answer_keys_accepted_by_the_runtime_decoder() {
        let schema = interview_schema();
        let properties = schema
            .as_object()
            .and_then(|root| root.get("properties"))
            .and_then(JsonValue::as_object)
            .expect("properties");
        let answer_keys = properties
            .get("answers")
            .and_then(JsonValue::as_object)
            .and_then(|answers| answers.get("propertyNames"))
            .and_then(JsonValue::as_object)
            .and_then(|property_names| property_names.get("enum"))
            .and_then(JsonValue::as_array)
            .expect("allowed answer keys")
            .iter()
            .map(|key| key.as_str().expect("string answer key"))
            .collect::<Vec<_>>();
        assert_eq!(answer_keys, IMAGE_ANSWER_KEYS);
    }

    #[test]
    fn schema_requires_string_answer_values_like_the_runtime_decoder() {
        let schema = interview_schema();
        let properties = schema
            .as_object()
            .and_then(|root| root.get("properties"))
            .and_then(JsonValue::as_object)
            .expect("properties");
        let answer_properties = properties
            .get("answers")
            .and_then(JsonValue::as_object)
            .and_then(|answers| answers.get("additionalProperties"))
            .and_then(JsonValue::as_object)
            .expect("answer value schema");
        assert_eq!(
            answer_properties.get("type").and_then(JsonValue::as_str),
            Some("string")
        );
    }
}
