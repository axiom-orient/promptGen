use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{TextRequirement, catalog, find_entry};
use crate::diagnostic::{CompilationOutcome, Diagnostic, PromptKind};
use crate::image::controls::{split_visual_brief, validate_controls};
use crate::image::{ImagePromptRequest, PRESET_NAMES, VisualControl, compile_image_prompt};
use crate::json::JsonValue;

use super::{
    IMAGE_ANSWER_KEYS, InterviewOption, InterviewOutcome, InterviewQuestion, InterviewRequest,
    InterviewRequestValue, InterviewStatus, answer, normalized_answers,
};

#[derive(Clone, Copy)]
struct AspectRatioPreset {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    width: u32,
    height: u32,
}

const ASPECT_RATIO_PRESETS: &[AspectRatioPreset] = &[
    AspectRatioPreset {
        id: "1:1",
        label: "1:1",
        description: "정사각형 1024×1024",
        width: 1024,
        height: 1024,
    },
    AspectRatioPreset {
        id: "2:3",
        label: "2:3",
        description: "세로형 1024×1536",
        width: 1024,
        height: 1536,
    },
    AspectRatioPreset {
        id: "3:2",
        label: "3:2",
        description: "가로형 1536×1024",
        width: 1536,
        height: 1024,
    },
    AspectRatioPreset {
        id: "3:4",
        label: "3:4",
        description: "세로형 1152×1536",
        width: 1152,
        height: 1536,
    },
    AspectRatioPreset {
        id: "4:5",
        label: "4:5",
        description: "세로형 1024×1280",
        width: 1024,
        height: 1280,
    },
    AspectRatioPreset {
        id: "4:3",
        label: "4:3",
        description: "가로형 1536×1152",
        width: 1536,
        height: 1152,
    },
    AspectRatioPreset {
        id: "16:9",
        label: "16:9",
        description: "와이드 1792×1008",
        width: 1792,
        height: 1008,
    },
    AspectRatioPreset {
        id: "9:16",
        label: "9:16",
        description: "세로 와이드 1008×1792",
        width: 1008,
        height: 1792,
    },
    AspectRatioPreset {
        id: "2048:2048",
        label: "2048×2048",
        description: "고해상도 정사각형",
        width: 2048,
        height: 2048,
    },
];
const SUPPORTED_LIGHTING: &[&str] = &[
    "soft_daylight",
    "hard_graphic",
    "low_key",
    "golden_hour",
    "neon_practical",
];
const SUPPORTED_LANGUAGES: &[&str] = &["ko", "en"];
const SUPPORTED_MEDIA: &[&str] = &["photo", "illustration", "3d", "graphic_design", "mixed"];
const SUPPORTED_DETAIL: &[&str] = &["auto", "low", "medium", "high"];
const SUPPORTED_TEXT_POSITIONS: &[&str] = &[
    "top_left",
    "top_center",
    "top_right",
    "middle_left",
    "center",
    "middle_right",
    "bottom_left",
    "bottom_center",
    "bottom_right",
];
const SUPPORTED_TEXT_STYLES: &[&str] = &["geometric", "condensed", "didone", "mono", "brush"];

#[path = "image/inference.rs"]
mod inference;
#[path = "image/questions.rs"]
mod questions;

use inference::*;
use questions::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImageInterviewStage {
    /// Only output language and category are decided here. Nothing route-dependent
    /// is written before the category has been validated.
    Route,
    /// Route-dependent inference, reached only after the category is accepted.
    Infer,
    Routing,
    TextContract,
    VisualDefaults,
    Validation,
    Compile,
}

struct ImageInterviewState {
    stage: ImageInterviewStage,
    answers: BTreeMap<String, String>,
    inferences: Vec<String>,
}

enum ImageInterviewStep {
    Advance(ImageInterviewStage),
    NeedsInput(Vec<InterviewQuestion>),
    Compile,
    Invalid(String),
}

impl ImageInterviewState {
    fn new(request: &InterviewRequest) -> Self {
        Self {
            stage: ImageInterviewStage::Route,
            answers: normalized_answers(request),
            inferences: Vec::new(),
        }
    }

    fn step(&mut self, request: &InterviewRequest) -> ImageInterviewStep {
        match self.stage {
            ImageInterviewStage::Route => {
                infer_route_from_brief(&request.brief, &mut self.answers, &mut self.inferences);
                if let Some(question) = category_routing_question(&self.answers) {
                    return ImageInterviewStep::NeedsInput(vec![question]);
                }
                ImageInterviewStep::Advance(ImageInterviewStage::Infer)
            }
            ImageInterviewStage::Infer => {
                // What the brief states comes first. Every inference step only fills an
                // absent answer, so running the route's default medium ahead of the
                // brief would let `photo` win over an explicitly requested illustration.
                infer_stated_content_from_brief(
                    &request.brief,
                    &mut self.answers,
                    &mut self.inferences,
                );
                if let Some(value) = answer(&self.answers, "image.visual_controls") {
                    let controls = value
                        .split_whitespace()
                        .map(VisualControl::parse)
                        .collect::<Result<Vec<_>, _>>()
                        .expect("controls were decoded at ingress");
                    for control in &controls {
                        if let Some(expected) = control.medium() {
                            if answer(&self.answers, "image.medium").is_some_and(|actual| {
                                actual != expected.as_str() && actual != "mixed"
                            }) {
                                return ImageInterviewStep::Invalid("visual control conflicts with the requested medium; change the control or medium".into());
                            }
                            self.answers
                                .entry("image.medium".into())
                                .or_insert_with(|| expected.as_str().into());
                        }
                    }
                }
                prune_irrelevant_category_answers(&mut self.answers, &mut self.inferences);
                infer_category_contract_from_brief(
                    &request.brief,
                    &mut self.answers,
                    &mut self.inferences,
                );
                infer_profile_from_brief(&request.brief, &mut self.answers, &mut self.inferences);
                infer_medium_from_route(&mut self.answers, &mut self.inferences);
                infer_route_content(&request.brief, &mut self.answers, &mut self.inferences);
                ImageInterviewStep::Advance(ImageInterviewStage::Routing)
            }
            ImageInterviewStage::Routing => {
                let questions = collect_routing_questions(&self.answers);
                if questions.is_empty() {
                    ImageInterviewStep::Advance(ImageInterviewStage::TextContract)
                } else {
                    ImageInterviewStep::NeedsInput(questions)
                }
            }
            ImageInterviewStage::TextContract => {
                resolve_text_mode(&mut self.answers, &mut self.inferences);
                apply_text_defaults(&mut self.answers, &mut self.inferences);
                let questions = collect_contract_questions(&self.answers);
                if questions.is_empty() {
                    ImageInterviewStep::Advance(ImageInterviewStage::VisualDefaults)
                } else {
                    ImageInterviewStep::NeedsInput(questions)
                }
            }
            ImageInterviewStage::VisualDefaults => {
                apply_visual_defaults(&request.brief, &mut self.answers, &mut self.inferences);
                ImageInterviewStep::Advance(ImageInterviewStage::Validation)
            }
            ImageInterviewStage::Validation => {
                let questions = collect_validation_questions(&self.answers);
                if questions.is_empty() {
                    ImageInterviewStep::Advance(ImageInterviewStage::Compile)
                } else {
                    ImageInterviewStep::NeedsInput(questions)
                }
            }
            ImageInterviewStage::Compile => ImageInterviewStep::Compile,
        }
    }

    fn needs_input(self, questions: Vec<InterviewQuestion>) -> InterviewOutcome {
        InterviewOutcome::needs_input(PromptKind::Image, self.answers, self.inferences, questions)
    }

    fn compile(self, request: &InterviewRequest) -> InterviewOutcome {
        let request_json = build_request_json(request, &self.answers);
        match ImagePromptRequest::from_json(request_json.clone()) {
            Ok(mut decoded) => {
                if answer(&self.answers, "image.task_mode") == Some("edit") {
                    decoded.task_mode = crate::image::ImageTaskMode::Edit;
                    decoded.references = vec![crate::image::ImageReference {
                        index: 1,
                        role: crate::image::ImageReferenceRole::Base,
                        description: answer(&self.answers, "image.reference_description")
                            .unwrap_or_default()
                            .into(),
                        use_for: vec![
                            "source photograph and preserved identity, pose and composition".into(),
                        ],
                    }];
                    decoded.change_contract = crate::image::ImageChangeContract {
                        preserve: answer(&self.answers, "image.preserve")
                            .unwrap_or_default()
                            .lines()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(String::from)
                            .collect(),
                        change_only: answer(&self.answers, "image.change_only")
                            .unwrap_or_default()
                            .lines()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(String::from)
                            .collect(),
                    };
                    decoded.camera = None;
                    decoded.composition.camera_angle = image_copy(
                        decoded.language.code(),
                        "원본 사진 내부의 카메라 각도를 유지",
                        "Preserve the camera angle inside the base photograph",
                    )
                    .into();
                    decoded.composition.viewpoint = image_copy(
                        decoded.language.code(),
                        "원본 사진 내부의 원근과 시점을 유지",
                        "Preserve the perspective and viewpoint inside the base photograph",
                    )
                    .into();
                    decoded.lighting = crate::image::LightingSpec {
                        key_direction: "Preserve source-photograph light direction".into(),
                        key_quality: "Preserve source-photograph light softness".into(),
                        key_temperature_kelvin: None, key_color_hex: None, key_to_fill_ratio: None,
                        fill_description: "Preserve source-photograph ambient fill".into(),
                        rim_description: None,
                        shadow_character: "Preserve source shadows; match decorative paper contact shadows to them".into(),
                        exposure: "Preserve source-photograph exposure".into(),
                    };
                    decoded.color.photo_lut = None;
                    decoded.color.harmony = "Keep source-photograph colors unchanged; apply the declared palette only to added paper and annotations".into();
                    decoded.color.contrast =
                        "Preserve source-photograph contrast; keep added text legible".into();
                    decoded.color.saturation = "Preserve source-photograph saturation".into();
                    for color in &mut decoded.color.palette {
                        color.usage = format!("Decoration only: {}", color.usage);
                    }
                }
                for value in answer(&self.answers, "image.visual_controls")
                    .unwrap_or("")
                    .split_whitespace()
                {
                    let control =
                        VisualControl::parse(value).expect("controls were decoded at ingress");
                    control.apply(&mut decoded);
                }
                if decoded.profile == crate::image::ImageProfile::TravelJournal {
                    let white_pen = contains_any(
                        &request.brief.to_lowercase(),
                        &["흰색 펜", "흰 펜", "white pen", "white handwriting"],
                    );
                    for text in &mut decoded.text_elements {
                        text.color_hex = if white_pen { "#F5F5F2" } else { "#252423" }.into();
                    }
                }
                let request_json = decoded.to_json();
                let compilation = compile_image_prompt(&decoded);
                InterviewOutcome::completed(
                    PromptKind::Image,
                    self.answers,
                    self.inferences,
                    request_json,
                    Some(InterviewRequestValue::Image(Box::new(decoded))),
                    compilation,
                )
            }
            Err(error) => {
                let compilation = CompilationOutcome::new(
                    PromptKind::Image,
                    None,
                    vec![Diagnostic::error(
                        "INTERVIEW_REQUEST_BUILD",
                        "$",
                        error.to_string(),
                    )],
                    JsonValue::Object(Default::default()),
                );
                InterviewOutcome::completed(
                    PromptKind::Image,
                    self.answers,
                    self.inferences,
                    request_json,
                    None,
                    compilation,
                )
            }
        }
    }
}

