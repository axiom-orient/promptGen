use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    CatalogEntry, CatalogSelectionIssue, exact_text_is_required, inspect_catalog_selection,
};
use crate::diagnostic::Diagnostic;

use super::lut::{PRESET_NAMES, resolve_photo_lut};
use super::model::{
    CanvasPlacement, ColorSpec, ImageConsistencyDimension, ImageDetail, ImageProfile,
    ImagePromptRequest, ImageReferenceRole, ImageRenderProfile, ImageTaskMode, PhotoLutSelection,
    VisualMedium,
};
use super::specificity::{
    contains_hangul_and_latin, inspect_description, inspect_exact_text, inspect_tier2_youth_terms,
};

const MAX_EDGE: u32 = 3840;
const MIN_PIXELS: u64 = 655_360;
const MAX_PIXELS: u64 = 8_294_400;
const EXPERIMENTAL_PIXELS: u64 = 3_686_400;
/// Free-text budget for one structured field, in characters.
///
/// Nothing else bounds these fields, so a single pasted document silently becomes a prompt
/// the image backend will reject — and that rejection costs a generation call, while this
/// check costs nothing. The bound keeps structured requests within a predictable
/// model input envelope.
const MAX_FREE_TEXT_CHARS: usize = 2_000;
const UI_SURFACE_TERMS: &[&str] = &[
    "screen",
    "interface",
    "dashboard",
    "viewport",
    "화면",
    "인터페이스",
    "대시보드",
    "뷰포트",
];
const UI_STRUCTURE_TERMS: &[&str] = &[
    "sidebar",
    "navigation",
    "table",
    "button",
    "form",
    "card",
    "panel",
    "component",
    "grid",
    "menu",
    "control",
    "label",
    "field",
    "status",
    "state",
    "chart",
    "legend",
    "cta",
    "사이드바",
    "탐색",
    "표",
    "버튼",
    "폼",
    "카드",
    "패널",
    "컴포넌트",
    "그리드",
    "메뉴",
    "컨트롤",
    "라벨",
    "필드",
    "상태",
    "차트",
    "범례",
];
const UI_RELATION_TERMS: &[&str] = &[
    "with",
    "including",
    "includes",
    "contains",
    "containing",
    "showing",
    "shows",
    "displaying",
    "displays",
    "featuring",
    "features",
    "has",
    "have",
    "와",
    "과",
    "및",
    "포함",
    "구성",
    "있는",
    "보이는",
];
const TEXTLESS_UI_FORBIDDEN_TERMS: &[&str] = &[
    "text",
    "texts",
    "letter",
    "letters",
    "numeral",
    "numerals",
    "number",
    "numbers",
    "numeric",
    "digit",
    "digits",
    "readable text",
    "readable copy",
    "pseudo-text",
    "pseudo text",
    "pseudo-writing",
    "pseudo writing",
    "lorem ipsum",
    "invented data",
    "data point",
    "chart",
    "charts",
    "graph",
    "graphs",
    "histogram",
    "텍스트",
    "문구",
    "글자",
    "숫자",
    "의사문자",
    "데이터",
    "차트",
    "그래프",
    "도표",
];
const APP_ICON_FORBIDDEN_TERMS: &[&str] = &[
    "text",
    "texts",
    "letter",
    "letters",
    "numeral",
    "numerals",
    "number",
    "numbers",
    "digit",
    "digits",
    "readable copy",
    "pseudo-writing",
    "pseudo writing",
    "ui screenshot",
    "ui screen",
    "device mockup",
    "device frame",
    "rounded-square mask",
    "rounded square mask",
    "app-icon frame",
    "app icon frame",
    "watermark",
    "텍스트",
    "문자",
    "글자",
    "숫자",
    "의사문자",
    "UI 스크린샷",
    "디바이스 목업",
    "둥근 사각형 마스크",
    "앱 아이콘 프레임",
    "워터마크",
];
const NEGATION_TERMS: &[&str] = &[
    "no ",
    "without ",
    "exclude ",
    "avoid ",
    "prohibit ",
    "not ",
    "do not ",
    "don't ",
    "없",
    "없는",
    "없음",
    "금지",
    "제외",
    "배제",
];
const NEGATION_SCOPE_BREAK_TERMS: &[&str] = &[
    "but",
    "however",
    "instead",
    "also",
    "plus",
    "하지만",
    "그러나",
    "대신",
    "또한",
];
const LIST_CONNECTOR_TERMS: &[&str] = &["and", "or", "및", "또는", "그리고", "와", "과"];
const NEGATION_EMPHASIS_TERMS: &[&str] = &["only", "just", "merely", "simply"];

fn is_ui_token_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
}

