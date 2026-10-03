use super::*;
use crate::image::model::ImagePromptRequest;
use crate::image::{
    BackgroundMode, CanvasPlacement, CanvasZone, ImageChangeContract, ImageConsistencyDimension,
    ImageConsistencySpec, ImageReference, ImageReferenceRole, ImageRenderProfile, ImageTaskMode,
    VisualMedium,
};
use crate::json::parse;

fn example() -> ImagePromptRequest {
    ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-photo-lut.json"
        )))
        .expect("JSON"),
    )
    .expect("request")
}

#[test]
fn category_bound_profiles_compile_for_logo_ui_and_poomsae_examples() {
    for path in [
        "image-logo-identity.json",
        "image-app-icon.json",
        "image-app-web-ui.json",
        "image-character-poomsae.json",
    ] {
        let request = ImagePromptRequest::from_json(
            parse(
                &std::fs::read_to_string(format!(
                    "{}/../../examples/{path}",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .expect("profile example"),
            )
            .expect("JSON"),
        )
        .expect("typed profile request");
        let (diagnostics, _) = validate_image_request(&request);
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
            "{path}: {diagnostics:#?}"
        );
    }
}

fn app_icon_example() -> ImagePromptRequest {
    ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-icon.json"
        )))
        .expect("JSON"),
    )
    .expect("app icon request")
}

type RequestMutation = fn(&mut ImagePromptRequest);

fn assert_app_icon_diagnostic(
    mut request: ImagePromptRequest,
    code: &str,
    mutate: impl FnOnce(&mut ImagePromptRequest),
) {
    mutate(&mut request);
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code == code),
        "missing {code}: {diagnostics:#?}"
    );
}

#[test]
fn app_icon_requires_the_opaque_1024_square_master_contract() {
    let request = app_icon_example();
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );
    assert!(!selected.is_empty());

    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_TASK_MODE", |request| {
        request.task_mode = ImageTaskMode::Edit;
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_RENDER_PROFILE", |request| {
        request.render_profile = ImageRenderProfile::PoseTransfer;
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_REFERENCES", |request| {
        request.references.push(ImageReference {
            index: 1,
            role: ImageReferenceRole::Base,
            description: "reference image".to_owned(),
            use_for: vec!["style".to_owned()],
        });
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_CHANGE_CONTRACT", |request| {
        request.change_contract = ImageChangeContract {
            change_only: vec!["color".to_owned()],
            preserve: Vec::new(),
        };
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_MEDIUM", |request| {
        request.medium = VisualMedium::Photo;
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_SUBJECT_COUNT", |request| {
        request.subjects[0].count = 2;
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_PLANES", |request| {
        request
            .composition
            .depth_layers
            .push("extra decorative plane".to_owned());
    });
    assert_app_icon_diagnostic(
        request.clone(),
        "IMG_APP_ICON_SUBJECT_PLACEMENT",
        |request| {
            request.subjects[0].placement = CanvasPlacement::Zone(CanvasZone::TopLeft);
        },
    );
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_TEXT", |request| {
        let text_source = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-typography-poster.json"
            )))
            .expect("JSON"),
        )
        .expect("text request");
        request.text_elements = text_source.text_elements;
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_FORMAT", |request| {
        request.output.format = "jpg".to_owned();
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_BACKGROUND", |request| {
        request.output.background = BackgroundMode::Auto;
    });
    assert_app_icon_diagnostic(request.clone(), "IMG_APP_ICON_DIMENSIONS", |request| {
        request.output.width = 512;
    });
    assert_app_icon_diagnostic(request, "IMG_APP_ICON_FORBIDDEN_CONTENT", |request| {
        request.subjects[0].description = "single letter inside a device frame".to_owned();
    });
}

#[test]
fn textless_app_web_ui_rejects_readable_copy_and_chart_content() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-web-ui.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.subjects[0].description =
        "responsive dashboard screen with sidebar and readable text chart".to_owned();

    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_UI_TEXTLESS_CONTENT"),
        "{diagnostics:#?}"
    );
}

