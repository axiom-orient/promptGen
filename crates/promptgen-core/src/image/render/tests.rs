use super::*;
use std::collections::BTreeSet;

use crate::catalog::catalog;
use crate::diagnostic::Severity;
use crate::image::compile_image_prompt;
use crate::image::model::ImagePromptRequest;
use crate::image::validate::validate_image_request;
use crate::json::{JsonValue, parse};

#[test]
fn rendered_prompt_has_stable_section_order_and_ar_tail() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-photo-lut.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error)
    );
    let prompt = render_image_prompt(&request, &selected);
    assert!(prompt.find("SCENE").is_none());
    assert!(prompt.contains("사진 LUT"));
    assert!(!prompt.contains("환경: \""));
    assert!(prompt.contains("환경: 오전의 조용한 콘크리트 로스터리"));
    assert!(prompt.trim_end().ends_with("AR 2:3"));
}

/// docs/DESIGN.md §8 and docs/IMAGE_STUDIO_UX_SPEC.md P2: provenance and implementation
/// vocabulary stays inside the inspector. The compiled prompt is the most user-facing
/// surface there is — it is printed by the CLI and handed to the image model — so a term
/// that leaks here is both a copy defect and payload the model cannot act on.
#[test]
fn compiled_prompt_never_exposes_internal_vocabulary() {
    const INTERNAL: &[&str] = &["공냥이", "Gongnyang", "line-addressed", "Tier-1 가드"];

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
    ] {
        let request =
            ImagePromptRequest::from_json(parse(example).expect("JSON")).expect("request");
        let (_, selected) = validate_image_request(&request);
        let prompt = render_image_prompt(&request, &selected);
        for term in INTERNAL {
            assert!(
                !prompt.contains(term),
                "compiled prompt leaks internal vocabulary {term:?}"
            );
        }
    }
}

#[test]
fn no_catalog_storage_metadata_reaches_compiled_prompts() {
    for entry in &catalog().expect("catalog").entries {
        let request = structured_request(entry, PromptLanguage::English);
        let (_, selected) = validate_image_request(&request);
        let prompt = render_image_prompt(&request, &selected);
        for internal in [
            &entry.id,
            &entry.source_path,
            &entry.source_sha256,
            &entry.asset_path,
        ] {
            assert!(!prompt.contains(internal), "{} leaked {internal}", entry.id);
        }
    }
}

#[test]
fn exact_text_is_quoted_and_ordered() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = PromptLanguage::English;
    let (_, selected) = validate_image_request(&request);
    let prompt = render_image_prompt(&request, &selected);
    assert!(prompt.contains("text_element_1:"));
    assert!(prompt.contains("1. \"URBAN\""));
    assert!(prompt.contains("2. \"SIGNAL\""));
    assert!(prompt.contains("Render only these quoted lines"));
    assert!(!prompt.contains("exact_text_line_"));
    assert!(prompt.contains("treatment=\"parallel shallow diagonal cuts rising rightward at 2 degrees across the lower third"));
    assert!(prompt.contains("separator_count_per_line=3"));
    assert!(
        prompt
            .find("Render only these quoted lines")
            .expect("copy instruction")
            < prompt.find("1. \"URBAN\"").expect("first line"),
        "the exact copy instruction must immediately precede the ordered quoted lines"
    );
    assert!(
        prompt.find("EXACT TEXT IN IMAGE").expect("exact text")
            < prompt.find("LIGHTING").expect("lighting"),
        "exact copy should be near the composition it controls, before lower-priority rendering detail"
    );
}

#[test]
fn exact_text_preserves_repeated_and_nonbreaking_spaces() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = PromptLanguage::English;
    let line = "URBAN  SIGNAL\u{00a0}ARCHIVE".to_owned();
    request.text_elements[0].lines = vec![line.clone()];

    let compilation = compile_image_prompt(&request);
    let prompt = compilation.prompt.as_deref().expect("valid compilation");
    let exact_line = JsonValue::from(line).to_compact_string();
    assert!(prompt.contains(&format!("1. {exact_line}")), "{prompt}");
}

