use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::OnceLock;

use crate::json::{
    DecodeError, JsonValue, into_array, into_object, parse, reject_unknown, take_bool,
    take_optional_string, take_optional_string_array, take_required, take_string,
    take_string_array, take_u64,
};

const CATALOG_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../catalog/image_catalog.json"
));
const CATALOG_SCHEMA_VERSION: u64 = 5;
const EXPECTED_ENTRY_COUNT: usize = 6;
pub const PRIMARY_OUTPUT_TIER_1: &str = "결과물";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirectiveSlot {
    Subject,
    Scene,
    Composition,
    Camera,
    Lighting,
    Color,
    Material,
    Typography,
    Motion,
    Constraint,
}

impl DirectiveSlot {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "subject" => Some(Self::Subject),
            "scene" => Some(Self::Scene),
            "composition" => Some(Self::Composition),
            "camera" => Some(Self::Camera),
            "lighting" => Some(Self::Lighting),
            "color" => Some(Self::Color),
            "material" => Some(Self::Material),
            "typography" => Some(Self::Typography),
            "motion" => Some(Self::Motion),
            "constraint" => Some(Self::Constraint),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Scene => "scene",
            Self::Composition => "composition",
            Self::Camera => "camera",
            Self::Lighting => "lighting",
            Self::Color => "color",
            Self::Material => "material",
            Self::Typography => "typography",
            Self::Motion => "motion",
            Self::Constraint => "constraint",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptDirective {
    pub slot: DirectiveSlot,
    pub text: String,
}

impl PromptDirective {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let slot_value = take_string(&mut fields, "slot", path)?;
        let slot = DirectiveSlot::parse(&slot_value).ok_or_else(|| {
            DecodeError::new(
                format!("{path}.slot"),
                format!("unknown directive slot {slot_value:?}"),
            )
        })?;
        let result = Self {
            slot,
            text: take_string(&mut fields, "text", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("slot", JsonValue::from(self.slot.as_str())),
            ("text", JsonValue::from(self.text.clone())),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextRequirement {
    Optional,
    Recommended,
    Required,
}

impl TextRequirement {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "optional" => Some(Self::Optional),
            "recommended" => Some(Self::Recommended),
            "required" => Some(Self::Required),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Optional => "optional",
            Self::Recommended => "recommended",
            Self::Required => "required",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceTextMode {
    None,
    BlankZone,
    SampleCopy,
}

impl ReferenceTextMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "blank_zone" => Some(Self::BlankZone),
            "sample_copy" => Some(Self::SampleCopy),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::BlankZone => "blank_zone",
            Self::SampleCopy => "sample_copy",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub id: String,
    pub tier_1: String,
    pub tier_2: String,
    pub tier_3: Option<String>,
    pub asset_path: String,
    pub name_ko: String,
    pub name_en: String,
    pub intent: String,
    pub recipe: Vec<String>,
    pub prompt_directives: Vec<PromptDirective>,
    pub profiles: Vec<String>,
    pub text_requirement: TextRequirement,
    pub reference_text_mode: ReferenceTextMode,
    pub reference_exact_text: Vec<String>,
    pub repetition_text: bool,
    pub suggested_aspect_ratios: Vec<String>,
    pub default_safety_tier: u8,
    pub source_path: String,
    pub source_sha256: String,
}

impl CatalogEntry {
    /// A primary result is selected once at the first catalog level.
    pub fn is_primary_output(&self) -> bool {
        self.tier_1 == PRIMARY_OUTPUT_TIER_1
    }

    /// The English name as prose, or `None` when it carries no information.
    ///
    /// Catalog authoring may use an ID echo or a snake_case slug in `name_en`.
    /// User-facing and model-facing text must be prose, so ID echoes are omitted and
    /// underscores are converted to spaces.
    pub fn display_name_en(&self) -> Option<String> {
        if self.name_en.is_empty() || self.name_en == self.id {
            return None;
        }
        Some(self.name_en.replace('_', " "))
    }

    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let id = take_string(&mut fields, "id", path)?;
        let default_safety_tier = take_u64(&mut fields, "default_safety_tier", path)?;
        let default_safety_tier = u8::try_from(default_safety_tier).map_err(|_| {
            DecodeError::new(
                format!("{path}.default_safety_tier"),
                "value does not fit in u8",
            )
        })?;
        let prompt_directive_values = into_array(
            take_required(&mut fields, "prompt_directives", path)?,
            &format!("{path}.prompt_directives"),
        )?;
        let prompt_directives = prompt_directive_values
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                PromptDirective::from_json(value, &format!("{path}.prompt_directives[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let text_requirement_value = take_string(&mut fields, "text_requirement", path)?;
        let text_requirement =
            TextRequirement::parse(&text_requirement_value).ok_or_else(|| {
                DecodeError::new(
                    format!("{path}.text_requirement"),
                    format!("unknown text requirement {text_requirement_value:?}"),
                )
            })?;
        let reference_text_mode_value = take_string(&mut fields, "reference_text_mode", path)?;
        let reference_text_mode =
            ReferenceTextMode::parse(&reference_text_mode_value).ok_or_else(|| {
                DecodeError::new(
                    format!("{path}.reference_text_mode"),
                    format!("unknown reference text mode {reference_text_mode_value:?}"),
                )
            })?;
        let result = Self {
            id,
            tier_1: take_string(&mut fields, "tier_1", path)?,
            tier_2: take_string(&mut fields, "tier_2", path)?,
            tier_3: take_optional_string(&mut fields, "tier_3", path)?,
            asset_path: take_string(&mut fields, "asset_path", path)?,
            name_ko: take_string(&mut fields, "name_ko", path)?,
            name_en: take_string(&mut fields, "name_en", path)?,
            intent: take_string(&mut fields, "intent", path)?,
            recipe: take_string_array(&mut fields, "recipe", path)?,
            prompt_directives,
            profiles: take_optional_string_array(&mut fields, "profiles", path)?
                .unwrap_or_default(),
            text_requirement,
            reference_text_mode,
            reference_exact_text: take_string_array(&mut fields, "reference_exact_text", path)?,
            repetition_text: take_bool(&mut fields, "repetition_text", path)?,
            suggested_aspect_ratios: take_string_array(
                &mut fields,
                "suggested_aspect_ratios",
                path,
            )?,
            default_safety_tier,
            source_path: take_string(&mut fields, "source_path", path)?,
            source_sha256: take_string(&mut fields, "source_sha256", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("asset_path", JsonValue::from(self.asset_path.clone())),
            (
                "default_safety_tier",
                JsonValue::from(u64::from(self.default_safety_tier)),
            ),
            ("id", JsonValue::from(self.id.clone())),
            ("intent", JsonValue::from(self.intent.clone())),
            ("name_en", JsonValue::from(self.name_en.clone())),
            ("name_ko", JsonValue::from(self.name_ko.clone())),
            (
                "prompt_directives",
                JsonValue::array(self.prompt_directives.iter().map(PromptDirective::to_json)),
            ),
            ("profiles", JsonValue::strings(&self.profiles)),
            (
                "reference_exact_text",
                JsonValue::strings(&self.reference_exact_text),
            ),
            (
                "reference_text_mode",
                JsonValue::from(self.reference_text_mode.as_str()),
            ),
            ("source_recipe", JsonValue::strings(&self.recipe)),
            ("repetition_text", JsonValue::from(self.repetition_text)),
            (
                "suggested_aspect_ratios",
                JsonValue::strings(&self.suggested_aspect_ratios),
            ),
            ("source_path", JsonValue::from(self.source_path.clone())),
            ("source_sha256", JsonValue::from(self.source_sha256.clone())),
            ("tier_1", JsonValue::from(self.tier_1.clone())),
            ("tier_2", JsonValue::from(self.tier_2.clone())),
            (
                "tier_3",
                self.tier_3.clone().map_or(JsonValue::Null, JsonValue::from),
            ),
            (
                "text_requirement",
                JsonValue::from(self.text_requirement.as_str()),
            ),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodSource {
    pub repository: String,
    pub artifact_sha256: String,
    pub license: String,
    pub usage: String,
}

impl MethodSource {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            repository: take_string(&mut fields, "repository", path)?,
            artifact_sha256: take_string(&mut fields, "artifact_sha256", path)?,
            license: take_string(&mut fields, "license", path)?,
            usage: take_string(&mut fields, "usage", path)?,
        };
        reject_unknown(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "artifact_sha256",
                JsonValue::from(self.artifact_sha256.clone()),
            ),
            ("license", JsonValue::from(self.license.clone())),
            ("repository", JsonValue::from(self.repository.clone())),
            ("usage", JsonValue::from(self.usage.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogMetadata {
    pub schema_version: u64,
    pub source_repository: String,
    pub source_license: String,
    pub adaptation: String,
    pub method_sources: Vec<MethodSource>,
}

impl CatalogMetadata {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("adaptation", JsonValue::from(self.adaptation.clone())),
            ("schema_version", JsonValue::from(self.schema_version)),
            (
                "source_license",
                JsonValue::from(self.source_license.clone()),
            ),
            (
                "source_repository",
                JsonValue::from(self.source_repository.clone()),
            ),
            (
                "method_sources",
                JsonValue::array(self.method_sources.iter().map(MethodSource::to_json)),
            ),
        ])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Catalog {
    pub metadata: CatalogMetadata,
    pub navigation: JsonValue,
    pub entries: Vec<CatalogEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogError(pub String);

impl fmt::Display for CatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CatalogError {}

#[derive(Debug)]
struct CatalogStore {
    catalog: Catalog,
    entry_index: BTreeMap<String, usize>,
}

static CATALOG_STORE: OnceLock<Result<CatalogStore, CatalogError>> = OnceLock::new();

fn catalog_store() -> Result<&'static CatalogStore, CatalogError> {
    match CATALOG_STORE.get_or_init(parse_catalog_store) {
        Ok(store) => Ok(store),
        Err(error) => Err(error.clone()),
    }
}

pub fn catalog() -> Result<&'static Catalog, CatalogError> {
    Ok(&catalog_store()?.catalog)
}

pub fn find_entry(id: &str) -> Result<Option<&'static CatalogEntry>, CatalogError> {
    let normalized = id.trim().to_ascii_uppercase();
    let store = catalog_store()?;
    let Some(position) = store.entry_index.get(&normalized).copied() else {
        return Ok(None);
    };
    Ok(store.catalog.entries.get(position))
}

pub fn catalog_json() -> Result<JsonValue, CatalogError> {
    let catalog = catalog()?;
    Ok(JsonValue::object([
        (
            "entries",
            JsonValue::array(catalog.entries.iter().map(CatalogEntry::to_json)),
        ),
        ("metadata", catalog.metadata.to_json()),
        ("navigation", catalog.navigation.clone()),
    ]))
}

fn parse_catalog_store() -> Result<CatalogStore, CatalogError> {
    let root = parse(CATALOG_JSON).map_err(|error| CatalogError(error.to_string()))?;
    let mut fields = into_object(root, "$catalog").map_err(decode_error)?;
    let schema_version =
        match take_required(&mut fields, "schema_version", "$catalog").map_err(decode_error)? {
            JsonValue::Number(value) => value.parse::<u64>().map_err(|_| {
                CatalogError("$catalog.schema_version must be an integer".to_owned())
            })?,
            _ => {
                return Err(CatalogError(
                    "$catalog.schema_version must be a number".to_owned(),
                ));
            }
        };
    if schema_version != CATALOG_SCHEMA_VERSION {
        return Err(CatalogError(format!(
            "$catalog.schema_version must be {CATALOG_SCHEMA_VERSION}, found {schema_version}"
        )));
    }
    let mut source = into_object(
        take_required(&mut fields, "source", "$catalog").map_err(decode_error)?,
        "$catalog.source",
    )
    .map_err(decode_error)?;
    let source_repository =
        take_string(&mut source, "repository", "$catalog.source").map_err(decode_error)?;
    let source_license =
        take_string(&mut source, "license", "$catalog.source").map_err(decode_error)?;
    let adaptation =
        take_string(&mut source, "adaptation", "$catalog.source").map_err(decode_error)?;
    reject_unknown(source, "$catalog.source").map_err(decode_error)?;

    let method_source_values = into_array(
        take_required(&mut fields, "method_sources", "$catalog").map_err(decode_error)?,
        "$catalog.method_sources",
    )
    .map_err(decode_error)?;
    let method_sources = method_source_values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            MethodSource::from_json(value, &format!("$catalog.method_sources[{index}]"))
                .map_err(decode_error)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let navigation = into_object(
        take_required(&mut fields, "navigation", "$catalog").map_err(decode_error)?,
        "$catalog.navigation",
    )
    .map(JsonValue::Object)
    .map_err(decode_error)?;

    let entries_value = take_required(&mut fields, "entries", "$catalog").map_err(decode_error)?;
    let values = into_array(entries_value, "$catalog.entries").map_err(decode_error)?;
    let entries = values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            CatalogEntry::from_json(value, &format!("$catalog.entries[{index}]"))
                .map_err(decode_error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    reject_unknown(fields, "$catalog").map_err(decode_error)?;

    validate_catalog(&entries)?;
    let mut entry_index = BTreeMap::new();
    for (position, entry) in entries.iter().enumerate() {
        let normalized = entry.id.to_ascii_uppercase();
        if entry_index.insert(normalized.clone(), position).is_some() {
            return Err(CatalogError(format!(
                "duplicate normalized catalog id {normalized}"
            )));
        }
    }
    Ok(CatalogStore {
        catalog: Catalog {
            metadata: CatalogMetadata {
                schema_version,
                source_repository,
                source_license,
                adaptation,
                method_sources,
            },
            navigation,
            entries,
        },
        entry_index,
    })
}

fn validate_catalog(entries: &[CatalogEntry]) -> Result<(), CatalogError> {
    if entries.len() != EXPECTED_ENTRY_COUNT {
        return Err(CatalogError(format!(
            "catalog must contain {EXPECTED_ENTRY_COUNT} entries, found {}",
            entries.len()
        )));
    }
    let mut ids = BTreeSet::new();
    let mut asset_paths = BTreeSet::new();
    for entry in entries {
        if !ids.insert(entry.id.clone()) {
            return Err(CatalogError(format!("duplicate catalog id {}", entry.id)));
        }
        if entry.id.trim().is_empty()
            || entry.tier_1.trim().is_empty()
            || entry.tier_2.trim().is_empty()
            || entry.name_ko.trim().is_empty()
            || entry.intent.trim().is_empty()
            || entry.source_path.trim().is_empty()
            || entry.asset_path.trim().is_empty()
        {
            return Err(CatalogError(format!(
                "catalog entry {} contains an empty required field",
                entry.id
            )));
        }
        if entry
            .tier_3
            .as_ref()
            .is_some_and(|tier| tier.trim().is_empty())
        {
            return Err(CatalogError(format!(
                "catalog entry {} contains an empty optional tier_3",
                entry.id
            )));
        }
        if !entry.asset_path.starts_with("assets/")
            || !entry.asset_path.ends_with(".png")
            || entry.asset_path.contains("..")
            || !asset_paths.insert(entry.asset_path.clone())
        {
            return Err(CatalogError(format!(
                "catalog entry {} has an invalid or duplicate asset path {}",
                entry.id, entry.asset_path
            )));
        }
        if entry.default_safety_tier > 2 {
            return Err(CatalogError(format!(
                "catalog entry {} has invalid safety tier {}",
                entry.id, entry.default_safety_tier
            )));
        }
        if entry.profiles.is_empty() || !entry.profiles.iter().any(|profile| profile == "standard")
        {
            return Err(CatalogError(format!(
                "catalog entry {} must advertise the standard image profile",
                entry.id
            )));
        }
        for profile in &entry.profiles {
            if !matches!(
                profile.as_str(),
                "standard"
                    | "travel_journal"
                    | "logo_identity"
                    | "app_icon"
                    | "app_web_ui"
                    | "information_design"
                    | "character_pose"
                    | "poomsae_pose"
            ) {
                return Err(CatalogError(format!(
                    "catalog entry {} advertises unknown image profile {}",
                    entry.id, profile
                )));
            }
        }
        if entry.default_safety_tier == 1 && entry.text_requirement != TextRequirement::Required {
            return Err(CatalogError(format!(
                "catalog entry {} defaults to text safety tier 1 but does not require exact text",
                entry.id
            )));
        }
        validate_reference_text(entry)?;
        let minimum_directives = 2;
        if entry.prompt_directives.len() < minimum_directives {
            return Err(CatalogError(format!(
                "catalog entry {} must contain at least {minimum_directives} prompt directives",
                entry.id
            )));
        }
        for (index, directive) in entry.prompt_directives.iter().enumerate() {
            validate_prompt_directive(entry, index, directive)?;
        }
        validate_suggested_tokens(entry)?;
    }
    Ok(())
}

fn validate_reference_text(entry: &CatalogEntry) -> Result<(), CatalogError> {
    let has_empty_line = entry
        .reference_exact_text
        .iter()
        .any(|line| line.trim().is_empty() || line.contains('\n') || line.contains('\r'));
    if has_empty_line {
        return Err(CatalogError(format!(
            "catalog entry {} has an empty or multiline reference exact-text line",
            entry.id
        )));
    }
    match entry.reference_text_mode {
        ReferenceTextMode::None => {
            if !entry.reference_exact_text.is_empty()
                || entry.text_requirement == TextRequirement::Required
            {
                return Err(CatalogError(format!(
                    "catalog entry {} has an invalid none reference-text contract",
                    entry.id
                )));
            }
        }
        ReferenceTextMode::BlankZone => {
            if !entry.reference_exact_text.is_empty()
                || entry.text_requirement != TextRequirement::Required
            {
                return Err(CatalogError(format!(
                    "catalog entry {} has an invalid blank-zone reference-text contract",
                    entry.id
                )));
            }
        }
        ReferenceTextMode::SampleCopy => {
            if entry.reference_exact_text.is_empty()
                || entry.text_requirement != TextRequirement::Required
            {
                return Err(CatalogError(format!(
                    "catalog entry {} has an invalid sample-copy reference-text contract",
                    entry.id
                )));
            }
        }
    }
    Ok(())
}

fn validate_prompt_directive(
    entry: &CatalogEntry,
    index: usize,
    directive: &PromptDirective,
) -> Result<(), CatalogError> {
    let text = directive.text.trim();
    if text.is_empty() {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] is empty",
            entry.id
        )));
    }
    let lower = text.to_lowercase();
    let structural_forbidden = [
        ("{", "placeholder brace"),
        ("}", "placeholder brace"),
        ("주의:", "source note"),
        ("references/", "source path"),
        ("skills/", "source path"),
        ("recipe", "implementation label"),
        ("```", "markdown fence"),
    ];
    // The markers are authored lowercase; the text is lowered once above.
    for (marker, kind) in structural_forbidden {
        if lower.contains(marker) {
            return Err(CatalogError(format!(
                "catalog entry {} prompt_directives[{index}] contains forbidden {kind}: {marker:?}",
                entry.id
            )));
        }
    }
    let provenance_tokens = [
        entry.source_path.as_str(),
        entry.source_sha256.as_str(),
        entry.asset_path.as_str(),
    ];
    for token in provenance_tokens {
        if !token.is_empty() && lower.contains(&token.to_lowercase()) {
            return Err(CatalogError(format!(
                "catalog entry {} prompt_directives[{index}] leaks entry-derived provenance {token:?}",
                entry.id
            )));
        }
    }
    if contains_identifier(text, &entry.id)
        || lower.contains(&format!("directive_{}", entry.id.to_lowercase()))
        || (entry.name_en.contains('_') && lower.contains(&entry.name_en.to_lowercase()))
    {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] leaks an entry-derived identifier",
            entry.id
        )));
    }
    let authoring_verbs = [
        "choose ",
        "declare ",
        "define ",
        "describe ",
        "specify ",
        "state ",
    ];
    if authoring_verbs.iter().any(|verb| lower.starts_with(verb)) {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] starts with an authoring-mode verb instead of an observable image instruction",
            entry.id
        )));
    }
    if lower.starts_with("ar ")
        || lower.contains(" default ar")
        || lower.contains("aspect ratio")
        || contains_aspect_ratio_token(text)
    {
        return Err(CatalogError(format!(
            "catalog entry {} prompt_directives[{index}] attempts to override typed output authority",
            entry.id
        )));
    }
    Ok(())
}