pub(super) fn run(request: &InterviewRequest) -> InterviewOutcome {
    let (controls, brief) = match split_visual_brief(&request.brief) {
        Ok(value) => value,
        Err(message) => return invalid_visual_brief(request, message),
    };
    let mut normalized = request.clone();
    normalized.brief = brief.to_owned();
    let values = if let Some(value) = normalized.answers.get("image.visual_controls") {
        match value
            .split_whitespace()
            .map(VisualControl::parse)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(values) if controls.is_empty() || values == controls => values,
            Ok(_) => {
                return invalid_visual_brief(
                    request,
                    "brief controls conflict with explicit visual controls".into(),
                );
            }
            Err(message) => return invalid_visual_brief(request, message),
        }
    } else {
        controls
    };
    if values.iter().any(|control| {
        matches!(
            control,
            VisualControl::TravelJournal | VisualControl::Scrapbook
        )
    }) {
        normalized
            .answers
            .entry("image.category".into())
            .or_insert_with(|| "C5".into());
        normalized
            .answers
            .entry("image.medium".into())
            .or_insert_with(|| "mixed".into());
        normalized.answers.entry("image.campaign_plan".into()).or_insert_with(||
            "여행 기록 한 장: 주 장면 하나와 사용자가 제공한 정확 메모를 구분하고, 인물과 사진의 주요 내용을 가리지 않음. 미지정 문구·날짜·상호·CTA 없음.".into());
        normalized
            .answers
            .entry("image.aspect_ratio".into())
            .or_insert_with(|| "4:5".into());
        normalized
            .answers
            .entry("image.text_position".into())
            .or_insert_with(|| "bottom_left".into());
    }
    if values.iter().any(|control| {
        matches!(
            control,
            VisualControl::TravelJournal | VisualControl::Scrapbook
        )
    }) {
        normalized
            .answers
            .entry("image.profile".into())
            .or_insert_with(|| "travel_journal".into());
    }
    let inferred_medium = values.iter().find_map(|control| control.medium());
    let medium = inferred_medium.unwrap_or(crate::image::VisualMedium::Mixed);
    if let Some(error) = validate_controls(&values, medium).first() {
        return invalid_visual_brief(request, error.message.clone());
    }
    if let Some(composition) = request.answers.get("image.composition")
        && values.iter().any(|control| control.controls_camera())
    {
        let category = answer(&normalized.answers, "image.category").unwrap_or("C4");
        let language = answer(&normalized.answers, "language").unwrap_or("ko");
        let context = if category == "C10" {
            format!(
                "{}\n{}",
                normalized.brief,
                answer(&normalized.answers, "image.panel_plan").unwrap_or_default()
            )
        } else {
            normalized.brief.clone()
        };
        let derived = [false, true].into_iter().any(|has_text| {
            composition == &default_composition(category, has_text, &context, language)
        });
        if !derived {
            return invalid_visual_brief(request, "choose camera controls or a custom composition answer; both cannot own the same camera direction".into());
        }
    }
    for control in &values {
        if let Some(expected) = control.lighting_key() {
            if normalized
                .answers
                .get("image.lighting")
                .is_some_and(|actual| actual != expected)
            {
                return invalid_visual_brief(
                    request,
                    "visual control conflicts with the explicit lighting answer".into(),
                );
            }
            normalized
                .answers
                .insert("image.lighting".into(), expected.into());
        }
    }
    if !values.is_empty() {
        normalized.answers.insert(
            "image.visual_controls".into(),
            values
                .iter()
                .map(|control| control.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    if values.contains(&VisualControl::Handwritten) {
        if normalized
            .answers
            .get("image.text_mode")
            .is_some_and(|value| value != "exact")
        {
            return invalid_visual_brief(request, "handwritten requires exact approved copy; remove the control for a text-free image".into());
        }
        normalized
            .answers
            .insert("image.text_mode".into(), "exact".into());
    }
    let source_photo = contains_any(
        &normalized.brief.to_lowercase(),
        &[
            "업로드한",
            "원본 사진",
            "uploaded photo",
            "input photo",
            "reference photo",
        ],
    );
    if source_photo {
        normalized
            .answers
            .entry("image.task_mode".into())
            .or_insert_with(|| "edit".into());
        normalized.answers.entry("image.reference_description".into()).or_insert_with(||
            "Base photograph to be supplied as image 1 at execution; use its depicted subjects and original scene as the preservation source".into());
        let clauses = normalized
            .brief
            .split(['.', '\n'])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        for (key, terms) in [
            (
                "image.preserve",
                &["유지", "보존", "preserve", "keep unchanged"][..],
            ),
            (
                "image.change_only",
                &["만 추가", "만 변경", "add only", "change only"][..],
            ),
        ] {
            if !normalized.answers.contains_key(key) {
                let matches = clauses
                    .iter()
                    .filter(|clause| contains_any(&clause.to_lowercase(), terms))
                    .copied()
                    .collect::<Vec<_>>();
                if !matches.is_empty() {
                    normalized.answers.insert(key.into(), matches.join("\n"));
                }
            }
        }
    }
    let mode = normalized
        .answers
        .get("image.task_mode")
        .map(String::as_str);
    if mode.is_some_and(|mode| mode != "generate" && mode != "edit") {
        return invalid_visual_brief(
            request,
            "guided interview supports generate or single-base edit only".into(),
        );
    }
    if source_photo && mode == Some("generate") {
        return invalid_visual_brief(
            request,
            "a request to preserve an input photo requires task_mode=edit and a bound base image"
                .into(),
        );
    }
    if mode == Some("edit")
        && values
            .iter()
            .any(|control| control.controls_camera() || control.lighting_key().is_some())
    {
        return invalid_visual_brief(request, "source-photo preservation cannot be combined with camera or lighting replacement controls; declare a precise edit in the typed request instead".into());
    }
    if mode == Some("edit") {
        let mut questions = Vec::new();
        for (key, label, help, placeholder) in [
            (
                "image.reference_description",
                "어떤 원본 사진을 사용할까요?",
                "원본에서 알아볼 수 있는 인물·장소·구도를 설명하세요. 실행할 때 실제 파일을 별도로 전달합니다.",
                "제공한 해안 여행 사진: 성인 한 명, 흰 셔츠, 바다를 보는 뒷모습",
            ),
            (
                "image.preserve",
                "무엇을 그대로 유지할까요?",
                "보존할 항목을 한 줄에 하나씩 적으세요.",
                "얼굴과 정체성\n옷과 자세\n사진 내부 구도와 장소",
            ),
            (
                "image.change_only",
                "어떤 부분만 바꿀까요?",
                "사진 주위의 장식·문구 등 실제로 변경할 항목만 적으세요.",
                "사진 바깥 여백에 크림색 종이와 지정한 손글씨 추가",
            ),
        ] {
            if answer(&normalized.answers, key).is_none() {
                questions.push(InterviewQuestion::text(key, label, help, placeholder));
            }
        }
        if !questions.is_empty() {
            return InterviewOutcome::needs_input(
                PromptKind::Image,
                normalized_answers(&normalized),
                vec![],
                questions,
            );
        }
    }
    let request = &normalized;
    let answers = normalized_answers(request);
    if let Some(key) = answers
        .keys()
        .find(|key| !IMAGE_ANSWER_KEYS.contains(&key.as_str()))
    {
        let compilation = CompilationOutcome::new(
            PromptKind::Image,
            None,
            vec![Diagnostic::error(
                "INTERVIEW_UNKNOWN_ANSWER",
                format!("$.answers.{key}"),
                "unknown image interview answer",
            )],
            JsonValue::Object(Default::default()),
        );
        return InterviewOutcome {
            status: InterviewStatus::Invalid,
            kind: PromptKind::Image,
            normalized_answers: answers,
            inferences: Vec::new(),
            questions: Vec::new(),
            request: None,
            request_value: None,
            compilation: Some(compilation),
        };
    }
    let mut state = ImageInterviewState::new(request);
    loop {
        match state.step(request) {
            ImageInterviewStep::Advance(next) => state.stage = next,
            ImageInterviewStep::NeedsInput(questions) => return state.needs_input(questions),
            ImageInterviewStep::Compile => return state.compile(request),
            ImageInterviewStep::Invalid(message) => return invalid_visual_brief(request, message),
        }
    }
}

fn invalid_visual_brief(request: &InterviewRequest, message: String) -> InterviewOutcome {
    InterviewOutcome::completed(
        PromptKind::Image,
        normalized_answers(request),
        vec![],
        JsonValue::Null,
        None,
        CompilationOutcome::new(
            PromptKind::Image,
            None,
            vec![Diagnostic::error(
                "INTERVIEW_VISUAL_CONTROL",
                "$.brief",
                message,
            )],
            JsonValue::Object(Default::default()),
        ),
    )
}

/// Route-selecting inference. Everything written here either identifies the
/// output language or narrows the catalog route, so it is safe to run before the
/// category has been validated.
fn infer_route_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    if answer(answers, "language").is_none() {
        let language = if brief.chars().any(is_hangul) {
            "ko"
        } else {
            "en"
        };
        answers.insert("language".to_owned(), language.to_owned());
        inferences.push(format!(
            "입력 문자 체계를 근거로 출력 언어를 {language}로 추정했다."
        ));
    }
    if answer(answers, "image.category").is_none()
        && let Some(category) = infer_category(brief)
    {
        answers.insert("image.category".to_owned(), category.to_owned());
        inferences.push(format!(
            "요청 키워드를 근거로 결과물 {category}를 추정했다."
        ));
    }
}

/// Content stated in the brief. This runs only after the route is accepted, so a
/// re-asked category never leaves subject, scene, or medium behind.
fn infer_stated_content_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    if answer(answers, "image.text_mode") == Some("exact")
        && answer(answers, "image.text").is_none()
    {
        let quoted = extract_quoted_texts(brief);
        if !quoted.is_empty() {
            answers.insert("image.text".into(), quoted.join("\n"));
            inferences.push("요청에 따옴표로 지정한 문구를 정확 문구로 사용했다.".into());
        }
    }
    if answer(answers, "image.medium").is_none()
        && let Some(medium) = infer_medium(brief)
    {
        answers.insert("image.medium".to_owned(), medium.to_owned());
        inferences.push(format!("요청 키워드를 근거로 매체 {medium}을 추정했다."));
    }
    if answer(answers, "image.aspect_ratio").is_none()
        && let Some(ratio) = find_ratio(brief)
    {
        answers.insert("image.aspect_ratio".to_owned(), ratio.clone());
        inferences.push(format!("요청에 명시된 화면 비율 {ratio}를 사용한다."));
    }
    if answer(answers, "image.palette").is_none()
        && let Some(colors) = extract_hex_palette(brief)
    {
        let value = colors.join(", ");
        answers.insert("image.palette".to_owned(), value.clone());
        inferences.push(format!("요청에 명시된 HEX 팔레트 {value}를 사용한다."));
    }
    if answer(answers, "image.text_mode").is_none() {
        if contains_any(
            &brief.to_lowercase(),
            &[
                "텍스트 없음",
                "텍스트 없이",
                "문구 없음",
                "문구 없이",
                "문구와 로고 없이",
                "글자 없음",
                "글자 없이",
                "텍스트를 넣지",
                "문구를 넣지",
                "글자를 넣지",
                "no text",
                "no readable text",
                "without text",
                "exclude text",
            ],
        ) {
            answers.insert("image.text_mode".to_owned(), "none".to_owned());
            inferences.push("요청의 명시적 무문구 조건을 확인했다.".to_owned());
        } else {
            let quoted = extract_quoted_texts(brief);
            if !quoted.is_empty() {
                answers.insert("image.text_mode".to_owned(), "exact".to_owned());
                answers
                    .entry("image.text".to_owned())
                    .or_insert_with(|| quoted.join("\n"));
                inferences.push("따옴표 안 문구를 이미지 내 정확 문구로 추정했다.".to_owned());
            }
        }
    }
    if answer(answers, "image.subject").is_none()
        && let Some(subject) = extract_subject_clause(brief)
    {
        answers.insert("image.subject".to_owned(), subject.clone());
        inferences.push(format!(
            "수량·피사체 표지가 있는 절을 주 피사체로 추정했다: {subject}"
        ));
    }
    if answer(answers, "image.scene").is_none()
        && let Some(scene) = extract_scene_clause(brief)
    {
        answers.insert("image.scene".to_owned(), scene.clone());
        inferences.push(format!(
            "장소·시간·대기 표지가 있는 절을 장면으로 추정했다: {scene}"
        ));
    }
    if answer(answers, "image.composition").is_none()
        && let Some(composition) = extract_composition_clause(brief)
    {
        answers.insert("image.composition".to_owned(), composition.clone());
        inferences.push(format!("요청의 구도 표지를 사용한다: {composition}"));
    }
    if answer(answers, "image.surface").is_none()
        && let Some(surface) = extract_surface_clause(brief)
    {
        answers.insert("image.surface".to_owned(), surface.clone());
        inferences.push(format!("요청의 재질·마감 표지를 사용한다: {surface}"));
    }
    if answer(answers, "image.category").is_some_and(|category| category.eq_ignore_ascii_case("C1"))
        && answer(answers, "image.wardrobe").is_none()
        && let Some(wardrobe) = extract_wardrobe_clause(brief)
        && wardrobe_is_specific(&wardrobe)
    {
        answers.insert("image.wardrobe".to_owned(), wardrobe.clone());
        inferences.push(format!(
            "요청의 색·소재·의류종·핏 정보를 화보 의상 슬롯으로 사용한다: {wardrobe}"
        ));
    }
    if answer(answers, "image.lighting").is_none()
        && let Some(lighting) = infer_lighting(brief)
    {
        answers.insert("image.lighting".to_owned(), lighting.to_owned());
        inferences.push(format!(
            "요청의 광원·시간대 표현을 {lighting} 조명 프로필로 해석했다."
        ));
    }
    if answer(answers, "image.lighting_detail").is_none()
        && let Some(detail) = extract_lighting_clause(brief)
    {
        answers.insert("image.lighting_detail".to_owned(), detail.clone());
        inferences.push(format!(
            "요청에 명시된 광원 방향과 광질을 보존한다: {detail}"
        ));
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && answer(answers, "image.text_position").is_none()
        && let Some(position) = infer_text_position(brief)
    {
        answers.insert("image.text_position".to_owned(), position.to_owned());
        inferences.push(format!(
            "요청의 위치 표현을 {position} 텍스트 영역으로 해석했다."
        ));
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && answer(answers, "image.text_style").is_none()
        && let Some(style) = infer_text_style(brief)
    {
        answers.insert("image.text_style".to_owned(), style.to_owned());
        inferences.push(format!("요청의 타이포 표현을 {style} 계열로 해석했다."));
    }
}

fn prune_irrelevant_category_answers(
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    const CATEGORY_KEYS: &[&str] = &[
        "image.wardrobe",
        "image.adult_editorial_confirm",
        "image.variants",
        "image.formulation",
        "image.product_guide",
        "image.app_icon_concept",
        "image.campaign_plan",
        "image.information_plan",
        "image.card_plan",
        "image.brand_applications",
        "image.icon_set",
        "image.panel_plan",
        "image.keyart_plan",
        "image.deck_plan",
        "image.character_sheet",
        "image.occlusion_plan",
        "image.series_plan",
        "image.meta_ui_plan",
        "image.collage_plan",
        "image.storyboard_plan",
        "image.stage_plan",
    ];
    let category = answer(answers, "image.category")
        .map(str::to_ascii_uppercase)
        .unwrap_or_default();
    let allowed = match category.as_str() {
        "C1" => &["image.wardrobe"][..],
        "C4" if is_app_icon_profile(answers) => &["image.app_icon_concept"][..],
        "C4" => &["image.product_guide"][..],
        "C5" => &["image.campaign_plan"][..],
        "C6" => &["image.information_plan"][..],
        "C10" => &["image.panel_plan"][..],
        "C11" => &["image.keyart_plan"][..],
        _ => &[][..],
    };
    let removed = CATEGORY_KEYS
        .iter()
        .filter(|key| !allowed.contains(key))
        .filter_map(|key| answers.remove(*key).map(|_| *key))
        .collect::<Vec<_>>();
    if !removed.is_empty() {
        inferences.push(format!(
            "현재 결과물 경로와 무관한 이전 전용 답변을 제거했다: {}",
            removed.join(", ")
        ));
    }
    if let Some(profile) = answers.get("image.profile").cloned()
        && profile_is_known(&profile)
        && !profile_is_compatible(&category, &profile)
    {
        answers.remove("image.profile");
        inferences.push(format!(
            "현재 {category} 결과물과 호환되지 않는 이전 image.profile={profile} 답변을 제거했다."
        ));
    }
}

fn infer_profile_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let Some(category) = answer(answers, "image.category") else {
        return;
    };
    if answer(answers, "image.profile").is_some() {
        return;
    }
    if let Some(profile) = inference::infer_profile(brief, category) {
        answers.insert("image.profile".to_owned(), profile.to_owned());
        inferences.push(format!(
            "요청의 세부 의도를 근거로 image.profile={profile}을 추정했다."
        ));
    }
}

fn profile_is_compatible(category: &str, profile: &str) -> bool {
    if profile == "travel_journal" {
        return category == "C5";
    }

    match profile.to_ascii_lowercase().as_str() {
        "standard" => true,
        "logo_identity" | "app_icon" => category.eq_ignore_ascii_case("C4"),
        "app_web_ui" | "information_design" => category.eq_ignore_ascii_case("C6"),
        "character_pose" | "poomsae_pose" => category.eq_ignore_ascii_case("C10"),
        _ => false,
    }
}

fn profile_is_known(profile: &str) -> bool {
    matches!(
        profile.to_ascii_lowercase().as_str(),
        "standard"
            | "travel_journal"
            | "logo_identity"
            | "app_icon"
            | "app_web_ui"
            | "information_design"
            | "character_pose"
            | "poomsae_pose"
    )
}

fn is_app_icon_profile(answers: &BTreeMap<String, String>) -> bool {
    answer(answers, "image.profile").is_some_and(|profile| profile.eq_ignore_ascii_case("app_icon"))
}

fn infer_medium_from_route(answers: &mut BTreeMap<String, String>, inferences: &mut Vec<String>) {
    if answer(answers, "image.medium").is_some() {
        return;
    }
    let Some(category) = answer(answers, "image.category") else {
        return;
    };
    let profile = answer(answers, "image.profile").map(str::to_ascii_lowercase);
    let medium = match (category.to_ascii_uppercase().as_str(), profile.as_deref()) {
        ("C4", Some("logo_identity" | "app_icon")) => "graphic_design",
        ("C6", Some("app_web_ui" | "information_design")) => "graphic_design",
        ("C10", Some("character_pose" | "poomsae_pose")) => "illustration",
        ("C6", _) => "graphic_design",
        ("C10" | "C11", _) => "illustration",
        _ => "photo",
    };
    answers.insert("image.medium".to_owned(), medium.to_owned());
    inferences.push(format!("선택한 결과물의 기본 매체 {medium}을 적용했다."));
}

fn profile_medium_options(profile: &str) -> Option<Vec<InterviewOption>> {
    let allowed = match profile.to_ascii_lowercase().as_str() {
        "logo_identity" => &["graphic_design", "3d", "mixed"][..],
        "app_icon" => &["graphic_design", "illustration", "3d", "mixed"][..],
        "app_web_ui" => &["graphic_design", "mixed", "3d"][..],
        "information_design" => &["graphic_design", "mixed", "illustration"][..],
        "character_pose" | "poomsae_pose" => &["illustration", "mixed"][..],
        _ => return None,
    };
    let options = medium_options();
    Some(
        allowed
            .iter()
            .filter_map(|value| {
                options
                    .iter()
                    .find(|option| option.value == *value)
                    .cloned()
            })
            .collect(),
    )
}

fn profile_medium_question(profile: &str, medium: &str) -> Option<InterviewQuestion> {
    let options = profile_medium_options(profile)?;
    Some(
        InterviewQuestion::choice(
            "image.medium",
            "선택한 프로필과 호환되는 시각 매체를 선택해 주세요.",
            "현재 매체를 자동으로 덮어쓰지 않습니다. 아래 호환 옵션 중 하나를 선택하면 다시 검증합니다.",
            options,
        )
        .with_error(format!(
            "{profile} 프로필은 현재 {medium} 매체와 호환되지 않습니다. 아래 호환 매체 중 하나를 선택해 주세요."
        )),
    )
}

fn category_routing_question(answers: &BTreeMap<String, String>) -> Option<InterviewQuestion> {
    match answer(answers, "image.category") {
        None => Some(category_question()),
        Some(category) if !is_primary_category(category) => {
            Some(category_question().with_error("지원하지 않는 결과물 종류입니다."))
        }
        Some(_) => None,
    }
}

fn collect_routing_questions(answers: &BTreeMap<String, String>) -> Vec<InterviewQuestion> {
    let mut questions = Vec::new();
    debug_assert!(category_routing_question(answers).is_none());

    if let Some(question) = missing_category_contract_question(answers) {
        questions.push(question);
        return questions;
    }

    if let Some(profile) = answer(answers, "image.profile")
        && !profile_is_compatible(
            answer(answers, "image.category").unwrap_or_default(),
            profile,
        )
    {
        questions.push(
            profile_question(answer(answers, "image.category").unwrap_or_default())
                .with_error("선택한 프로필이 결과물 카테고리와 호환되지 않습니다."),
        );
        return questions;
    }

    push_missing(
        answers,
        "image.subject",
        InterviewQuestion::multiline(
            "image.subject",
            "주 피사체를 눈에 보이게 정의해 주세요.",
            "수량, 형태, 자세 또는 방향, 표면 특징을 포함합니다.",
            "무광 아이보리 도자기 컵 1개, 손잡이는 오른쪽 3시 방향, 짙은 필터 커피가 보인다.",
        ),
        &mut questions,
    );
    push_missing(
        answers,
        "image.scene",
        InterviewQuestion::multiline(
            "image.scene",
            "장면과 배경을 어떻게 구성합니까?",
            "환경, 후경, 시간대, 대기 상태를 관찰 가능한 문장으로 적습니다.",
            "오전의 콘크리트 로스터리, 후면은 중간 회색 벽, 창을 통과한 확산 일광.",
        ),
        &mut questions,
    );

    questions
}

#[derive(Clone, Copy)]
struct CategoryContractSpec {
    key: &'static str,
    label: &'static str,
    help: &'static str,
    placeholder: &'static str,
    brief_markers: &'static [&'static str],
}