#[test]
fn identical_environment_and_background_are_rendered_once() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-photo-lut.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = PromptLanguage::English;
    let shared = "single shared studio context".to_owned();
    request.scene.environment = shared.clone();
    request.scene.background = shared.clone();
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );

    let prompt = render_image_prompt(&request, &selected);
    assert_eq!(prompt.matches(&shared).count(), 1);
    assert!(prompt.contains("Environment and background: single shared studio context"));
}

#[test]
fn edit_prompt_binds_reference_roles_changes_and_invariants() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-edit-product.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = PromptLanguage::English;
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );
    let prompt = render_image_prompt(&request, &selected);
    assert!(prompt.contains("INPUT REFERENCES"));
    assert!(prompt.contains("image_1: role=base"));
    assert!(prompt.contains("CHANGE CONTRACT"));
    assert!(
        prompt.find("INPUT REFERENCES").expect("references")
            < prompt.find("CHANGE CONTRACT").expect("change contract")
    );
    assert!(
        prompt.find("CHANGE CONTRACT").expect("change contract")
            < prompt.find("SCENE").expect("scene")
    );
    assert!(prompt.contains("Change only"));
    assert!(prompt.contains("Preserve"));
    assert!(prompt.contains("Reapply this complete preserve list in every later iteration"));
}

#[test]
fn pose_transfer_prompt_locks_pose_identity_and_three_fidelity_gates() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-pose-transfer.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = PromptLanguage::English;
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );

    let prompt = render_image_prompt(&request, &selected);
    for token in [
        "image_1: role=pose",
        "image_2: role=subject",
        "POSE FIDELITY CONTRACT",
        "reference roles disjoint",
        "pose reference pixels are final authority",
        "Never mirror",
        "Fidelity gate 1",
        "Fidelity gate 2",
        "Fidelity gate 3",
        "fails this gate is still incorrect",
        "change only the earliest failing gate",
    ] {
        assert!(prompt.contains(token), "missing {token:?}\n{prompt}");
    }
    assert!(
        prompt.find("INPUT REFERENCES").expect("references")
            < prompt.find("CHANGE CONTRACT").expect("change contract")
    );
    assert!(
        prompt.find("CHANGE CONTRACT").expect("change contract")
            < prompt
                .find("POSE FIDELITY CONTRACT")
                .expect("pose fidelity")
    );
    assert!(
        prompt
            .find("POSE FIDELITY CONTRACT")
            .expect("pose fidelity")
            < prompt.find("SCENE").expect("scene")
    );
}

#[test]
fn cinematic_storyboard_prompt_locks_textless_panels_eyelines_and_identity() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-cinematic-storyboard.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = PromptLanguage::English;
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );

    let prompt = render_image_prompt(&request, &selected);
    for token in [
        "TEXTLESS CINEMATIC STORYBOARD CONTRACT",
        "MULTI-IMAGE CONSISTENCY CONTRACT",
        "multi_view, character, temporal, semantic",
        "Condition on previous output: yes",
        "panel_1",
        "panel_2",
        "panel_3",
        "Never render panel numbers, captions, speech balloons, dialogue, logos, watermarks, UI, or pseudo-text.",
        "Do not reverse screen direction without a neutral re-establishing shot",
    ] {
        assert!(prompt.contains(token), "missing {token:?}\n{prompt}");
    }
    assert!(
        prompt
            .find("TEXTLESS CINEMATIC STORYBOARD CONTRACT")
            .expect("storyboard")
            < prompt.find("SCENE").expect("scene")
    );
    assert!(
        prompt
            .find("MULTI-IMAGE CONSISTENCY CONTRACT")
            .expect("consistency")
            < prompt.find("SCENE").expect("scene")
    );
}

