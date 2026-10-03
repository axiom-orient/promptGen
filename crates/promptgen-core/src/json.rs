use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

const DEFAULT_MAX_BYTES: usize = 16 * 1024 * 1024;
const DEFAULT_MAX_DEPTH: usize = 128;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub fn object(entries: impl IntoIterator<Item = (impl Into<String>, JsonValue)>) -> Self {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn array(values: impl IntoIterator<Item = JsonValue>) -> Self {
        Self::Array(values.into_iter().collect())
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// Builds a JSON array of strings from any borrowed string sequence.
    ///
    /// Every request model serializes string collections, so the conversion
    /// lives on the value type instead of being re-spelled at each call site.
    pub fn strings<'a>(
        values: impl IntoIterator<Item = &'a (impl AsRef<str> + 'a + ?Sized)>,
    ) -> Self {
        Self::Array(
            values
                .into_iter()
                .map(|value| Self::String(value.as_ref().to_owned()))
                .collect(),
        )
    }

    pub fn number(value: impl ToString) -> Self {
        Self::Number(value.to_string())
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub fn to_compact_string(&self) -> String {
        let mut output = String::new();
        write_value(self, &mut output, 0, false).expect("writing to String cannot fail");
        output
    }

    pub fn to_pretty_string(&self) -> String {
        let mut output = String::new();
        write_value(self, &mut output, 0, true).expect("writing to String cannot fail");
        output.push('\n');
        output
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<u64> for JsonValue {
    fn from(value: u64) -> Self {
        Self::Number(value.to_string())
    }
}

impl From<usize> for JsonValue {
    fn from(value: usize) -> Self {
        Self::Number(value.to_string())
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Number(value.to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub message: String,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at line {}, column {} (byte {})",
            self.message, self.line, self.column, self.offset
        )
    }
}

impl std::error::Error for JsonError {}

pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
    parse_with_limits(input, DEFAULT_MAX_BYTES, DEFAULT_MAX_DEPTH)
}

pub fn parse_with_limits(
    input: &str,
    max_bytes: usize,
    max_depth: usize,
) -> Result<JsonValue, JsonError> {
    if input.len() > max_bytes {
        return Err(JsonError {
            message: format!("JSON exceeds maximum size of {max_bytes} bytes"),
            offset: 0,
            line: 1,
            column: 1,
        });
    }
    let mut parser = Parser {
        input,
        position: 0,
        max_depth,
    };
    parser.skip_whitespace();
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.position != input.len() {
        return Err(parser.error("trailing content after JSON value"));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
    max_depth: usize,
}

impl Parser<'_> {
    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        if depth > self.max_depth {
            return Err(self.error("JSON nesting depth exceeded"));
        }
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'n') => self.parse_literal("null", JsonValue::Null),
            Some(b't') => self.parse_literal("true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal("false", JsonValue::Bool(false)),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(_) => Err(self.error("unexpected character while parsing JSON value")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: JsonValue) -> Result<JsonValue, JsonError> {
        if self.input[self.position..].starts_with(literal) {
            self.position += literal.len();
            Ok(value)
        } else {
            Err(self.error(&format!("expected {literal}")))
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect_byte(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.consume_byte(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
            self.skip_whitespace();
            if self.peek_byte() == Some(b']') {
                return Err(self.error("trailing comma in array"));
            }
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect_byte(b'{')?;
        self.skip_whitespace();
        let mut fields = BTreeMap::new();
        if self.consume_byte(b'}') {
            return Ok(JsonValue::Object(fields));
        }
        loop {
            if self.peek_byte() != Some(b'"') {
                return Err(self.error("object key must be a JSON string"));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            self.skip_whitespace();
            let value = self.parse_value(depth)?;
            if fields.insert(key.clone(), value).is_some() {
                return Err(self.error(&format!("duplicate object key {key:?}")));
            }
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
            self.skip_whitespace();
            if self.peek_byte() == Some(b'}') {
                return Err(self.error("trailing comma in object"));
            }
        }
        Ok(JsonValue::Object(fields))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect_byte(b'"')?;
        let mut output = String::new();
        loop {
            let Some(byte) = self.peek_byte() else {
                return Err(self.error("unterminated JSON string"));
            };
            match byte {
                b'"' => {
                    self.position += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.position += 1;
                    let escaped = self
                        .next_byte()
                        .ok_or_else(|| self.error("unterminated escape sequence"))?;
                    match escaped {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{0008}'),
                        b'f' => output.push('\u{000C}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let first = self.parse_hex_quad()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if self.next_byte() != Some(b'\\') || self.next_byte() != Some(b'u')
                                {
                                    return Err(self.error(
                                        "high surrogate must be followed by a low surrogate",
                                    ));
                                }
                                let second = self.parse_hex_quad()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(self.error("invalid low surrogate"));
                                }
                                0x10000
                                    + (((first as u32 - 0xD800) << 10) | (second as u32 - 0xDC00))
                            } else if (0xDC00..=0xDFFF).contains(&first) {
                                return Err(self.error("unexpected low surrogate"));
                            } else {
                                first as u32
                            };
                            let character = char::from_u32(scalar)
                                .ok_or_else(|| self.error("invalid Unicode scalar value"))?;
                            output.push(character);
                        }
                        _ => return Err(self.error("invalid JSON escape sequence")),
                    }
                }
                0x00..=0x1F => {
                    return Err(self.error("unescaped control character in JSON string"));
                }
                0x20..=0x7F => {
                    self.position += 1;
                    output.push(byte as char);
                }
                _ => {
                    let character = self.input[self.position..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error("invalid UTF-8 in JSON string"))?;
                    self.position += character.len_utf8();
                    output.push(character);
                }
            }
        }
    }

    fn parse_hex_quad(&mut self) -> Result<u16, JsonError> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let byte = self
                .next_byte()
                .ok_or_else(|| self.error("truncated Unicode escape"))?;
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.error("non-hexadecimal digit in Unicode escape")),
            };
            value = (value << 4) | u16::from(digit);
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, JsonError> {
        let start = self.position;
        self.consume_byte(b'-');
        match self.peek_byte() {
            Some(b'0') => {
                self.position += 1;
                if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    return Err(self.error("leading zero in JSON number"));
                }
            }
            Some(b'1'..=b'9') => {
                self.position += 1;
                while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(self.error("invalid JSON number")),
        }
        if self.consume_byte(b'.') {
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.error("fraction must contain at least one digit"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(self.error("exponent must contain at least one digit"));
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), JsonError> {
        if self.consume_byte(expected) {
            Ok(())
        } else {
            Err(self.error(&format!("expected {:?}", expected as char)))
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let value = self.peek_byte()?;
        self.position += 1;
        Some(value)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }

    fn error(&self, message: &str) -> JsonError {
        let prefix = &self.input[..self.position.min(self.input.len())];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix
            .rsplit_once('\n')
            .map_or(prefix.chars().count() + 1, |(_, tail)| {
                tail.chars().count() + 1
            });
        JsonError {
            message: message.to_owned(),
            offset: self.position,
            line,
            column,
        }
    }
}

fn write_value(value: &JsonValue, output: &mut String, depth: usize, pretty: bool) -> fmt::Result {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        JsonValue::Number(value) => output.push_str(value),
        JsonValue::String(value) => write_string(value, output)?,
        JsonValue::Array(values) => {
            output.push('[');
            if !values.is_empty() {
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    if pretty {
                        output.push('\n');
                        indent(output, depth + 1);
                    }
                    write_value(value, output, depth + 1, pretty)?;
                }
                if pretty {
                    output.push('\n');
                    indent(output, depth);
                }
            }
            output.push(']');
        }
        JsonValue::Object(fields) => {
            output.push('{');
            if !fields.is_empty() {
                for (index, (key, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    if pretty {
                        output.push('\n');
                        indent(output, depth + 1);
                    }
                    write_string(key, output)?;
                    output.push(':');
                    if pretty {
                        output.push(' ');
                    }
                    write_value(value, output, depth + 1, pretty)?;
                }
                if pretty {
                    output.push('\n');
                    indent(output, depth);
                }
            }
            output.push('}');
        }
    }
    Ok(())
}

fn write_string(value: &str, output: &mut String) -> fmt::Result {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000C}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{0000}'..='\u{001F}' => write!(output, "\\u{:04X}", character as u32)?,
            _ => output.push(character),
        }
    }
    output.push('"');
    Ok(())
}