#[test]
fn forbidden_content_negation_does_not_cross_independent_comma_phrases() {
    let mut icon = app_icon_example();
    icon.subjects[0].description = "single star mark, no outline, readable text".to_owned();
    let (diagnostics, _) = validate_image_request(&icon);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_APP_ICON_FORBIDDEN_CONTENT"),
        "{diagnostics:#?}"
    );

    let mut ui = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-web-ui.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    ui.subjects[0].description =
        "responsive workspace screen with sidebar navigation blocks, no circles, add readable text"
            .to_owned();
    let (diagnostics, _) = validate_image_request(&ui);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_UI_TEXTLESS_CONTENT"),
        "{diagnostics:#?}"
    );
}

#[test]
fn emphasis_qualifiers_remain_positive_forbidden_content_in_both_profiles() {
    let mut icon = app_icon_example();
    icon.subjects[0].description = "single star mark, not only readable text".to_owned();
    let (diagnostics, _) = validate_image_request(&icon);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_APP_ICON_FORBIDDEN_CONTENT"),
        "{diagnostics:#?}"
    );

    let mut ui = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-web-ui.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    ui.subjects[0].description =
        "responsive workspace screen with sidebar navigation blocks, not only readable text"
            .to_owned();
    let (diagnostics, _) = validate_image_request(&ui);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_UI_TEXTLESS_CONTENT"),
        "{diagnostics:#?}"
    );
}

#[test]
fn emphasis_qualifiers_do_not_create_a_negative_scope() {
    for qualifier in [
        "not only",
        "do not only",
        "don't only",
        "not just",
        "not merely",
        "not simply",
    ] {
        assert!(
            contains_positive_forbidden_term(
                &format!("{qualifier} readable text"),
                TEXTLESS_UI_FORBIDDEN_TERMS
            ),
            "term after emphasis qualifier: {qualifier}"
        );
        assert!(
            contains_positive_forbidden_term(
                &format!("readable text {qualifier}"),
                TEXTLESS_UI_FORBIDDEN_TERMS
            ),
            "term before emphasis qualifier: {qualifier}"
        );
    }
}

#[test]
fn forbidden_content_preserves_local_negative_lists_and_korean_negation() {
    assert!(!contains_positive_forbidden_term(
        "no readable text, numerals, pseudo-writing, invented data, or charts",
        TEXTLESS_UI_FORBIDDEN_TERMS
    ));
    assert!(!contains_positive_forbidden_term(
        "without readable copy",
        TEXTLESS_UI_FORBIDDEN_TERMS
    ));
    assert!(!contains_positive_forbidden_term(
        "텍스트 없음, 숫자 없음, 의사문자 없음, 데이터 없음, 차트 없음",
        TEXTLESS_UI_FORBIDDEN_TERMS
    ));
    assert!(!contains_positive_forbidden_term(
        "no readable text",
        TEXTLESS_UI_FORBIDDEN_TERMS
    ));
    assert!(contains_positive_forbidden_term(
        "no circles, readable text",
        TEXTLESS_UI_FORBIDDEN_TERMS
    ));
    assert!(contains_positive_forbidden_term(
        "no circles, add readable text",
        TEXTLESS_UI_FORBIDDEN_TERMS
    ));
}

#[test]
fn textless_app_web_ui_checks_all_rendered_free_text_groups() {
    let cases: [(&str, RequestMutation); 4] = [
        (
            "$.composition.depth_layers[0]",
            |request: &mut ImagePromptRequest| {
                request.composition.depth_layers[0] =
                    "readable text and 123 numerals in a chart".to_owned();
            },
        ),
        (
            "$.color.palette[0].usage",
            |request: &mut ImagePromptRequest| {
                request.color.palette[0].usage = "readable text and 123 numerals".to_owned();
            },
        ),
        (
            "$.surfaces[0].material",
            |request: &mut ImagePromptRequest| {
                request.surfaces[0].material = "surface with readable text".to_owned();
            },
        ),
        ("$.lighting.exposure", |request: &mut ImagePromptRequest| {
            request.lighting.exposure = "exposure with 123 numerals".to_owned();
        }),
    ];
    for (path, mutate) in cases {
        let mut request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-app-web-ui.json"
            )))
            .expect("JSON"),
        )
        .expect("request");
        mutate(&mut request);
        let (diagnostics, _) = validate_image_request(&request);
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "IMG_UI_TEXTLESS_CONTENT" && diagnostic.path == path
            }),
            "{path}: {diagnostics:#?}"
        );
    }
}