fn category_contract_spec(category: &str) -> Option<CategoryContractSpec> {
    match category.to_ascii_uppercase().as_str() {
        "C4" => Some(CategoryContractSpec {
            key: "image.product_guide",
            label: "제품과 브랜드에서 반드시 보여 줄 증거를 정해 주세요.",
            help: "제품·패키지·브랜드 적용물, 수량, 핵심 재질, 보여 줄 면과 라벨 사용 여부를 한 번에 적습니다.",
            placeholder: "무광 알루미늄 헤드폰 1개와 재생지 패키지 1개, 3/4 외관, 이어컵과 헤드밴드 재질 차이, 로고·문구 없음.",
            brief_markers: &[
                "제품",
                "브랜드",
                "패키지",
                "목업",
                "재질",
                "라벨",
                "외관",
                "로고",
            ],
        }),
        "C5" => Some(CategoryContractSpec {
            key: "image.campaign_plan",
            label: "캠페인의 목적과 히어로를 정해 주세요.",
            help: "대상 고객, 전달할 한 가지 메시지, 주 제품 또는 모델, CTA가 들어갈 안전 영역을 한 답변으로 확인합니다.",
            placeholder: "출근길 직장인을 위한 콜드브루 출시 캠페인, 무광 검정 캔 1개가 히어로, 좌하단 CTA 여백 18%.",
            brief_markers: &[
                "캠페인",
                "광고",
                "포스터",
                "키비주얼",
                "브랜드",
                "대상",
                "고객",
                "히어로",
                "제품",
                "모델",
                "cta",
            ],
        }),
        "C6" => Some(CategoryContractSpec {
            key: "image.information_plan",
            label: "전달할 정보와 읽는 순서를 정해 주세요.",
            help: "핵심 메시지, 실제 단계·수치, 각 라벨, 시작에서 끝까지의 읽기 방향을 한 답변으로 적습니다.",
            placeholder: "원두 수확→세척→건조→로스팅→추출 5단계, 단계별 한 줄 설명, 좌상단에서 우하단으로 읽기.",
            brief_markers: &[
                "인포그래픽",
                "단계",
                "수치",
                "데이터",
                "라벨",
                "흐름",
                "순서",
                "과정",
            ],
        }),
        "C10" => Some(CategoryContractSpec {
            key: "image.panel_plan",
            label: "그림의 장면 수와 이야기 흐름을 정해 주세요.",
            help: "단일 장면·패널·스티커·스토리보드 중 형식, 개수, 반복할 캐릭터 특징과 장면별 사건을 적습니다.",
            placeholder: "같은 붉은 목도리 여우가 등장하는 4패널, 좌→우 읽기. 씨앗 발견→심기→비 기다리기→새싹과 인사, 문구 없음.",
            brief_markers: &[
                "일러스트",
                "이야기",
                "만화",
                "패널",
                "스티커",
                "스토리보드",
                "컷",
            ],
        }),
        "C11" => Some(CategoryContractSpec {
            key: "image.keyart_plan",
            label: "컨셉 아트의 세계와 초점을 정해 주세요.",
            help: "장르, 주인공 또는 핵심 오브젝트, 환경 스케일, 감정, 카메라와 안전 영역을 한 답변으로 확인합니다.",
            placeholder: "고대 수몰 도시에 도착한 탐험가 1명, 거대한 석조 아치와 작은 인물의 대비, 경외감, 24mm 와이드, 문구 없음.",
            brief_markers: &[
                "컨셉",
                "키아트",
                "영화",
                "게임",
                "장르",
                "주인공",
                "환경",
                "세계",
            ],
        }),
        _ => None,
    }
}