fn ui_term_spans(value: &str, terms: &[&str]) -> Vec<(usize, usize)> {
    let lower = value.to_ascii_lowercase();
    terms
        .iter()
        .flat_map(|term| {
            if term
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                lower
                    .match_indices(term)
                    .filter_map(|(start, _)| {
                        let end = start + term.len();
                        let before = lower[..start].chars().next_back();
                        let after = lower[end..].chars().next();
                        (before.is_none_or(|character| !is_ui_token_character(character))
                            && after.is_none_or(|character| !is_ui_token_character(character)))
                        .then_some((start, end))
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
            } else {
                lower
                    .match_indices(term)
                    .map(|(start, _)| (start, start + term.len()))
                    .collect::<Vec<_>>()
                    .into_iter()
            }
        })
        .collect()
}

fn contains_ui_term(value: &str, terms: &[&str]) -> bool {
    !ui_term_spans(value, terms).is_empty()
}

fn contains_ui_surface_component_relation(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let surfaces = ui_term_spans(value, UI_SURFACE_TERMS);
    let components = ui_term_spans(value, UI_STRUCTURE_TERMS);
    surfaces.iter().any(|(_, surface_end)| {
        components.iter().any(|(component_start, _)| {
            *component_start >= *surface_end
                && contains_ui_term(&lower[*surface_end..*component_start], UI_RELATION_TERMS)
        })
    })
}

fn non_overlapping_forbidden_spans(value: &str, terms: &[&str]) -> Vec<(usize, usize)> {
    let mut spans = ui_term_spans(value, terms);
    spans.sort_by(|(left_start, left_end), (right_start, right_end)| {
        left_start
            .cmp(right_start)
            .then_with(|| (right_end - right_start).cmp(&(left_end - left_start)))
    });
    let mut selected = Vec::new();
    for span @ (start, end) in spans {
        if selected
            .iter()
            .all(|(selected_start, selected_end)| end <= *selected_start || start >= *selected_end)
        {
            selected.push(span);
        }
    }
    selected.sort_unstable_by_key(|(start, _)| *start);
    selected
}

fn contains_list_connector(value: &str) -> bool {
    !ui_term_spans(value, LIST_CONNECTOR_TERMS).is_empty()
}

fn contains_scope_break(value: &str, terms: &[&str]) -> bool {
    if !ui_term_spans(value, NEGATION_SCOPE_BREAK_TERMS).is_empty() {
        return true;
    }
    // "add" is a scope break after an already-mentioned forbidden item, but
    // remains a normal verb in "do not add readable text".
    ui_term_spans(value, &["add", "adding", "added"])
        .iter()
        .any(|(start, _)| !non_overlapping_forbidden_spans(&value[..*start], terms).is_empty())
}

fn local_negation_start(value: &str) -> Option<(usize, usize)> {
    NEGATION_TERMS
        .iter()
        .flat_map(|negation| {
            value
                .match_indices(negation)
                .map(move |(start, _)| (start, start + negation.len()))
        })
        .max_by_key(|(start, _)| *start)
}

fn starts_with_negation_emphasis(value: &str) -> bool {
    let value = value.trim_start();
    NEGATION_EMPHASIS_TERMS.iter().any(|term| {
        value.strip_prefix(term).is_some_and(|remainder| {
            remainder
                .chars()
                .next()
                .is_none_or(|character| !is_ui_token_character(character))
        })
    })
}

fn starts_with_not_emphasis(value: &str) -> bool {
    let value = value.trim_start();
    ["not", "do not", "don't"].iter().any(|negation| {
        value
            .strip_prefix(negation)
            .is_some_and(starts_with_negation_emphasis)
    })
}

fn term_is_locally_negated(
    segment: &str,
    start: usize,
    end: usize,
    terms: &[&str],
    spans: &[(usize, usize)],
) -> bool {
    let suffix = segment[end..].chars().take(32).collect::<String>();
    if starts_with_not_emphasis(&suffix) {
        return false;
    }
    if NEGATION_TERMS
        .iter()
        .any(|negation| suffix.trim_start().starts_with(negation.trim()))
    {
        return true;
    }
    let Some((_, negation_end)) = local_negation_start(&segment[..start]) else {
        return false;
    };
    let between = &segment[negation_end..start];
    if starts_with_negation_emphasis(between) {
        return false;
    }
    if contains_scope_break(between, terms) {
        return false;
    }
    // "no circles and readable text" is not a list. A connector can inherit
    // negation only after the same segment already named a forbidden item.
    if contains_list_connector(between)
        && !spans
            .iter()
            .any(|(prior_start, prior_end)| *prior_end <= start && *prior_start >= negation_end)
    {
        return false;
    }
    true
}

fn is_forbidden_list_continuation(segment: &str, spans: &[(usize, usize)]) -> bool {
    if spans.is_empty() {
        return false;
    }
    let mut remainder = String::new();
    let mut cursor = 0;
    for (start, end) in spans {
        remainder.push_str(&segment[cursor..*start]);
        cursor = *end;
    }
    remainder.push_str(&segment[cursor..]);
    remainder.split_whitespace().all(|word| {
        let word = word.trim_matches(|character: char| {
            matches!(
                character,
                ',' | '.' | ';' | ':' | '!' | '?' | '"' | '\'' | '(' | ')'
            )
        });
        matches!(
            word,
            "and"
                | "or"
                | "&"
                | "및"
                | "또는"
                | "그리고"
                | "와"
                | "과"
                | "no"
                | "without"
                | "exclude"
                | "avoid"
                | "prohibit"
                | "not"
                | "do"
                | "don't"
                | "없"
                | "없는"
                | "없음"
                | "금지"
                | "제외"
                | "배제"
                | "readable"
        )
    })
}

fn contains_positive_forbidden_term(value: &str, terms: &[&str]) -> bool {
    let lower = value.to_lowercase();
    let mut segments = Vec::new();
    let mut start = 0;
    let mut preceded_by_comma = false;
    for (index, character) in lower.char_indices() {
        if matches!(
            character,
            ',' | '，' | ';' | '；' | '.' | '。' | ':' | '!' | '！' | '?' | '？' | '|' | '\n'
        ) {
            segments.push((&lower[start..index], preceded_by_comma));
            start = index + character.len_utf8();
            preceded_by_comma = matches!(character, ',' | '，');
        }
    }
    segments.push((&lower[start..], preceded_by_comma));

    let mut negative_list_active = false;
    for (segment, continued_from_comma) in segments {
        if !continued_from_comma {
            negative_list_active = false;
        }
        let spans = non_overlapping_forbidden_spans(segment, terms);
        let direct_negation = spans
            .iter()
            .map(|(start, end)| term_is_locally_negated(segment, *start, *end, terms, &spans))
            .collect::<Vec<_>>();
        if spans.iter().enumerate().any(|(index, _)| {
            !direct_negation[index]
                && !(negative_list_active && is_forbidden_list_continuation(segment, &spans))
        }) {
            return true;
        }
        let has_scope_break_after_first = spans
            .first()
            .is_some_and(|(_, first_end)| contains_scope_break(&segment[*first_end..], terms));
        negative_list_active = direct_negation.iter().any(|negated| *negated)
            && !has_scope_break_after_first
            || continued_from_comma
                && negative_list_active
                && is_forbidden_list_continuation(segment, &spans);
    }
    false
}

fn collect_rendered_free_text(request: &ImagePromptRequest) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut push = |path: String, value: &str| fields.push((path, value.to_owned()));

    push("$.use_case".to_owned(), &request.use_case);
    push("$.scene.environment".to_owned(), &request.scene.environment);
    push("$.scene.background".to_owned(), &request.scene.background);
    push("$.scene.atmosphere".to_owned(), &request.scene.atmosphere);
    if let Some(value) = &request.scene.time_of_day {
        push("$.scene.time_of_day".to_owned(), value);
    }
    if let Some(value) = &request.scene.weather {
        push("$.scene.weather".to_owned(), value);
    }

    for (index, reference) in request.references.iter().enumerate() {
        let base = format!("$.references[{index}]");
        push(format!("{base}.description"), &reference.description);
        for (use_for_index, value) in reference.use_for.iter().enumerate() {
            push(format!("{base}.use_for[{use_for_index}]"), value);
        }
    }
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        push(format!("$.change_contract.change_only[{index}]"), value);
    }
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        push(format!("$.change_contract.preserve[{index}]"), value);
    }

    for (index, subject) in request.subjects.iter().enumerate() {
        let base = format!("$.subjects[{index}]");
        push(format!("{base}.id"), &subject.id);
        push(format!("{base}.description"), &subject.description);
        push(format!("{base}.scale"), &subject.scale);
        push(format!("{base}.pose"), &subject.pose);
        push(format!("{base}.gaze"), &subject.gaze);
        push(format!("{base}.action"), &subject.action);
        if let Some(value) = &subject.face {
            push(format!("{base}.face"), value);
        }
        if let Some(value) = &subject.hair {
            push(format!("{base}.hair"), value);
        }
        if let Some(value) = &subject.appearance {
            push(format!("{base}.appearance"), value);
        }
        for (feature_index, value) in subject.distinguishing_features.iter().enumerate() {
            push(
                format!("{base}.distinguishing_features[{feature_index}]"),
                value,
            );
        }
    }

    push(
        "$.composition.framing".to_owned(),
        &request.composition.framing,
    );
    push(
        "$.composition.viewpoint".to_owned(),
        &request.composition.viewpoint,
    );
    push(
        "$.composition.camera_angle".to_owned(),
        &request.composition.camera_angle,
    );
    push(
        "$.composition.balance".to_owned(),
        &request.composition.balance,
    );
    for (index, value) in request.composition.visual_hierarchy.iter().enumerate() {
        push(format!("$.composition.visual_hierarchy[{index}]"), value);
    }
    for (index, value) in request.composition.depth_layers.iter().enumerate() {
        push(format!("$.composition.depth_layers[{index}]"), value);
    }

    push(
        "$.lighting.key_direction".to_owned(),
        &request.lighting.key_direction,
    );
    push(
        "$.lighting.key_quality".to_owned(),
        &request.lighting.key_quality,
    );
    if let Some(value) = &request.lighting.key_color_hex {
        push("$.lighting.key_color_hex".to_owned(), value);
    }
    push(
        "$.lighting.fill_description".to_owned(),
        &request.lighting.fill_description,
    );
    if let Some(value) = &request.lighting.rim_description {
        push("$.lighting.rim_description".to_owned(), value);
    }
    push(
        "$.lighting.shadow_character".to_owned(),
        &request.lighting.shadow_character,
    );
    push("$.lighting.exposure".to_owned(), &request.lighting.exposure);

    for (index, entry) in request.color.palette.iter().enumerate() {
        push(format!("$.color.palette[{index}].hex"), &entry.hex);
        push(format!("$.color.palette[{index}].usage"), &entry.usage);
    }
    push("$.color.harmony".to_owned(), &request.color.harmony);
    push("$.color.contrast".to_owned(), &request.color.contrast);
    push("$.color.saturation".to_owned(), &request.color.saturation);
    if let Some(selection) = &request.color.photo_lut
        && let Some(lut) = resolve_photo_lut(selection)
    {
        push("$.color.photo_lut.preset".to_owned(), &lut.preset);
        push("$.color.photo_lut.tint".to_owned(), &lut.tint);
        push("$.color.photo_lut.tone_curve".to_owned(), &lut.tone_curve);
        push(
            "$.color.photo_lut.black_response".to_owned(),
            &lut.black_response,
        );
        push("$.color.photo_lut.contrast".to_owned(), &lut.contrast);
        push(
            "$.color.photo_lut.shadow_bias_hex".to_owned(),
            &lut.shadow_bias_hex,
        );
        push(
            "$.color.photo_lut.highlight_bias_hex".to_owned(),
            &lut.highlight_bias_hex,
        );
        push(
            "$.color.photo_lut.highlight_rolloff".to_owned(),
            &lut.highlight_rolloff,
        );
        push(
            "$.color.photo_lut.skin_tone_policy".to_owned(),
            &lut.skin_tone_policy,
        );
        push("$.color.photo_lut.grain_size".to_owned(), &lut.grain_size);
    }

    for (index, surface) in request.surfaces.iter().enumerate() {
        let base = format!("$.surfaces[{index}]");
        push(format!("{base}.id"), &surface.id);
        push(format!("{base}.material"), &surface.material);
        push(format!("{base}.finish"), &surface.finish);
        push(format!("{base}.micro_detail"), &surface.micro_detail);
        push(format!("{base}.light_response"), &surface.light_response);
    }

    for (index, value) in request.constraints.required_elements.iter().enumerate() {
        push(format!("$.constraints.required_elements[{index}]"), value);
    }

    if let Some(camera) = &request.camera {
        push("$.camera.field_of_view".to_owned(), &camera.field_of_view);
        push("$.camera.perspective".to_owned(), &camera.perspective);
        push("$.camera.depth_of_field".to_owned(), &camera.depth_of_field);
        push("$.camera.focus".to_owned(), &camera.focus);
        push(
            "$.camera.motion_rendering".to_owned(),
            &camera.motion_rendering,
        );
    }

    if let Some(consistency) = &request.consistency {
        for (index, dimension) in consistency.dimensions.iter().enumerate() {
            push(
                format!("$.consistency.dimensions[{index}]"),
                dimension.as_str(),
            );
        }
        for (index, value) in consistency.shared_anchors.iter().enumerate() {
            push(format!("$.consistency.shared_anchors[{index}]"), value);
        }
        if let Some(sequence) = &consistency.sequence {
            for (index, value) in sequence.steps.iter().enumerate() {
                push(format!("$.consistency.sequence.steps[{index}]"), value);
            }
        }
    }

    if let Some(storyboard) = &request.storyboard {
        push("$.storyboard.layout".to_owned(), &storyboard.layout);
        push(
            "$.storyboard.screen_direction".to_owned(),
            &storyboard.screen_direction,
        );
        for (index, value) in storyboard.identity_anchors.iter().enumerate() {
            push(format!("$.storyboard.identity_anchors[{index}]"), value);
        }
        for (index, panel) in storyboard.panels.iter().enumerate() {
            let base = format!("$.storyboard.panels[{index}]");
            push(format!("{base}.id"), &panel.id);
            push(format!("{base}.shot_size"), &panel.shot_size);
            push(format!("{base}.camera_angle"), &panel.camera_angle);
            push(format!("{base}.camera_move"), &panel.camera_move);
            push(format!("{base}.action"), &panel.action);
            push(format!("{base}.body_facing"), &panel.body_facing);
            push(format!("{base}.gaze_target"), &panel.gaze_target);
            push(format!("{base}.emotional_beat"), &panel.emotional_beat);
        }
    }

    fields
}

