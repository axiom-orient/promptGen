use crate::json::JsonValue;

pub fn image_schema() -> JsonValue {
    let mut schema = object_schema(
        &[
            "use_case",
            "medium",
            "taxonomy",
            "scene",
            "subjects",
            "composition",
            "lighting",
            "color",
            "constraints",
            "output",
        ],
        [
            ("language", enum_schema(&["ko", "en"])),
            (
                "render_profile",
                enum_schema(&["structured", "pose_transfer", "cinematic_storyboard"]),
            ),
            (
                "profile",
                enum_schema(&[
                    "standard",
                    "logo_identity",
                    "travel_journal",
                    "app_icon",
                    "app_web_ui",
                    "information_design",
                    "character_pose",
                    "poomsae_pose",
                ]),
            ),
            ("task_mode", enum_schema(&["generate", "edit", "composite"])),
            (
                "references",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("maxItems", JsonValue::from(255_u64)),
                    ("items", reference_schema()),
                ]),
            ),
            ("change_contract", change_contract_schema()),
            ("use_case", string_schema()),
            (
                "medium",
                enum_schema(&["photo", "illustration", "3d", "graphic_design", "mixed"]),
            ),
            (
                "taxonomy",
                object_schema(&[], [("category", nullable(string_schema()))]),
            ),
            (
                "scene",
                object_schema(
                    &["environment", "background", "atmosphere"],
                    [
                        ("environment", string_schema()),
                        ("background", string_schema()),
                        ("atmosphere", string_schema()),
                        ("time_of_day", nullable(string_schema())),
                        ("weather", nullable(string_schema())),
                    ],
                ),
            ),
            ("subjects", array_schema(subject_schema())),
            ("composition", composition_schema()),
            ("storyboard", nullable(storyboard_schema())),
            ("consistency", nullable(consistency_schema())),
            ("camera", nullable(camera_schema())),
            ("lighting", lighting_schema()),
            ("color", color_schema()),
            ("surfaces", array_schema(surface_schema())),
            ("text_elements", array_schema(text_element_schema())),
            ("constraints", constraints_schema()),
            ("output", output_schema()),
        ],
    );
    let JsonValue::Object(root) = &mut schema else {
        unreachable!("object_schema must produce an object");
    };
    root.insert(
        "$schema".to_owned(),
        JsonValue::from("https://json-schema.org/draft/2020-12/schema"),
    );
    root.insert("title".to_owned(), JsonValue::from("ImagePromptRequest"));
    schema
}

fn consistency_schema() -> JsonValue {
    object_schema(
        &["dimensions", "shared_anchors"],
        [
            (
                "dimensions",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(1_u64)),
                    ("maxItems", JsonValue::from(4_u64)),
                    (
                        "items",
                        enum_schema(&["multi_view", "character", "temporal", "semantic"]),
                    ),
                ]),
            ),
            (
                "shared_anchors",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(1_u64)),
                    ("maxItems", JsonValue::from(8_u64)),
                    ("items", string_schema()),
                ]),
            ),
            ("sequence", nullable(consistency_sequence_schema())),
        ],
    )
}

fn consistency_sequence_schema() -> JsonValue {
    object_schema(
        &["steps", "condition_on_previous"],
        [
            (
                "steps",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(2_u64)),
                    ("maxItems", JsonValue::from(24_u64)),
                    ("items", string_schema()),
                ]),
            ),
            (
                "condition_on_previous",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
        ],
    )
}

fn storyboard_schema() -> JsonValue {
    object_schema(
        &["layout", "screen_direction", "identity_anchors", "panels"],
        [
            (
                "layout",
                enum_schema(&["three_panel_strip", "two_by_two", "three_by_two"]),
            ),
            (
                "screen_direction",
                enum_schema(&["left_to_right", "right_to_left"]),
            ),
            (
                "identity_anchors",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(3_u64)),
                    ("maxItems", JsonValue::from(6_u64)),
                    ("items", string_schema()),
                ]),
            ),
            (
                "panels",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(3_u64)),
                    ("maxItems", JsonValue::from(6_u64)),
                    ("items", storyboard_panel_schema()),
                ]),
            ),
        ],
    )
}