fn app_icon_contract_spec() -> CategoryContractSpec {
    CategoryContractSpec {
        key: "image.app_icon_concept",
        label: "앱 아이콘의 제품 목적과 핵심 시각 은유를 정해 주세요.",
        help: "제품이 무엇을 돕는지, 하나의 중심 상징과 그 상징을 이루는 간결한 형태·색·깊이를 적습니다. 글자·UI·디바이스 프레임은 넣지 않습니다.",
        placeholder: "개인 기억 앱, 작은 별 안에 접힌 종이 형태를 넣은 단일 중심 마크, 남청색 바탕과 따뜻한 노랑 포인트, 문구·UI·기기 프레임 없음.",
        brief_markers: &[
            "앱 아이콘",
            "app icon",
            "application icon",
            "상징",
            "은유",
            "symbol",
            "metaphor",
            "제품 목적",
            "product purpose",
        ],
    }
}

fn infer_category_contract_from_brief(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let Some(category) = answer(answers, "image.category") else {
        return;
    };
    let spec = if category.eq_ignore_ascii_case("C4") && is_app_icon_profile(answers) {
        app_icon_contract_spec()
    } else {
        let Some(spec) = category_contract_spec(category) else {
            return;
        };
        spec
    };
    if answer(answers, spec.key).is_some() {
        return;
    }

    let trimmed = brief.trim();
    if trimmed.chars().count() < 6 {
        return;
    }
    let normalized = trimmed.to_lowercase();
    let marker_count = spec
        .brief_markers
        .iter()
        .filter(|marker| normalized.contains(**marker))
        .count();
    let observable_c4_subject = category.eq_ignore_ascii_case("C4")
        && contains_subject_marker(trimmed)
        && parse_subjects(trimmed)
            .iter()
            .any(|subject| subject.count > 0);
    if marker_count < 2 && !observable_c4_subject {
        return;
    }

    answers.insert(spec.key.to_owned(), trimmed.to_owned());
    inferences.push(format!(
        "요청에서 {}개의 결과물 핵심 단서를 찾아 {} 답변으로 사용했다.",
        marker_count.max(usize::from(observable_c4_subject)),
        spec.key
    ));
}

fn infer_route_content(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let Some(category) = answer(answers, "image.category").map(str::to_ascii_uppercase) else {
        return;
    };

    if answer(answers, "image.subject").is_none() {
        let contract = category_contract_value(answers).map(|(_, value)| value.to_owned());
        let fallback = (!brief.trim().is_empty()).then(|| brief.trim().to_owned());
        if let Some(subject) = contract.or(fallback) {
            answers.insert("image.subject".to_owned(), subject);
            inferences.push(
                "결과물 전용 답변과 요청 본문을 주 피사체·핵심 내용으로 정리했다.".to_owned(),
            );
        }
    }

    if answer(answers, "image.scene").is_none() {
        let scene = if category == "C4" && is_app_icon_profile(answers) {
            "opaque neutral canvas for one isolated centered app-icon mark"
        } else {
            default_route_scene(&category)
        };
        answers.insert("image.scene".to_owned(), scene.to_owned());
        inferences.push(format!(
            "{category} 결과물의 제작 캔버스 기본값을 장면으로 적용했다."
        ));
    }
}

fn category_contract_value(answers: &BTreeMap<String, String>) -> Option<(&'static str, &str)> {
    let category = answer(answers, "image.category")?.to_ascii_uppercase();
    if category == "C4" && is_app_icon_profile(answers) {
        return answer(answers, "image.app_icon_concept").map(|value| ("앱 아이콘 계약", value));
    }
    match category.as_str() {
        "C1" => answer(answers, "image.wardrobe").map(|value| ("의상 계약", value)),
        _ => {
            let spec = category_contract_spec(&category)?;
            answer(answers, spec.key).map(|value| (spec.label, value))
        }
    }
}

fn default_route_scene(category: &str) -> &'static str {
    match category {
        "C1" => "인물의 행동·표정·의상과 환경 관계가 분리되는 자연스러운 라이프스타일 세트",
        "C4" => "제품·패키지·브랜드 적용물의 형태와 재질을 비교하는 중립 스튜디오",
        "C5" => "히어로와 카피·CTA 안전 영역이 분리되는 캠페인 캔버스",
        "C6" => "단계·수치·UI 상태를 읽기 순서대로 배치하는 정보 캔버스",
        "C10" => "캐릭터 정체성과 장면 순서가 유지되는 일러스트 보드",
        "C11" => "주인공·핵심 오브젝트·거대한 환경의 스케일이 분리되는 컨셉 아트 캔버스",
        _ => "주 피사체와 정보 구조가 분리되는 중립 제작 캔버스",
    }
}

fn missing_category_contract_question(
    answers: &BTreeMap<String, String>,
) -> Option<InterviewQuestion> {
    let category = answer(answers, "image.category")?.to_ascii_uppercase();
    if category == "C1" {
        let question = InterviewQuestion::multiline(
            "image.wardrobe",
            "의상을 색·소재·의류종·핏으로 정의해 주세요.",
            "화보에서 결과를 크게 바꾸는 의상 제품과 실루엣만 확인합니다.",
            "아이보리 오버사이즈 울 코트, 크림 터틀넥 니트, 하이웨이스트 와이드 슬랙스, 여유로운 핏.",
        );
        return match answer(answers, "image.wardrobe") {
            None => Some(question),
            Some(value) if !wardrobe_is_specific(value) => Some(question.with_error(
                "색, 소재, 의류종, 핏/재단 중 최소 3가지를 관찰 가능한 말로 지정해 주세요.",
            )),
            _ => None,
        };
    }
    let spec = if category == "C4" && is_app_icon_profile(answers) {
        app_icon_contract_spec()
    } else {
        category_contract_spec(&category)?
    };
    answer(answers, spec.key)
        .is_none()
        .then(|| InterviewQuestion::multiline(spec.key, spec.label, spec.help, spec.placeholder))
}