#[test]
fn app_icon_prompt_exposes_ordered_master_concept_and_delivery_boundary() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-icon.json"
        )))
        .expect("JSON"),
    )
    .expect("app icon request");
    let mut request = request;
    request.language = PromptLanguage::English;
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );

    let prompt = render_image_prompt(&request, &selected);
    for token in [
        "APP ICON MASTER CONCEPT CONTRACT",
        "Product purpose",
        "Core metaphor",
        "Component geometry",
        "Centered composition",
        "32px silhouette check",
        "No text, letters, numerals, pseudo-writing, UI screenshot, device mockup, rounded-square mask, app-icon frame, or watermark",
        "Promise one opaque 1024x1024 PNG master concept only",
    ] {
        assert!(prompt.contains(token), "missing {token:?}\n{prompt}");
    }
    assert!(
        prompt.find("Product purpose").expect("purpose")
            < prompt.find("Core metaphor").expect("metaphor")
            && prompt.find("Core metaphor").expect("metaphor")
                < prompt.find("Component geometry").expect("geometry")
            && prompt.find("Component geometry").expect("geometry")
                < prompt.find("Centered composition").expect("composition")
            && prompt.find("Centered composition").expect("composition")
                < prompt.find("32px silhouette check").expect("silhouette"),
        "app icon contract must state purpose -> metaphor -> geometry -> centered composition -> 32px check"
    );
    for catalog_directive in [
        "label placement",
        "secondary view",
        "callout",
        "package surface",
        "brand application",
        "SELECTED VISUAL SYSTEM",
    ] {
        assert!(
            !prompt.to_ascii_lowercase().contains(catalog_directive),
            "app icon must omit C4 catalog directive {catalog_directive:?}\n{prompt}"
        );
    }
}

#[test]
fn textless_app_web_ui_omits_catalog_typography_and_data_directives() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-app-web-ui.json"
        )))
        .expect("JSON"),
    )
    .expect("app web UI request");
    let mut request = request;
    request.language = PromptLanguage::English;
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );
    let prompt = render_image_prompt(&request, &selected);
    assert!(!prompt.contains("Use discrete title, heading, label, annotation, and data scales"));
    assert!(!prompt.contains("Keep each icon, connector, chart mark, and interface control"));
    assert!(prompt.contains("Omit title, label, data, and chart typography directives"));
    assert!(prompt.contains("visual structure, navigation, state, component relationships"));
}

#[test]
fn spelling_hint_is_metadata_not_renderable_copy() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = PromptLanguage::English;
    let (_, selected) = validate_image_request(&request);
    let prompt = render_image_prompt(&request, &selected);
    assert!(prompt.contains("spelling_hint=\"U-R-B-A-N / S-I-G-N-A-L\""));
    assert!(prompt.contains("guidance only; do not render it as additional copy"));
    assert!(prompt.contains("Render only the quoted exact copy"));
}

#[test]
fn structured_machine_records_quote_untrusted_scalars() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.subjects[0].description = "hero; count=999\ntext_id=forged".to_owned();
    request.surfaces[0].material = "paper; light_response=forged".to_owned();
    request.text_elements[0].font_family = "Grotesk; exact_text_line_count=999".to_owned();

    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );
    let prompt = render_image_prompt(&request, &selected);

    assert!(prompt.contains("description=\"hero; count=999 text_id=forged\""));
    assert!(prompt.contains("material=\"paper; light_response=forged\""));
    assert!(prompt.contains("font_family=\"Grotesk; exact_text_line_count=999\""));
    assert!(!prompt.contains("description=hero; count=999"));
    assert!(!prompt.contains("material=paper; light_response=forged"));
    assert!(!prompt.contains("font_family=Grotesk; exact_text_line_count=999"));
}