fn storyboard_panel_schema() -> JsonValue {
    object_schema(
        &[
            "id",
            "shot_size",
            "camera_angle",
            "camera_move",
            "action",
            "body_facing",
            "gaze_target",
            "emotional_beat",
        ],
        [
            ("id", string_schema()),
            (
                "shot_size",
                enum_schema(&[
                    "establishing",
                    "wide",
                    "medium",
                    "medium_close_up",
                    "close_up",
                    "extreme_close_up",
                    "over_the_shoulder",
                    "insert",
                ]),
            ),
            ("camera_angle", string_schema()),
            ("camera_move", string_schema()),
            ("action", string_schema()),
            (
                "body_facing",
                enum_schema(&[
                    "screen_left",
                    "screen_right",
                    "toward_camera",
                    "away_from_camera",
                    "three_quarter_left",
                    "three_quarter_right",
                ]),
            ),
            (
                "gaze_target",
                enum_schema(&[
                    "screen_left",
                    "screen_right",
                    "toward_camera",
                    "up",
                    "down",
                    "offscreen_left",
                    "offscreen_right",
                    "object_in_frame",
                ]),
            ),
            ("emotional_beat", string_schema()),
        ],
    )
}

fn reference_schema() -> JsonValue {
    object_schema(
        &["index", "role", "description", "use_for"],
        [
            ("index", integer_schema(1, 255)),
            (
                "role",
                enum_schema(&[
                    "base", "pose", "subject", "style", "product", "layout", "palette", "mask",
                    "other",
                ]),
            ),
            ("description", string_schema()),
            (
                "use_for",
                JsonValue::object([
                    ("type", JsonValue::from("array")),
                    ("minItems", JsonValue::from(1_u64)),
                    ("items", string_schema()),
                ]),
            ),
        ],
    )
}

fn change_contract_schema() -> JsonValue {
    object_schema(
        &["change_only", "preserve"],
        [
            ("change_only", array_schema(string_schema())),
            ("preserve", array_schema(string_schema())),
        ],
    )
}

fn subject_schema() -> JsonValue {
    object_schema(
        &[
            "id",
            "count",
            "description",
            "placement",
            "scale",
            "pose",
            "gaze",
            "action",
        ],
        [
            ("id", string_schema()),
            ("count", integer_schema(1, 65535)),
            ("description", string_schema()),
            ("placement", placement_schema()),
            ("scale", string_schema()),
            ("pose", string_schema()),
            ("gaze", string_schema()),
            ("action", string_schema()),
            ("face", nullable(string_schema())),
            ("hair", nullable(string_schema())),
            ("appearance", nullable(string_schema())),
            ("distinguishing_features", array_schema(string_schema())),
        ],
    )
}

fn placement_schema() -> JsonValue {
    JsonValue::object([(
        "oneOf",
        JsonValue::array([
            object_schema(
                &["zone"],
                [(
                    "zone",
                    enum_schema(&[
                        "top_left",
                        "top_center",
                        "top_right",
                        "middle_left",
                        "center",
                        "middle_right",
                        "bottom_left",
                        "bottom_center",
                        "bottom_right",
                    ]),
                )],
            ),
            object_schema(
                &[
                    "zone",
                    "x_percent",
                    "y_percent",
                    "width_percent",
                    "height_percent",
                ],
                [
                    ("zone", enum_schema(&["custom"])),
                    ("x_percent", integer_schema(0, 100)),
                    ("y_percent", integer_schema(0, 100)),
                    ("width_percent", integer_schema(1, 100)),
                    ("height_percent", integer_schema(1, 100)),
                ],
            ),
        ]),
    )])
}