fn resolve_text_mode(answers: &mut BTreeMap<String, String>, inferences: &mut Vec<String>) {
    if answer(answers, "image.text_mode").is_some() {
        return;
    }
    match selected_text_requirement(answers) {
        TextRequirement::Required => {
            answers.insert("image.text_mode".to_owned(), "exact".to_owned());
            inferences.push(
                "선택한 시각 시스템은 글자 형태가 공간 구조를 결정하므로 정확 문구 모드를 적용했다."
                    .to_owned(),
            );
        }
        TextRequirement::Recommended => {}
        TextRequirement::Optional => {
            answers.insert("image.text_mode".to_owned(), "none".to_owned());
            inferences.push(
                "선택한 결과물과 스타일이 정확 문구를 요구하지 않아 문구 없음으로 설정했다."
                    .to_owned(),
            );
        }
    }
}

fn apply_text_defaults(answers: &mut BTreeMap<String, String>, inferences: &mut Vec<String>) {
    if answer(answers, "image.text_mode") != Some("exact") {
        return;
    }
    let category = answer(answers, "image.category")
        .unwrap_or("C5")
        .to_ascii_uppercase();
    let (position, style) = match category.as_str() {
        "C4" => ("middle_right", "mono"),
        "C11" => ("top_left", "condensed"),
        _ => ("top_left", "geometric"),
    };
    infer_default(
        answers,
        inferences,
        "image.text_position",
        position.to_owned(),
        "선택한 결과물의 기본 읽기 순서",
    );
    infer_default(
        answers,
        inferences,
        "image.text_style",
        style.to_owned(),
        "선택한 결과물의 기본 타이포그래피",
    );
}

fn collect_contract_questions(answers: &BTreeMap<String, String>) -> Vec<InterviewQuestion> {
    let mut questions = Vec::new();
    if let Some(value) = answer(answers, "image.text_mode")
        && !matches!(value, "none" | "exact")
    {
        questions.push(text_mode_question().with_error(
            "지원하지 않는 문구 모드입니다. 문구 없음 또는 정확 문구 입력 중 하나를 선택하세요.",
        ));
        return questions;
    }
    if is_app_icon_profile(answers) && answer(answers, "image.text_mode") == Some("exact") {
        return vec![
            InterviewQuestion::choice(
                "image.text_mode",
                "앱 아이콘은 문구 없는 마스터 콘셉트만 지원합니다.",
                "앱 아이콘 안에는 텍스트·문자·숫자·의사문자를 넣지 않습니다.",
                vec![option(
                    "none",
                    "문구 없음",
                    "읽을 수 있는 문자와 문구를 생성하지 않음",
                )],
            )
            .with_error("app_icon은 image.text_mode=none만 허용합니다."),
        ];
    }
    let text_requirement = selected_text_requirement(answers);
    if text_requirement == TextRequirement::Required
        && answer(answers, "image.text_mode") == Some("none")
    {
        questions.push(
            InterviewQuestion::choice(
                "image.text_mode",
                "선택한 시각 시스템에 정확 문구가 필요합니다.",
                "글자 형태가 마스크·터널·조각·공간 구조를 만들기 때문에 무문구 상태로는 해당 패턴을 성립시킬 수 없습니다.",
                vec![option(
                    "exact",
                    "정확 문구 입력",
                    "지정한 문구와 줄바꿈을 그대로 사용",
                )],
            )
            .with_error("정확 문구를 입력하거나 글자 의존 패턴을 제거해야 합니다."),
        );
        return questions;
    }
    if answer(answers, "image.text_mode").is_none() {
        questions.push(text_mode_question());
    }
    if answer(answers, "image.text_mode") == Some("exact") {
        push_missing(
            answers,
            "image.text",
            InterviewQuestion::multiline(
                "image.text",
                "정확히 렌더할 문구를 줄마다 입력해 주세요.",
                "한 줄에는 한 문자 체계만 사용합니다. 줄바꿈은 그대로 보존됩니다.",
                "봄의 한 잔\nSPRING CUP",
            ),
            &mut questions,
        );
    }
    questions
}

fn apply_visual_defaults(
    brief: &str,
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
) {
    let category = answer(answers, "image.category")
        .unwrap_or("C5")
        .to_ascii_uppercase();
    let language = answer(answers, "language").unwrap_or("ko").to_owned();
    let has_text = answer(answers, "image.text_mode") == Some("exact");
    let is_app_icon = is_app_icon_profile(answers);
    let composition_context = if category == "C10" {
        format!(
            "{brief}\n{}",
            answer(answers, "image.panel_plan").unwrap_or_default()
        )
    } else {
        brief.to_owned()
    };

    if category == "C1" {
        infer_default(
            answers,
            inferences,
            "image.face",
            "차분한 자신감이 드러나는 표정, 자연스러운 피부 결, 보이는 모공, 과도한 보정이 없는 균형 잡힌 이목구비".to_owned(),
            "패션 룩북의 얼굴·피부 기본값",
        );
        infer_default(
            answers,
            inferences,
            "image.hair",
            "의상 실루엣을 가리지 않는 정돈된 헤어, 잔머리 몇 가닥과 실제 모발 결이 보임"
                .to_owned(),
            "패션 룩북의 헤어 기본값",
        );
    }

    infer_default(
        answers,
        inferences,
        "image.composition",
        if answer(answers, "image.profile") == Some("travel_journal") {
            image_copy(&language, "주 장면을 중심에 두고 종이와 메모는 사진을 가리지 않는 주변 여백에 비대칭 배치", "Keep the main scene dominant; arrange paper and notes asymmetrically in surrounding margins without covering the photograph").to_owned()
        } else if is_app_icon {
            "one dominant centered mark filling roughly 80% of the square; simple 2–3 visual planes with no secondary objects".to_owned()
        } else {
            default_composition(&category, has_text, &composition_context, &language)
        },
        "선택한 결과물의 구도 문법",
    );
    infer_default(
        answers,
        inferences,
        "image.lighting",
        if is_app_icon {
            "hard_graphic".to_owned()
        } else {
            default_lighting(&category, brief)
        },
        "요청과 선택한 결과물의 조명 문법",
    );
    infer_default(
        answers,
        inferences,
        "image.surface",
        if is_app_icon {
            "soft matte artwork with restrained dimensionality and crisp component edges".to_owned()
        } else {
            default_surface(&category, &language)
        },
        "선택한 결과물의 재질 문법",
    );
    if answer(answers, "image.palette").is_none() {
        let palette = fallback_palette(&category);
        let value = palette.join(", ");
        answers.insert("image.palette".to_owned(), value.clone());
        inferences.push(format!(
            "결과물 {category}의 기본 HEX 팔레트 {value}를 적용했다."
        ));
    }
    infer_default(
        answers,
        inferences,
        "image.aspect_ratio",
        if is_app_icon {
            "1:1".to_owned()
        } else {
            default_aspect_ratio(&category, brief)
        },
        "선택한 결과물의 화면 비율 기본값",
    );
    infer_default(
        answers,
        inferences,
        "image.detail",
        "high".to_owned(),
        "최종 출력의 세부 조건을 검수할 최고 품질 기본값",
    );
    if answer(answers, "image.lut").is_none() && answer(answers, "image.medium") == Some("photo") {
        let (lut, reason) = default_lut(brief);
        answers.insert("image.lut".to_owned(), lut.to_owned());
        inferences.push(format!(
            "{reason}에 따라 사진 색감 프리셋 {lut}을 적용했다."
        ));
    }
    infer_default(
        answers,
        inferences,
        "image.exclusions",
        if is_app_icon {
            "text, letters, numerals, pseudo-writing, readable copy, UI screenshot, device mockup, rounded-square mask, app-icon frame, watermark, secondary objects".to_owned()
        } else {
            default_exclusions(has_text)
        },
        "프레임 밖 불필요 요소 배제 규칙",
    );
}

fn collect_validation_questions(answers: &BTreeMap<String, String>) -> Vec<InterviewQuestion> {
    let mut questions = Vec::new();
    if let Some(value) = answer(answers, "language")
        && !SUPPORTED_LANGUAGES.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "language",
                "출력 언어를 다시 선택해 주세요.",
                "컴파일러가 지원하는 언어 코드를 정확히 사용합니다.",
                vec![
                    option("ko", "한국어", "컴파일러 소유 제목과 규칙을 한국어로 출력"),
                    option(
                        "en",
                        "English",
                        "Render compiler-owned headings and rules in English",
                    ),
                ],
            )
            .with_error("지원하지 않는 출력 언어입니다."),
        );
    }
    if let Some(value) = answer(answers, "image.medium") {
        if !SUPPORTED_MEDIA.contains(&value) {
            questions.push(
                InterviewQuestion::choice(
                    "image.medium",
                    "시각 매체를 다시 선택해 주세요.",
                    "카메라·재질·렌더링 규칙의 기준이 되는 매체를 선택합니다.",
                    medium_options(),
                )
                .with_error("지원하지 않는 시각 매체입니다."),
            );
        } else if let Some(profile) = answer(answers, "image.profile")
            && let Some(question) = profile_medium_options(profile)
                .filter(|options| !options.iter().any(|option| option.value == value))
                .and_then(|_| profile_medium_question(profile, value))
        {
            questions.push(question);
        }
    }
    if let Some(value) = answer(answers, "image.aspect_ratio")
        && aspect_ratio_preset(value).is_none()
    {
        questions.push(
            InterviewQuestion::choice(
                "image.aspect_ratio",
                "지원하는 화면 비율을 다시 선택해 주세요.",
                "입력한 비율을 다른 비율로 바꾸지 않고, 지원 범위 안에서 정확히 적용합니다.",
                aspect_ratio_options(),
            )
            .with_error("지원하지 않는 화면 비율입니다."),
        );
    }
    if is_app_icon_profile(answers)
        && let Some(value) = answer(answers, "image.aspect_ratio")
        && value != "1:1"
    {
        questions.push(
            InterviewQuestion::choice(
                "image.aspect_ratio",
                "앱 아이콘은 1:1 정사각형만 지원합니다.",
                "1024×1024 PNG 마스터 콘셉트 계약을 위해 다른 비율을 사용하지 않습니다.",
                vec![option("1:1", "1:1", "정사각형 1024×1024")],
            )
            .with_error("app_icon은 image.aspect_ratio=1:1만 허용합니다."),
        );
    }
    if let Some(value) = answer(answers, "image.lighting")
        && !SUPPORTED_LIGHTING.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.lighting",
                "조명 프로필을 다시 선택해 주세요.",
                "지원하는 조명 프로필 중 하나를 정확히 적용합니다.",
                lighting_options(),
            )
            .with_error("지원하지 않는 조명 프로필입니다."),
        );
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && let Some(value) = answer(answers, "image.text_style")
        && !SUPPORTED_TEXT_STYLES.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.text_style",
                "타이포그래피 스타일을 다시 선택해 주세요.",
                "지원하는 자형 계열 중 하나를 정확히 적용합니다.",
                text_style_options(),
            )
            .with_error("지원하지 않는 타이포그래피 스타일입니다."),
        );
    }
    if answer(answers, "image.text_mode") == Some("exact")
        && let Some(value) = answer(answers, "image.text_position")
        && !SUPPORTED_TEXT_POSITIONS.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.text_position",
                "문구 위치를 다시 선택해 주세요.",
                "지원하는 9개 캔버스 영역 중 하나를 정확히 적용합니다.",
                text_position_options(),
            )
            .with_error("지원하지 않는 문구 위치입니다."),
        );
    }
    if let Some(value) = answer(answers, "image.palette")
        && parse_hex_palette(value).is_none()
    {
        questions.push(
            InterviewQuestion::text(
                "image.palette",
                "HEX 팔레트를 3–5개 입력해 주세요.",
                "쉼표로 구분하며 첫 색부터 면적 우선순위가 높습니다.",
                "#F3F0E8, #6B655F, #5B351F, #A87945",
            )
            .with_error("#RRGGBB 형식의 서로 다른 색 3–5개가 필요합니다."),
        );
    }
    if answer(answers, "image.medium") == Some("photo")
        && let Some(value) = answer(answers, "image.lut")
        && value != "none"
        && !PRESET_NAMES.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.lut",
                "사진 색감 프리셋을 다시 선택해 주세요.",
                "현재 지원하는 8개 사진 색감 중 하나를 적용합니다. 그래픽·일러스트·3D에는 적용하지 않습니다.",
                lut_options(),
            )
            .optional()
            .with_error("지원하지 않는 사진 색감 프리셋입니다."),
        );
    }
    if let Some(value) = answer(answers, "image.detail")
        && !SUPPORTED_DETAIL.contains(&value)
    {
        questions.push(
            InterviewQuestion::choice(
                "image.detail",
                "출력 품질을 다시 선택해 주세요.",
                "지원하는 codex-subscription 품질 값 중 하나를 사용합니다.",
                detail_options(),
            )
            .with_error("지원하지 않는 출력 품질입니다."),
        );
    }
    questions
}