#[test]
fn textless_app_web_ui_checks_reference_change_and_consistency_text() {
    let cases: [(&str, RequestMutation); 4] = [
        (
            "$.references[0].description",
            |request: &mut ImagePromptRequest| {
                request.references = vec![ImageReference {
                    index: 1,
                    role: ImageReferenceRole::Style,
                    description: "reference with readable text".to_owned(),
                    use_for: vec!["layout".to_owned()],
                }];
            },
        ),
        (
            "$.references[0].use_for[0]",
            |request: &mut ImagePromptRequest| {
                request.references = vec![ImageReference {
                    index: 1,
                    role: ImageReferenceRole::Style,
                    description: "reference style".to_owned(),
                    use_for: vec!["text layout".to_owned()],
                }];
            },
        ),
        (
            "$.change_contract.preserve[0]",
            |request: &mut ImagePromptRequest| {
                request.references = vec![ImageReference {
                    index: 1,
                    role: ImageReferenceRole::Style,
                    description: "reference style".to_owned(),
                    use_for: vec!["layout".to_owned()],
                }];
                request.change_contract.preserve = vec!["preserve readable text".to_owned()];
            },
        ),
        (
            "$.consistency.shared_anchors[0]",
            |request: &mut ImagePromptRequest| {
                request.consistency = Some(ImageConsistencySpec {
                    dimensions: vec![ImageConsistencyDimension::Semantic],
                    shared_anchors: vec!["readable text".to_owned()],
                    sequence: None,
                });
            },
        ),
    ];
    for (path, mutate) in cases {
        let mut request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-app-web-ui.json"
            )))
            .expect("JSON"),
        )
        .expect("request");
        mutate(&mut request);
        let (diagnostics, _) = validate_image_request(&request);
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "IMG_UI_TEXTLESS_CONTENT" && diagnostic.path == path
            }),
            "{path}: {diagnostics:#?}"
        );
    }
}

#[test]
fn app_icon_checks_rendered_palette_surface_and_lighting_text() {
    let cases: [(&str, RequestMutation); 5] = [
        (
            "$.color.palette[0].usage",
            |request: &mut ImagePromptRequest| {
                request.color.palette[0].usage = "app icon with 123 numerals".to_owned();
            },
        ),
        (
            "$.surfaces[0].material",
            |request: &mut ImagePromptRequest| {
                request.surfaces[0].material = "app icon with readable text".to_owned();
            },
        ),
        ("$.lighting.exposure", |request: &mut ImagePromptRequest| {
            request.lighting.exposure = "app icon with 123 numerals".to_owned();
        }),
        (
            "$.constraints.required_elements[0]",
            |request: &mut ImagePromptRequest| {
                request.constraints.required_elements[0] = "app icon with 123 numerals".to_owned();
            },
        ),
        (
            "$.subjects[0].description",
            |request: &mut ImagePromptRequest| {
                request.subjects[0].description = "single icon with digits".to_owned();
            },
        ),
    ];
    for (path, mutate) in cases {
        let mut request = app_icon_example();
        mutate(&mut request);
        let (diagnostics, _) = validate_image_request(&request);
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "IMG_APP_ICON_FORBIDDEN_CONTENT" && diagnostic.path == path
            }),
            "{path}: {diagnostics:#?}"
        );
    }
}

#[test]
fn app_web_ui_rejects_a_static_non_interface_subject() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-web-ui.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.subjects[0].description =
        "a static red apple product photo for a web catalog with a button-shaped stem".to_owned();

    let (diagnostics, _) = validate_image_request(&request);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "IMG_UI_PROFILE_EVIDENCE")
        .expect("UI evidence diagnostic");
    assert_eq!(diagnostic.path, "$.subjects[*].description");
    assert!(diagnostic.message.contains("explicit screen/interface"));
}