fn composition_schema() -> JsonValue {
    object_schema(
        &[
            "framing",
            "viewpoint",
            "camera_angle",
            "balance",
            "visual_hierarchy",
            "depth_layers",
        ],
        [
            ("framing", string_schema()),
            ("viewpoint", string_schema()),
            ("camera_angle", string_schema()),
            ("balance", string_schema()),
            ("visual_hierarchy", array_schema(string_schema())),
            ("depth_layers", array_schema(string_schema())),
            (
                "negative_space",
                array_schema(enum_schema(&[
                    "top_left",
                    "top_center",
                    "top_right",
                    "middle_left",
                    "center",
                    "middle_right",
                    "bottom_left",
                    "bottom_center",
                    "bottom_right",
                ])),
            ),
        ],
    )
}

fn camera_schema() -> JsonValue {
    object_schema(
        &[
            "field_of_view",
            "perspective",
            "depth_of_field",
            "focus",
            "motion_rendering",
        ],
        [
            ("field_of_view", string_schema()),
            ("perspective", string_schema()),
            ("depth_of_field", string_schema()),
            ("focus", string_schema()),
            ("motion_rendering", string_schema()),
        ],
    )
}

fn lighting_schema() -> JsonValue {
    object_schema(
        &[
            "key_direction",
            "key_quality",
            "fill_description",
            "shadow_character",
            "exposure",
        ],
        [
            ("key_direction", string_schema()),
            ("key_quality", string_schema()),
            (
                "key_temperature_kelvin",
                nullable(integer_schema(1000, 20000)),
            ),
            ("key_color_hex", nullable(hex_schema())),
            ("key_to_fill_ratio", nullable(number_schema(0.01, 100.0))),
            ("fill_description", string_schema()),
            ("rim_description", nullable(string_schema())),
            ("shadow_character", string_schema()),
            ("exposure", string_schema()),
        ],
    )
}

fn color_schema() -> JsonValue {
    object_schema(
        &["palette", "harmony", "contrast", "saturation"],
        [
            (
                "palette",
                JsonValue::object([
                    ("items", palette_entry_schema()),
                    ("maxItems", JsonValue::from(5_u64)),
                    ("minItems", JsonValue::from(3_u64)),
                    ("type", JsonValue::from("array")),
                ]),
            ),
            ("harmony", string_schema()),
            ("contrast", string_schema()),
            ("saturation", string_schema()),
            ("photo_lut", nullable(photo_lut_schema())),
        ],
    )
}

fn palette_entry_schema() -> JsonValue {
    object_schema(
        &["hex", "proportion_percent", "usage"],
        [
            ("hex", hex_schema()),
            ("proportion_percent", integer_schema(1, 100)),
            ("usage", string_schema()),
        ],
    )
}

fn photo_lut_schema() -> JsonValue {
    object_schema(
        &["preset", "strength_percent"],
        [
            (
                "preset",
                enum_schema(&[
                    "clean_neutral",
                    "warm_pastel_filmic",
                    "cool_steel",
                    "restrained_teal_orange",
                    "bleach_bypass",
                    "tungsten_night",
                    "faded_print",
                    "monochrome_high_contrast",
                ]),
            ),
            ("strength_percent", integer_schema(0, 100)),
            (
                "white_balance_kelvin",
                nullable(integer_schema(1000, 20000)),
            ),
            ("tint", nullable(string_schema())),
            ("tone_curve", nullable(string_schema())),
            ("black_response", nullable(string_schema())),
            ("contrast", nullable(string_schema())),
            ("saturation_percent", nullable(integer_schema(0, 100))),
            ("shadow_bias_hex", nullable(hex_schema())),
            ("highlight_bias_hex", nullable(hex_schema())),
            ("highlight_rolloff", nullable(string_schema())),
            ("skin_tone_policy", nullable(string_schema())),
            ("grain_size", nullable(string_schema())),
            ("grain_amount_percent", nullable(integer_schema(0, 100))),
            ("halation_percent", nullable(integer_schema(0, 100))),
            ("vignette_percent", nullable(integer_schema(0, 100))),
        ],
    )
}