fn contains_identifier(text: &str, identifier: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|token| token.eq_ignore_ascii_case(identifier))
}

fn contains_aspect_ratio_token(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        let token =
            token.trim_matches(|character: char| !character.is_ascii_digit() && character != ':');
        let Some((left, right)) = token.split_once(':') else {
            return false;
        };
        !left.is_empty()
            && !right.is_empty()
            && left.chars().all(|character| character.is_ascii_digit())
            && right.chars().all(|character| character.is_ascii_digit())
    })
}

fn validate_suggested_tokens(entry: &CatalogEntry) -> Result<(), CatalogError> {
    let valid_ratios = ["1:1", "2:3", "3:2", "3:4", "4:5", "4:3", "16:9", "9:16"];
    let mut seen_ratios = BTreeSet::new();
    for ratio in &entry.suggested_aspect_ratios {
        if !valid_ratios.contains(&ratio.as_str()) || !seen_ratios.insert(ratio) {
            return Err(CatalogError(format!(
                "catalog entry {} has invalid or duplicate suggested aspect ratio {ratio:?}",
                entry.id
            )));
        }
    }
    Ok(())
}

fn decode_error(error: DecodeError) -> CatalogError {
    CatalogError(error.to_string())
}

pub fn selected_entries(ids: &[String]) -> Result<Vec<&'static CatalogEntry>, CatalogError> {
    let mut selected = Vec::new();
    for id in ids {
        let entry =
            find_entry(id)?.ok_or_else(|| CatalogError(format!("unknown catalog id {id:?}")))?;
        selected.push(entry);
    }
    Ok(selected)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogSelectionIssue {
    UnknownCategory { id: String },
    CategoryNotPrimary { id: String, tier_1: String },
}