#[test]
fn profile_rejects_unknown_and_category_incompatible_values() {
    let mut request = example();
    let unknown = parse(
        &include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-photo-lut.json"
        ))
        .replace(
            "\"language\": \"ko\",",
            "\"language\": \"ko\",\n  \"profile\": \"not-a-profile\",",
        ),
    )
    .expect("JSON");
    assert!(ImagePromptRequest::from_json(unknown).is_err());

    request.profile = crate::image::ImageProfile::LogoIdentity;
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_PROFILE_CATEGORY")
    );
}

fn pose_transfer_example() -> ImagePromptRequest {
    ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-pose-transfer.json"
        )))
        .expect("JSON"),
    )
    .expect("request")
}

fn cinematic_storyboard_example() -> ImagePromptRequest {
    ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-cinematic-storyboard.json"
        )))
        .expect("JSON"),
    )
    .expect("cinematic storyboard request")
}

#[test]
fn cinematic_storyboard_requires_textless_typed_panel_continuity() {
    let request = cinematic_storyboard_example();
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );

    let mut missing_plan = request.clone();
    missing_plan.storyboard = None;
    let (diagnostics, _) = validate_image_request(&missing_plan);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_CINEMATIC_STORYBOARD_REQUIRED")
    );

    let text_source = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .expect("JSON"),
    )
    .expect("typography request");
    let mut text_added = request;
    text_added
        .text_elements
        .push(text_source.text_elements.into_iter().next().expect("text"));
    let (diagnostics, _) = validate_image_request(&text_added);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_CINEMATIC_STORYBOARD_TEXT")
    );
}

#[test]
fn cinematic_storyboard_requires_research_backed_consistency_axes_and_order() {
    let request = cinematic_storyboard_example();
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );

    let mut missing_temporal = request.clone();
    missing_temporal
        .consistency
        .as_mut()
        .expect("consistency")
        .dimensions
        .retain(|dimension| *dimension != crate::image::ImageConsistencyDimension::Temporal);
    let (diagnostics, _) = validate_image_request(&missing_temporal);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "IMG_CINEMATIC_STORYBOARD_CONSISTENCY_DIMENSION"
        }),
        "{diagnostics:#?}"
    );

    let mut wrong_order = request;
    wrong_order
        .consistency
        .as_mut()
        .expect("consistency")
        .sequence
        .as_mut()
        .expect("sequence")
        .steps
        .swap(0, 1);
    let (diagnostics, _) = validate_image_request(&wrong_order);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "IMG_CINEMATIC_STORYBOARD_SEQUENCE_ORDER" }),
        "{diagnostics:#?}"
    );
}

#[test]
fn pose_transfer_requires_one_pose_and_one_character_reference() {
    let request = pose_transfer_example();
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );

    let mut missing_character = request.clone();
    missing_character.references.pop();
    let (diagnostics, _) = validate_image_request(&missing_character);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_POSE_TRANSFER_REFERENCES")
    );

    let mut wrong_mode = request;
    wrong_mode.task_mode = ImageTaskMode::Generate;
    let (diagnostics, _) = validate_image_request(&wrong_mode);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_POSE_TRANSFER_TASK_MODE")
    );
}

#[test]
fn pose_role_cannot_bypass_the_pose_transfer_profile() {
    let mut request = pose_transfer_example();
    request.render_profile = ImageRenderProfile::Structured;
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_POSE_REFERENCE_PROFILE_REQUIRED")
    );
}

#[test]
fn example_is_valid() {
    let (diagnostics, selected) = validate_image_request(&example());
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );
    assert!(!selected.is_empty());
}

#[test]
fn current_documented_3840_edge_is_accepted() {
    let mut request = example();
    request.output.width = 3840;
    request.output.height = 2160;
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        !diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == crate::diagnostic::Severity::Error
                && diagnostic.path == "$.output"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_EXPERIMENTAL_SIZE")
    );
}

#[test]
fn over_long_free_text_field_is_reported_before_a_generation_call() {
    let mut request = example();
    request.scene.atmosphere = "가".repeat(MAX_FREE_TEXT_CHARS + 1);
    let (diagnostics, _) = validate_image_request(&request);
    let field = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "IMG_FIELD_TOO_LONG")
        .expect("an over-long field is reported");
    assert_eq!(field.path, "$.scene.atmosphere");
    // A backend ceiling is not a contract violation, so this must not block compilation.
    assert_eq!(field.severity, crate::diagnostic::Severity::Warning);
}