fn validate_textless_ui_content(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    if !request.text_elements.is_empty() {
        return;
    }
    for (path, value) in collect_rendered_free_text(request) {
        if contains_positive_forbidden_term(&value, TEXTLESS_UI_FORBIDDEN_TERMS) {
            diagnostics.push(
                Diagnostic::error(
                    "IMG_UI_TEXTLESS_CONTENT",
                    path,
                    "textless app/web/agent screens may describe only visual structure, navigation, state, and component relationships; readable text, numerals, pseudo-writing, invented data, and charts are not allowed",
                )
                .with_hint("keep text_elements empty and describe geometry/state without copy or data content"),
            );
        }
    }
}

fn validate_app_icon_content(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    for (path, value) in collect_rendered_free_text(request) {
        if contains_positive_forbidden_term(&value, APP_ICON_FORBIDDEN_TERMS) {
            diagnostics.push(
                Diagnostic::error(
                    "IMG_APP_ICON_FORBIDDEN_CONTENT",
                    path,
                    "app_icon forbids text, letters, numerals, pseudo-writing, UI screenshots, device mockups, rounded-square masks, app-icon frames, and watermarks",
                )
                .with_hint("describe one abstract core metaphor and keep prohibited content in exclusions only"),
            );
        }
    }
}

pub fn validate_image_request(
    request: &ImagePromptRequest,
) -> (Vec<Diagnostic>, Vec<&'static CatalogEntry>) {
    let mut diagnostics = Vec::new();
    validate_nonempty("$.use_case", &request.use_case, &mut diagnostics);
    inspect_description("$.use_case", &request.use_case, &mut diagnostics);
    validate_task_contract(request, &mut diagnostics);
    validate_consistency(request, &mut diagnostics);

    let selected = match inspect_catalog_selection(request.taxonomy.category.as_deref()) {
        Ok(selection) => {
            for issue in selection.issues {
                push_taxonomy_issue(issue, &mut diagnostics);
            }
            selection.entries
        }
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                "IMG_TAXONOMY_LOAD",
                "$.taxonomy",
                error.to_string(),
            ));
            Vec::new()
        }
    };
    validate_taxonomy_safety(request, &selected, &mut diagnostics);
    validate_image_profile(request, &selected, &mut diagnostics);
    validate_render_profile(request, &selected, &mut diagnostics);
    validate_scene(request, &mut diagnostics);
    validate_elements(request, &mut diagnostics);
    validate_composition(request, &mut diagnostics);
    validate_camera_and_lighting(request, &mut diagnostics);
    validate_color(&request.color, request, &mut diagnostics);
    validate_text(request, &selected, &mut diagnostics);
    validate_constraints(request, &selected, &mut diagnostics);
    validate_complex_output_detail(request, &selected, &mut diagnostics);
    validate_output(request, &mut diagnostics);
    validate_free_text_budget(request, &mut diagnostics);
    (diagnostics, selected)
}

fn validate_task_contract(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    let mut seen_indices = BTreeSet::new();
    let mut base_count = 0usize;
    let mut has_anchor = false;
    let mut has_pose = false;
    let mut has_generate_forbidden_role = false;
    for (position, reference) in request.references.iter().enumerate() {
        let base = format!("$.references[{position}]");
        match reference.role {
            ImageReferenceRole::Base => {
                base_count += 1;
                has_anchor = true;
                has_generate_forbidden_role = true;
            }
            ImageReferenceRole::Pose => {
                has_pose = true;
                has_anchor = true;
                has_generate_forbidden_role = true;
            }
            ImageReferenceRole::Layout => has_anchor = true,
            ImageReferenceRole::Mask => has_generate_forbidden_role = true,
            _ => {}
        }
        if reference.index == 0 {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_INDEX_ZERO",
                format!("{base}.index"),
                "reference indices start at 1",
            ));
        }
        let expected = u8::try_from(position + 1).unwrap_or(u8::MAX);
        if reference.index != expected {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_ORDER",
                format!("{base}.index"),
                format!(
                    "references must be ordered and contiguous as 1..N; expected {expected}, found {}",
                    reference.index
                ),
            ));
        }
        if !seen_indices.insert(reference.index) {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_DUPLICATE",
                format!("{base}.index"),
                format!("reference index {} is duplicated", reference.index),
            ));
        }
        validate_nonempty(
            &format!("{base}.description"),
            &reference.description,
            diagnostics,
        );
        inspect_description(
            &format!("{base}.description"),
            &reference.description,
            diagnostics,
        );
        if reference.use_for.is_empty() {
            diagnostics.push(Diagnostic::error(
                "IMG_REFERENCE_USE_EMPTY",
                format!("{base}.use_for"),
                "declare exactly what this reference controls",
            ));
        }
        let mut use_keys = BTreeSet::new();
        for (index, value) in reference.use_for.iter().enumerate() {
            let path = format!("{base}.use_for[{index}]");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
            let key = value.trim().to_lowercase();
            if !use_keys.insert(key) {
                diagnostics.push(Diagnostic::error(
                    "IMG_REFERENCE_USE_DUPLICATE",
                    path,
                    "use_for contains a duplicate responsibility",
                ));
            }
        }
    }

    let mut change_keys = BTreeSet::new();
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        let path = format!("$.change_contract.change_only[{index}]");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
        let key = value.trim().to_lowercase();
        if !change_keys.insert(key) {
            diagnostics.push(Diagnostic::error(
                "IMG_CHANGE_DUPLICATE",
                path,
                "change_only contains a duplicate instruction",
            ));
        }
    }
    let mut preserve_keys = BTreeSet::new();
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        let path = format!("$.change_contract.preserve[{index}]");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
        let key = value.trim().to_lowercase();
        if !preserve_keys.insert(key.clone()) {
            diagnostics.push(Diagnostic::error(
                "IMG_PRESERVE_DUPLICATE",
                path.clone(),
                "preserve contains a duplicate invariant",
            ));
        }
        if change_keys.contains(&key) {
            diagnostics.push(Diagnostic::error(
                "IMG_CHANGE_PRESERVE_CONFLICT",
                path,
                "the same instruction cannot be both changed and preserved",
            ));
        }
    }
    let has_base = base_count > 0;

    match request.task_mode {
        ImageTaskMode::Generate => {
            if has_generate_forbidden_role {
                diagnostics.push(Diagnostic::error(
                    "IMG_GENERATE_EDIT_REFERENCE_ROLE",
                    "$.references",
                    "generate mode may use subject, style, product, layout, or palette references; base and mask roles require edit or composite mode",
                ));
            }
            if !request.change_contract.change_only.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_GENERATE_CHANGE_ONLY",
                    "$.change_contract.change_only",
                    "generate mode describes the desired result directly; change_only is reserved for edit and composite modes",
                ));
            }
            if request.references.is_empty() && !request.change_contract.preserve.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_GENERATE_PRESERVE_WITHOUT_REFERENCE",
                    "$.change_contract.preserve",
                    "preserve invariants require at least one declared reference image",
                ));
            }
        }
        ImageTaskMode::Edit => {
            if request.references.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_EDIT_REFERENCE_REQUIRED",
                    "$.references",
                    "edit mode requires at least one reference image",
                ));
            }
            if !has_base {
                diagnostics.push(Diagnostic::error(
                    "IMG_EDIT_BASE_REQUIRED",
                    "$.references",
                    "edit mode requires exactly one reference with role=base",
                ));
            } else if base_count != 1 {
                diagnostics.push(Diagnostic::error(
                    "IMG_EDIT_BASE_AMBIGUOUS",
                    "$.references",
                    "edit mode requires exactly one base image; classify other references by their actual role",
                ));
            }
            require_change_contract(request, diagnostics);
        }
        ImageTaskMode::Composite => {
            if base_count > 1 {
                diagnostics.push(Diagnostic::error(
                    "IMG_COMPOSITE_BASE_AMBIGUOUS",
                    "$.references",
                    "composite mode permits at most one base image; classify other inputs by product, subject, layout, style, palette, or mask role",
                ));
            }
            if request.references.len() < 2 {
                diagnostics.push(Diagnostic::error(
                    "IMG_COMPOSITE_REFERENCES_REQUIRED",
                    "$.references",
                    "composite mode requires at least two reference images",
                ));
            }
            if !has_anchor {
                diagnostics.push(Diagnostic::error(
                    "IMG_COMPOSITE_ANCHOR_REQUIRED",
                    "$.references",
                    "composite mode requires a base or layout reference to anchor the result",
                ));
            }
            require_change_contract(request, diagnostics);
        }
    }

    if has_pose && request.render_profile != ImageRenderProfile::PoseTransfer {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_REFERENCE_PROFILE_REQUIRED",
            "$.render_profile",
            "role=pose is reserved for render_profile=pose_transfer so pose authority cannot be applied accidentally",
        ));
    }

    if request.change_contract.change_only.len() > 3 {
        diagnostics.push(
            Diagnostic::warning(
                "IMG_EDIT_CHANGE_SCOPE",
                "$.change_contract.change_only",
                "more than three independent changes increase edit drift and make failures hard to attribute",
            )
            .with_hint("split the edit into smaller iterations and repeat the preserve invariants each time"),
        );
    }
    if request.task_mode != ImageTaskMode::Generate && request.output.detail == ImageDetail::Low {
        diagnostics.push(Diagnostic::warning(
            "IMG_EDIT_DETAIL",
            "$.output.detail",
            "identity-sensitive edits and composites should use medium or high detail",
        ));
    }
}