impl fmt::Display for CatalogSelectionIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCategory { id } => write!(formatter, "unknown category id {id:?}"),
            Self::CategoryNotPrimary { id, tier_1 } => write!(
                formatter,
                "{id} belongs to tier_1 {tier_1:?}, not the primary-output tier"
            ),
        }
    }
}

#[derive(Debug)]
pub struct CatalogSelection {
    pub entries: Vec<&'static CatalogEntry>,
    pub issues: Vec<CatalogSelectionIssue>,
}

pub fn inspect_catalog_selection(category: Option<&str>) -> Result<CatalogSelection, CatalogError> {
    let mut entries = Vec::new();
    let mut issues = Vec::new();

    if let Some(category) = category {
        match find_entry(category)? {
            Some(entry) => {
                entries.push(entry);
                if !entry.is_primary_output() {
                    issues.push(CatalogSelectionIssue::CategoryNotPrimary {
                        id: entry.id.clone(),
                        tier_1: entry.tier_1.clone(),
                    });
                }
            }
            None => issues.push(CatalogSelectionIssue::UnknownCategory {
                id: category.to_owned(),
            }),
        }
    }
    Ok(CatalogSelection { entries, issues })
}

pub fn text_guard_is_repetition_aware(entries: &[&CatalogEntry]) -> bool {
    entries.iter().any(|entry| entry.repetition_text)
}

