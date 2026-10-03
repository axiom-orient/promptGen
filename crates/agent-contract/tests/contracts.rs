#![cfg(feature = "serde")]
use agent_contract::{
    compilation::{CompilationOutcome, CompilationStatus, Diagnostic, PromptKind},
    identity::{ArtifactId, ToolCallId},
    json::{self, JsonValue},
    model::{MessageRole, ModelMessage, ReasoningEffort},
    provider::{ProviderOperation, ProviderRequest},
};
use std::borrow::Cow;

fn request(messages: &[ModelMessage]) -> ProviderRequest<'_> {
    ProviderRequest {
        schema_version: 2,
        request_id: "request-1".into(),
        operation: ProviderOperation::ModelTurn,
        messages: Cow::Borrowed(messages),
        observations: Cow::Borrowed(&[]),
        staging_root: None,
        tools: Cow::Borrowed(&[]),
        reasoning: ReasoningEffort::High,
        timeout_ms: 1000,
        maximum_response_bytes: 1024,
        prompt: None,
        image_options: None,
    }
}

#[test]
fn borrowed_producer_and_owned_consumer_use_identical_wire_values() {
    let messages = [ModelMessage::text(MessageRole::User, "한국어 prompt")];
    let original = request(&messages);
    assert!(matches!(original.messages, Cow::Borrowed(_)));
    original.validate_wire().unwrap();
    let bytes = serde_json::to_vec(&original).unwrap();
    let decoded: ProviderRequest<'static> = serde_json::from_slice(&bytes).unwrap();
    assert!(matches!(decoded.messages, Cow::Owned(_)));
    assert_eq!(original, decoded);
    assert_eq!(bytes, serde_json::to_vec(&decoded).unwrap());
}

#[test]
fn common_request_limits_reject_both_operations_at_the_boundary() {
    let mut value = request(&[]);
    for operation in [
        ProviderOperation::ModelTurn,
        ProviderOperation::ImageGenerate,
    ] {
        value.operation = operation;
        value.timeout_ms = 0;
        assert_eq!(value.validate_wire().unwrap_err().code, "invalid_request");
        value.timeout_ms = agent_contract::limits::MAXIMUM_PROVIDER_TIMEOUT_MS;
        value.maximum_response_bytes = agent_contract::limits::MAXIMUM_IMAGE_BYTES;
        assert!(value.validate_wire().is_ok());
        value.maximum_response_bytes += 1;
        assert!(value.validate_wire().is_err());
        value.maximum_response_bytes = 1024;
    }
    value.messages = vec![ModelMessage::text(MessageRole::User, "x"); 129].into();
    assert_eq!(value.validate_wire().unwrap_err().code, "resource_limit");
    value.messages = Cow::Borrowed(&[]);
    value.request_id = "x".repeat(128);
    assert!(value.validate_wire().is_ok());
    value.request_id.push('x');
    assert!(value.validate_wire().is_err());
    value.request_id = "\0".into();
    assert!(value.validate_wire().is_err());
    value.request_id = " ".into();
    assert!(value.validate_wire().is_err());
}

#[test]
fn identifiers_have_one_validation_path_for_rust_and_json() {
    for value in ["", "한국어", "has space", "quote\"", "back\\slash"] {
        assert!(ToolCallId::parse(value).is_err());
        assert!(serde_json::from_value::<ToolCallId>(serde_json::json!(value)).is_err());
    }
    assert!(ToolCallId::parse("call-1/part#2").is_ok());
    assert!(ToolCallId::parse("x".repeat(192)).is_ok());
    assert!(ToolCallId::parse("x".repeat(193)).is_err());
    assert!(ArtifactId::parse("art:a_b.c-1").is_ok());
    assert!(ArtifactId::parse("a/b").is_err());
}