fn validate_consistency(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    let Some(consistency) = &request.consistency else {
        if request.render_profile == ImageRenderProfile::CinematicStoryboard {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_CONSISTENCY_REQUIRED",
                "$.consistency",
                "cinematic_storyboard requires explicit multi-image consistency dimensions, shared anchors, and temporal sequence",
            ));
        }
        return;
    };

    if consistency.dimensions.is_empty() || consistency.dimensions.len() > 4 {
        diagnostics.push(Diagnostic::error(
            "IMG_CONSISTENCY_DIMENSIONS_COUNT",
            "$.consistency.dimensions",
            "declare one to four consistency dimensions",
        ));
    }
    let mut dimensions = BTreeSet::new();
    for (index, dimension) in consistency.dimensions.iter().enumerate() {
        if !dimensions.insert(dimension.as_str()) {
            diagnostics.push(Diagnostic::error(
                "IMG_CONSISTENCY_DIMENSION_DUPLICATE",
                format!("$.consistency.dimensions[{index}]"),
                format!("consistency dimension {} is duplicated", dimension.as_str()),
            ));
        }
    }

    if consistency.shared_anchors.is_empty() || consistency.shared_anchors.len() > 8 {
        diagnostics.push(Diagnostic::error(
            "IMG_CONSISTENCY_ANCHORS_COUNT",
            "$.consistency.shared_anchors",
            "declare one to eight observable invariants shared by the requested images",
        ));
    }
    let mut anchors = BTreeSet::new();
    for (index, anchor) in consistency.shared_anchors.iter().enumerate() {
        let path = format!("$.consistency.shared_anchors[{index}]");
        validate_nonempty(&path, anchor, diagnostics);
        inspect_description(&path, anchor, diagnostics);
        if !anchors.insert(anchor.trim().to_lowercase()) {
            diagnostics.push(Diagnostic::error(
                "IMG_CONSISTENCY_ANCHOR_DUPLICATE",
                path,
                "shared_anchors contains a duplicate invariant",
            ));
        }
    }

    let has_temporal = dimensions.contains("temporal");
    match (&consistency.sequence, has_temporal) {
        (None, true) => diagnostics.push(Diagnostic::error(
            "IMG_CONSISTENCY_TEMPORAL_SEQUENCE_REQUIRED",
            "$.consistency.sequence",
            "temporal consistency requires an ordered sequence",
        )),
        (Some(sequence), true) => {
            if sequence.steps.len() < 2 || sequence.steps.len() > 24 {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_SEQUENCE_COUNT",
                    "$.consistency.sequence.steps",
                    "temporal sequences must contain two to twenty-four ordered steps",
                ));
            }
            if !sequence.condition_on_previous {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_TEMPORAL_CONDITIONING",
                    "$.consistency.sequence.condition_on_previous",
                    "temporal consistency must explicitly condition each step on the previous output",
                ));
            }
        }
        (Some(sequence), false) => {
            if sequence.steps.len() < 2 || sequence.steps.len() > 24 {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_SEQUENCE_COUNT",
                    "$.consistency.sequence.steps",
                    "ordered sequences must contain two to twenty-four steps",
                ));
            }
        }
        (None, false) => {}
    }
    if let Some(sequence) = &consistency.sequence {
        let mut steps = BTreeSet::new();
        for (index, step) in sequence.steps.iter().enumerate() {
            let path = format!("$.consistency.sequence.steps[{index}]");
            validate_nonempty(&path, step, diagnostics);
            inspect_description(&path, step, diagnostics);
            if !steps.insert(step.trim().to_lowercase()) {
                diagnostics.push(Diagnostic::error(
                    "IMG_CONSISTENCY_SEQUENCE_DUPLICATE",
                    path,
                    "sequence steps must be unique and ordered",
                ));
            }
        }
    }

    if request.render_profile != ImageRenderProfile::CinematicStoryboard {
        return;
    }
    let required_dimensions = [
        ImageConsistencyDimension::Character.as_str(),
        ImageConsistencyDimension::Temporal.as_str(),
        ImageConsistencyDimension::Semantic.as_str(),
    ];
    for dimension in required_dimensions {
        if !dimensions.contains(dimension) {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_CONSISTENCY_DIMENSION",
                "$.consistency.dimensions",
                format!("cinematic_storyboard requires the {dimension} consistency dimension"),
            ));
        }
    }
    let Some(storyboard) = &request.storyboard else {
        return;
    };
    for (index, identity_anchor) in storyboard.identity_anchors.iter().enumerate() {
        if !consistency
            .shared_anchors
            .iter()
            .any(|candidate| candidate.trim() == identity_anchor.trim())
        {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_ANCHOR_MISSING",
                format!("$.storyboard.identity_anchors[{index}]"),
                "every storyboard identity anchor must also be declared as a shared consistency anchor",
            ));
        }
    }
    if let Some(sequence) = &consistency.sequence {
        if sequence.steps.len() != storyboard.panels.len() {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_SEQUENCE_COUNT",
                "$.consistency.sequence.steps",
                format!(
                    "storyboard sequence must contain exactly {} panel steps, found {}",
                    storyboard.panels.len(),
                    sequence.steps.len()
                ),
            ));
        } else {
            for (index, (step, panel)) in sequence
                .steps
                .iter()
                .zip(storyboard.panels.iter())
                .enumerate()
            {
                if step.trim() != panel.id.trim() {
                    diagnostics.push(Diagnostic::error(
                        "IMG_CINEMATIC_STORYBOARD_SEQUENCE_ORDER",
                        format!("$.consistency.sequence.steps[{index}]"),
                        format!(
                            "storyboard sequence step must match panel id {:?}, found {:?}",
                            panel.id, step
                        ),
                    ));
                }
            }
        }
    }
}

fn require_change_contract(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    if request.change_contract.change_only.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_CHANGE_ONLY_REQUIRED",
            "$.change_contract.change_only",
            "edit and composite modes require at least one explicit change",
        ));
    }
    if request.change_contract.preserve.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_PRESERVE_REQUIRED",
            "$.change_contract.preserve",
            "edit and composite modes require explicit invariants to preserve",
        ));
    }
}

fn push_taxonomy_issue(issue: CatalogSelectionIssue, diagnostics: &mut Vec<Diagnostic>) {
    let (code, path) = match &issue {
        CatalogSelectionIssue::UnknownCategory { .. } => {
            ("IMG_TAXONOMY_UNKNOWN", "$.taxonomy.category".to_owned())
        }
        CatalogSelectionIssue::CategoryNotPrimary { .. } => (
            "IMG_TAXONOMY_CATEGORY_TIER",
            "$.taxonomy.category".to_owned(),
        ),
    };
    diagnostics.push(Diagnostic::error(code, path, issue.to_string()));
}

fn validate_taxonomy_safety(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for entry in selected {
        if request.constraints.safety_tier < entry.default_safety_tier {
            diagnostics.push(Diagnostic::error(
                "IMG_TAXONOMY_TIER",
                "$.constraints.safety_tier",
                format!(
                    "{} requires safety tier {}, request declares {}",
                    entry.id, entry.default_safety_tier, request.constraints.safety_tier
                ),
            ));
        }
    }
}

fn validate_render_profile(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.render_profile == ImageRenderProfile::PoseTransfer {
        validate_pose_transfer_profile(request, diagnostics);
        return;
    }
    if request.render_profile == ImageRenderProfile::CinematicStoryboard {
        validate_cinematic_storyboard_profile(request, selected, diagnostics);
    }
}

