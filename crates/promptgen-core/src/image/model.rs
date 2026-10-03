use std::collections::BTreeMap;

use crate::common::{
    PromptLanguage, finish_object, take_language, take_object_array, take_optional_object_array,
};
use crate::json::{
    DecodeError, JsonValue, expect_string, into_object, take_bool, take_optional,
    take_optional_bool, take_optional_f64, take_optional_string, take_optional_string_array,
    take_optional_u64, take_required, take_string, take_string_array, take_u64,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualMedium {
    Photo,
    Illustration,
    ThreeD,
    GraphicDesign,
    Mixed,
}

impl VisualMedium {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "photo" => Ok(Self::Photo),
            "illustration" => Ok(Self::Illustration),
            "3d" => Ok(Self::ThreeD),
            "graphic_design" => Ok(Self::GraphicDesign),
            "mixed" => Ok(Self::Mixed),
            _ => Err(DecodeError::new(
                path,
                "expected one of: photo, illustration, 3d, graphic_design, mixed",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Illustration => "illustration",
            Self::ThreeD => "3d",
            Self::GraphicDesign => "graphic_design",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaxonomySelection {
    pub category: Option<String>,
}

impl TaxonomySelection {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let category = match take_optional(&mut fields, "category") {
            None | Some(JsonValue::Null) => None,
            Some(value) => Some(expect_string(value, &format!("{path}.category"))?),
        };
        finish_object(fields, path)?;
        Ok(Self { category })
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([(
            "category",
            self.category
                .as_ref()
                .map_or(JsonValue::Null, |value| JsonValue::from(value.clone())),
        )])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasZone {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl CanvasZone {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "top_left" => Ok(Self::TopLeft),
            "top_center" => Ok(Self::TopCenter),
            "top_right" => Ok(Self::TopRight),
            "middle_left" => Ok(Self::MiddleLeft),
            "center" | "dead_center" => Ok(Self::Center),
            "middle_right" => Ok(Self::MiddleRight),
            "bottom_left" => Ok(Self::BottomLeft),
            "bottom_center" => Ok(Self::BottomCenter),
            "bottom_right" => Ok(Self::BottomRight),
            _ => Err(DecodeError::new(
                path,
                "unknown canvas zone; use top_left/top_center/top_right/middle_left/center/middle_right/bottom_left/bottom_center/bottom_right/custom",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopCenter => "top_center",
            Self::TopRight => "top_right",
            Self::MiddleLeft => "middle_left",
            Self::Center => "center",
            Self::MiddleRight => "middle_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomCenter => "bottom_center",
            Self::BottomRight => "bottom_right",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasPlacement {
    Zone(CanvasZone),
    Custom {
        x_percent: u8,
        y_percent: u8,
        width_percent: u8,
        height_percent: u8,
    },
}

impl CanvasPlacement {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let zone = take_string(&mut fields, "zone", path)?;
        if zone == "custom" {
            let x = take_u8(&mut fields, "x_percent", path)?;
            let y = take_u8(&mut fields, "y_percent", path)?;
            let width = take_u8(&mut fields, "width_percent", path)?;
            let height = take_u8(&mut fields, "height_percent", path)?;
            finish_object(fields, path)?;
            Ok(Self::Custom {
                x_percent: x,
                y_percent: y,
                width_percent: width,
                height_percent: height,
            })
        } else {
            let zone = CanvasZone::parse(&zone, &format!("{path}.zone"))?;
            finish_object(fields, path)?;
            Ok(Self::Zone(zone))
        }
    }

    pub fn to_json(&self) -> JsonValue {
        match self {
            Self::Zone(zone) => JsonValue::object([("zone", JsonValue::from(zone.as_str()))]),
            Self::Custom {
                x_percent,
                y_percent,
                width_percent,
                height_percent,
            } => JsonValue::object([
                (
                    "height_percent",
                    JsonValue::from(u64::from(*height_percent)),
                ),
                ("width_percent", JsonValue::from(u64::from(*width_percent))),
                ("x_percent", JsonValue::from(u64::from(*x_percent))),
                ("y_percent", JsonValue::from(u64::from(*y_percent))),
                ("zone", JsonValue::from("custom")),
            ]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneSpec {
    pub environment: String,
    pub background: String,
    pub atmosphere: String,
    pub time_of_day: Option<String>,
    pub weather: Option<String>,
}

impl SceneSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            environment: take_string(&mut fields, "environment", path)?,
            background: take_string(&mut fields, "background", path)?,
            atmosphere: take_string(&mut fields, "atmosphere", path)?,
            time_of_day: take_optional_string(&mut fields, "time_of_day", path)?,
            weather: take_optional_string(&mut fields, "weather", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        optional_fields(
            vec![
                ("atmosphere", JsonValue::from(self.atmosphere.clone())),
                ("background", JsonValue::from(self.background.clone())),
                ("environment", JsonValue::from(self.environment.clone())),
            ],
            [
                ("time_of_day", self.time_of_day.as_ref()),
                ("weather", self.weather.as_ref()),
            ],
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectSpec {
    pub id: String,
    pub count: u16,
    pub description: String,
    pub placement: CanvasPlacement,
    pub scale: String,
    pub pose: String,
    pub gaze: String,
    pub action: String,
    pub face: Option<String>,
    pub hair: Option<String>,
    pub appearance: Option<String>,
    pub distinguishing_features: Vec<String>,
}

impl SubjectSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let count = take_u64(&mut fields, "count", path)?;
        let count = u16::try_from(count)
            .map_err(|_| DecodeError::new(format!("{path}.count"), "value exceeds u16"))?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            count,
            description: take_string(&mut fields, "description", path)?,
            placement: CanvasPlacement::from_json(
                take_required(&mut fields, "placement", path)?,
                &format!("{path}.placement"),
            )?,
            scale: take_string(&mut fields, "scale", path)?,
            pose: take_string(&mut fields, "pose", path)?,
            gaze: take_string(&mut fields, "gaze", path)?,
            action: take_string(&mut fields, "action", path)?,
            face: take_optional_string(&mut fields, "face", path)?,
            hair: take_optional_string(&mut fields, "hair", path)?,
            appearance: take_optional_string(&mut fields, "appearance", path)?,
            distinguishing_features: take_optional_string_array(
                &mut fields,
                "distinguishing_features",
                path,
            )?
            .unwrap_or_default(),
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        let mut values = vec![
            ("action", JsonValue::from(self.action.clone())),
            ("count", JsonValue::from(u64::from(self.count))),
            ("description", JsonValue::from(self.description.clone())),
            (
                "distinguishing_features",
                JsonValue::strings(&self.distinguishing_features),
            ),
            ("gaze", JsonValue::from(self.gaze.clone())),
            ("id", JsonValue::from(self.id.clone())),
            ("placement", self.placement.to_json()),
            ("pose", JsonValue::from(self.pose.clone())),
            ("scale", JsonValue::from(self.scale.clone())),
        ];
        if let Some(value) = &self.face {
            values.push(("face", JsonValue::from(value.clone())));
        }
        if let Some(value) = &self.hair {
            values.push(("hair", JsonValue::from(value.clone())));
        }
        if let Some(value) = &self.appearance {
            values.push(("appearance", JsonValue::from(value.clone())));
        }
        JsonValue::object(values)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionSpec {
    pub framing: String,
    pub viewpoint: String,
    pub camera_angle: String,
    pub balance: String,
    pub visual_hierarchy: Vec<String>,
    pub depth_layers: Vec<String>,
    pub negative_space: Vec<CanvasZone>,
}

impl CompositionSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let negative_space = take_optional(&mut fields, "negative_space")
            .map(|value| {
                let values = match value {
                    JsonValue::Array(values) => Ok(values),
                    _ => Err(DecodeError::new(
                        format!("{path}.negative_space"),
                        "expected array",
                    )),
                }?;
                values
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let path = format!("{path}.negative_space[{index}]");
                        let value = expect_string(value, &path)?;
                        CanvasZone::parse(&value, &path)
                    })
                    .collect()
            })
            .transpose()?
            .unwrap_or_default();
        let result = Self {
            framing: take_string(&mut fields, "framing", path)?,
            viewpoint: take_string(&mut fields, "viewpoint", path)?,
            camera_angle: take_string(&mut fields, "camera_angle", path)?,
            balance: take_string(&mut fields, "balance", path)?,
            visual_hierarchy: take_string_array(&mut fields, "visual_hierarchy", path)?,
            depth_layers: take_string_array(&mut fields, "depth_layers", path)?,
            negative_space,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("balance", JsonValue::from(self.balance.clone())),
            ("camera_angle", JsonValue::from(self.camera_angle.clone())),
            ("depth_layers", JsonValue::strings(&self.depth_layers)),
            ("framing", JsonValue::from(self.framing.clone())),
            (
                "negative_space",
                JsonValue::array(
                    self.negative_space
                        .iter()
                        .map(|zone| JsonValue::from(zone.as_str())),
                ),
            ),
            ("viewpoint", JsonValue::from(self.viewpoint.clone())),
            (
                "visual_hierarchy",
                JsonValue::strings(&self.visual_hierarchy),
            ),
        ])
    }
}

/// A textless, designer-facing visual sequence. This is intentionally distinct
/// from a generic multi-panel illustration: every panel has an explicit camera
/// and eyeline responsibility, while identity and screen direction are shared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CinematicStoryboardPanel {
    pub id: String,
    pub shot_size: String,
    pub camera_angle: String,
    pub camera_move: String,
    pub action: String,
    pub body_facing: String,
    pub gaze_target: String,
    pub emotional_beat: String,
}

impl CinematicStoryboardPanel {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            shot_size: take_string(&mut fields, "shot_size", path)?,
            camera_angle: take_string(&mut fields, "camera_angle", path)?,
            camera_move: take_string(&mut fields, "camera_move", path)?,
            action: take_string(&mut fields, "action", path)?,
            body_facing: take_string(&mut fields, "body_facing", path)?,
            gaze_target: take_string(&mut fields, "gaze_target", path)?,
            emotional_beat: take_string(&mut fields, "emotional_beat", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("action", JsonValue::from(self.action.clone())),
            ("body_facing", JsonValue::from(self.body_facing.clone())),
            ("camera_angle", JsonValue::from(self.camera_angle.clone())),
            ("camera_move", JsonValue::from(self.camera_move.clone())),
            (
                "emotional_beat",
                JsonValue::from(self.emotional_beat.clone()),
            ),
            ("gaze_target", JsonValue::from(self.gaze_target.clone())),
            ("id", JsonValue::from(self.id.clone())),
            ("shot_size", JsonValue::from(self.shot_size.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CinematicStoryboardSpec {
    pub layout: String,
    pub screen_direction: String,
    pub identity_anchors: Vec<String>,
    pub panels: Vec<CinematicStoryboardPanel>,
}

impl CinematicStoryboardSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let panels = take_object_array(
            &mut fields,
            "panels",
            path,
            CinematicStoryboardPanel::from_json,
        )?;
        let result = Self {
            layout: take_string(&mut fields, "layout", path)?,
            screen_direction: take_string(&mut fields, "screen_direction", path)?,
            identity_anchors: take_string_array(&mut fields, "identity_anchors", path)?,
            panels,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "identity_anchors",
                JsonValue::strings(&self.identity_anchors),
            ),
            ("layout", JsonValue::from(self.layout.clone())),
            (
                "panels",
                JsonValue::array(self.panels.iter().map(CinematicStoryboardPanel::to_json)),
            ),
            (
                "screen_direction",
                JsonValue::from(self.screen_direction.clone()),
            ),
        ])
    }
}

/// Cross-image invariants distilled from the multi-image generation research
/// taxonomy. The enum is deliberately provider-neutral: it describes what
/// must remain stable, not which model or paper is used to achieve it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageConsistencyDimension {
    MultiView,
    Character,
    Temporal,
    Semantic,
}

impl ImageConsistencyDimension {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "multi_view" => Ok(Self::MultiView),
            "character" => Ok(Self::Character),
            "temporal" => Ok(Self::Temporal),
            "semantic" => Ok(Self::Semantic),
            _ => Err(DecodeError::new(
                path,
                "expected multi_view, character, temporal, or semantic",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MultiView => "multi_view",
            Self::Character => "character",
            Self::Temporal => "temporal",
            Self::Semantic => "semantic",
        }
    }
}

/// An ordered handoff between images or storyboard panels. The
/// condition_on_previous flag is an explicit execution requirement; adapters
/// that cannot bind the prior output must fail closed instead of silently
/// treating a temporal request as independent generations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageConsistencySequence {
    pub steps: Vec<String>,
    pub condition_on_previous: bool,
}

impl ImageConsistencySequence {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            steps: take_string_array(&mut fields, "steps", path)?,
            condition_on_previous: take_bool(&mut fields, "condition_on_previous", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "condition_on_previous",
                JsonValue::from(self.condition_on_previous),
            ),
            ("steps", JsonValue::strings(&self.steps)),
        ])
    }
}

/// Optional cross-image consistency contract shared by structured image
/// requests and the cinematic storyboard profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageConsistencySpec {
    pub dimensions: Vec<ImageConsistencyDimension>,
    pub shared_anchors: Vec<String>,
    pub sequence: Option<ImageConsistencySequence>,
}

impl ImageConsistencySpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let dimensions = take_string_array(&mut fields, "dimensions", path)?
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                ImageConsistencyDimension::parse(&value, &format!("{path}.dimensions[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let result = Self {
            dimensions,
            shared_anchors: take_string_array(&mut fields, "shared_anchors", path)?,
            sequence: take_optional(&mut fields, "sequence")
                .map(|value| {
                    ImageConsistencySequence::from_json(value, &format!("{path}.sequence"))
                })
                .transpose()?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "dimensions",
                JsonValue::array(
                    self.dimensions
                        .iter()
                        .map(|dimension| JsonValue::from(dimension.as_str())),
                ),
            ),
            ("shared_anchors", JsonValue::strings(&self.shared_anchors)),
            (
                "sequence",
                self.sequence
                    .as_ref()
                    .map_or(JsonValue::Null, ImageConsistencySequence::to_json),
            ),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraSpec {
    pub field_of_view: String,
    pub perspective: String,
    pub depth_of_field: String,
    pub focus: String,
    pub motion_rendering: String,
}

impl CameraSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            field_of_view: take_string(&mut fields, "field_of_view", path)?,
            perspective: take_string(&mut fields, "perspective", path)?,
            depth_of_field: take_string(&mut fields, "depth_of_field", path)?,
            focus: take_string(&mut fields, "focus", path)?,
            motion_rendering: take_string(&mut fields, "motion_rendering", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "depth_of_field",
                JsonValue::from(self.depth_of_field.clone()),
            ),
            ("field_of_view", JsonValue::from(self.field_of_view.clone())),
            ("focus", JsonValue::from(self.focus.clone())),
            (
                "motion_rendering",
                JsonValue::from(self.motion_rendering.clone()),
            ),
            ("perspective", JsonValue::from(self.perspective.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LightingSpec {
    pub key_direction: String,
    pub key_quality: String,
    pub key_temperature_kelvin: Option<u16>,
    pub key_color_hex: Option<String>,
    pub key_to_fill_ratio: Option<f64>,
    pub fill_description: String,
    pub rim_description: Option<String>,
    pub shadow_character: String,
    pub exposure: String,
}

impl LightingSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let kelvin = take_optional_u64(&mut fields, "key_temperature_kelvin", path)?
            .map(|value| {
                u16::try_from(value).map_err(|_| {
                    DecodeError::new(
                        format!("{path}.key_temperature_kelvin"),
                        "value exceeds u16",
                    )
                })
            })
            .transpose()?;
        let result = Self {
            key_direction: take_string(&mut fields, "key_direction", path)?,
            key_quality: take_string(&mut fields, "key_quality", path)?,
            key_temperature_kelvin: kelvin,
            key_color_hex: take_optional_string(&mut fields, "key_color_hex", path)?,
            key_to_fill_ratio: take_optional_f64(&mut fields, "key_to_fill_ratio", path)?,
            fill_description: take_string(&mut fields, "fill_description", path)?,
            rim_description: take_optional_string(&mut fields, "rim_description", path)?,
            shadow_character: take_string(&mut fields, "shadow_character", path)?,
            exposure: take_string(&mut fields, "exposure", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("exposure", JsonValue::from(self.exposure.clone())),
            (
                "fill_description",
                JsonValue::from(self.fill_description.clone()),
            ),
            ("key_direction", JsonValue::from(self.key_direction.clone())),
            ("key_quality", JsonValue::from(self.key_quality.clone())),
            (
                "shadow_character",
                JsonValue::from(self.shadow_character.clone()),
            ),
        ];
        if let Some(value) = &self.key_color_hex {
            fields.push(("key_color_hex", JsonValue::from(value.clone())));
        }
        if let Some(value) = self.key_temperature_kelvin {
            fields.push(("key_temperature_kelvin", JsonValue::from(u64::from(value))));
        }
        if let Some(value) = self.key_to_fill_ratio {
            fields.push(("key_to_fill_ratio", number_from_f64(value)));
        }
        if let Some(value) = &self.rim_description {
            fields.push(("rim_description", JsonValue::from(value.clone())));
        }
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteEntry {
    pub hex: String,
    pub proportion_percent: u8,
    pub usage: String,
}

impl PaletteEntry {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            hex: take_string(&mut fields, "hex", path)?,
            proportion_percent: take_u8(&mut fields, "proportion_percent", path)?,
            usage: take_string(&mut fields, "usage", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("hex", JsonValue::from(self.hex.clone())),
            (
                "proportion_percent",
                JsonValue::from(u64::from(self.proportion_percent)),
            ),
            ("usage", JsonValue::from(self.usage.clone())),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhotoLutSelection {
    pub preset: String,
    pub strength_percent: u8,
    pub white_balance_kelvin: Option<u16>,
    pub tint: Option<String>,
    pub tone_curve: Option<String>,
    pub black_response: Option<String>,
    pub contrast: Option<String>,
    pub saturation_percent: Option<u8>,
    pub shadow_bias_hex: Option<String>,
    pub highlight_bias_hex: Option<String>,
    pub highlight_rolloff: Option<String>,
    pub skin_tone_policy: Option<String>,
    pub grain_size: Option<String>,
    pub grain_amount_percent: Option<u8>,
    pub halation_percent: Option<u8>,
    pub vignette_percent: Option<u8>,
}

impl PhotoLutSelection {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let kelvin = take_optional_u64(&mut fields, "white_balance_kelvin", path)?
            .map(|value| {
                u16::try_from(value).map_err(|_| {
                    DecodeError::new(format!("{path}.white_balance_kelvin"), "value exceeds u16")
                })
            })
            .transpose()?;
        let result = Self {
            preset: take_string(&mut fields, "preset", path)?,
            strength_percent: take_optional_u8(&mut fields, "strength_percent", path)?
                .unwrap_or(100),
            white_balance_kelvin: kelvin,
            tint: take_optional_string(&mut fields, "tint", path)?,
            tone_curve: take_optional_string(&mut fields, "tone_curve", path)?,
            black_response: take_optional_string(&mut fields, "black_response", path)?,
            contrast: take_optional_string(&mut fields, "contrast", path)?,
            saturation_percent: take_optional_u8(&mut fields, "saturation_percent", path)?,
            shadow_bias_hex: take_optional_string(&mut fields, "shadow_bias_hex", path)?,
            highlight_bias_hex: take_optional_string(&mut fields, "highlight_bias_hex", path)?,
            highlight_rolloff: take_optional_string(&mut fields, "highlight_rolloff", path)?,
            skin_tone_policy: take_optional_string(&mut fields, "skin_tone_policy", path)?,
            grain_size: take_optional_string(&mut fields, "grain_size", path)?,
            grain_amount_percent: take_optional_u8(&mut fields, "grain_amount_percent", path)?,
            halation_percent: take_optional_u8(&mut fields, "halation_percent", path)?,
            vignette_percent: take_optional_u8(&mut fields, "vignette_percent", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("preset", JsonValue::from(self.preset.clone())),
            (
                "strength_percent",
                JsonValue::from(u64::from(self.strength_percent)),
            ),
        ];
        push_optional_u16(
            &mut fields,
            "white_balance_kelvin",
            self.white_balance_kelvin,
        );
        push_optional_string(&mut fields, "tint", self.tint.as_ref());
        push_optional_string(&mut fields, "tone_curve", self.tone_curve.as_ref());
        push_optional_string(&mut fields, "black_response", self.black_response.as_ref());
        push_optional_string(&mut fields, "contrast", self.contrast.as_ref());
        push_optional_u8(&mut fields, "saturation_percent", self.saturation_percent);
        push_optional_string(
            &mut fields,
            "shadow_bias_hex",
            self.shadow_bias_hex.as_ref(),
        );
        push_optional_string(
            &mut fields,
            "highlight_bias_hex",
            self.highlight_bias_hex.as_ref(),
        );
        push_optional_string(
            &mut fields,
            "highlight_rolloff",
            self.highlight_rolloff.as_ref(),
        );
        push_optional_string(
            &mut fields,
            "skin_tone_policy",
            self.skin_tone_policy.as_ref(),
        );
        push_optional_string(&mut fields, "grain_size", self.grain_size.as_ref());
        push_optional_u8(
            &mut fields,
            "grain_amount_percent",
            self.grain_amount_percent,
        );
        push_optional_u8(&mut fields, "halation_percent", self.halation_percent);
        push_optional_u8(&mut fields, "vignette_percent", self.vignette_percent);
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorSpec {
    pub palette: Vec<PaletteEntry>,
    pub harmony: String,
    pub contrast: String,
    pub saturation: String,
    pub photo_lut: Option<PhotoLutSelection>,
}

impl ColorSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            palette: take_object_array(&mut fields, "palette", path, PaletteEntry::from_json)?,
            harmony: take_string(&mut fields, "harmony", path)?,
            contrast: take_string(&mut fields, "contrast", path)?,
            saturation: take_string(&mut fields, "saturation", path)?,
            photo_lut: take_optional(&mut fields, "photo_lut")
                .map(|value| PhotoLutSelection::from_json(value, &format!("{path}.photo_lut")))
                .transpose()?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("contrast", JsonValue::from(self.contrast.clone())),
            ("harmony", JsonValue::from(self.harmony.clone())),
            (
                "palette",
                JsonValue::array(self.palette.iter().map(PaletteEntry::to_json)),
            ),
            ("saturation", JsonValue::from(self.saturation.clone())),
        ];
        if let Some(value) = &self.photo_lut {
            fields.push(("photo_lut", value.to_json()));
        }
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceSpec {
    pub id: String,
    pub material: String,
    pub finish: String,
    pub micro_detail: String,
    pub light_response: String,
}

impl SurfaceSpec {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            material: take_string(&mut fields, "material", path)?,
            finish: take_string(&mut fields, "finish", path)?,
            micro_detail: take_string(&mut fields, "micro_detail", path)?,
            light_response: take_string(&mut fields, "light_response", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("finish", JsonValue::from(self.finish.clone())),
            ("id", JsonValue::from(self.id.clone())),
            (
                "light_response",
                JsonValue::from(self.light_response.clone()),
            ),
            ("material", JsonValue::from(self.material.clone())),
            ("micro_detail", JsonValue::from(self.micro_detail.clone())),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageTaskMode {
    Generate,
    Edit,
    Composite,
}

impl ImageTaskMode {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "generate" => Ok(Self::Generate),
            "edit" => Ok(Self::Edit),
            "composite" => Ok(Self::Composite),
            _ => Err(DecodeError::new(path, "expected generate/edit/composite")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generate => "generate",
            Self::Edit => "edit",
            Self::Composite => "composite",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageReferenceRole {
    Base,
    Pose,
    Subject,
    Style,
    Product,
    Layout,
    Palette,
    Mask,
    Other,
}

impl ImageReferenceRole {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "base" => Ok(Self::Base),
            "pose" => Ok(Self::Pose),
            "subject" => Ok(Self::Subject),
            "style" => Ok(Self::Style),
            "product" => Ok(Self::Product),
            "layout" => Ok(Self::Layout),
            "palette" => Ok(Self::Palette),
            "mask" => Ok(Self::Mask),
            "other" => Ok(Self::Other),
            _ => Err(DecodeError::new(
                path,
                "expected base/pose/subject/style/product/layout/palette/mask/other",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Pose => "pose",
            Self::Subject => "subject",
            Self::Style => "style",
            Self::Product => "product",
            Self::Layout => "layout",
            Self::Palette => "palette",
            Self::Mask => "mask",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageReference {
    pub index: u8,
    pub role: ImageReferenceRole,
    pub description: String,
    pub use_for: Vec<String>,
}

impl ImageReference {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let role = take_string(&mut fields, "role", path)?;
        let result = Self {
            index: take_u8(&mut fields, "index", path)?,
            role: ImageReferenceRole::parse(&role, &format!("{path}.role"))?,
            description: take_string(&mut fields, "description", path)?,
            use_for: take_string_array(&mut fields, "use_for", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("description", JsonValue::from(self.description.clone())),
            ("index", JsonValue::from(u64::from(self.index))),
            ("role", JsonValue::from(self.role.as_str())),
            ("use_for", JsonValue::strings(&self.use_for)),
        ])
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImageChangeContract {
    pub change_only: Vec<String>,
    pub preserve: Vec<String>,
}

impl ImageChangeContract {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            change_only: take_string_array(&mut fields, "change_only", path)?,
            preserve: take_string_array(&mut fields, "preserve", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("change_only", JsonValue::strings(&self.change_only)),
            ("preserve", JsonValue::strings(&self.preserve)),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Headline,
    Subhead,
    Callout,
    Caption,
    Badge,
    Cta,
    Wordmark,
}

impl TextRole {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "headline" => Ok(Self::Headline),
            "subhead" => Ok(Self::Subhead),
            "callout" => Ok(Self::Callout),
            "caption" => Ok(Self::Caption),
            "badge" => Ok(Self::Badge),
            "cta" => Ok(Self::Cta),
            "wordmark" => Ok(Self::Wordmark),
            _ => Err(DecodeError::new(
                path,
                "expected headline/subhead/callout/caption/badge/cta/wordmark",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Headline => "headline",
            Self::Subhead => "subhead",
            Self::Callout => "callout",
            Self::Caption => "caption",
            Self::Badge => "badge",
            Self::Cta => "cta",
            Self::Wordmark => "wordmark",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextElement {
    pub id: String,
    pub role: TextRole,
    pub lines: Vec<String>,
    pub language: String,
    pub placement: CanvasPlacement,
    pub font_family: String,
    pub weight: String,
    pub alignment: String,
    pub size_percent: u8,
    pub color_hex: String,
    pub letter_spacing: String,
    pub treatment: String,
    pub spelling_hint: Option<String>,
    pub separator_count: Option<u8>,
}

impl TextElement {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let role_value = take_string(&mut fields, "role", path)?;
        let result = Self {
            id: take_string(&mut fields, "id", path)?,
            role: TextRole::parse(&role_value, &format!("{path}.role"))?,
            lines: take_string_array(&mut fields, "lines", path)?,
            language: take_string(&mut fields, "language", path)?,
            placement: CanvasPlacement::from_json(
                take_required(&mut fields, "placement", path)?,
                &format!("{path}.placement"),
            )?,
            font_family: take_string(&mut fields, "font_family", path)?,
            weight: take_string(&mut fields, "weight", path)?,
            alignment: take_string(&mut fields, "alignment", path)?,
            size_percent: take_u8(&mut fields, "size_percent", path)?,
            color_hex: take_string(&mut fields, "color_hex", path)?,
            letter_spacing: take_string(&mut fields, "letter_spacing", path)?,
            treatment: take_string(&mut fields, "treatment", path)?,
            spelling_hint: take_optional_string(&mut fields, "spelling_hint", path)?,
            separator_count: take_optional_u64(&mut fields, "separator_count", path)?
                .map(|value| {
                    u8::try_from(value).map_err(|_| {
                        DecodeError::new(
                            format!("{path}.separator_count"),
                            "separator_count exceeds u8",
                        )
                    })
                })
                .transpose()?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("alignment", JsonValue::from(self.alignment.clone())),
            ("color_hex", JsonValue::from(self.color_hex.clone())),
            ("font_family", JsonValue::from(self.font_family.clone())),
            ("id", JsonValue::from(self.id.clone())),
            ("language", JsonValue::from(self.language.clone())),
            (
                "letter_spacing",
                JsonValue::from(self.letter_spacing.clone()),
            ),
            ("lines", JsonValue::strings(&self.lines)),
            ("placement", self.placement.to_json()),
            ("role", JsonValue::from(self.role.as_str())),
            (
                "size_percent",
                JsonValue::from(u64::from(self.size_percent)),
            ),
            ("treatment", JsonValue::from(self.treatment.clone())),
            ("weight", JsonValue::from(self.weight.clone())),
        ];
        fields.push((
            "separator_count",
            self.separator_count
                .map(|value| JsonValue::from(u64::from(value)))
                .unwrap_or(JsonValue::Null),
        ));
        push_optional_string(&mut fields, "spelling_hint", self.spelling_hint.as_ref());
        JsonValue::object(fields)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageConstraints {
    pub required_elements: Vec<String>,
    pub excluded_elements: Vec<String>,
    pub safety_tier: u8,
    pub adult_subjects_only: bool,
    pub original_characters_only: bool,
    pub clean_unbranded_finish: bool,
}

impl ImageConstraints {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let result = Self {
            required_elements: take_string_array(&mut fields, "required_elements", path)?,
            excluded_elements: take_string_array(&mut fields, "excluded_elements", path)?,
            safety_tier: take_u8(&mut fields, "safety_tier", path)?,
            adult_subjects_only: take_optional_bool(&mut fields, "adult_subjects_only", path)?
                .unwrap_or(false),
            original_characters_only: take_optional_bool(
                &mut fields,
                "original_characters_only",
                path,
            )?
            .unwrap_or(true),
            clean_unbranded_finish: take_optional_bool(
                &mut fields,
                "clean_unbranded_finish",
                path,
            )?
            .unwrap_or(true),
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "adult_subjects_only",
                JsonValue::from(self.adult_subjects_only),
            ),
            (
                "clean_unbranded_finish",
                JsonValue::from(self.clean_unbranded_finish),
            ),
            (
                "excluded_elements",
                JsonValue::strings(&self.excluded_elements),
            ),
            (
                "original_characters_only",
                JsonValue::from(self.original_characters_only),
            ),
            (
                "required_elements",
                JsonValue::strings(&self.required_elements),
            ),
            ("safety_tier", JsonValue::from(u64::from(self.safety_tier))),
        ])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageDetail {
    Auto,
    Low,
    Medium,
    High,
}

impl ImageDetail {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            _ => Err(DecodeError::new(path, "expected auto/low/medium/high")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMode {
    Auto,
    Opaque,
}

impl BackgroundMode {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "auto" => Ok(Self::Auto),
            "opaque" => Ok(Self::Opaque),
            "transparent" => Err(DecodeError::new(
                path,
                "this product publishes opaque PNGs; transparent output is outside its artifact contract",
            )),
            _ => Err(DecodeError::new(path, "expected auto or opaque")),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Opaque => "opaque",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageDeliveryContract {
    pub backend: String,
    pub width: u32,
    pub height: u32,
    pub detail: ImageDetail,
    pub background: BackgroundMode,
    pub format: String,
}

impl ImageDeliveryContract {
    fn from_json(value: JsonValue, path: &str) -> Result<Self, DecodeError> {
        let mut fields = into_object(value, path)?;
        let width = take_u64(&mut fields, "width", path)?;
        let height = take_u64(&mut fields, "height", path)?;
        let width = u32::try_from(width)
            .map_err(|_| DecodeError::new(format!("{path}.width"), "value exceeds u32"))?;
        let height = u32::try_from(height)
            .map_err(|_| DecodeError::new(format!("{path}.height"), "value exceeds u32"))?;
        let detail_value = take_string(&mut fields, "detail", path)?;
        let background_value = take_string(&mut fields, "background", path)?;
        let result = Self {
            backend: take_string(&mut fields, "backend", path)?,
            width,
            height,
            detail: ImageDetail::parse(&detail_value, &format!("{path}.detail"))?,
            background: BackgroundMode::parse(&background_value, &format!("{path}.background"))?,
            format: take_string(&mut fields, "format", path)?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("background", JsonValue::from(self.background.as_str())),
            ("format", JsonValue::from(self.format.clone())),
            ("height", JsonValue::from(u64::from(self.height))),
            ("backend", JsonValue::from(self.backend.clone())),
            ("detail", JsonValue::from(self.detail.as_str())),
            ("width", JsonValue::from(u64::from(self.width))),
        ])
    }

    pub fn aspect_ratio_label(&self) -> String {
        let divisor = gcd(self.width, self.height);
        format!("{}:{}", self.width / divisor, self.height / divisor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageRenderProfile {
    Structured,
    PoseTransfer,
    CinematicStoryboard,
}

/// Category-bound image intent profiles. These refine one of the six primary
/// catalog outcomes without creating a new catalog card or execution route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageProfile {
    Standard,
    LogoIdentity,
    TravelJournal,
    AppIcon,
    AppWebUi,
    InformationDesign,
    CharacterPose,
    PoomsaePose,
}

impl ImageProfile {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "standard" => Ok(Self::Standard),
            "logo_identity" => Ok(Self::LogoIdentity),
            "travel_journal" => Ok(Self::TravelJournal),
            "app_icon" => Ok(Self::AppIcon),
            "app_web_ui" => Ok(Self::AppWebUi),
            "information_design" => Ok(Self::InformationDesign),
            "character_pose" => Ok(Self::CharacterPose),
            "poomsae_pose" => Ok(Self::PoomsaePose),
            _ => Err(DecodeError::new(
                path,
                "expected standard, travel_journal, logo_identity, app_icon, app_web_ui, information_design, character_pose, or poomsae_pose",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::LogoIdentity => "logo_identity",
            Self::TravelJournal => "travel_journal",
            Self::AppIcon => "app_icon",
            Self::AppWebUi => "app_web_ui",
            Self::InformationDesign => "information_design",
            Self::CharacterPose => "character_pose",
            Self::PoomsaePose => "poomsae_pose",
        }
    }
}

impl ImageRenderProfile {
    fn parse(value: &str, path: &str) -> Result<Self, DecodeError> {
        match value {
            "structured" => Ok(Self::Structured),
            "pose_transfer" => Ok(Self::PoseTransfer),
            "cinematic_storyboard" => Ok(Self::CinematicStoryboard),
            _ => Err(DecodeError::new(
                path,
                "expected structured, pose_transfer, or cinematic_storyboard",
            )),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Structured => "structured",
            Self::PoseTransfer => "pose_transfer",
            Self::CinematicStoryboard => "cinematic_storyboard",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImagePromptRequest {
    pub language: PromptLanguage,
    pub profile: ImageProfile,
    pub render_profile: ImageRenderProfile,
    pub task_mode: ImageTaskMode,
    pub references: Vec<ImageReference>,
    pub change_contract: ImageChangeContract,
    pub use_case: String,
    pub medium: VisualMedium,
    pub taxonomy: TaxonomySelection,
    pub scene: SceneSpec,
    pub subjects: Vec<SubjectSpec>,
    pub composition: CompositionSpec,
    pub storyboard: Option<CinematicStoryboardSpec>,
    pub consistency: Option<ImageConsistencySpec>,
    pub camera: Option<CameraSpec>,
    pub lighting: LightingSpec,
    pub color: ColorSpec,
    pub surfaces: Vec<SurfaceSpec>,
    pub text_elements: Vec<TextElement>,
    pub constraints: ImageConstraints,
    pub output: ImageDeliveryContract,
}

impl ImagePromptRequest {
    pub fn from_json(value: JsonValue) -> Result<Self, DecodeError> {
        let path = "$";
        let mut fields = into_object(value, path)?;
        let medium_value = take_string(&mut fields, "medium", path)?;
        let render_profile = match take_optional(&mut fields, "render_profile") {
            None | Some(JsonValue::Null) => ImageRenderProfile::Structured,
            Some(value) => {
                let value = expect_string(value, "$.render_profile")?;
                ImageRenderProfile::parse(&value, "$.render_profile")?
            }
        };
        let profile = match take_optional(&mut fields, "profile") {
            None | Some(JsonValue::Null) => ImageProfile::Standard,
            Some(value) => {
                let value = expect_string(value, "$.profile")?;
                ImageProfile::parse(&value, "$.profile")?
            }
        };
        let task_mode = match take_optional(&mut fields, "task_mode") {
            None | Some(JsonValue::Null) => ImageTaskMode::Generate,
            Some(value) => {
                let value = expect_string(value, "$.task_mode")?;
                ImageTaskMode::parse(&value, "$.task_mode")?
            }
        };
        let references =
            take_optional_object_array(&mut fields, "references", path, ImageReference::from_json)?;
        let change_contract = match take_optional(&mut fields, "change_contract") {
            None | Some(JsonValue::Null) => ImageChangeContract::default(),
            Some(value) => ImageChangeContract::from_json(value, "$.change_contract")?,
        };
        let result = Self {
            language: take_language(&mut fields, path)?,
            profile,
            render_profile,
            task_mode,
            references,
            change_contract,
            use_case: take_string(&mut fields, "use_case", path)?,
            medium: VisualMedium::parse(&medium_value, "$.medium")?,
            taxonomy: TaxonomySelection::from_json(
                take_required(&mut fields, "taxonomy", path)?,
                "$.taxonomy",
            )?,
            scene: SceneSpec::from_json(take_required(&mut fields, "scene", path)?, "$.scene")?,
            subjects: take_object_array(&mut fields, "subjects", path, SubjectSpec::from_json)?,
            composition: CompositionSpec::from_json(
                take_required(&mut fields, "composition", path)?,
                "$.composition",
            )?,
            storyboard: take_optional(&mut fields, "storyboard")
                .map(|value| CinematicStoryboardSpec::from_json(value, "$.storyboard"))
                .transpose()?,
            consistency: take_optional(&mut fields, "consistency")
                .map(|value| ImageConsistencySpec::from_json(value, "$.consistency"))
                .transpose()?,
            camera: take_optional(&mut fields, "camera")
                .map(|value| CameraSpec::from_json(value, "$.camera"))
                .transpose()?,
            lighting: LightingSpec::from_json(
                take_required(&mut fields, "lighting", path)?,
                "$.lighting",
            )?,
            color: ColorSpec::from_json(take_required(&mut fields, "color", path)?, "$.color")?,
            surfaces: take_optional_object_array(
                &mut fields,
                "surfaces",
                path,
                SurfaceSpec::from_json,
            )?,
            text_elements: take_optional_object_array(
                &mut fields,
                "text_elements",
                path,
                TextElement::from_json,
            )?,
            constraints: ImageConstraints::from_json(
                take_required(&mut fields, "constraints", path)?,
                "$.constraints",
            )?,
            output: ImageDeliveryContract::from_json(
                take_required(&mut fields, "output", path)?,
                "$.output",
            )?,
        };
        finish_object(fields, path)?;
        Ok(result)
    }

    pub fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("color", self.color.to_json()),
            ("composition", self.composition.to_json()),
            ("constraints", self.constraints.to_json()),
            ("language", JsonValue::from(self.language.code())),
            ("profile", JsonValue::from(self.profile.as_str())),
            ("lighting", self.lighting.to_json()),
            ("medium", JsonValue::from(self.medium.as_str())),
            ("output", self.output.to_json()),
            (
                "render_profile",
                JsonValue::from(self.render_profile.as_str()),
            ),
            ("task_mode", JsonValue::from(self.task_mode.as_str())),
            (
                "references",
                JsonValue::array(self.references.iter().map(ImageReference::to_json)),
            ),
            ("change_contract", self.change_contract.to_json()),
            ("scene", self.scene.to_json()),
            (
                "storyboard",
                self.storyboard
                    .as_ref()
                    .map_or(JsonValue::Null, CinematicStoryboardSpec::to_json),
            ),
            (
                "consistency",
                self.consistency
                    .as_ref()
                    .map_or(JsonValue::Null, ImageConsistencySpec::to_json),
            ),
            (
                "subjects",
                JsonValue::array(self.subjects.iter().map(SubjectSpec::to_json)),
            ),
            (
                "surfaces",
                JsonValue::array(self.surfaces.iter().map(SurfaceSpec::to_json)),
            ),
            ("taxonomy", self.taxonomy.to_json()),
            (
                "text_elements",
                JsonValue::array(self.text_elements.iter().map(TextElement::to_json)),
            ),
            ("use_case", JsonValue::from(self.use_case.clone())),
        ];
        if let Some(camera) = &self.camera {
            fields.push(("camera", camera.to_json()));
        }
        JsonValue::object(fields)
    }
}

fn take_u8(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<u8, DecodeError> {
    let value = take_u64(fields, key, path)?;
    u8::try_from(value).map_err(|_| DecodeError::new(format!("{path}.{key}"), "value exceeds u8"))
}

fn take_optional_u8(
    fields: &mut BTreeMap<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<Option<u8>, DecodeError> {
    take_optional_u64(fields, key, path)?
        .map(|value| {
            u8::try_from(value)
                .map_err(|_| DecodeError::new(format!("{path}.{key}"), "value exceeds u8"))
        })
        .transpose()
}

fn optional_fields<const N: usize>(
    mut base: Vec<(&'static str, JsonValue)>,
    optional: [(&'static str, Option<&String>); N],
) -> JsonValue {
    for (key, value) in optional {
        if let Some(value) = value {
            base.push((key, JsonValue::from(value.clone())));
        }
    }
    JsonValue::object(base)
}

fn push_optional_string(
    fields: &mut Vec<(&'static str, JsonValue)>,
    key: &'static str,
    value: Option<&String>,
) {
    if let Some(value) = value {
        fields.push((key, JsonValue::from(value.clone())));
    }
}

fn push_optional_u8(
    fields: &mut Vec<(&'static str, JsonValue)>,
    key: &'static str,
    value: Option<u8>,
) {
    if let Some(value) = value {
        fields.push((key, JsonValue::from(u64::from(value))));
    }
}

fn push_optional_u16(
    fields: &mut Vec<(&'static str, JsonValue)>,
    key: &'static str,
    value: Option<u16>,
) {
    if let Some(value) = value {
        fields.push((key, JsonValue::from(u64::from(value))));
    }
}

fn number_from_f64(value: f64) -> JsonValue {
    let mut text = value.to_string();
    if !text.contains(['.', 'e', 'E']) {
        text.push_str(".0");
    }
    JsonValue::Number(text)
}

const fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[cfg(test)]
#[path = "model/tests.rs"]
mod tests;