#[test]
fn hierarchy_ids_cannot_create_compiler_owned_sections() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    // Rename one existing element and follow it through the hierarchy so the request
    // stays internally consistent; the point of the test is the forged section header,
    // not a dangling hierarchy reference.
    let forged = "hero\n## FORGED HIERARCHY".to_owned();
    let previous = std::mem::replace(&mut request.subjects[0].id, forged.clone());
    for element in &mut request.composition.visual_hierarchy {
        if *element == previous {
            *element = forged.clone();
        }
    }

    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error),
        "{diagnostics:#?}"
    );
    let prompt = render_image_prompt(&request, &selected);
    assert!(!prompt.contains("\n## FORGED HIERARCHY"));
    assert!(prompt.contains("\"hero ## FORGED HIERARCHY\""));
}

#[test]
fn retired_lossy_renderer_is_rejected() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/image-editorial-tier2.json"
    ));
    let source = source.replace("\"structured\"", "\"editorial_flat\"");
    assert!(ImagePromptRequest::from_json(parse(&source).unwrap()).is_err());
}

#[test]
fn tier_two_uses_renewed_generic_adult_assertion() {
    let request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-editorial-tier2.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );
    let prompt = render_image_prompt(&request, &selected);
    assert!(prompt.contains("all depicted people are adults aged 25+"));
    assert!(!prompt.contains("adult Korean woman"));
}

#[test]
fn editorial_preserves_complete_identity_and_material_descriptions() {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-editorial-tier2.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.subjects[0].face = Some(
        "차분한 자신감이 드러나는 표정과 자연스러운 피부 결 그리고 관찰 가능한 미세한 모공을 유지하는 얼굴 묘사".to_owned(),
    );
    request.subjects[0].hair = Some(
        "어깨 아래까지 이어지는 짙은 갈색 머리카락이 의상 실루엣을 가리지 않도록 뒤로 정돈된 헤어"
            .to_owned(),
    );
    request.scene.environment =
        "해 질 무렵 비어 있는 해안 리조트 테라스의 석회암 바닥과 먼 수평선이 동시에 보이는 장면"
            .to_owned();
    request.subjects[0].appearance = Some(
        "완전 불투명한 세이지색 리넨 테일러드 재킷과 높은 라운드넥 크림 이너 및 발목 길이 와이드 팬츠의 여유로운 핏".to_owned(),
    );
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error),
        "{diagnostics:#?}"
    );
    let prompt = render_image_prompt(&request, &selected);
    for value in [
        request.subjects[0].face.as_deref().unwrap(),
        request.subjects[0].hair.as_deref().unwrap(),
        request.scene.environment.as_str(),
        request.subjects[0].appearance.as_deref().unwrap(),
    ] {
        assert!(
            prompt.contains(value),
            "required detail was truncated: {value}"
        );
    }
}