fn validate_image_profile(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.profile == ImageProfile::Standard {
        return;
    }

    let category = selected
        .iter()
        .find(|entry| entry.is_primary_output())
        .map(|entry| entry.id.as_str());
    let expected_category = match request.profile {
        ImageProfile::LogoIdentity => "C4",
        ImageProfile::TravelJournal => "C5",
        ImageProfile::AppIcon => "C4",
        ImageProfile::AppWebUi | ImageProfile::InformationDesign => "C6",
        ImageProfile::CharacterPose | ImageProfile::PoomsaePose => "C10",
        ImageProfile::Standard => return,
    };
    if category != Some(expected_category) {
        diagnostics.push(Diagnostic::error(
            "IMG_PROFILE_CATEGORY",
            "$.profile",
            format!(
                "profile={} is category-bound to {expected_category}; select that primary catalog category",
                request.profile.as_str()
            ),
        ));
    }
    if !selected
        .iter()
        .filter(|entry| entry.is_primary_output())
        .any(|entry| {
            entry
                .profiles
                .iter()
                .any(|profile| profile == request.profile.as_str())
        })
    {
        diagnostics.push(Diagnostic::error(
            "IMG_PROFILE_CATALOG",
            "$.profile",
            format!(
                "profile={} is not advertised by the selected catalog outcome",
                request.profile.as_str()
            ),
        ));
    }

    if request.task_mode != ImageTaskMode::Generate
        && request.profile != ImageProfile::TravelJournal
    {
        diagnostics.push(Diagnostic::error(
            "IMG_PROFILE_TASK_MODE",
            "$.profile",
            "category-bound profiles are text-directed generate prompts; use the separate render_profile contract for edits or composites",
        ));
    }

    match request.profile {
        ImageProfile::TravelJournal => {
            if request.task_mode == ImageTaskMode::Composite {
                diagnostics.push(Diagnostic::error(
                    "IMG_JOURNAL_TASK_MODE",
                    "$.task_mode",
                    "travel_journal supports generate or a single-base edit",
                ));
            }
        }
        ImageProfile::AppIcon => {
            if request.task_mode != ImageTaskMode::Generate {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_TASK_MODE",
                    "$.task_mode",
                    "app_icon is a generate-only profile",
                ));
            }
            if request.render_profile != ImageRenderProfile::Structured {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_RENDER_PROFILE",
                    "$.render_profile",
                    "app_icon uses the structured generate renderer only",
                ));
            }
            if !request.references.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_REFERENCES",
                    "$.references",
                    "app_icon does not accept reference images; provide one typed master subject",
                ));
            }
            if !request.change_contract.change_only.is_empty()
                || !request.change_contract.preserve.is_empty()
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_CHANGE_CONTRACT",
                    "$.change_contract",
                    "app_icon does not accept edit/change contracts",
                ));
            }
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign
                    | VisualMedium::Illustration
                    | VisualMedium::ThreeD
                    | VisualMedium::Mixed
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_MEDIUM",
                    "$.medium",
                    "app_icon requires a non-photo medium: graphic_design, illustration, 3d, or mixed",
                ));
            }
            if request.subjects.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_SUBJECT_COUNT",
                    "$.subjects",
                    "app_icon requires exactly one typed subject",
                ));
            } else {
                let subject = &request.subjects[0];
                if subject.count != 1 {
                    diagnostics.push(Diagnostic::error(
                        "IMG_APP_ICON_SUBJECT_COUNT",
                        "$.subjects[0].count",
                        "app_icon requires the single typed subject to have count=1",
                    ));
                }
                if !matches!(
                    subject.placement,
                    CanvasPlacement::Zone(super::model::CanvasZone::Center)
                ) {
                    diagnostics.push(Diagnostic::error(
                        "IMG_APP_ICON_SUBJECT_PLACEMENT",
                        "$.subjects[0].placement",
                        "app_icon requires the single typed subject to use the center placement zone",
                    ));
                }
            }
            if !(2..=3).contains(&request.composition.depth_layers.len()) {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_PLANES",
                    "$.composition.depth_layers",
                    "app_icon requires simple 2-3 visual planes",
                ));
            }
            if !request.text_elements.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_TEXT",
                    "$.text_elements",
                    "app_icon is textless and requires text_elements to be empty",
                ));
            }
            if !request.output.format.eq_ignore_ascii_case("png") {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_FORMAT",
                    "$.output.format",
                    "app_icon produces an opaque PNG master concept only",
                ));
            }
            if request.output.background != super::model::BackgroundMode::Opaque {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_BACKGROUND",
                    "$.output.background",
                    "app_icon requires an opaque background",
                ));
            }
            if request.output.width != 1024 || request.output.height != 1024 {
                diagnostics.push(Diagnostic::error(
                    "IMG_APP_ICON_DIMENSIONS",
                    "$.output",
                    "app_icon requires exactly 1024x1024 output",
                ));
            }
            validate_app_icon_content(request, diagnostics);
        }
        ImageProfile::LogoIdentity => {
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign | VisualMedium::ThreeD | VisualMedium::Mixed
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_LOGO_PROFILE_MEDIUM",
                    "$.medium",
                    "logo_identity requires graphic_design, 3d, or mixed medium",
                ));
            }
            if !request.output.format.eq_ignore_ascii_case("png") {
                diagnostics.push(Diagnostic::error(
                    "IMG_LOGO_PROFILE_FORMAT",
                    "$.output.format",
                    "logo_identity produces an opaque PNG concept/brand-mark direction; SVG or vector final assets are outside this contract",
                ));
            }
            if request.output.background != super::model::BackgroundMode::Opaque {
                diagnostics.push(Diagnostic::error(
                    "IMG_LOGO_PROFILE_BACKGROUND",
                    "$.output.background",
                    "logo_identity requires an opaque background; transparent/vector final delivery is not promised",
                ));
            }
        }
        ImageProfile::AppWebUi => {
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign | VisualMedium::Mixed | VisualMedium::ThreeD
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_UI_PROFILE_MEDIUM",
                    "$.medium",
                    "app_web_ui requires graphic_design, mixed, or 3d medium",
                ));
            }
            if request.subjects.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "IMG_UI_PROFILE_SCREEN",
                    "$.subjects",
                    "app_web_ui requires at least one typed screen or interface subject",
                ));
            } else if !request
                .subjects
                .iter()
                .any(|subject| contains_ui_surface_component_relation(&subject.description))
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_UI_PROFILE_EVIDENCE",
                    "$.subjects[*].description",
                    "app_web_ui requires $.subjects[*].description to name an explicit screen/interface surface and relate it to at least one component or state (for example dashboard screen with sidebar, table, or status)",
                ));
            }
            validate_textless_ui_content(request, diagnostics);
        }
        ImageProfile::InformationDesign => {
            if !matches!(
                request.medium,
                VisualMedium::GraphicDesign | VisualMedium::Mixed | VisualMedium::Illustration
            ) {
                diagnostics.push(Diagnostic::error(
                    "IMG_INFORMATION_PROFILE_MEDIUM",
                    "$.medium",
                    "information_design requires graphic_design, mixed, or illustration medium",
                ));
            }
        }
        ImageProfile::CharacterPose => {
            if request.medium != VisualMedium::Illustration && request.medium != VisualMedium::Mixed
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_CHARACTER_POSE_PROFILE_MEDIUM",
                    "$.medium",
                    "character_pose requires illustration or mixed medium",
                ));
            }
        }
        ImageProfile::PoomsaePose => {
            if request.medium != VisualMedium::Illustration && request.medium != VisualMedium::Mixed
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_POOMSAE_PROFILE_MEDIUM",
                    "$.medium",
                    "poomsae_pose requires illustration or mixed medium",
                ));
            }
            let brief = format!(
                "{} {} {}",
                request.use_case,
                request
                    .subjects
                    .iter()
                    .map(|subject| format!("{} {}", subject.pose, subject.action))
                    .collect::<Vec<_>>()
                    .join(" "),
                request.scene.environment
            )
            .to_ascii_lowercase();
            if !["poomsae", "품새", "taekwondo", "태권도"]
                .iter()
                .any(|term| brief.contains(term))
            {
                diagnostics.push(Diagnostic::error(
                    "IMG_POOMSAE_PROFILE_INTENT",
                    "$.profile",
                    "poomsae_pose requires an explicit text-directed poomsae/taekwondo action in use_case, subject pose/action, or scene",
                ));
            }
        }
        ImageProfile::Standard => {}
    }
}

fn validate_cinematic_storyboard_profile(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.task_mode != ImageTaskMode::Generate
        || !request.references.is_empty()
        || !request.change_contract.change_only.is_empty()
        || !request.change_contract.preserve.is_empty()
    {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_TASK_MODE",
            "$.render_profile",
            "cinematic_storyboard supports text-to-image generation only; preserve references and edits through structured or pose_transfer",
        ));
    }
    if request.medium != VisualMedium::Illustration {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_MEDIUM",
            "$.medium",
            "cinematic_storyboard requires illustration so panel line, color, and character continuity remain explicit",
        ));
    }
    if !selected.iter().any(|entry| entry.id == "C10") {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_CATEGORY",
            "$.taxonomy.category",
            "cinematic_storyboard is the C10 illustration-and-story outcome profile",
        ));
    }
    if !request.text_elements.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_TEXT",
            "$.text_elements",
            "cinematic_storyboard is a textless visual handoff; keep labels, dialogue, panel numbers, logos, and UI outside the generated sheet",
        ));
    }
    if !matches!(request.output.detail, ImageDetail::High) {
        diagnostics.push(
            Diagnostic::warning(
                "IMG_CINEMATIC_STORYBOARD_DETAIL",
                "$.output.detail",
                "panel-to-panel character, gaze, and framing continuity are most reliable at high detail",
            )
            .with_hint("use detail=high when precise anatomy and surface depiction is required"),
        );
    }

    let Some(storyboard) = &request.storyboard else {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_REQUIRED",
            "$.storyboard",
            "cinematic_storyboard requires a typed layout, identity anchors, screen direction, and ordered panel plan",
        ));
        return;
    };

    let expected_count = match storyboard.layout.as_str() {
        "three_panel_strip" => 3,
        "two_by_two" => 4,
        "three_by_two" => 6,
        _ => {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_LAYOUT",
                "$.storyboard.layout",
                "layout must be three_panel_strip, two_by_two, or three_by_two",
            ));
            0
        }
    };
    if storyboard.panels.len() != expected_count {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_PANEL_COUNT",
            "$.storyboard.panels",
            format!(
                "{} requires exactly {expected_count} ordered panels, found {}",
                storyboard.layout,
                storyboard.panels.len()
            ),
        ));
    }
    if !matches!(
        storyboard.screen_direction.as_str(),
        "left_to_right" | "right_to_left"
    ) {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_DIRECTION",
            "$.storyboard.screen_direction",
            "screen_direction must be left_to_right or right_to_left",
        ));
    }
    if !(3..=6).contains(&storyboard.identity_anchors.len()) {
        diagnostics.push(Diagnostic::error(
            "IMG_CINEMATIC_STORYBOARD_IDENTITY",
            "$.storyboard.identity_anchors",
            "declare 3 to 6 visible identity anchors such as hair, costume, prop, silhouette, and palette role",
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, panel) in storyboard.panels.iter().enumerate() {
        let base = format!("$.storyboard.panels[{index}]");
        if !ids.insert(panel.id.trim()) {
            diagnostics.push(Diagnostic::error(
                "IMG_CINEMATIC_STORYBOARD_PANEL_ID",
                format!("{base}.id"),
                "every storyboard panel id must be unique",
            ));
        }
        for (name, value) in [
            ("id", panel.id.as_str()),
            ("camera_angle", panel.camera_angle.as_str()),
            ("camera_move", panel.camera_move.as_str()),
            ("action", panel.action.as_str()),
            ("emotional_beat", panel.emotional_beat.as_str()),
        ] {
            validate_nonempty(&format!("{base}.{name}"), value, diagnostics);
            inspect_description(&format!("{base}.{name}"), value, diagnostics);
        }
    }
}

fn validate_pose_transfer_profile(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    if request.task_mode != ImageTaskMode::Composite {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_TRANSFER_TASK_MODE",
            "$.task_mode",
            "pose_transfer requires composite mode because pose geometry and character identity come from separate references",
        ));
    }

    let pose_count = request
        .references
        .iter()
        .filter(|reference| reference.role == ImageReferenceRole::Pose)
        .count();
    let subject_count = request
        .references
        .iter()
        .filter(|reference| reference.role == ImageReferenceRole::Subject)
        .count();
    if request.references.len() != 2 || pose_count != 1 || subject_count != 1 {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_TRANSFER_REFERENCES",
            "$.references",
            "pose_transfer requires exactly two references: one role=pose source and one role=subject character sheet",
        ));
    }

    if request.subjects.len() != 1
        || request
            .subjects
            .first()
            .is_some_and(|subject| subject.count != 1)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_POSE_TRANSFER_SUBJECT",
            "$.subjects",
            "pose_transfer requires exactly one output subject with count=1",
        ));
    }

    if !matches!(request.output.detail, ImageDetail::High) {
        diagnostics.push(
            Diagnostic::warning(
                "IMG_POSE_TRANSFER_DETAIL",
                "$.output.detail",
                "pose, hands, feet, facing, and gaze are most reliable at high detail",
            )
            .with_hint("use detail=high when precise anatomy and surface depiction is required"),
        );
    }
}

