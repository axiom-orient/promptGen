//! Deterministic screen design contract. No provider, tool or approval authority.
use crate::{
    image_artifact::sha256::Sha256,
    json::{JsonValue, parse},
};
use std::collections::{BTreeMap, BTreeSet};

mod decode;
mod render;
use decode::{array, finish, number, object, string};

pub const HANDOFF_SCHEMA: &str = "promptgen-screen-handoff-v1";
/// Inclusive UTF-8 byte bound, shared by compiler ingress and its adapters.
pub const MAX_INPUT_BYTES: usize = 128 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fidelity {
    Wireframe,
    Styled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMode {
    Placeholder,
    ProvidedSynthetic,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataState {
    Empty,
    Loading,
    Populated,
    NoResults,
    Error,
}
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub x: u64,
    pub y: u64,
    pub width: u64,
    pub height: u64,
}
#[derive(Debug, Clone)]
pub struct Region {
    pub id: String,
    pub bounds: Bounds,
    pub state: DataState,
    pub message: String,
    pub rows: Vec<Vec<String>>,
}
/// Product intent is design context, never executable instructions or authority.
#[derive(Debug, Clone)]
pub struct ServiceBrief {
    pub audience: String,
    pub user_task: String,
    pub primary_action: String,
    pub density: String,
}
#[derive(Debug, Clone)]
pub struct Element {
    pub id: String,
    pub region: String,
    pub bounds: Option<Bounds>,
    pub role: String,
    pub text: String,
    pub state: String,
}
#[derive(Debug, Clone)]
pub struct ScreenRequest {
    pub purpose: String,
    pub brief: Option<ServiceBrief>,
    pub theme: BTreeMap<String, String>,
    pub fidelity: Fidelity,
    pub content_mode: ContentMode,
    pub viewport: Bounds,
    pub raster: Bounds,
    pub regions: Vec<Region>,
    pub elements: Vec<Element>,
}

impl ScreenRequest {
    pub fn from_json(input: &str) -> Result<Self, String> {
        if input.len() > MAX_INPUT_BYTES {
            return Err("screen input exceeds 128 KiB".into());
        }
        let mut root = object(parse(input).map_err(|e| e.to_string())?)?;
        if string(&mut root, "schema")? != "promptgen-screen-v1" {
            return Err("unsupported screen schema".into());
        }
        let purpose = string(&mut root, "purpose")?;
        let brief = root
            .remove("brief")
            .map(|v| {
                let mut b = object(v)?;
                let brief = ServiceBrief {
                    audience: string(&mut b, "audience")?,
                    user_task: string(&mut b, "userTask")?,
                    primary_action: string(&mut b, "primaryAction")?,
                    density: string(&mut b, "density")?,
                };
                finish(b)?;
                Ok::<_, String>(brief)
            })
            .transpose()?;
        let mut theme_obj = object(root.remove("theme").ok_or("missing theme")?)?;
        let mut theme = BTreeMap::new();
        for key in ["canvas", "surface", "text", "accent", "border"] {
            theme.insert(key.to_owned(), string(&mut theme_obj, key)?);
        }
        finish(theme_obj)?;
        let fidelity = match string(&mut root, "fidelity")?.as_str() {
            "wireframe" => Fidelity::Wireframe,
            "styled" => Fidelity::Styled,
            _ => return Err("invalid fidelity".into()),
        };
        let content_mode = match string(&mut root, "contentMode")?.as_str() {
            "placeholder" => ContentMode::Placeholder,
            "provided_synthetic" => ContentMode::ProvidedSynthetic,
            _ => return Err("invalid contentMode".into()),
        };
        let viewport = decode::dimensions(root.remove("viewport").ok_or("missing viewport")?)?;
        let raster = decode::dimensions(root.remove("raster").ok_or("missing raster")?)?;
        let mut regions = Vec::new();
        for value in array(&mut root, "regions")? {
            let mut r = object(value)?;
            let id = string(&mut r, "id")?;
            let mut b = object(r.remove("bounds").ok_or("missing bounds")?)?;
            let bounds = Bounds {
                x: number(&mut b, "x")?,
                y: number(&mut b, "y")?,
                width: number(&mut b, "width")?,
                height: number(&mut b, "height")?,
            };
            finish(b)?;
            let state = match string(&mut r, "dataState")?.as_str() {
                "empty" => DataState::Empty,
                "loading" => DataState::Loading,
                "populated" => DataState::Populated,
                "no_results" => DataState::NoResults,
                "error" => DataState::Error,
                _ => return Err("invalid dataState".into()),
            };
            let message = string(&mut r, "stateText")?;
            let rows = array(&mut r, "rows")?
                .into_iter()
                .map(|row| {
                    let cells = row.as_array().ok_or("row must be array")?;
                    if cells.is_empty() || cells.len() > 8 {
                        return Err("row must have 1..8 cells".into());
                    }
                    cells
                        .iter()
                        .map(|c| {
                            c.as_str()
                                .map(str::to_owned)
                                .ok_or_else(|| "cell must be string".into())
                        })
                        .collect::<Result<Vec<_>, String>>()
                })
                .collect::<Result<Vec<_>, _>>()?;
            finish(r)?;
            regions.push(Region {
                id,
                bounds,
                state,
                message,
                rows,
            });
        }
        let mut elements = Vec::new();
        for value in array(&mut root, "elements")? {
            let mut e = object(value)?;
            elements.push(Element {
                id: string(&mut e, "id")?,
                region: string(&mut e, "region")?,
                bounds: e.remove("bounds").map(decode::bounds).transpose()?,
                role: string(&mut e, "role")?,
                text: string(&mut e, "text")?,
                state: string(&mut e, "componentState")?,
            });
            finish(e)?;
        }
        finish(root)?;
        let request = Self {
            purpose,
            brief,
            theme,
            fidelity,
            content_mode,
            viewport,
            raster,
            regions,
            elements,
        };
        request.validate()?;
        Ok(request)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.purpose.trim().is_empty() || !valid_text(&self.purpose) {
            return Err("invalid purpose".into());
        }
        if !(240..=4096).contains(&self.viewport.width)
            || !(240..=4096).contains(&self.viewport.height)
            || !matches!(
                (self.raster.width, self.raster.height),
                (1024, 1024) | (1536, 1024) | (1024, 1536)
            )
        {
            return Err("unsupported viewport or requested raster".into());
        }
        if self.theme.len() != 5
            || ["canvas", "surface", "text", "accent", "border"]
                .iter()
                .any(|k| {
                    self.theme.get(*k).is_none_or(|s| {
                        s.len() != 7
                            || !s.starts_with('#')
                            || !s[1..].bytes().all(|c| c.is_ascii_hexdigit())
                    })
                })
        {
            return Err("theme requires five #RRGGBB tokens".into());
        }
        if self.regions.is_empty()
            || self.regions.len() > 12
            || self.elements.is_empty()
            || self.elements.len() > 64
        {
            return Err("screen region/element count out of bounds".into());
        }
        let mut ids = BTreeSet::new();
        for r in &self.regions {
            if !valid_id(&r.id) || !ids.insert(r.id.clone()) {
                return Err("duplicate or invalid region ID".into());
            }
            let b = r.bounds;
            if b.width == 0
                || b.height == 0
                || b.width > self.viewport.width
                || b.height > self.viewport.height
                || b.x > self.viewport.width - b.width
                || b.y > self.viewport.height - b.height
            {
                return Err("region outside viewport".into());
            }
            if r.rows.len() > 12
                || r.rows.iter().any(|row| row.is_empty() || row.len() > 8)
                || r.rows
                    .iter()
                    .flatten()
                    .any(|s| !valid_text(s) || s.is_empty())
            {
                return Err("invalid supplied rows".into());
            }
            if r.rows.windows(2).any(|w| w[0].len() != w[1].len()) {
                return Err("rows must have consistent column counts".into());
            }
            if !valid_text(&r.message) {
                return Err("invalid stateText".into());
            }
            if r.state == DataState::Populated {
                if r.rows.is_empty()
                    || !r.message.is_empty()
                    || self.content_mode == ContentMode::Placeholder
                {
                    return Err(
                        "populated requires supplied synthetic rows and no stateText".into(),
                    );
                }
            } else if !r.rows.is_empty() {
                return Err("non-populated region cannot contain data".into());
            }
            if matches!(
                r.state,
                DataState::Loading | DataState::NoResults | DataState::Error
            ) && r.message.is_empty()
            {
                return Err("loading/no_results/error requires explicit stateText".into());
            }
            if self.content_mode == ContentMode::Placeholder
                && (r.state != DataState::Empty || !r.message.is_empty())
            {
                return Err("placeholder uses empty geometry, not state text or loading".into());
            }
        }
        if let Some(b) = &self.brief
            && ([&b.audience, &b.user_task]
                .iter()
                .any(|s| s.trim().is_empty() || !valid_text(s))
                || !matches!(b.density.as_str(), "compact" | "comfortable")
                || !self.elements.iter().any(|e| {
                    e.id == b.primary_action && e.role == "button" && e.state != "disabled"
                }))
        {
            return Err("brief requires audience, userTask, compact|comfortable density and an enabled button primaryAction reference".into());
        }
        let regions = ids.clone();
        for e in &self.elements {
            if !valid_id(&e.id) || !ids.insert(e.id.clone()) {
                return Err("duplicate or invalid element ID".into());
            }
            if e.role == "table_header"
                && self.content_mode == ContentMode::ProvidedSynthetic
                && self
                    .regions
                    .iter()
                    .filter(|r| r.id == e.region)
                    .flat_map(|r| &r.rows)
                    .any(|row| row.len() != e.text.split('|').count())
            {
                return Err("table header and row column counts differ".into());
            }
            if !regions.contains(&e.region) {
                return Err("unknown element region reference".into());
            }
            if let Some(b) = e.bounds {
                let parent = self
                    .regions
                    .iter()
                    .find(|r| r.id == e.region)
                    .ok_or("unknown region")?
                    .bounds;
                if b.width == 0
                    || b.height == 0
                    || b.x < parent.x
                    || b.y < parent.y
                    || b.width > parent.width
                    || b.height > parent.height
                    || b.x - parent.x > parent.width - b.width
                    || b.y - parent.y > parent.height - b.height
                {
                    return Err(
                        "element bounds must be inside its region in viewport coordinates".into(),
                    );
                }
            }
            if !matches!(
                e.role.as_str(),
                "brand"
                    | "navigation"
                    | "heading"
                    | "label"
                    | "button"
                    | "input"
                    | "table_header"
                    | "placeholder"
            ) {
                return Err("invalid element role".into());
            }
            if !matches!(
                e.state.as_str(),
                "default" | "selected" | "disabled" | "focused" | "hovered"
            ) {
                return Err("invalid componentState".into());
            }
            if !valid_text(&e.text)
                || (self.content_mode == ContentMode::Placeholder && !e.text.is_empty())
                || (self.content_mode == ContentMode::ProvidedSynthetic
                    && e.text.is_empty()
                    && e.role != "placeholder")
            {
                return Err("element text contradicts contentMode".into());
            }
        }
        Ok(())
    }
    pub fn render(&self) -> Result<String, String> {
        self.validate()?;
        Ok(render::render(self))
    }
}
fn valid_text(s: &str) -> bool {
    s.len() <= 1024
        && !s
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
pub fn sha256(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
/// Hashes bind content; they do not authenticate a compiler or confer authority.
pub fn compile(input: &str) -> Result<JsonValue, String> {
    let prompt = ScreenRequest::from_json(input)?.render()?;
    if prompt.len() > 128 * 1024 {
        return Err("compiled screen prompt exceeds 128 KiB".into());
    }
    Ok(JsonValue::object([
        ("schema", JsonValue::string(HANDOFF_SCHEMA)),
        ("status", JsonValue::string("compiled")),
        ("inputJson", JsonValue::string(input)),
        ("inputSha256", JsonValue::string(sha256(input))),
        ("prompt", JsonValue::string(&prompt)),
        ("promptSha256", JsonValue::string(sha256(&prompt))),
    ]))
}
#[cfg(test)]
mod tests;
