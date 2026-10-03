use std::fmt;

/// Rejection of an identifier outside its stable wire grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifierError {
    Invalid { label: &'static str, maximum: usize },
}
impl fmt::Display for IdentifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Invalid { label, maximum } = self;
        write!(
            f,
            "{label} must be 1..={maximum} bytes and contain only stable identifier characters"
        )
    }
}
impl std::error::Error for IdentifierError {}

macro_rules! identifier {
    ($name:ident, $label:literal, $maximum:literal, $pattern:literal, $valid:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(String);
        #[cfg(feature = "schema")]
        impl schemars::JsonSchema for $name {
            fn schema_name() -> String {
                stringify!($name).to_owned()
            }
            fn is_referenceable() -> bool {
                false
            }
            fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
                schemars::schema::SchemaObject {
                    instance_type: Some(schemars::schema::InstanceType::String.into()),
                    string: Some(Box::new(schemars::schema::StringValidation {
                        min_length: Some(1),
                        max_length: Some($maximum),
                        pattern: Some($pattern.to_owned()),
                    })),
                    ..Default::default()
                }
                .into()
            }
        }
        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                let valid: fn(&str) -> bool = $valid;
                if value.is_empty() || value.len() > $maximum || !valid(&value) {
                    return Err(IdentifierError::Invalid {
                        label: $label,
                        maximum: $maximum,
                    });
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn into_inner(self) -> String {
                self.0
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                self.as_str()
            }
        }
        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let value = <String as serde::Deserialize>::deserialize(d)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}
identifier!(
    ArtifactId,
    "artifact_id",
    160,
    r"^[A-Za-z0-9_.:-]+(?![\s\S])",
    |value| value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
);
identifier!(
    ToolCallId,
    "tool_call_id",
    192,
    r#"^[!#-&(-\[\]-~]+(?![\s\S])"#,
    |value| value.trim() == value
        && value
            .bytes()
            .all(|b| (33..=126).contains(&b) && !matches!(b, b'"' | b'\'' | b'\\'))
);