#[test]
fn optional_text_fields_reject_whitespace_only_values() {
    let base = example();
    let mut cases = Vec::new();

    let mut request = base.clone();
    request.scene.time_of_day = Some("   ".to_owned());
    cases.push(("$.scene.time_of_day", request));

    let mut request = base.clone();
    request.scene.weather = Some("   ".to_owned());
    cases.push(("$.scene.weather", request));

    let mut request = base.clone();
    request.subjects[0].appearance = Some("   ".to_owned());
    cases.push(("$.subjects[0].appearance", request));

    let mut request = base.clone();
    request.lighting.rim_description = Some("   ".to_owned());
    cases.push(("$.lighting.rim_description", request));

    for field in [
        "tint",
        "tone_curve",
        "black_response",
        "contrast",
        "highlight_rolloff",
        "skin_tone_policy",
        "grain_size",
    ] {
        let mut request = base.clone();
        let lut = request.color.photo_lut.as_mut().expect("photo LUT");
        match field {
            "tint" => lut.tint = Some("   ".to_owned()),
            "tone_curve" => lut.tone_curve = Some("   ".to_owned()),
            "black_response" => lut.black_response = Some("   ".to_owned()),
            "contrast" => lut.contrast = Some("   ".to_owned()),
            "highlight_rolloff" => lut.highlight_rolloff = Some("   ".to_owned()),
            "skin_tone_policy" => lut.skin_tone_policy = Some("   ".to_owned()),
            "grain_size" => lut.grain_size = Some("   ".to_owned()),
            _ => unreachable!(),
        }
        cases.push((
            match field {
                "tint" => "$.color.photo_lut.tint",
                "tone_curve" => "$.color.photo_lut.tone_curve",
                "black_response" => "$.color.photo_lut.black_response",
                "contrast" => "$.color.photo_lut.contrast",
                "highlight_rolloff" => "$.color.photo_lut.highlight_rolloff",
                "skin_tone_policy" => "$.color.photo_lut.skin_tone_policy",
                "grain_size" => "$.color.photo_lut.grain_size",
                _ => unreachable!(),
            },
            request,
        ));
    }

    for (path, request) in cases {
        let (diagnostics, _) = validate_image_request(&request);
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "IMG_EMPTY_FIELD" && diagnostic.path == path
            }),
            "{path}: {diagnostics:#?}"
        );
    }
}

#[test]
fn realistic_requests_stay_inside_the_free_text_budget() {
    // The budget is only useful if ordinary authored requests clear it comfortably.
    for example in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-photo-lut.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-editorial-tier2.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-cinematic-keyart.json"
        )),
    ] {
        let request =
            ImagePromptRequest::from_json(parse(example).expect("JSON")).expect("request");
        let (diagnostics, _) = validate_image_request(&request);
        assert!(
            !diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.ends_with("TOO_LONG")),
            "a shipped example tripped the budget: {diagnostics:#?}"
        );
    }
}

#[test]
fn palette_sum_is_enforced() {
    let mut request = example();
    request.color.palette[0].proportion_percent = 59;
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_PALETTE_SUM")
    );
}

#[test]
fn custom_placement_bounds_are_enforced() {
    let mut request = example();
    request.subjects[0].placement = CanvasPlacement::Custom {
        x_percent: 80,
        y_percent: 10,
        width_percent: 30,
        height_percent: 30,
    };
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_PLACEMENT_BOUNDS")
    );
}

#[test]
fn tier_one_requires_exact_text() {
    let mut request = example();
    request.constraints.safety_tier = 1;
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_TIER1_WITHOUT_TEXT")
    );
}

#[test]
fn complex_multi_item_output_recommends_high_detail() {
    let mut request = example();
    request.taxonomy.category = Some("C10".to_owned());
    request.output.detail = ImageDetail::Medium;
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_COMPLEX_OUTPUT_DETAIL")
    );
}

#[test]
fn subscription_output_rejects_edges_above_3840() {
    let mut request = example();
    request.output.width = 3856;
    request.output.height = 2160;
    let (diagnostics, _) = validate_image_request(&request);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "IMG_DIMENSION_EDGE"
            && diagnostic.severity == crate::diagnostic::Severity::Error
    }));
}