fn infer_default(
    answers: &mut BTreeMap<String, String>,
    inferences: &mut Vec<String>,
    key: &str,
    value: String,
    source: &str,
) {
    if answer(answers, key).is_none() {
        answers.insert(key.to_owned(), value.clone());
        inferences.push(format!("{source}에 따라 {key}={value}를 채웠다."));
    }
}

fn build_request_json(request: &InterviewRequest, answers: &BTreeMap<String, String>) -> JsonValue {
    let category = answer(answers, "image.category")
        .unwrap_or("C5")
        .to_ascii_uppercase();
    let language = answer(answers, "language").unwrap_or("ko");
    let raw_profile = answer(answers, "image.profile").unwrap_or("standard");
    let profile = raw_profile;
    let is_app_icon = profile == "app_icon";
    let medium = answer(answers, "image.medium").unwrap_or(if is_app_icon {
        "graphic_design"
    } else {
        "photo"
    });
    let text_mode = answer(answers, "image.text_mode").unwrap_or("none");
    let text_lines = if text_mode == "exact" {
        split_nonempty(answer(answers, "image.text").unwrap_or_default())
    } else {
        Vec::new()
    };
    let has_text = !text_lines.is_empty();
    let use_case = use_case_without_exact_copy(request.brief.trim(), &text_lines);
    let safety_tier = selected_safety_tier(&category, has_text);
    let subject_description = answer(answers, "image.subject")
        .unwrap_or_default()
        .to_owned();
    let panel_plan = (category == "C10")
        .then(|| answer(answers, "image.panel_plan"))
        .flatten();
    let repeated_count = panel_plan.and_then(repeated_panel_count);
    let parsed_subjects = if let Some(count) = repeated_count {
        vec![repeated_panel_identity(&subject_description, count)]
    } else {
        parse_subjects(&subject_description)
    };
    let repeated_action = repeated_count.zip(panel_plan).map(|(count, plan)| {
        if language == "en" {
            format!("Ordered panel/sticker action map, instances 1 through {count}: {plan}")
        } else {
            format!("1번부터 {count}번까지 읽기 순서를 고정한 패널·스티커 행동 맵: {plan}")
        }
    });
    let relationship = subject_relationship(&subject_description);
    let mut required_elements = vec![JsonValue::from(image_copy(
        language,
        "각 typed subject의 count와 placement를 정확히 유지",
        "Preserve every typed subject count and placement exactly",
    ))];
    if is_app_icon {
        required_elements.extend([
            JsonValue::from(image_copy(
                language,
                "제품 목적에서 도출한 하나의 핵심 은유와 컴포넌트 기하",
                "One core metaphor and component geometry derived from the product purpose",
            )),
            JsonValue::from(image_copy(
                language,
                "단순한 2–3개 시각 평면과 32px 실루엣 판독성",
                "Simple 2–3 visual planes and a silhouette readable at 32px",
            )),
        ]);
    }
    if category == "C10" {
        if repeated_count.is_some() {
            required_elements.push(JsonValue::from(image_copy(
                language,
                "패널·스티커 개수와 subject action의 읽기 순서를 정확히 유지",
                "Preserve the panel/sticker count and typed subject action order exactly",
            )));
        } else if let Some((_, value)) = category_contract_value(answers)
            && value.trim() != subject_description.trim()
            && !request.brief.contains(value.trim())
        {
            required_elements.push(JsonValue::from(if language == "en" {
                format!("Panel plan: {value}")
            } else {
                format!("패널 계획: {value}")
            }));
        }
    } else if category != "C1"
        && let Some((label, value)) = category_contract_value(answers)
        && value.trim() != subject_description.trim()
        && !request.brief.contains(value.trim())
    {
        required_elements.push(JsonValue::from(format!("{label}: {value}")));
    }
    let mut hierarchy = parsed_subjects
        .iter()
        .map(|subject| JsonValue::from(subject.id.clone()))
        .collect::<Vec<_>>();
    let text_elements = if !has_text {
        JsonValue::array([])
    } else {
        hierarchy.push(JsonValue::from("headline"));
        JsonValue::array([build_text_element(answers, text_lines)])
    };
    let (width, height) = dimensions(answer(answers, "image.aspect_ratio").unwrap_or("1:1"));
    let palette = parse_hex_palette(answer(answers, "image.palette").unwrap_or_default())
        .unwrap_or_else(|| {
            vec![
                "#F2F2EF".to_owned(),
                "#6B655F".to_owned(),
                "#1A1A1A".to_owned(),
            ]
        });
    let proportions = proportions(palette.len());
    let palette_json =
        palette
            .into_iter()
            .zip(proportions)
            .enumerate()
            .map(|(index, (hex, proportion))| {
                JsonValue::object([
                    ("hex", JsonValue::from(hex)),
                    ("proportion_percent", JsonValue::from(u64::from(proportion))),
                    (
                        "usage",
                        JsonValue::from(format!("시각 면적 우선순위 {}의 색", index + 1)),
                    ),
                ])
            });
    let lighting = if repeated_count.is_some() {
        flat_illustration_lighting_json(language)
    } else {
        lighting_json(
            answer(answers, "image.lighting").unwrap_or("soft_daylight"),
            answer(answers, "image.lighting_detail"),
        )
    };
    let lut = if medium == "photo" {
        answer(answers, "image.lut")
            .filter(|value| *value != "none")
            .map(|value| {
                JsonValue::object([
                    ("preset", JsonValue::from(value)),
                    ("strength_percent", JsonValue::from(70_u64)),
                ])
            })
    } else {
        None
    };
    let mut color_fields = vec![
        (
            "contrast",
            JsonValue::from("주 피사체와 배경 사이 명도 차이를 20% 이상 유지"),
        ),
        (
            "harmony",
            JsonValue::from("지정 HEX 팔레트만 사용하고 첫 색부터 면적 우선순위를 유지"),
        ),
        ("palette", JsonValue::array(palette_json)),
        (
            "saturation",
            JsonValue::from("전체 채도는 재질 식별이 가능한 중간 범위로 제한"),
        ),
    ];
    if let Some(lut) = lut {
        color_fields.push(("photo_lut", lut));
    }
    let render_profile = "structured";
    let viewpoint = if is_app_icon {
        image_copy(
            language,
            "캔버스 면을 정면으로 보는 직교 투영",
            "Front-facing orthographic projection of the square canvas",
        )
    } else if repeated_count.is_some() {
        image_copy(
            language,
            "캔버스 면을 정면으로 보는 직교 투영",
            "Front-facing orthographic projection of the canvas plane",
        )
    } else {
        "주 피사체의 전면과 측면 특징이 동시에 식별되는 관찰 시점"
    };
    let camera_angle = if is_app_icon {
        image_copy(
            language,
            "캔버스와 평행한 정면, 기울기·원근 왜곡 없음",
            "Straight-on and parallel to the canvas, with no tilt or perspective distortion",
        )
    } else if repeated_count.is_some() {
        image_copy(
            language,
            "캔버스 면과 평행한 정면, 기울기와 원근 왜곡 없음",
            "Straight-on and parallel to the canvas, with no tilt or perspective distortion",
        )
    } else {
        "주 피사체의 형태를 판독할 수 있는 수평 기준 10도 이내의 3/4 각도"
    };
    let depth_layers = if is_app_icon {
        vec![
            JsonValue::from(image_copy(
                language,
                "불투명 배경 평면",
                "Opaque background plane",
            )),
            JsonValue::from(image_copy(
                language,
                "핵심 은유의 단일 색면·기하",
                "Core metaphor color plane and geometry",
            )),
            JsonValue::from(image_copy(
                language,
                "절제된 깊이를 위한 선택적 보조 평면",
                "Optional restrained secondary plane for depth",
            )),
        ]
    } else if repeated_count.is_some() {
        vec![JsonValue::from(image_copy(
            language,
            "단일 평면 시트 안에서 배경, 다이컷 테두리, 캐릭터 색면만 분리",
            "Separate only the background, die-cut border, and character color planes on one flat sheet",
        ))]
    } else {
        vec![
            JsonValue::from("전경은 주 피사체의 접지면과 가장 가까운 재질"),
            JsonValue::from("중경은 주 피사체와 핵심 행동"),
            JsonValue::from("후경은 지정 장면과 분리된 배경 면"),
        ]
    };
    let negative_space = if is_app_icon || repeated_count.is_some() {
        Vec::new()
    } else {
        vec![JsonValue::from(if has_text {
            answer(answers, "image.text_position").unwrap_or("top_left")
        } else {
            "top_center"
        })]
    };
    let subject_total = parsed_subjects.len();
    let subject_json = parsed_subjects
        .iter()
        .enumerate()
        .map(|(index, subject)| {
            let is_person = contains_any(
                &subject.description.to_lowercase(),
                &["인물", "사람", "모델", "person", "model"],
            );
            let repeated = subject.count > 1;
            let mut distinguishing_features = if repeated_count.is_some() {
                repeated_identity_anchors(&subject.description)
                    .into_iter()
                    .map(JsonValue::from)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            if repeated {
                distinguishing_features.push(JsonValue::from(image_copy(
                    language,
                    "같은 subject id의 모든 instance에서 얼굴 기하·복장·소품·선 굵기·팔레트 역할을 일관되게 유지",
                    "Keep face geometry, costume, props, line weight, and palette roles identical across every instance of this subject id",
                )));
            }
            let appearance = if category == "C1" {
                Some(
                    answer(answers, "image.wardrobe")
                        .or_else(|| answer(answers, "image.surface"))
                        .unwrap_or_default()
                        .to_owned(),
                )
            } else {
                subject.appearance.clone()
            };
            JsonValue::object([
                (
                    "action",
                    JsonValue::from(
                        repeated_action
                            .as_deref()
                            .or(relationship.as_deref())
                            .unwrap_or("정지 상태"),
                    ),
                ),
                ("count", JsonValue::from(u64::from(subject.count))),
                ("description", JsonValue::from(subject.description.clone())),
                (
                    "distinguishing_features",
                    JsonValue::array(distinguishing_features),
                ),
                (
                    "face",
                    if index == 0 {
                        answer(answers, "image.face").map_or(JsonValue::Null, JsonValue::from)
                    } else {
                        JsonValue::Null
                    },
                ),
                (
                    "gaze",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "각 셀의 감정과 행동에 맞는 시선만 바꾸고 얼굴 기하는 유지",
                            "Vary only gaze for each cell's emotion and action; preserve face geometry",
                        )
                    } else if is_person {
                        "요청에 명시된 시선 방향을 유지"
                    } else {
                        "시선 해당 없음; 식별 가능한 대표 면을 카메라에 노출"
                    }),
                ),
                (
                    "hair",
                    if index == 0 {
                        answer(answers, "image.hair").map_or(JsonValue::Null, JsonValue::from)
                    } else {
                        JsonValue::Null
                    },
                ),
                ("id", JsonValue::from(subject.id.clone())),
                (
                    "placement",
                    if is_app_icon {
                        JsonValue::object([("zone", JsonValue::from("center"))])
                    } else if repeated_count.is_some() {
                        repeated_panel_placement()
                    } else {
                        subject_placement(subject_total)
                    },
                ),
                (
                    "pose",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "각 셀에 전신 instance 한 개만 배치하고 셀 경계와 서로 겹치지 않음",
                            "Place exactly one full-body instance in each cell without overlap across subjects or cell boundaries",
                        )
                    } else if is_app_icon {
                        image_copy(
                            language,
                            "32px 축소에서도 하나의 중심 실루엣이 즉시 판독됨",
                            "One core silhouette remains immediately readable when reduced to 32px",
                        )
                    } else if repeated {
                        "각 instance의 실루엣이 서로 겹치지 않고 개별 식별됨"
                    } else {
                        "주요 실루엣과 기능 면이 가려지지 않음"
                    }),
                ),
                (
                    "scale",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "모든 셀의 instance가 동일한 시각 비중과 기준 크기를 유지",
                            "Keep every cell instance at the same visual weight and reference size",
                        )
                    } else if repeated {
                        "모든 instance가 동일한 시각 비중으로 개별 식별됨"
                    } else if is_app_icon {
                        image_copy(
                            language,
                            "정사각형 캔버스의 약 80%를 채우는 중앙 마크",
                            "Centered mark filling roughly 80% of the square canvas",
                        )
                    } else if subject_total > 1 {
                        "다른 typed subject와 함께 프레임 안에서 완전히 식별됨"
                    } else {
                        "주 피사체의 주요 실루엣은 캔버스 높이의 58%를 점유"
                    }),
                ),
                (
                    "appearance",
                    appearance.map_or(JsonValue::Null, JsonValue::from),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let surface_answer = answer(answers, "image.surface").unwrap_or_default();
    let (surface_material, surface_finish, surface_micro_detail, surface_light_response) =
        if repeated_count.is_some() {
            (
                image_copy(
                    language,
                    "2D 디지털 일러스트의 잉크·브러시 선과 평면 색",
                    "Ink or brush lines and flat color planes in a 2D digital illustration",
                )
                .to_owned(),
                surface_answer.to_owned(),
                image_copy(
                    language,
                    "일정한 외곽선 굵기, 선명한 면색 경계, 동일한 다이컷 테두리",
                    "Consistent outline weight, crisp color boundaries, and identical die-cut borders",
                )
                .to_owned(),
                image_copy(
                    language,
                    "사진식 반사·림라이트 없이 색면과 외곽선 대비로 형태를 분리",
                    "Separate forms through color planes and outlines, without photographic reflections or rim light",
                )
                .to_owned(),
            )
        } else {
            let (material, finish) = surface_fields(surface_answer, &category);
            (
                material,
                finish,
                "확대 시 재질을 식별할 수 있는 미세 요철과 가장자리 마감".to_owned(),
                "표면 곡률과 마감에 맞는 연속 하이라이트와 접점 그림자".to_owned(),
            )
        };
    let composition = answer(answers, "image.composition").unwrap_or_default();
    JsonValue::object([
        (
            "camera",
            if repeated_count.is_some() {
                flat_graphic_camera_json(language)
            } else {
                camera_json(medium)
            },
        ),
        ("color", JsonValue::object(color_fields)),
        (
            "composition",
            JsonValue::object([
                (
                    "balance",
                    JsonValue::from(if is_app_icon {
                        image_copy(
                            language,
                            "하나의 지배적 중심 마크와 균일한 사방 여백",
                            "One dominant centered mark with even breathing room on all sides",
                        )
                    } else if repeated_count.is_some() {
                        image_copy(
                            language,
                            "모든 셀의 크기·간격·캐릭터 비중을 동일하게 유지",
                            "Keep cell size, spacing, and character scale uniform across the sheet",
                        )
                    } else {
                        composition_balance(composition)
                    }),
                ),
                ("camera_angle", JsonValue::from(camera_angle)),
                ("depth_layers", JsonValue::array(depth_layers)),
                ("framing", JsonValue::from(composition)),
                ("negative_space", JsonValue::array(negative_space)),
                ("viewpoint", JsonValue::from(viewpoint)),
                ("visual_hierarchy", JsonValue::array(hierarchy)),
            ]),
        ),
        (
            "constraints",
            JsonValue::object([
                ("adult_subjects_only", JsonValue::from(safety_tier == 2)),
                ("clean_unbranded_finish", JsonValue::from(true)),
                (
                    "excluded_elements",
                    string_array(answer(answers, "image.exclusions").unwrap_or_default()),
                ),
                (
                    "original_characters_only",
                    JsonValue::from(matches!(category.as_str(), "C1" | "C10" | "C11")),
                ),
                ("required_elements", JsonValue::array(required_elements)),
                ("safety_tier", JsonValue::from(u64::from(safety_tier))),
            ]),
        ),
        (
            "language",
            JsonValue::from(answer(answers, "language").unwrap_or("ko")),
        ),
        ("profile", JsonValue::from(profile)),
        ("lighting", lighting),
        ("medium", JsonValue::from(medium)),
        (
            "output",
            JsonValue::object([
                ("background", JsonValue::from("opaque")),
                ("format", JsonValue::from("png")),
                ("height", JsonValue::from(u64::from(height))),
                ("backend", JsonValue::from(crate::image::IMAGE_BACKEND)),
                (
                    "detail",
                    JsonValue::from(answer(answers, "image.detail").unwrap_or("high")),
                ),
                ("width", JsonValue::from(u64::from(width))),
            ]),
        ),
        ("render_profile", JsonValue::from(render_profile)),
        (
            "scene",
            JsonValue::object([
                (
                    "atmosphere",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "모든 셀에서 동일한 얼굴·복장·선 굵기·팔레트 역할을 유지",
                            "Keep face, costume, line weight, and palette roles identical in every cell",
                        )
                    } else {
                        "장면 요소는 지정된 공간 관계와 시각 밀도를 유지"
                    }),
                ),
                (
                    "background",
                    JsonValue::from(answer(answers, "image.scene").unwrap_or_default()),
                ),
                (
                    "environment",
                    JsonValue::from(if repeated_count.is_some() {
                        image_copy(
                            language,
                            "정면 직교 투영의 균일한 패널·스티커 일러스트 시트",
                            "Uniform front-facing orthographic panel or sticker illustration sheet",
                        )
                    } else {
                        answer(answers, "image.scene").unwrap_or_default()
                    }),
                ),
            ]),
        ),
        ("subjects", JsonValue::array(subject_json)),
        (
            "surfaces",
            JsonValue::array([JsonValue::object([
                ("finish", JsonValue::from(surface_finish)),
                ("id", JsonValue::from("primary_surface")),
                ("light_response", JsonValue::from(surface_light_response)),
                ("material", JsonValue::from(surface_material)),
                ("micro_detail", JsonValue::from(surface_micro_detail)),
            ])]),
        ),
        (
            "taxonomy",
            JsonValue::object([("category", JsonValue::from(category))]),
        ),
        ("text_elements", text_elements),
        ("use_case", JsonValue::from(use_case)),
    ])
}