/// Warns when one free-text field is large enough to hide multiple responsibilities.
///
/// The complete rendered prompt is measured after rendering in `compile_image_prompt`; measuring
/// only a hand-picked subset of source fields cannot establish the actual execution payload size.
fn validate_free_text_budget(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    let mut fields: Vec<(String, usize)> = vec![
        ("$.use_case".to_owned(), request.use_case.chars().count()),
        (
            "$.scene.environment".to_owned(),
            request.scene.environment.chars().count(),
        ),
        (
            "$.scene.background".to_owned(),
            request.scene.background.chars().count(),
        ),
        (
            "$.scene.atmosphere".to_owned(),
            request.scene.atmosphere.chars().count(),
        ),
    ];
    for (index, subject) in request.subjects.iter().enumerate() {
        fields.push((
            format!("$.subjects[{index}].description"),
            subject.description.chars().count(),
        ));
        fields.push((
            format!("$.subjects[{index}].pose"),
            subject.pose.chars().count(),
        ));
    }
    for (index, reference) in request.references.iter().enumerate() {
        fields.push((
            format!("$.references[{index}].description"),
            reference.description.chars().count(),
        ));
        for (use_index, value) in reference.use_for.iter().enumerate() {
            fields.push((
                format!("$.references[{index}].use_for[{use_index}]"),
                value.chars().count(),
            ));
        }
    }
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        fields.push((
            format!("$.change_contract.change_only[{index}]"),
            value.chars().count(),
        ));
    }
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        fields.push((
            format!("$.change_contract.preserve[{index}]"),
            value.chars().count(),
        ));
    }
    if let Some(storyboard) = &request.storyboard {
        for (index, value) in storyboard.identity_anchors.iter().enumerate() {
            fields.push((
                format!("$.storyboard.identity_anchors[{index}]"),
                value.chars().count(),
            ));
        }
        for (index, panel) in storyboard.panels.iter().enumerate() {
            for (name, value) in [
                ("camera_angle", panel.camera_angle.as_str()),
                ("camera_move", panel.camera_move.as_str()),
                ("action", panel.action.as_str()),
                ("emotional_beat", panel.emotional_beat.as_str()),
            ] {
                fields.push((
                    format!("$.storyboard.panels[{index}].{name}"),
                    value.chars().count(),
                ));
            }
        }
    }
    if let Some(consistency) = &request.consistency {
        for (index, value) in consistency.shared_anchors.iter().enumerate() {
            fields.push((
                format!("$.consistency.shared_anchors[{index}]"),
                value.chars().count(),
            ));
        }
        if let Some(sequence) = &consistency.sequence {
            for (index, value) in sequence.steps.iter().enumerate() {
                fields.push((
                    format!("$.consistency.sequence.steps[{index}]"),
                    value.chars().count(),
                ));
            }
        }
    }
    for (index, text) in request.text_elements.iter().enumerate() {
        if let Some(value) = &text.spelling_hint {
            fields.push((
                format!("$.text_elements[{index}].spelling_hint"),
                value.chars().count(),
            ));
        }
    }

    for (path, count) in &fields {
        if *count > MAX_FREE_TEXT_CHARS {
            diagnostics.push(
                Diagnostic::warning(
                    "IMG_FIELD_TOO_LONG",
                    path,
                    format!(
                        "{count} characters exceeds the {MAX_FREE_TEXT_CHARS}-character field budget"
                    ),
                )
                .with_hint("split the field into the structured slots it describes"),
            );
        }
    }
}

fn validate_scene(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    let fields = [
        ("$.scene.environment", request.scene.environment.as_str()),
        ("$.scene.background", request.scene.background.as_str()),
        ("$.scene.atmosphere", request.scene.atmosphere.as_str()),
    ];
    for (path, value) in fields {
        validate_nonempty(path, value, diagnostics);
        inspect_description(path, value, diagnostics);
    }
    if let Some(value) = &request.scene.time_of_day {
        validate_nonempty("$.scene.time_of_day", value, diagnostics);
        inspect_description("$.scene.time_of_day", value, diagnostics);
    }
    if let Some(value) = &request.scene.weather {
        validate_nonempty("$.scene.weather", value, diagnostics);
        inspect_description("$.scene.weather", value, diagnostics);
    }
}