#[test]
fn serde_json_bridge_preserves_canonical_compiler_output() {
    let metadata =
        json::parse(r#"{"n":1.25,"null":null,"items":[true,9223372036854775808]}"#).unwrap();
    let outcome =
        CompilationOutcome::new(PromptKind::Image, Some("compile".into()), vec![], metadata);
    let canonical =
        serde_json::from_str::<serde_json::Value>(&outcome.to_json().to_compact_string()).unwrap();
    assert_eq!(canonical, serde_json::to_value(&outcome).unwrap());
    let decoded: CompilationOutcome = serde_json::from_value(canonical).unwrap();
    assert_eq!(outcome, decoded);
    decoded.validate().unwrap();
}

#[test]
fn duplicate_metadata_and_unknown_wire_fields_fail_closed() {
    assert!(serde_json::from_str::<JsonValue>(r#"{"x":1,"x":2}"#).is_err());
    let mut value = serde_json::to_value(request(&[])).unwrap();
    value["ignoredField"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProviderRequest<'static>>(value).is_err());
}

#[test]
fn compiler_status_requires_matching_diagnostic_evidence() {
    let mut outcome = CompilationOutcome::new(
        PromptKind::Image,
        Some("x".into()),
        vec![Diagnostic::warning("WARN", "$", "warning")],
        JsonValue::object([] as [(&str, JsonValue); 0]),
    );
    assert!(outcome.validate().is_ok());
    outcome.status = CompilationStatus::Valid;
    assert!(outcome.validate().is_err());
    outcome.status = CompilationStatus::Invalid;
    outcome.diagnostics = vec![Diagnostic::error("ERR", "$", "failure")];
    assert!(outcome.validate().is_err());
    outcome.prompt = None;
    assert!(outcome.validate().is_ok());
    outcome.metadata = JsonValue::Null;
    assert!(outcome.validate().is_err());
}

#[cfg(feature = "schema")]
#[test]
fn schema_catalog_has_all_first_party_boundaries_and_is_deterministic() {
    let catalog = agent_contract::schema::catalog();
    assert_eq!(catalog["schema"], "agent.shared-contract/1");
    assert_eq!(catalog["providerProtocol"], "vergerail.upagent/2");
    for name in [
        "providerRequest",
        "modelResponse",
        "imageResponse",
        "failureResponse",
        "artifactRef",
        "compilation",
        "semanticEditReceipt",
    ] {
        assert!(catalog["schemas"][name].is_object(), "{name}");
    }
    assert_eq!(
        agent_contract::schema::catalog_json(),
        agent_contract::schema::catalog_json()
    );
    let request = &catalog["schemas"]["providerRequest"]["properties"];
    let id = &catalog["schemas"]["modelResponse"]["definitions"]["ToolCall"]["properties"]["id"];
    assert_eq!(id["minLength"], 1);
    assert_eq!(id["maxLength"], 192);
    assert!(id["pattern"].as_str().unwrap().contains("[\\s\\S]"));
    assert_eq!(
        catalog["schemas"]["artifactRef"]["properties"]["id"]["maxLength"],
        160
    );
    assert_eq!(
        request["messages"]["maxItems"],
        agent_contract::limits::MAXIMUM_MODEL_MESSAGES_PER_TURN
    );
    assert_eq!(
        request["tools"]["maxItems"],
        agent_contract::limits::MAXIMUM_MODEL_TOOLS_PER_TURN
    );
    assert_eq!(
        request["timeoutMs"]["maximum"].as_f64(),
        Some(agent_contract::limits::MAXIMUM_PROVIDER_TIMEOUT_MS as f64)
    );
    assert_eq!(
        request["maximumResponseBytes"]["maximum"].as_f64(),
        Some(agent_contract::limits::MAXIMUM_MODEL_RESPONSE_BYTES as f64)
    );
}

#[test]
fn semantic_evidence_has_one_strict_serialization_and_schema_shape() {
    use agent_contract::receipt::PromptRefinementEvidence;
    let evidence = PromptRefinementEvidence {
        provider: "codex-cli".into(),
        model: "model".into(),
        review_summary: "approved".into(),
        additions: vec![],
        source_prompt_sha256: "a".repeat(64),
        refined_prompt_sha256: "a".repeat(64),
        source_prompt_chars: 1,
        refined_prompt_chars: 1,
        prompt: "x".into(),
    };
    let manual =
        serde_json::from_str::<serde_json::Value>(&evidence.to_json().to_compact_string()).unwrap();
    assert_eq!(manual, serde_json::to_value(&evidence).unwrap());
    let mut invalid = manual.clone();
    invalid["approved"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PromptRefinementEvidence>(invalid).is_err());
    let mut missing = manual;
    missing
        .as_object_mut()
        .unwrap()
        .remove("source_prompt_sha256");
    assert!(serde_json::from_value::<PromptRefinementEvidence>(missing).is_err());
}