fn use_case_without_exact_copy(brief: &str, lines: &[String]) -> String {
    let copy_label = if brief.chars().any(is_hangul) {
        "지정 문구"
    } else {
        "the supplied copy"
    };
    let mut value = brief.to_owned();
    for line in lines {
        for (open, close) in [
            ("\"", "\""),
            ("“", "”"),
            ("‘", "’"),
            ("「", "」"),
            ("『", "』"),
        ] {
            value = value.replace(&format!("{open}{line}{close}"), copy_label);
        }
    }
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn image_copy<'a>(language: &str, korean: &'a str, english: &'a str) -> &'a str {
    if language == "en" { english } else { korean }
}

fn subject_placement(subject_total: usize) -> JsonValue {
    let (x, y, width, height) = if subject_total > 1 {
        (10_u64, 20_u64, 80_u64, 70_u64)
    } else {
        (19_u64, 24_u64, 62_u64, 62_u64)
    };
    JsonValue::object([
        ("height_percent", JsonValue::from(height)),
        ("width_percent", JsonValue::from(width)),
        ("x_percent", JsonValue::from(x)),
        ("y_percent", JsonValue::from(y)),
        ("zone", JsonValue::from("custom")),
    ])
}

fn repeated_panel_placement() -> JsonValue {
    JsonValue::object([
        ("height_percent", JsonValue::from(90_u64)),
        ("width_percent", JsonValue::from(90_u64)),
        ("x_percent", JsonValue::from(5_u64)),
        ("y_percent", JsonValue::from(5_u64)),
        ("zone", JsonValue::from("custom")),
    ])
}

fn composition_balance(_framing: &str) -> &'static str {
    "프레이밍에 명시된 피사체 위치와 여백 관계를 바꾸지 않고 시각 무게를 안정적으로 유지"
}