fn validate_elements(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    if request.subjects.is_empty()
        && request.text_elements.is_empty()
        && request.surfaces.is_empty()
    {
        diagnostics.push(Diagnostic::error(
            "IMG_NO_VISUAL_ELEMENT",
            "$",
            "at least one subject, text element, or surface is required",
        ));
    }
    let mut ids: BTreeMap<String, String> = BTreeMap::new();
    for (index, subject) in request.subjects.iter().enumerate() {
        let base = format!("$.subjects[{index}]");
        register_id(&mut ids, &subject.id, &format!("{base}.id"), diagnostics);
        if subject.count == 0 {
            diagnostics.push(Diagnostic::error(
                "IMG_SUBJECT_COUNT",
                format!("{base}.count"),
                "subject count must be at least 1",
            ));
        }
        let fields = [
            ("description", subject.description.as_str()),
            ("scale", subject.scale.as_str()),
            ("pose", subject.pose.as_str()),
            ("gaze", subject.gaze.as_str()),
            ("action", subject.action.as_str()),
        ];
        for (name, value) in fields {
            let path = format!("{base}.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
        if let Some(value) = &subject.face {
            validate_nonempty(&format!("{base}.face"), value, diagnostics);
            inspect_description(&format!("{base}.face"), value, diagnostics);
        }
        if let Some(value) = &subject.hair {
            validate_nonempty(&format!("{base}.hair"), value, diagnostics);
            inspect_description(&format!("{base}.hair"), value, diagnostics);
        }
        if let Some(value) = &subject.appearance {
            validate_nonempty(&format!("{base}.appearance"), value, diagnostics);
            inspect_description(&format!("{base}.appearance"), value, diagnostics);
        }
        for (feature_index, value) in subject.distinguishing_features.iter().enumerate() {
            let path = format!("{base}.distinguishing_features[{feature_index}]");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
        validate_placement(
            &subject.placement,
            &format!("{base}.placement"),
            diagnostics,
        );
    }
    for (index, surface) in request.surfaces.iter().enumerate() {
        let base = format!("$.surfaces[{index}]");
        register_id(&mut ids, &surface.id, &format!("{base}.id"), diagnostics);
        for (name, value) in [
            ("material", surface.material.as_str()),
            ("finish", surface.finish.as_str()),
            ("micro_detail", surface.micro_detail.as_str()),
            ("light_response", surface.light_response.as_str()),
        ] {
            let path = format!("{base}.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    for (index, text) in request.text_elements.iter().enumerate() {
        register_id(
            &mut ids,
            &text.id,
            &format!("$.text_elements[{index}].id"),
            diagnostics,
        );
    }
}

fn validate_composition(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    for (name, value) in [
        ("framing", request.composition.framing.as_str()),
        ("viewpoint", request.composition.viewpoint.as_str()),
        ("camera_angle", request.composition.camera_angle.as_str()),
        ("balance", request.composition.balance.as_str()),
    ] {
        let path = format!("$.composition.{name}");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    for (index, value) in request.composition.depth_layers.iter().enumerate() {
        let path = format!("$.composition.depth_layers[{index}]");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    if request.composition.visual_hierarchy.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_HIERARCHY_EMPTY",
            "$.composition.visual_hierarchy",
            "visual hierarchy must name every subject and exact text element in priority order",
        ));
        return;
    }
    let known = request
        .subjects
        .iter()
        .map(|subject| subject.id.as_str())
        .chain(request.text_elements.iter().map(|text| text.id.as_str()))
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for (index, id) in request.composition.visual_hierarchy.iter().enumerate() {
        if !known.contains(id.as_str()) {
            diagnostics.push(Diagnostic::error(
                "IMG_HIERARCHY_UNKNOWN",
                format!("$.composition.visual_hierarchy[{index}]"),
                format!("unknown visual element id {id:?}"),
            ));
        }
        if !seen.insert(id.as_str()) {
            diagnostics.push(Diagnostic::error(
                "IMG_HIERARCHY_DUPLICATE",
                format!("$.composition.visual_hierarchy[{index}]"),
                format!("visual element id {id:?} appears more than once"),
            ));
        }
    }
    for id in known.difference(&seen) {
        diagnostics.push(Diagnostic::error(
            "IMG_HIERARCHY_INCOMPLETE",
            "$.composition.visual_hierarchy",
            format!("visual element id {id:?} is missing from the hierarchy"),
        ));
    }
}

fn validate_camera_and_lighting(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    if request.medium == VisualMedium::Photo && request.camera.is_none() {
        diagnostics.push(Diagnostic::warning(
            "IMG_PHOTO_CAMERA_MISSING",
            "$.camera",
            "photo medium is more deterministic with explicit camera outcome fields",
        ));
    }
    if let Some(camera) = &request.camera {
        for (name, value) in [
            ("field_of_view", camera.field_of_view.as_str()),
            ("perspective", camera.perspective.as_str()),
            ("depth_of_field", camera.depth_of_field.as_str()),
            ("focus", camera.focus.as_str()),
            ("motion_rendering", camera.motion_rendering.as_str()),
        ] {
            let path = format!("$.camera.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    for (name, value) in [
        ("key_direction", request.lighting.key_direction.as_str()),
        ("key_quality", request.lighting.key_quality.as_str()),
        (
            "fill_description",
            request.lighting.fill_description.as_str(),
        ),
        (
            "shadow_character",
            request.lighting.shadow_character.as_str(),
        ),
        ("exposure", request.lighting.exposure.as_str()),
    ] {
        let path = format!("$.lighting.{name}");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    if let Some(value) = &request.lighting.rim_description {
        validate_nonempty("$.lighting.rim_description", value, diagnostics);
        inspect_description("$.lighting.rim_description", value, diagnostics);
    }
    if let Some(kelvin) = request.lighting.key_temperature_kelvin
        && !(1_000..=20_000).contains(&kelvin)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_KELVIN_RANGE",
            "$.lighting.key_temperature_kelvin",
            "color temperature must be between 1000 K and 20000 K",
        ));
    }
    if let Some(color) = &request.lighting.key_color_hex {
        validate_hex("$.lighting.key_color_hex", color, diagnostics);
    }
    if let Some(ratio) = request.lighting.key_to_fill_ratio
        && (!ratio.is_finite() || ratio <= 0.0 || ratio > 20.0)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_LIGHT_RATIO",
            "$.lighting.key_to_fill_ratio",
            "key-to-fill ratio must be finite and within (0, 20]",
        ));
    }
}

fn validate_color(
    color: &ColorSpec,
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !(3..=5).contains(&color.palette.len()) {
        diagnostics.push(Diagnostic::error(
            "IMG_PALETTE_COUNT",
            "$.color.palette",
            "palette must contain 3 to 5 colors",
        ));
    }
    let mut total = 0_u16;
    let mut colors = BTreeSet::new();
    for (index, entry) in color.palette.iter().enumerate() {
        validate_hex(
            &format!("$.color.palette[{index}].hex"),
            &entry.hex,
            diagnostics,
        );
        total += u16::from(entry.proportion_percent);
        if !colors.insert(entry.hex.to_ascii_uppercase()) {
            diagnostics.push(Diagnostic::error(
                "IMG_PALETTE_DUPLICATE",
                format!("$.color.palette[{index}].hex"),
                "palette colors must be unique",
            ));
        }
        validate_nonempty(
            &format!("$.color.palette[{index}].usage"),
            &entry.usage,
            diagnostics,
        );
        inspect_description(
            &format!("$.color.palette[{index}].usage"),
            &entry.usage,
            diagnostics,
        );
    }
    if total != 100 {
        diagnostics.push(Diagnostic::error(
            "IMG_PALETTE_SUM",
            "$.color.palette",
            format!("palette proportions must sum to 100, found {total}"),
        ));
    }
    for (name, value) in [
        ("harmony", color.harmony.as_str()),
        ("contrast", color.contrast.as_str()),
        ("saturation", color.saturation.as_str()),
    ] {
        let path = format!("$.color.{name}");
        validate_nonempty(&path, value, diagnostics);
        inspect_description(&path, value, diagnostics);
    }
    if let Some(selection) = &color.photo_lut {
        validate_lut(selection, request, diagnostics);
    }
}

fn validate_lut(
    selection: &PhotoLutSelection,
    request: &ImagePromptRequest,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !PRESET_NAMES.contains(&selection.preset.as_str()) {
        diagnostics.push(Diagnostic::error(
            "IMG_LUT_PRESET",
            "$.color.photo_lut.preset",
            format!("unknown LUT preset {:?}", selection.preset),
        ));
    }
    for (name, value) in [
        ("strength_percent", Some(selection.strength_percent)),
        ("saturation_percent", selection.saturation_percent),
        ("grain_amount_percent", selection.grain_amount_percent),
        ("halation_percent", selection.halation_percent),
        ("vignette_percent", selection.vignette_percent),
    ] {
        if let Some(value) = value
            && value > 100
        {
            diagnostics.push(Diagnostic::error(
                "IMG_LUT_PERCENT",
                format!("$.color.photo_lut.{name}"),
                "percentage must be between 0 and 100",
            ));
        }
    }
    if let Some(kelvin) = selection.white_balance_kelvin
        && !(1_000..=20_000).contains(&kelvin)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_LUT_KELVIN",
            "$.color.photo_lut.white_balance_kelvin",
            "white balance must be between 1000 K and 20000 K",
        ));
    }
    if let Some(value) = &selection.shadow_bias_hex {
        validate_hex("$.color.photo_lut.shadow_bias_hex", value, diagnostics);
    }
    if let Some(value) = &selection.highlight_bias_hex {
        validate_hex("$.color.photo_lut.highlight_bias_hex", value, diagnostics);
    }
    for (name, value) in [
        ("tint", selection.tint.as_deref()),
        ("tone_curve", selection.tone_curve.as_deref()),
        ("black_response", selection.black_response.as_deref()),
        ("contrast", selection.contrast.as_deref()),
        ("highlight_rolloff", selection.highlight_rolloff.as_deref()),
        ("skin_tone_policy", selection.skin_tone_policy.as_deref()),
        ("grain_size", selection.grain_size.as_deref()),
    ] {
        if let Some(value) = value {
            let path = format!("$.color.photo_lut.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    if resolve_photo_lut(selection).is_none() && PRESET_NAMES.contains(&selection.preset.as_str()) {
        diagnostics.push(Diagnostic::error(
            "IMG_LUT_RESOLUTION",
            "$.color.photo_lut",
            "LUT could not be resolved",
        ));
    }
    if request.medium != VisualMedium::Photo {
        diagnostics.push(Diagnostic::warning(
            "IMG_LUT_NONPHOTO",
            "$.color.photo_lut",
            "photo LUT is a prompt-level color response profile; non-photo media may interpret it loosely",
        ));
    }
}

fn validate_text(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut languages = BTreeSet::new();
    if exact_text_is_required(selected) && request.text_elements.is_empty() {
        let ids = selected
            .iter()
            .filter(|entry| entry.text_requirement == crate::catalog::TextRequirement::Required)
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        diagnostics.push(
            Diagnostic::error(
                "IMG_TAXONOMY_EXACT_TEXT_REQUIRED",
                "$.text_elements",
                format!("selected visual system requires exact visible text: {ids}"),
            )
            .with_hint("supply exact text lines or choose a non-letterform visual system"),
        );
    }
    for (index, text) in request.text_elements.iter().enumerate() {
        let base = format!("$.text_elements[{index}]");
        if text.separator_count.is_some_and(|count| count > 8) {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_SEPARATOR_COUNT",
                format!("{base}.separator_count"),
                "separator_count must be between 0 and 8 per line",
            ));
        }
        let treatment = text.treatment.to_lowercase();
        if text.separator_count.is_none()
            && [" cuts", "cut lines", "cut segments", "절개선", "절단선"]
                .iter()
                .any(|marker| treatment.contains(marker))
        {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_SEPARATOR_COUNT_REQUIRED",
                format!("{base}.separator_count"),
                "cut or separator treatments require one explicit separator_count per text line",
            ));
        }
        validate_nonempty(&format!("{base}.id"), &text.id, diagnostics);
        if text.lines.is_empty() {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_LINES_EMPTY",
                format!("{base}.lines"),
                "at least one exact text line is required",
            ));
        }
        if text.lines.len() > 6 {
            diagnostics.push(Diagnostic::warning(
                "IMG_TEXT_LINES_DENSE",
                format!("{base}.lines"),
                "more than six exact lines materially increases rendering error risk",
            ));
        }
        for (line_index, line) in text.lines.iter().enumerate() {
            let path = format!("{base}.lines[{line_index}]");
            validate_nonempty(&path, line, diagnostics);
            inspect_exact_text(&path, line, diagnostics);
            if contains_hangul_and_latin(line) {
                diagnostics.push(
                    Diagnostic::error(
                        "IMG_TEXT_MIXED_SCRIPT",
                        path,
                        "a single exact text line mixes Hangul and Latin letters",
                    )
                    .with_hint("split Korean and English copy into separate labeled text elements"),
                );
            }
        }
        if !matches!(text.language.as_str(), "ko" | "en") {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_LANGUAGE",
                format!("{base}.language"),
                "expected ko or en",
            ));
        } else {
            languages.insert(text.language.as_str());
        }
        for (name, value) in [
            ("font_family", text.font_family.as_str()),
            ("weight", text.weight.as_str()),
            ("alignment", text.alignment.as_str()),
            ("letter_spacing", text.letter_spacing.as_str()),
            ("treatment", text.treatment.as_str()),
        ] {
            let path = format!("{base}.{name}");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
        if let Some(value) = &text.spelling_hint {
            let path = format!("{base}.spelling_hint");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
            if !matches!(request.output.detail, ImageDetail::High) {
                diagnostics.push(Diagnostic::warning(
                    "IMG_SPELLING_HINT_DETAIL",
                    "$.output.detail",
                    "spelling-sensitive exact text should use high detail",
                ));
            }
        }
        if text.size_percent == 0 || text.size_percent > 100 {
            diagnostics.push(Diagnostic::error(
                "IMG_TEXT_SIZE",
                format!("{base}.size_percent"),
                "text size percentage must be within 1..=100",
            ));
        }
        validate_hex(&format!("{base}.color_hex"), &text.color_hex, diagnostics);
        validate_placement(&text.placement, &format!("{base}.placement"), diagnostics);
    }
    let text_heavy = request.text_elements.len() >= 3
        || request
            .text_elements
            .iter()
            .any(|text| text.lines.len() >= 3)
        || languages.len() > 1;
    if text_heavy && request.constraints.safety_tier == 0 {
        diagnostics.push(Diagnostic::warning(
            "IMG_TEXT_TIER_RECOMMENDED",
            "$.constraints.safety_tier",
            "dense or mixed-language text benefits from tier 1 exact-text guard",
        ));
    }
    if text_heavy && !matches!(request.output.detail, ImageDetail::High) {
        diagnostics.push(Diagnostic::warning(
            "IMG_TEXT_DETAIL",
            "$.output.detail",
            "dense exact text should use high detail",
        ));
    }
}