#[test]
fn catalog_contains_no_parallel_supporting_entries() {
    assert!(
        catalog()
            .expect("catalog")
            .entries
            .iter()
            .all(CatalogEntry::is_primary_output)
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PromptViolation {
    Leakage,
    Completeness,
    Language,
    Structure,
    Determinism,
}

fn audit_determinism(first: &str, second: &str) -> BTreeSet<PromptViolation> {
    let mut violations = BTreeSet::new();
    if first != second {
        violations.insert(PromptViolation::Determinism);
    }
    violations
}

fn contains_identifier(text: &str, identifier: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|token| token.eq_ignore_ascii_case(identifier))
}

fn audit_runtime_prompt(
    entry: &CatalogEntry,
    prompt: &str,
    expected_directives: &[&str],
    language: PromptLanguage,
) -> BTreeSet<PromptViolation> {
    let mut violations = BTreeSet::new();

    if prompt.contains(&entry.source_path)
        || prompt.contains(&entry.source_sha256)
        || prompt.contains(&entry.asset_path)
        || contains_identifier(prompt, &entry.id)
        || prompt
            .to_ascii_lowercase()
            .contains(&format!("directive_{}", entry.id.to_ascii_lowercase()))
        || (entry.name_en.contains('_') && prompt.contains(&entry.name_en))
    {
        violations.insert(PromptViolation::Leakage);
    }

    for directive in expected_directives {
        if prompt.matches(*directive).count() != 1 {
            violations.insert(PromptViolation::Completeness);
        }
    }

    if language == PromptLanguage::English {
        for line in prompt.lines() {
            let compiler_catalog_line = line.starts_with("- Subject rule:")
                || line.starts_with("- Scene rule:")
                || line.starts_with("- Composition rule:")
                || line.starts_with("- Camera rule:")
                || line.starts_with("- Lighting rule:")
                || line.starts_with("- Color rule:")
                || line.starts_with("- Material rule:")
                || line.starts_with("- Typography rule:")
                || line.starts_with("- Motion rule:")
                || line.starts_with("- Constraint rule:");
            if compiler_catalog_line
                && line
                    .chars()
                    .any(|character| ('\u{ac00}'..='\u{d7a3}').contains(&character))
            {
                violations.insert(PromptViolation::Language);
            }
        }
        if prompt.contains(&format!("{}:", entry.name_ko)) {
            violations.insert(PromptViolation::Language);
        }
    }

    // A bullet that ends in `:` is a defect only when nothing follows it. A lead-in
    // that introduces a deeper-indented block (quoted exact copy, for example) is the
    // intended shape, so the audit compares indentation with the following line.
    let lines = prompt.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();
        let introduces_block = lines.get(index + 1).is_some_and(|next| {
            !next.trim().is_empty() && next.len() - next.trim_start().len() > indent
        });
        let empty_label = trimmed.starts_with("- ")
            && trimmed.ends_with(':')
            && trimmed.matches(':').count() == 1
            && !introduces_block;
        let empty_assignment = trimmed.starts_with("- ")
            && trimmed.ends_with('=')
            && trimmed.matches('=').count() == 1;
        if empty_label || empty_assignment {
            violations.insert(PromptViolation::Structure);
        }
    }
    if prompt.contains("::") {
        violations.insert(PromptViolation::Structure);
    }
    violations
}

fn structured_request(entry: &CatalogEntry, language: PromptLanguage) -> ImagePromptRequest {
    let mut request = ImagePromptRequest::from_json(
        parse(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/image-typography-poster.json"
        )))
        .expect("JSON"),
    )
    .expect("request");
    request.language = language;
    request.render_profile = ImageRenderProfile::Structured;
    request.taxonomy.category = Some(entry.id.clone());
    if entry.default_safety_tier == 2 {
        request.constraints.safety_tier = 2;
        request.constraints.adult_subjects_only = true;
        request.constraints.original_characters_only = true;
    }
    request
}