fn build_text_element(answers: &BTreeMap<String, String>, lines: Vec<String>) -> JsonValue {
    let position = answer(answers, "image.text_position").unwrap_or("top_left");
    let style = answer(answers, "image.text_style").unwrap_or("geometric");
    let (font, weight, treatment) = match style {
        "condensed" => (
            "condensed sans-serif with tall narrow letterforms",
            "black",
            "자형 골격을 유지한 수직 압축",
        ),
        "didone" => (
            "high-contrast didone serif",
            "bold",
            "얇은 세리프와 굵은 세로획의 대비",
        ),
        "mono" => (
            "monospace with typewriter character",
            "medium",
            "균일 폭과 직선 기준선",
        ),
        "brush" => (
            "controlled Korean brush calligraphy",
            "bold",
            "붓 가장자리는 유지하되 자모 골격은 또렷하게",
        ),
        _ => (
            "clean geometric sans-serif",
            "bold",
            "기하학적 획과 균일한 기준선",
        ),
    };
    JsonValue::object([
        (
            "alignment",
            JsonValue::from(if position.ends_with("right") {
                "right aligned"
            } else if position.contains("center") {
                "center aligned"
            } else {
                "left aligned"
            }),
        ),
        ("color_hex", JsonValue::from("#F5F5F2")),
        ("font_family", JsonValue::from(font)),
        ("id", JsonValue::from("headline")),
        (
            "language",
            JsonValue::from(if lines.iter().any(|line| line.chars().any(is_hangul)) {
                "ko"
            } else {
                "en"
            }),
        ),
        (
            "letter_spacing",
            JsonValue::from("글자 높이의 1% 이내로 통제된 자간"),
        ),
        (
            "lines",
            JsonValue::array(lines.into_iter().map(JsonValue::from)),
        ),
        (
            "placement",
            JsonValue::object([("zone", JsonValue::from(position))]),
        ),
        ("role", JsonValue::from("headline")),
        ("size_percent", JsonValue::from(12_u64)),
        ("treatment", JsonValue::from(treatment)),
        ("weight", JsonValue::from(weight)),
    ])
}

fn camera_json(medium: &str) -> JsonValue {
    let fields = match medium {
        "photo" => [
            (
                "depth_of_field",
                "주 피사체 전체는 선명하고 후경은 윤곽만 식별되는 완만한 초점 이탈",
            ),
            (
                "field_of_view",
                "표준 시야, 직선 가장자리에 배럴 왜곡이 보이지 않음",
            ),
            (
                "focus",
                "주 피사체의 전면 가장자리와 핵심 재질 경계에 초점면 고정",
            ),
            (
                "motion_rendering",
                "정지 촬영, 명시된 움직임만 하나의 연속 궤적으로 표현",
            ),
            (
                "perspective",
                "전경이 과장되지 않는 자연 원근과 균형 잡힌 평면 압축",
            ),
        ],
        _ => [
            (
                "depth_of_field",
                "앞·중·뒤 레이어의 겹침이 명확하고 주 피사체 경계는 선명",
            ),
            (
                "field_of_view",
                "장면 전체 배치를 읽을 수 있는 균형 잡힌 시야",
            ),
            ("focus", "주 피사체의 외곽선과 핵심 정보에 시각 초점 고정"),
            (
                "motion_rendering",
                "명시된 움직임만 방향성 있는 형태 반복으로 표현",
            ),
            (
                "perspective",
                "주 피사체와 배경의 상대 크기를 유지하는 통제된 원근",
            ),
        ],
    };
    JsonValue::object(
        fields
            .into_iter()
            .map(|(key, value)| (key, JsonValue::from(value))),
    )
}

fn flat_graphic_camera_json(language: &str) -> JsonValue {
    JsonValue::object([
        (
            "depth_of_field",
            JsonValue::from(image_copy(
                language,
                "단일 평면 그래픽으로 모든 셀과 외곽선을 같은 선명도로 유지",
                "Keep every cell and outline equally sharp in a single flat graphic plane",
            )),
        ),
        (
            "field_of_view",
            JsonValue::from(image_copy(
                language,
                "패널·스티커 시트 전체가 잘리지 않고 보이는 정면 시야",
                "Straight-on view showing the entire panel or sticker sheet without cropping",
            )),
        ),
        (
            "focus",
            JsonValue::from(image_copy(
                language,
                "모든 셀의 얼굴, 행동, 다이컷 외곽선에 동일한 시각 선명도",
                "Give every face, action, and die-cut outline equal visual sharpness",
            )),
        ),
        (
            "motion_rendering",
            JsonValue::from(image_copy(
                language,
                "각 셀에 지정된 한 가지 행동만 또렷한 실루엣으로 표현",
                "Render only the assigned action in each cell as one clear silhouette",
            )),
        ),
        (
            "perspective",
            JsonValue::from(image_copy(
                language,
                "정면 직교 투영, 원근 축소와 렌즈 왜곡 없음",
                "Front-facing orthographic projection with no perspective scaling or lens distortion",
            )),
        ),
    ])
}

fn flat_illustration_lighting_json(language: &str) -> JsonValue {
    JsonValue::object([
        (
            "exposure",
            JsonValue::from(image_copy(
                language,
                "모든 셀의 지정 팔레트와 외곽선 대비를 균일하게 유지",
                "Keep the specified palette and outline contrast uniform across every cell",
            )),
        ),
        (
            "fill_description",
            JsonValue::from(image_copy(
                language,
                "별도 사진식 필라이트 없이 평면 색을 그대로 유지",
                "Preserve flat colors without photographic fill lighting",
            )),
        ),
        (
            "key_direction",
            JsonValue::from(image_copy(
                language,
                "캔버스 정면 전체에 균일한 그래픽 명도",
                "Uniform graphic luminance across the full front of the canvas",
            )),
        ),
        (
            "key_quality",
            JsonValue::from(image_copy(
                language,
                "광원 방향보다 평면 채색과 일관된 선 굵기를 우선",
                "Prioritize flat color and consistent line weight over directional lighting",
            )),
        ),
        (
            "shadow_character",
            JsonValue::from(image_copy(
                language,
                "입체 캐스트 그림자와 림라이트 없이 필요한 접지 그림자만 단순화",
                "Use only simplified contact shadows where needed; no volumetric cast shadows or rim light",
            )),
        ),
    ])
}

fn lighting_json(value: &str, detail: Option<&str>) -> JsonValue {
    let (direction, quality, kelvin, color, ratio, fill, rim, shadow, exposure) = match value {
        "hard_graphic" => (
            "화면 왼쪽 9시 방향",
            "경계 전이가 4mm 이하인 하드 키 라이트",
            5600_u64,
            "#F3EEE2",
            5.0,
            "카메라 오른쪽의 약한 중성 반사광",
            "후면 가장자리에 폭 2mm의 차가운 림",
            "오른쪽 아래로 길게 이어지는 모서리가 선명한 캐스트 그림자",
            "밝은 면 세부와 짙은 그림자 내부의 재질 경계를 함께 보존",
        ),
        "low_key" => (
            "화면 왼쪽 상단 10시 방향",
            "폭이 좁은 키 라이트와 제한된 확산",
            4300,
            "#D8B18A",
            6.0,
            "카메라 축의 매우 약한 중성 필",
            "후면 오른쪽의 얇은 청회색 림",
            "배경에 깊은 그림자 풀을 만들되 피사체 외곽은 분리",
            "하이라이트 클리핑 없이 중간톤 대비를 유지",
        ),
        "golden_hour" => (
            "화면 오른쪽 낮은 3시 방향",
            "낮은 각도의 따뜻한 측광과 부드러운 가장자리",
            4200,
            "#F2B77B",
            3.0,
            "하늘 방향의 차가운 확산 필",
            "피사체 반대편 외곽의 약한 골드 림",
            "화면 왼쪽으로 길게 이어지는 부드러운 그림자",
            "따뜻한 하이라이트의 층과 중성 피부·재질 색을 함께 보존",
        ),
        "neon_practical" => (
            "화면 왼쪽의 청록 프랙티컬과 오른쪽 후면의 마젠타 림",
            "두 색 광원이 피사체 면에서 겹치지 않도록 분리",
            4800,
            "#36D7D1",
            2.0,
            "카메라 축의 낮은 강도 중성 필",
            "오른쪽 후면 #E05AA8 림",
            "바닥 접점은 짙게 유지하고 반사면에 두 색의 길쭉한 반사",
            "네온 색은 포화되지만 표면 재질과 어두운 영역의 층을 보존",
        ),
        _ => (
            "화면 왼쪽 상단 10시 방향",
            "폭이 넓고 가장자리 전이가 25mm 이상인 확산 키 라이트",
            5200,
            "#F3D9B5",
            3.0,
            "카메라 오른쪽의 중성 반사광이 그림자 내부 재질을 유지",
            "후면 오른쪽에서 외곽에 폭 2mm의 따뜻한 림",
            "접점에서 시작해 오른쪽으로 이어지는 부드러운 그림자",
            "밝은 면의 세부와 어두운 재질의 색층을 함께 보존",
        ),
    };
    let direction = detail
        .and_then(lighting_detail_direction)
        .unwrap_or_else(|| direction.to_owned());
    let quality = detail
        .and_then(lighting_detail_quality)
        .unwrap_or_else(|| quality.to_owned());
    JsonValue::object([
        ("exposure", JsonValue::from(exposure)),
        ("fill_description", JsonValue::from(fill)),
        ("key_color_hex", JsonValue::from(color)),
        ("key_direction", JsonValue::from(direction)),
        ("key_quality", JsonValue::from(quality)),
        ("key_temperature_kelvin", JsonValue::from(kelvin)),
        ("key_to_fill_ratio", JsonValue::Number(format!("{ratio}"))),
        ("rim_description", JsonValue::from(rim)),
        ("shadow_character", JsonValue::from(shadow)),
    ])
}

fn lighting_detail_direction(detail: &str) -> Option<String> {
    let lower = detail.to_lowercase();
    let direction = if contains_any(
        &lower,
        &["왼쪽 위", "왼쪽 상단", "좌상", "top left", "upper left"],
    ) {
        "화면 왼쪽 상단"
    } else if contains_any(
        &lower,
        &[
            "오른쪽 위",
            "오른쪽 상단",
            "우상",
            "top right",
            "upper right",
        ],
    ) {
        "화면 오른쪽 상단"
    } else if contains_any(&lower, &["왼쪽", "좌측", "from left", "left side"]) {
        "화면 왼쪽"
    } else if contains_any(&lower, &["오른쪽", "우측", "from right", "right side"]) {
        "화면 오른쪽"
    } else if contains_any(&lower, &["상단", "위에서", "from above", "top down"]) {
        "화면 상단"
    } else {
        return None;
    };
    let source = if contains_any(&lower, &["창가", "창문", "window"]) {
        "창가광"
    } else {
        "키 라이트"
    };
    Some(format!("{direction}에서 들어오는 {source}"))
}

fn lighting_detail_quality(detail: &str) -> Option<String> {
    let lower = detail.to_lowercase();
    let window = contains_any(&lower, &["창가", "창문", "window"]);
    let soft = contains_any(&lower, &["부드러운", "확산", "soft", "diffused", "diffuse"]);
    match (window, soft) {
        (true, true) => Some("부드러운 창가 확산광, 넓고 완만한 명암 전이".to_owned()),
        (true, false) => Some("창을 통과한 자연광, 창의 크기에 맞는 명암 전이".to_owned()),
        (false, true) => Some("부드러운 확산광, 넓고 완만한 명암 전이".to_owned()),
        (false, false) => None,
    }
}

#[cfg(test)]
#[path = "image/tests.rs"]
mod tests;
