use super::*;
use crate::json::parse;

#[test]
fn example_deserializes_strictly() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-photo-lut.json"
        )))
        .expect("JSON"),
    )
    .expect("image request");
    assert_eq!(request.output.aspect_ratio_label(), "2:3");
    assert_eq!(request.subjects.len(), 2);
}

#[test]
fn taxonomy_accepts_only_a_primary_category() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/image-photo-lut.json"
    ))
    .replace(
        "\"category\": \"C5\"",
        "\"category\": \"C5\", \"unsupported\": \"value\"",
    );
    let error = ImagePromptRequest::from_json(parse(&source).expect("fixture JSON"))
        .expect_err("taxonomy must be strict");
    assert!(error.message.contains("unknown field"));
}

#[test]
fn app_icon_profile_roundtrips_as_the_canonical_typed_value() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-icon.json"
        )))
        .expect("JSON"),
    )
    .expect("app icon request");
    assert_eq!(request.profile, ImageProfile::AppIcon);
    assert_eq!(request.output.width, 1024);
    assert_eq!(request.output.height, 1024);
    assert_eq!(request.text_elements, Vec::new());

    let roundtrip = ImagePromptRequest::from_json(request.to_json()).expect("roundtrip");
    assert_eq!(roundtrip, request);
    assert_eq!(
        roundtrip.to_json().to_pretty_string(),
        request.to_json().to_pretty_string()
    );
}

#[test]
fn image_profile_runtime_aliases_are_rejected() {
    for alias in [
        "default",
        "logo",
        "brand_mark",
        "ui_screen",
        "web_ui",
        "app_ui",
        "info_design",
        "information_ui",
        "character",
        "pose",
        "poomsae",
        "taekwondo_poomsae",
        "icon",
        "app-icon",
    ] {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-icon.json"
        ))
        .replace("\"app_icon\"", &format!("\"{alias}\""));
        let error = ImagePromptRequest::from_json(parse(&source).expect("alias fixture JSON"))
            .expect_err("non-canonical app icon alias must be rejected");
        assert!(error.path.contains("profile"), "{alias}: {error:?}");
    }
}

#[test]
fn subscription_auto_detail_is_accepted_during_decode() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/image-photo-lut.json"
    ))
    .replace("\"detail\": \"high\"", "\"detail\": \"auto\"");
    let value = parse(&source).expect("JSON");
    let request = ImagePromptRequest::from_json(value).expect("auto detail must decode");
    assert_eq!(request.output.detail, ImageDetail::Auto);
}

#[test]
fn transparent_background_is_rejected_during_decode() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/image-photo-lut.json"
    ))
    .replace("\"opaque\"", "\"transparent\"");
    let value = parse(&source).expect("JSON");
    let error = ImagePromptRequest::from_json(value).expect_err("transparent must fail");
    assert!(error.message.contains("opaque PNGs"));
}

#[test]
fn pose_transfer_example_decodes_two_disjoint_reference_roles() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-pose-transfer.json"
        )))
        .expect("JSON"),
    )
    .expect("pose transfer request");
    assert_eq!(request.render_profile, ImageRenderProfile::PoseTransfer);
    assert_eq!(request.task_mode, ImageTaskMode::Composite);
    assert_eq!(request.references.len(), 2);
    assert_eq!(request.references[0].role, ImageReferenceRole::Pose);
    assert_eq!(request.references[1].role, ImageReferenceRole::Subject);
}

#[test]
fn cinematic_storyboard_example_decodes_a_typed_three_panel_handoff() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-cinematic-storyboard.json"
        )))
        .expect("JSON"),
    )
    .expect("cinematic storyboard request");
    assert_eq!(
        request.render_profile,
        ImageRenderProfile::CinematicStoryboard
    );
    assert_eq!(
        request
            .storyboard
            .as_ref()
            .expect("storyboard")
            .panels
            .len(),
        3
    );
    let consistency = request.consistency.as_ref().expect("consistency");
    assert_eq!(consistency.dimensions.len(), 4);
    assert_eq!(
        consistency.sequence.as_ref().expect("sequence").steps,
        vec![
            "arrival-space".to_owned(),
            "letter-realization".to_owned(),
            "decision-gaze".to_owned()
        ]
    );
}

#[test]
fn delivery_contract_rejects_retired_api_controls() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-icon.json"
        )))
        .unwrap(),
    )
    .unwrap();
    for (key, value) in [
        ("model", "gpt-image-2.5-sunburst"),
        ("quality", "max"),
        ("detail", "max"),
        ("detail", "xhigh"),
    ] {
        let mut json = request.to_json();
        if let JsonValue::Object(root) = &mut json
            && let Some(JsonValue::Object(output)) = root.get_mut("output")
        {
            output.insert(key.to_owned(), JsonValue::from(value));
        }
        assert!(
            ImagePromptRequest::from_json(json).is_err(),
            "unsupported control {key}={value}"
        );
    }
}

#[test]
fn numeric_text_separator_contract_is_explicit_and_bounded() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(request.text_elements[0].separator_count, Some(3));
    let roundtrip = ImagePromptRequest::from_json(request.to_json()).unwrap();
    assert_eq!(roundtrip.text_elements[0].separator_count, Some(3));
    request.text_elements[0].separator_count = None;
    let missing = crate::image::compile_image_prompt(&request);
    assert!(
        missing
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_TEXT_SEPARATOR_COUNT_REQUIRED")
    );
    assert!(!missing.is_valid());
    request.text_elements[0].separator_count = Some(9);
    assert!(!crate::image::compile_image_prompt(&request).is_valid());
}