fn validate_constraints(
    request: &ImagePromptRequest,
    _selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if request.constraints.safety_tier > 2 {
        diagnostics.push(Diagnostic::error(
            "IMG_SAFETY_TIER",
            "$.constraints.safety_tier",
            "safety tier must be 0, 1, or 2",
        ));
    }
    for (name, values) in [
        ("required_elements", &request.constraints.required_elements),
        ("excluded_elements", &request.constraints.excluded_elements),
    ] {
        for (index, value) in values.iter().enumerate() {
            let path = format!("$.constraints.{name}[{index}]");
            validate_nonempty(&path, value, diagnostics);
            inspect_description(&path, value, diagnostics);
        }
    }
    if request.constraints.safety_tier == 1 && request.text_elements.is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_TIER1_WITHOUT_TEXT",
            "$.constraints.safety_tier",
            "tier 1 is the exact-text guard lane and requires at least one text element",
        ));
    }
    if request.constraints.safety_tier == 2 {
        if !request.constraints.adult_subjects_only {
            diagnostics.push(Diagnostic::error(
                "IMG_TIER2_ADULT_ASSERT",
                "$.constraints.adult_subjects_only",
                "tier 2 requires adult_subjects_only=true",
            ));
        }
        if !request.constraints.original_characters_only {
            diagnostics.push(Diagnostic::error(
                "IMG_TIER2_ORIGINAL_CHARACTER",
                "$.constraints.original_characters_only",
                "tier 2 requires original_characters_only=true",
            ));
        }
        for (path, value) in descriptive_fields(request) {
            inspect_tier2_youth_terms(&path, value, diagnostics);
        }
    }
}

fn validate_complex_output_detail(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let complex_output = selected.iter().any(|entry| entry.id == "C10");
    if complex_output && !matches!(request.output.detail, ImageDetail::High) {
        diagnostics.push(Diagnostic::warning(
            "IMG_COMPLEX_OUTPUT_DETAIL",
            "$.output.detail",
            "multi-item character, storyboard, and comic outputs should use high detail",
        ));
    }
}

fn validate_output(request: &ImagePromptRequest, diagnostics: &mut Vec<Diagnostic>) {
    let output = &request.output;
    if output.backend != super::IMAGE_BACKEND {
        diagnostics.push(Diagnostic::error(
            "IMG_BACKEND",
            "$.output.backend",
            "this compiler profile currently supports codex-subscription only",
        ));
    }
    if output.format != "png" {
        diagnostics.push(Diagnostic::error(
            "IMG_FORMAT",
            "$.output.format",
            "Codex artifact verification currently supports png only",
        ));
    }
    if output.width == 0 || output.height == 0 {
        diagnostics.push(Diagnostic::error(
            "IMG_DIMENSION_ZERO",
            "$.output",
            "width and height must be positive",
        ));
        return;
    }
    if !output.width.is_multiple_of(16) || !output.height.is_multiple_of(16) {
        diagnostics.push(Diagnostic::error(
            "IMG_DIMENSION_MULTIPLE",
            "$.output",
            "codex-subscription width and height must be multiples of 16",
        ));
    }
    if output.width > MAX_EDGE || output.height > MAX_EDGE {
        diagnostics.push(Diagnostic::error(
            "IMG_DIMENSION_EDGE",
            "$.output",
            format!("each edge must be less than or equal to {MAX_EDGE} pixels"),
        ));
    }
    let pixels = u64::from(output.width) * u64::from(output.height);
    if !(MIN_PIXELS..=MAX_PIXELS).contains(&pixels) {
        diagnostics.push(Diagnostic::error(
            "IMG_PIXEL_COUNT",
            "$.output",
            format!("pixel count must be within {MIN_PIXELS}..={MAX_PIXELS}, found {pixels}"),
        ));
    } else if pixels > EXPERIMENTAL_PIXELS {
        diagnostics.push(Diagnostic::warning(
            "IMG_EXPERIMENTAL_SIZE",
            "$.output",
            "this size is within hard limits but above the documented standard range",
        ));
    }
    let ratio = output.width as f64 / output.height as f64;
    if !(1.0 / 3.0..=3.0).contains(&ratio) {
        diagnostics.push(Diagnostic::error(
            "IMG_ASPECT_RATIO",
            "$.output",
            "aspect ratio must be between 1:3 and 3:1",
        ));
    }
}

fn validate_placement(placement: &CanvasPlacement, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    if let CanvasPlacement::Custom {
        x_percent,
        y_percent,
        width_percent,
        height_percent,
    } = placement
    {
        if *width_percent == 0 || *height_percent == 0 {
            diagnostics.push(Diagnostic::error(
                "IMG_PLACEMENT_SIZE",
                path,
                "custom placement width and height must be positive",
            ));
        }
        if u16::from(*x_percent) + u16::from(*width_percent) > 100
            || u16::from(*y_percent) + u16::from(*height_percent) > 100
        {
            diagnostics.push(Diagnostic::error(
                "IMG_PLACEMENT_BOUNDS",
                path,
                "custom placement must remain inside the 0..=100 canvas",
            ));
        }
    }
}

fn register_id(
    ids: &mut BTreeMap<String, String>,
    id: &str,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_nonempty(path, id, diagnostics);
    if let Some(previous) = ids.insert(id.to_owned(), path.to_owned()) {
        diagnostics.push(Diagnostic::error(
            "IMG_ELEMENT_ID_DUPLICATE",
            path,
            format!("element id {id:?} was already declared at {previous}"),
        ));
    }
}

fn validate_nonempty(path: &str, value: &str, diagnostics: &mut Vec<Diagnostic>) {
    if value.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "IMG_EMPTY_FIELD",
            path,
            "value must not be empty",
        ));
    }
}

fn validate_hex(path: &str, value: &str, diagnostics: &mut Vec<Diagnostic>) {
    let bytes = value.as_bytes();
    if bytes.len() != 7
        || bytes.first() != Some(&b'#')
        || !bytes[1..].iter().all(u8::is_ascii_hexdigit)
    {
        diagnostics.push(Diagnostic::error(
            "IMG_HEX",
            path,
            "expected a six-digit color such as #1A2B3C",
        ));
    }
}

fn descriptive_fields(request: &ImagePromptRequest) -> Vec<(String, &str)> {
    let mut fields = vec![
        ("$.use_case".to_owned(), request.use_case.as_str()),
        (
            "$.scene.environment".to_owned(),
            request.scene.environment.as_str(),
        ),
        (
            "$.scene.background".to_owned(),
            request.scene.background.as_str(),
        ),
        (
            "$.scene.atmosphere".to_owned(),
            request.scene.atmosphere.as_str(),
        ),
    ];
    for (index, subject) in request.subjects.iter().enumerate() {
        fields.push((
            format!("$.subjects[{index}].description"),
            subject.description.as_str(),
        ));
        fields.push((format!("$.subjects[{index}].pose"), subject.pose.as_str()));
        fields.push((
            format!("$.subjects[{index}].action"),
            subject.action.as_str(),
        ));
        if let Some(value) = &subject.face {
            fields.push((format!("$.subjects[{index}].face"), value.as_str()));
        }
        if let Some(value) = &subject.hair {
            fields.push((format!("$.subjects[{index}].hair"), value.as_str()));
        }
        if let Some(value) = &subject.appearance {
            fields.push((format!("$.subjects[{index}].appearance"), value.as_str()));
        }
    }
    for (index, reference) in request.references.iter().enumerate() {
        fields.push((
            format!("$.references[{index}].description"),
            reference.description.as_str(),
        ));
        for (use_index, value) in reference.use_for.iter().enumerate() {
            fields.push((
                format!("$.references[{index}].use_for[{use_index}]"),
                value.as_str(),
            ));
        }
    }
    for (index, value) in request.change_contract.change_only.iter().enumerate() {
        fields.push((
            format!("$.change_contract.change_only[{index}]"),
            value.as_str(),
        ));
    }
    for (index, value) in request.change_contract.preserve.iter().enumerate() {
        fields.push((
            format!("$.change_contract.preserve[{index}]"),
            value.as_str(),
        ));
    }
    if let Some(storyboard) = &request.storyboard {
        for (index, value) in storyboard.identity_anchors.iter().enumerate() {
            fields.push((
                format!("$.storyboard.identity_anchors[{index}]"),
                value.as_str(),
            ));
        }
        for (index, panel) in storyboard.panels.iter().enumerate() {
            for (name, value) in [
                ("camera_angle", panel.camera_angle.as_str()),
                ("camera_move", panel.camera_move.as_str()),
                ("action", panel.action.as_str()),
                ("emotional_beat", panel.emotional_beat.as_str()),
            ] {
                fields.push((format!("$.storyboard.panels[{index}].{name}"), value));
            }
        }
    }
    if let Some(consistency) = &request.consistency {
        for (index, value) in consistency.shared_anchors.iter().enumerate() {
            fields.push((
                format!("$.consistency.shared_anchors[{index}]"),
                value.as_str(),
            ));
        }
        if let Some(sequence) = &consistency.sequence {
            for (index, value) in sequence.steps.iter().enumerate() {
                fields.push((
                    format!("$.consistency.sequence.steps[{index}]"),
                    value.as_str(),
                ));
            }
        }
    }
    fields
}

#[cfg(test)]
#[path = "validate/tests.rs"]
mod tests;