fn surface_schema() -> JsonValue {
    object_schema(
        &["id", "material", "finish", "micro_detail", "light_response"],
        [
            ("id", string_schema()),
            ("material", string_schema()),
            ("finish", string_schema()),
            ("micro_detail", string_schema()),
            ("light_response", string_schema()),
        ],
    )
}

fn text_element_schema() -> JsonValue {
    object_schema(
        &[
            "id",
            "role",
            "lines",
            "language",
            "placement",
            "font_family",
            "weight",
            "alignment",
            "size_percent",
            "color_hex",
            "letter_spacing",
            "treatment",
        ],
        [
            ("id", string_schema()),
            (
                "role",
                enum_schema(&[
                    "headline", "subhead", "callout", "caption", "badge", "cta", "wordmark",
                ]),
            ),
            ("lines", array_schema(string_schema())),
            ("language", string_schema()),
            ("placement", placement_schema()),
            ("font_family", string_schema()),
            ("weight", string_schema()),
            ("alignment", string_schema()),
            ("size_percent", integer_schema(1, 100)),
            ("color_hex", hex_schema()),
            ("letter_spacing", string_schema()),
            ("treatment", string_schema()),
            ("spelling_hint", nullable(string_schema())),
            ("separator_count", nullable(integer_schema(0, 8))),
        ],
    )
}