#[test]
fn specialized_profiles_render_their_category_contracts() {
    for (file, marker) in [
        ("image-logo-identity.json", "로고 아이덴티티 프로필"),
        ("image-app-web-ui.json", "앱·웹 UI 프로필"),
        ("image-character-poomsae.json", "품새 자세 프로필"),
    ] {
        let source = std::fs::read_to_string(format!(
            "{}/../../examples/{file}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("profile example");
        let request =
            ImagePromptRequest::from_json(parse(&source).expect("JSON")).expect("request");
        let outcome = compile_image_prompt(&request);
        assert!(
            outcome
                .prompt
                .as_deref()
                .is_some_and(|prompt| prompt.contains(marker)),
            "{file}: {:?}",
            outcome.diagnostics
        );
    }
}

/// Checks every category directive in both languages through the one lossless renderer.
#[test]
fn every_catalog_entry_compiles_to_a_clean_prompt() {
    let catalog = catalog().expect("catalog");
    let mut cases = 0usize;
    let mut structured_directives = 0usize;

    for entry in &catalog.entries {
        for language in [PromptLanguage::Korean, PromptLanguage::English] {
            let request = structured_request(entry, language);
            let (diagnostics, selected) = validate_image_request(&request);
            let errors = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == Severity::Error)
                .collect::<Vec<_>>();
            assert!(
                errors.is_empty(),
                "{} structured {language:?}: {errors:#?}",
                entry.id
            );
            let prompt = render_image_prompt(&request, &selected);
            let expected = entry
                .prompt_directives
                .iter()
                .map(|directive| directive.text.as_str())
                .collect::<Vec<_>>();
            let violations = audit_runtime_prompt(entry, &prompt, &expected, language);
            assert!(
                violations.is_empty(),
                "{} structured {language:?}: {violations:?}\n{prompt}",
                entry.id
            );
            assert_eq!(render_image_prompt(&request, &selected), prompt);
            structured_directives += expected.len();
            cases += 1;
        }
    }

    assert_eq!(cases, catalog.entries.len() * 2);
    assert_eq!(
        structured_directives,
        catalog
            .entries
            .iter()
            .map(|entry| entry.prompt_directives.len())
            .sum::<usize>()
            * 2,
        "every structured directive must be checked once per language"
    );
}

#[test]
fn prompt_auditor_detects_deliberate_mutations() {
    let entry = catalog()
        .expect("catalog")
        .entries
        .iter()
        .find(|entry| entry.id == "C1")
        .expect("C1");
    let request = structured_request(entry, PromptLanguage::English);
    let (diagnostics, selected) = validate_image_request(&request);
    assert!(
        !diagnostics
            .iter()
            .any(|item| item.severity == Severity::Error)
    );
    let prompt = render_image_prompt(&request, &selected);
    let expected = entry
        .prompt_directives
        .iter()
        .map(|directive| directive.text.as_str())
        .collect::<Vec<_>>();

    let leakage = format!("{prompt}\n{}", entry.source_path);
    assert_ne!(leakage, prompt);
    assert!(
        audit_runtime_prompt(entry, &leakage, &expected, PromptLanguage::English)
            .contains(&PromptViolation::Leakage)
    );

    let completeness_missing = prompt.replacen(expected[0], "", 1);
    assert_ne!(completeness_missing, prompt);
    assert!(
        audit_runtime_prompt(
            entry,
            &completeness_missing,
            &expected,
            PromptLanguage::English,
        )
        .contains(&PromptViolation::Completeness)
    );

    let completeness_duplicate = format!("{prompt}\n{}", expected[0]);
    assert_ne!(completeness_duplicate, prompt);
    assert!(
        audit_runtime_prompt(
            entry,
            &completeness_duplicate,
            &expected,
            PromptLanguage::English,
        )
        .contains(&PromptViolation::Completeness)
    );

    let language = format!("{prompt}\n- Subject rule: 번역되지 않은 원문");
    assert_ne!(language, prompt);
    assert!(
        audit_runtime_prompt(entry, &language, &expected, PromptLanguage::English)
            .contains(&PromptViolation::Language)
    );

    let structure = format!("{prompt}\n- Empty label:");
    assert_ne!(structure, prompt);
    assert!(
        audit_runtime_prompt(entry, &structure, &expected, PromptLanguage::English)
            .contains(&PromptViolation::Structure)
    );

    let determinism = format!("{prompt}\nchanged second render");
    assert_ne!(determinism, prompt);
    assert!(audit_determinism(&prompt, &determinism).contains(&PromptViolation::Determinism));
    assert!(audit_determinism(&prompt, &prompt).is_empty());
}

#[test]
fn journal_profile_direction_is_rendered_once_in_each_language() {
    let mut request = structured_request(&catalog().unwrap().entries[2], PromptLanguage::Korean);
    request.profile = ImageProfile::TravelJournal;
    for (language, sentence) in [
        (
            PromptLanguage::Korean,
            "중심 장면 또는 제공한 사진을 주인공으로 두고 종이·메모는 주변에 배치한다.",
        ),
        (
            PromptLanguage::English,
            "Keep the main scene or supplied photograph dominant and place paper and notes around it.",
        ),
    ] {
        request.language = language;
        let outcome = compile_image_prompt(&request);
        let prompt = outcome.prompt.unwrap();
        assert_eq!(prompt.matches(sentence).count(), 1);
        assert!(!prompt.contains("Build the campaign"));
    }
}
