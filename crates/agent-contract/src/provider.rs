//! Provider-neutral wire values. Runtime and admission policy are owned by callers.
use crate::{
    identity::ArtifactId,
    model::{ModelMessage, ModelUsage, ReasoningEffort, ToolCall, ToolDefinition},
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, path::PathBuf};

pub const PROVIDER_PROTOCOL_VERSION: u32 = 2;
pub const PROVIDER_PROTOCOL_SCHEMA: &str = "vergerail.upagent/2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperation {
    ModelTurn,
    ImageGenerate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProviderRequest<'a> {
    #[cfg_attr(feature = "schema", schemars(range(min = 2, max = 2)))]
    pub schema_version: u32,
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = 128)))]
    pub request_id: String,
    pub operation: ProviderOperation,
    #[serde(default)]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<ModelMessage>", length(max = 128))
    )]
    pub messages: Cow<'a, [ModelMessage]>,
    #[serde(default, skip_serializing_if = "slice_is_empty")]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<ProviderObservation>"))]
    pub observations: Cow<'a, [ProviderObservation]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging_root: Option<PathBuf>,
    #[serde(default)]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "Vec<ToolDefinition>", length(max = 64))
    )]
    pub tools: Cow<'a, [ToolDefinition]>,
    pub reasoning: ReasoningEffort,
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 1800000)))]
    pub timeout_ms: u64,
    #[cfg_attr(feature = "schema", schemars(range(min = 1, max = 8388608)))]
    pub maximum_response_bytes: usize,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_options: Option<ImageGenerationRequirements>,
}
fn slice_is_empty<T>(v: &[T]) -> bool {
    v.is_empty()
}

impl ProviderRequest<'_> {
    /// Common wire admission. File authority and backend capability stay with the adapter.
    pub fn validate_wire(&self) -> Result<(), ProviderFailure> {
        let invalid = |message: &str| ProviderFailure {
            code: "invalid_request".to_owned(),
            message: message.to_owned(),
            retryable: false,
        };
        if self.schema_version != PROVIDER_PROTOCOL_VERSION
            || self.request_id.trim().is_empty()
            || self.request_id.contains('\0')
        {
            return Err(invalid(
                "schemaVersion must be 2 and requestId must be non-empty",
            ));
        }
        if self.request_id.len() > 128 {
            return Err(invalid("requestId is too long"));
        }
        if self.timeout_ms == 0 || self.timeout_ms > crate::limits::MAXIMUM_PROVIDER_TIMEOUT_MS {
            return Err(invalid("timeoutMs is outside the bounded provider range"));
        }
        let cap = match self.operation {
            ProviderOperation::ModelTurn => crate::limits::MAXIMUM_MODEL_RESPONSE_BYTES,
            ProviderOperation::ImageGenerate => crate::limits::MAXIMUM_IMAGE_BYTES,
        };
        if self.maximum_response_bytes == 0 || self.maximum_response_bytes > cap {
            return Err(invalid(
                "maximumResponseBytes is outside the bounded provider range",
            ));
        }
        if self.messages.len() > crate::limits::MAXIMUM_MODEL_MESSAGES_PER_TURN
            || self.tools.len() > crate::limits::MAXIMUM_MODEL_TOOLS_PER_TURN
        {
            return Err(ProviderFailure {
                code: "resource_limit".to_owned(),
                message: "messages or tools exceed the bounded provider limit".to_owned(),
                retryable: false,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImageGenerationRequirements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<ImageBackgroundRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSizeRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<ImageQualityRequirement>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImageBackgroundRequirement {
    Auto,
    Transparent,
    Opaque,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ImageQualityRequirement {
    Auto,
    Low,
    Medium,
    High,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ImageSizeRequirement {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "1024x1024")]
    Square,
    #[serde(rename = "1536x1024")]
    Landscape,
    #[serde(rename = "1024x1536")]
    Portrait,
}
impl ImageSizeRequirement {
    pub const fn dimensions(self) -> Option<(u32, u32)> {
        match self {
            Self::Auto => None,
            Self::Square => Some((1024, 1024)),
            Self::Landscape => Some((1536, 1024)),
            Self::Portrait => Some((1024, 1536)),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub sha256: String,
    pub media_type: String,
    pub byte_length: u64,
    pub relative_path: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ImageObservationRole {
    Full,
    Crop,
    Mask,
    AlphaCheckerboard,
    Diff,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ImageObservationDetail {
    Low,
    #[default]
    Auto,
    High,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImageObservation {
    pub artifact: ArtifactRef,
    pub role: ImageObservationRole,
    #[serde(default)]
    pub detail: ImageObservationDetail,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProviderObservation {
    pub message_index: usize,
    pub part_index: usize,
    pub media_type: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub role: ImageObservationRole,
    pub base64: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ModelResponse {
    #[cfg_attr(feature = "schema", schemars(range(min = 2, max = 2)))]
    pub schema_version: u32,
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = 128)))]
    pub request_id: String,
    pub operation: ProviderOperation,
    pub text: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub usage: Option<ModelUsage>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImageResponse {
    #[cfg_attr(feature = "schema", schemars(range(min = 2, max = 2)))]
    pub schema_version: u32,
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = 128)))]
    pub request_id: String,
    pub operation: ProviderOperation,
    pub image: ImagePayload,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImagePayload {
    pub media_type: String,
    pub base64: String,
    pub byte_length: usize,
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transparent_background: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FailureResponse {
    #[cfg_attr(feature = "schema", schemars(range(min = 2, max = 2)))]
    pub schema_version: u32,
    #[cfg_attr(feature = "schema", schemars(length(min = 1, max = 128)))]
    pub request_id: String,
    pub ok: bool,
    pub error: ProviderFailure,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProviderFailure {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
