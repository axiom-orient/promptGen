//! The same discoverable schema catalog is emitted by every mechanism.
use serde_json::{Value, json};
pub const CONTRACT_SCHEMA: &str = "agent.shared-contract/1";
pub fn catalog() -> Value {
    json!({
        "schema": CONTRACT_SCHEMA,
        "contractVersion": env!("CARGO_PKG_VERSION"),
        "providerProtocol": crate::provider::PROVIDER_PROTOCOL_SCHEMA,
        "schemas": {
            "providerRequest": schemars::schema_for!(crate::provider::ProviderRequest<'static>),
            "modelResponse": schemars::schema_for!(crate::provider::ModelResponse),
            "imageResponse": schemars::schema_for!(crate::provider::ImageResponse),
            "failureResponse": schemars::schema_for!(crate::provider::FailureResponse),
            "semanticEditReceipt": schemars::schema_for!(crate::receipt::SemanticEditReceipt),
            "artifactRef": schemars::schema_for!(crate::provider::ArtifactRef),
            "compilation": serde_json::from_str::<Value>(&crate::compilation::compilation_schema().to_compact_string()).expect("the canonical compilation schema is valid JSON")
        }
    })
}
pub fn catalog_json() -> String {
    serde_json::to_string_pretty(&catalog()).expect("schema catalog is serializable")
}
pub fn catalog_value() -> crate::json::JsonValue {
    crate::json::parse(&catalog_json()).expect("schema catalog is valid JSON")
}