#[test]
fn edit_requires_a_bound_base_and_explicit_change_contract() {
    let mut request = example();
    request.task_mode = ImageTaskMode::Edit;
    let (diagnostics, _) = validate_image_request(&request);
    for code in [
        "IMG_EDIT_REFERENCE_REQUIRED",
        "IMG_EDIT_BASE_REQUIRED",
        "IMG_CHANGE_ONLY_REQUIRED",
        "IMG_PRESERVE_REQUIRED",
    ] {
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code == code),
            "missing {code}: {diagnostics:#?}"
        );
    }
}

#[test]
fn a_bounded_edit_contract_is_valid() {
    let mut request = example();
    request.task_mode = ImageTaskMode::Edit;
    request.references = vec![ImageReference {
        index: 1,
        role: ImageReferenceRole::Base,
        description: "existing campaign image".to_owned(),
        use_for: vec!["layout and subject identity".to_owned()],
    }];
    request.change_contract = ImageChangeContract {
        change_only: vec!["change the cup glaze to cobalt blue".to_owned()],
        preserve: vec!["preserve all geometry, lighting, and background".to_owned()],
    };

    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );
}

#[test]
fn reference_indices_are_contiguous_and_unambiguous() {
    let mut request = example();
    request.references = vec![ImageReference {
        index: 2,
        role: ImageReferenceRole::Style,
        description: "style reference".to_owned(),
        use_for: vec!["surface treatment".to_owned()],
    }];
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_REFERENCE_ORDER")
    );
}

#[test]
fn generate_mode_rejects_edit_only_change_instructions() {
    let mut request = example();
    request.change_contract.change_only = vec!["change only the background".to_owned()];
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_GENERATE_CHANGE_ONLY")
    );
}

#[test]
fn edit_rejects_multiple_base_references() {
    let mut request = example();
    request.task_mode = ImageTaskMode::Edit;
    request.references = vec![
        ImageReference {
            index: 1,
            role: ImageReferenceRole::Base,
            description: "first base image".to_owned(),
            use_for: vec!["layout".to_owned()],
        },
        ImageReference {
            index: 2,
            role: ImageReferenceRole::Base,
            description: "second base image".to_owned(),
            use_for: vec!["subject identity".to_owned()],
        },
    ];
    request.change_contract = ImageChangeContract {
        change_only: vec!["change the product color".to_owned()],
        preserve: vec!["preserve layout and subject geometry".to_owned()],
    };
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_EDIT_BASE_AMBIGUOUS")
    );
}

#[test]
fn reference_rejects_duplicate_use_responsibilities() {
    let mut request = example();
    request.references = vec![ImageReference {
        index: 1,
        role: ImageReferenceRole::Style,
        description: "style reference".to_owned(),
        use_for: vec![
            "surface treatment".to_owned(),
            " Surface Treatment ".to_owned(),
        ],
    }];
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_REFERENCE_USE_DUPLICATE")
    );
}

#[test]
fn generate_rejects_edit_only_reference_roles() {
    let mut request = example();
    request.references = vec![ImageReference {
        index: 1,
        role: ImageReferenceRole::Base,
        description: "existing image".to_owned(),
        use_for: vec!["layout".to_owned()],
    }];
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_GENERATE_EDIT_REFERENCE_ROLE")
    );
}

#[test]
fn composite_rejects_multiple_base_images() {
    let mut request = example();
    request.task_mode = ImageTaskMode::Composite;
    request.references = vec![
        ImageReference {
            index: 1,
            role: ImageReferenceRole::Base,
            description: "first base".to_owned(),
            use_for: vec!["background".to_owned()],
        },
        ImageReference {
            index: 2,
            role: ImageReferenceRole::Base,
            description: "second base".to_owned(),
            use_for: vec!["product".to_owned()],
        },
    ];
    request.change_contract = ImageChangeContract {
        change_only: vec!["place the product into the background".to_owned()],
        preserve: vec!["preserve the background and product identity".to_owned()],
    };
    let (diagnostics, _) = validate_image_request(&request);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "IMG_COMPOSITE_BASE_AMBIGUOUS")
    );
}
