use super::{BTreeMap, Bounds, JsonValue};
pub(super) fn object(v: JsonValue) -> Result<BTreeMap<String, JsonValue>, String> {
    match v {
        JsonValue::Object(o) => Ok(o),
        _ => Err("expected object".into()),
    }
}
pub(super) fn string(m: &mut BTreeMap<String, JsonValue>, k: &str) -> Result<String, String> {
    match m.remove(k) {
        Some(JsonValue::String(s)) => Ok(s),
        _ => Err(format!("{k}: expected string")),
    }
}
pub(super) fn number(m: &mut BTreeMap<String, JsonValue>, k: &str) -> Result<u64, String> {
    m.remove(k)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("{k}: expected unsigned integer"))
}
pub(super) fn array(
    m: &mut BTreeMap<String, JsonValue>,
    k: &str,
) -> Result<Vec<JsonValue>, String> {
    match m.remove(k) {
        Some(JsonValue::Array(a)) => Ok(a),
        _ => Err(format!("{k}: expected array")),
    }
}
pub(super) fn finish(m: BTreeMap<String, JsonValue>) -> Result<(), String> {
    if m.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "unknown fields: {:?}",
            m.keys().collect::<Vec<_>>()
        ))
    }
}
pub(super) fn dimensions(v: JsonValue) -> Result<Bounds, String> {
    let mut m = object(v)?;
    let width = number(&mut m, "width")?;
    let height = number(&mut m, "height")?;
    finish(m)?;
    if !(240..=4096).contains(&width) || !(240..=4096).contains(&height) {
        return Err("dimensions outside 240..4096".into());
    }
    Ok(Bounds {
        x: 0,
        y: 0,
        width,
        height,
    })
}

pub(super) fn bounds(v: JsonValue) -> Result<Bounds, String> {
    let mut m = object(v)?;
    let bounds = Bounds {
        x: number(&mut m, "x")?,
        y: number(&mut m, "y")?,
        width: number(&mut m, "width")?,
        height: number(&mut m, "height")?,
    };
    finish(m)?;
    Ok(bounds)
}