fn indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str("  ");
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    pub path: String,
    pub message: String,
}

impl DecodeError {
    pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for DecodeError {}

pub fn into_object(
    value: JsonValue,
    path: &str,
) -> Result<BTreeMap<String, JsonValue>, DecodeError> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(DecodeError::new(path, "expected object")),
    }
}

pub fn into_array(value: JsonValue, path: &str) -> Result<Vec<JsonValue>, DecodeError> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(DecodeError::new(path, "expected array")),
    }
}

pub fn take_required(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<JsonValue, DecodeError> {
    fields
        .remove(key)
        .ok_or_else(|| DecodeError::new(format!("{path}.{key}"), "required field is missing"))
}

pub fn take_optional(fields: &mut BTreeMap<String, JsonValue>, key: &str) -> Option<JsonValue> {
    match fields.remove(key) {
        Some(JsonValue::Null) | None => None,
        value => value,
    }
}

pub fn take_string(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<String, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_string(value, &format!("{path}.{key}"))
}

pub fn take_optional_string(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<String>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_string(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_bool(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<bool, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_bool(value, &format!("{path}.{key}"))
}

pub fn take_optional_bool(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<bool>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_bool(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_u64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<u64, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_u64(value, &format!("{path}.{key}"))
}

pub fn take_optional_u64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<u64>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_u64(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_f64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<f64, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_f64(value, &format!("{path}.{key}"))
}

pub fn take_optional_f64(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<f64>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_f64(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn take_string_array(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Vec<String>, DecodeError> {
    let value = take_required(fields, key, path)?;
    expect_string_array(value, &format!("{path}.{key}"))
}

pub fn take_optional_string_array(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<Vec<String>>, DecodeError> {
    take_optional(fields, key)
        .map(|value| expect_string_array(value, &format!("{path}.{key}")))
        .transpose()
}

pub fn expect_string(value: JsonValue, path: &str) -> Result<String, DecodeError> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(DecodeError::new(path, "expected string")),
    }
}

pub fn expect_bool(value: JsonValue, path: &str) -> Result<bool, DecodeError> {
    match value {
        JsonValue::Bool(value) => Ok(value),
        _ => Err(DecodeError::new(path, "expected boolean")),
    }
}

pub fn expect_u64(value: JsonValue, path: &str) -> Result<u64, DecodeError> {
    match value {
        JsonValue::Number(value) => value
            .parse::<u64>()
            .map_err(|_| DecodeError::new(path, "expected non-negative integer")),
        _ => Err(DecodeError::new(path, "expected non-negative integer")),
    }
}

pub fn expect_f64(value: JsonValue, path: &str) -> Result<f64, DecodeError> {
    match value {
        JsonValue::Number(value) => value
            .parse::<f64>()
            .ok()
            .filter(|number| number.is_finite())
            .ok_or_else(|| DecodeError::new(path, "expected finite number")),
        _ => Err(DecodeError::new(path, "expected finite number")),
    }
}

pub fn expect_string_array(value: JsonValue, path: &str) -> Result<Vec<String>, DecodeError> {
    let values = into_array(value, path)?;
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| expect_string(value, &format!("{path}[{index}]")))
        .collect()
}

pub fn reject_unknown(fields: BTreeMap<String, JsonValue>, path: &str) -> Result<(), DecodeError> {
    if let Some(key) = fields.keys().next() {
        Err(DecodeError::new(format!("{path}.{key}"), "unknown field"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_round_trips_unicode_and_numbers() {
        let source = r#"{"한글":"문자열","array":[null,true,-12.5e2,"\uD83D\uDE00"]}"#;
        let value = parse(source).expect("valid JSON");
        let reparsed = parse(&value.to_compact_string()).expect("serialized JSON");
        assert_eq!(value, reparsed);
    }

    #[test]
    fn rejects_duplicate_keys() {
        let error = parse(r#"{"a":1,"a":2}"#).expect_err("duplicate key must fail");
        assert!(error.message.contains("duplicate"));
    }

    #[test]
    fn rejects_trailing_commas_and_leading_zero() {
        assert!(parse("[1,]").is_err());
        assert!(parse("{\"a\":1,}").is_err());
        assert!(parse("01").is_err());
    }

    #[test]
    fn enforces_depth_limit() {
        let error = parse_with_limits("[[[0]]]", 100, 1).expect_err("depth must fail");
        assert!(error.message.contains("depth"));
    }

    #[test]
    fn pretty_output_is_deterministic_and_sorted() {
        let value = JsonValue::object([("z", JsonValue::from(1_u64)), ("a", JsonValue::from("x"))]);
        assert_eq!(
            value.to_pretty_string(),
            "{\n  \"a\": \"x\",\n  \"z\": 1\n}\n"
        );
    }
}