fn constraints_schema() -> JsonValue {
    object_schema(
        &["required_elements", "excluded_elements", "safety_tier"],
        [
            ("required_elements", array_schema(string_schema())),
            ("excluded_elements", array_schema(string_schema())),
            ("safety_tier", integer_schema(0, 2)),
            (
                "adult_subjects_only",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
            (
                "original_characters_only",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
            (
                "clean_unbranded_finish",
                JsonValue::object([("type", JsonValue::from("boolean"))]),
            ),
        ],
    )
}

fn output_schema() -> JsonValue {
    object_schema(
        &[
            "backend",
            "width",
            "height",
            "detail",
            "background",
            "format",
        ],
        [
            ("backend", enum_schema(&[super::IMAGE_BACKEND])),
            ("width", integer_schema(16, 3840)),
            ("height", integer_schema(16, 3840)),
            ("detail", enum_schema(&["auto", "low", "medium", "high"])),
            ("background", enum_schema(&["auto", "opaque"])),
            ("format", enum_schema(&["png"])),
        ],
    )
}

fn string_schema() -> JsonValue {
    JsonValue::object([
        ("minLength", JsonValue::from(1_u64)),
        ("pattern", JsonValue::from("\\S")),
        ("type", JsonValue::from("string")),
    ])
}

fn hex_schema() -> JsonValue {
    JsonValue::object([
        ("pattern", JsonValue::from("^#[0-9A-Fa-f]{6}$")),
        ("type", JsonValue::from("string")),
    ])
}

fn enum_schema(values: &[&str]) -> JsonValue {
    JsonValue::object([
        (
            "enum",
            JsonValue::array(values.iter().map(|value| JsonValue::from(*value))),
        ),
        ("type", JsonValue::from("string")),
    ])
}

fn array_schema(items: JsonValue) -> JsonValue {
    JsonValue::object([("items", items), ("type", JsonValue::from("array"))])
}

fn integer_schema(minimum: u64, maximum: u64) -> JsonValue {
    JsonValue::object([
        ("maximum", JsonValue::from(maximum)),
        ("minimum", JsonValue::from(minimum)),
        ("type", JsonValue::from("integer")),
    ])
}

fn number_schema(minimum: f64, maximum: f64) -> JsonValue {
    JsonValue::object([
        ("maximum", JsonValue::Number(maximum.to_string())),
        ("minimum", JsonValue::Number(minimum.to_string())),
        ("type", JsonValue::from("number")),
    ])
}

fn nullable(schema: JsonValue) -> JsonValue {
    JsonValue::object([(
        "anyOf",
        JsonValue::array([
            schema,
            JsonValue::object([("type", JsonValue::from("null"))]),
        ]),
    )])
}

fn object_schema<const N: usize>(
    required: &[&str],
    properties: [(&str, JsonValue); N],
) -> JsonValue {
    JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        (
            "properties",
            JsonValue::object(
                properties
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), value)),
            ),
        ),
        (
            "required",
            JsonValue::array(required.iter().map(|value| JsonValue::from(*value))),
        ),
        ("type", JsonValue::from("object")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_is_optional_like_the_runtime_decoder() {
        let schema = image_schema();
        let required = schema
            .as_object()
            .and_then(|root| root.get("required"))
            .and_then(JsonValue::as_array)
            .expect("required");
        assert!(
            !required
                .iter()
                .any(|value| value.as_str() == Some("language"))
        );
    }

    #[test]
    fn free_text_schema_rejects_whitespace_only_values() {
        let schema = string_schema();
        let object = schema.as_object().expect("string schema");
        assert_eq!(object.get("minLength").and_then(JsonValue::as_u64), Some(1));
        assert_eq!(
            object.get("pattern").and_then(JsonValue::as_str),
            Some("\\S")
        );
    }

    #[test]
    fn schema_is_valid_json_and_rejects_transparent_enum() {
        let text = image_schema().to_pretty_string();
        let parsed = crate::json::parse(&text).expect("schema must parse");
        assert_eq!(
            parsed
                .as_object()
                .and_then(|root| root.get("$schema"))
                .and_then(JsonValue::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert!(!text.contains("transparent"));
        assert!(text.contains("codex-subscription"));
    }

    #[test]
    fn schema_matches_subscription_output_task_and_output_contract() {
        let schema = image_schema();
        let root = schema.as_object().expect("root object");
        let properties = root
            .get("properties")
            .and_then(JsonValue::as_object)
            .expect("root properties");
        for key in [
            "task_mode",
            "references",
            "change_contract",
            "consistency",
            "text_elements",
        ] {
            assert!(properties.contains_key(key), "missing {key}");
        }
        assert!(schema.to_pretty_string().contains("\"multi_view\""));

        let output = properties
            .get("output")
            .and_then(JsonValue::as_object)
            .expect("output schema");
        let output_properties = output
            .get("properties")
            .and_then(JsonValue::as_object)
            .expect("output properties");
        let detail = output_properties
            .get("detail")
            .and_then(JsonValue::as_object)
            .and_then(|value| value.get("enum"))
            .and_then(JsonValue::as_array)
            .expect("detail enum");
        assert_eq!(
            detail,
            &[
                JsonValue::from("auto"),
                JsonValue::from("low"),
                JsonValue::from("medium"),
                JsonValue::from("high"),
            ]
        );
        for dimension in ["width", "height"] {
            let maximum = output_properties
                .get(dimension)
                .and_then(JsonValue::as_object)
                .and_then(|value| value.get("maximum"));
            assert_eq!(maximum, Some(&JsonValue::from(3840_u64)));
        }

        assert!(schema.to_pretty_string().contains("spelling_hint"));
    }

    #[test]
    fn placement_union_keeps_branch_level_property_authority() {
        let schema = placement_schema();
        let root = schema
            .as_object()
            .expect("placement schema must be an object");
        assert!(!root.contains_key("additionalProperties"));
        let branches = root
            .get("oneOf")
            .and_then(JsonValue::as_array)
            .expect("placement schema must contain oneOf");
        assert_eq!(branches.len(), 2);
        for branch in branches {
            let object = branch.as_object().expect("branch must be an object");
            assert_eq!(
                object
                    .get("additionalProperties")
                    .and_then(JsonValue::as_bool),
                Some(false)
            );
        }
    }
}