pub fn exact_text_is_required(entries: &[&CatalogEntry]) -> bool {
    entries
        .iter()
        .any(|entry| entry.text_requirement == TextRequirement::Required)
}

pub fn exact_text_is_recommended(entries: &[&CatalogEntry]) -> bool {
    entries.iter().any(|entry| {
        matches!(
            entry.text_requirement,
            TextRequirement::Required | TextRequirement::Recommended
        )
    })
}

pub fn selected_catalog_to_json(entries: &[&CatalogEntry]) -> JsonValue {
    JsonValue::array(entries.iter().map(|entry| entry.to_json()))
}

pub fn catalog_schema() -> JsonValue {
    JsonValue::object([
        ("type", JsonValue::from("object")),
        ("additionalProperties", JsonValue::from(false)),
        ("required", JsonValue::array([JsonValue::from("category")])),
        (
            "properties",
            JsonValue::object([(
                "category",
                JsonValue::object([
                    (
                        "anyOf",
                        JsonValue::array([
                            JsonValue::object([("type", JsonValue::from("string"))]),
                            JsonValue::object([("type", JsonValue::from("null"))]),
                        ]),
                    ),
                    (
                        "description",
                        JsonValue::from("a primary-output catalog ID (tier_1=결과물) or null"),
                    ),
                ]),
            )]),
        ),
    ])
}

#[cfg(test)]
#[path = "catalog/tests.rs"]
mod tests;
