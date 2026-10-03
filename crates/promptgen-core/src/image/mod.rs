pub mod controls;
mod lut;
mod model;
mod render;
mod schema;
mod specificity;
mod validate;

pub use lut::{PRESET_NAMES, ResolvedPhotoLut, available_lut_presets, resolve_photo_lut};
pub use model::{
    BackgroundMode, CameraSpec, CanvasPlacement, CanvasZone, CinematicStoryboardPanel,
    CinematicStoryboardSpec, ColorSpec, CompositionSpec, ImageChangeContract,
    ImageConsistencyDimension, ImageConsistencySequence, ImageConsistencySpec, ImageConstraints,
    ImageDeliveryContract, ImageDetail, ImageProfile, ImagePromptRequest, ImageReference,
    ImageReferenceRole, ImageRenderProfile, ImageTaskMode, LightingSpec, PaletteEntry,
    PhotoLutSelection, SceneSpec, SubjectSpec, SurfaceSpec, TaxonomySelection, TextElement,
    TextRole, VisualMedium,
};
pub use schema::image_schema;
pub use validate::validate_image_request;

use crate::catalog::CatalogEntry;
use crate::diagnostic::{CompilationOutcome, Diagnostic, PromptKind};
use crate::json::JsonValue;

pub use controls::VisualControl;
pub const IMAGE_BACKEND: &str = "codex-subscription";

pub const MAX_RENDERED_PROMPT_CHARS: usize = 16_000;

pub fn compile_image_prompt(request: &ImagePromptRequest) -> CompilationOutcome {
    let (mut diagnostics, selected) = validate_image_request(request);
    let prompt = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::diagnostic::Severity::Error)
    {
        None
    } else {
        let prompt = render::render_image_prompt(request, &selected);
        let length = prompt.chars().count();
        if length > MAX_RENDERED_PROMPT_CHARS {
            diagnostics.push(
                Diagnostic::warning(
                    "IMG_PROMPT_TOO_LONG",
                    "$",
                    format!(
                        "rendered prompt is {length} characters, over the local {MAX_RENDERED_PROMPT_CHARS}-character execution budget"
                    ),
                )
                .with_hint("shorten the longest structured fields before spending a generation call"),
            );
        }
        Some(prompt)
    };
    CompilationOutcome::new(
        PromptKind::Image,
        prompt,
        diagnostics,
        JsonValue::object([
            (
                "catalog_entries",
                crate::catalog::selected_catalog_to_json(&selected),
            ),
            ("compiler", JsonValue::from("promptgen-core/3")),
            ("generation_plan", generation_plan(request, &selected)),
            ("language", JsonValue::from(request.language.code())),
            ("profile", JsonValue::from(request.profile.as_str())),
            ("request", request.to_json()),
        ]),
    )
}

fn generation_plan(request: &ImagePromptRequest, selected: &[&CatalogEntry]) -> JsonValue {
    let primary = selected
        .iter()
        .find(|entry| entry.is_primary_output())
        .map(|entry| entry.id.as_str())
        .unwrap_or("");
    let profile = request.profile.as_str();
    let pose_transfer = request.render_profile == ImageRenderProfile::PoseTransfer;
    let cinematic_storyboard = request.render_profile == ImageRenderProfile::CinematicStoryboard;
    let (axis_id, axis_label, locks) = if pose_transfer {
        (
            "pose_fidelity_gate",
            "가장 이른 실패 자세 게이트 하나만 교정",
            &[
                "캐릭터 정체성·얼굴·표정",
                "이미 통과한 자세 게이트",
                "화풍·배경·출력",
            ][..],
        )
    } else if cinematic_storyboard {
        (
            "earliest_storyboard_gate",
            "가장 이른 실패 게이트 하나만 교정",
            &[
                "캐릭터 정체성 앵커",
                "이미 통과한 패널·카메라·시선선",
                "텍스트 없음·화풍·출력",
            ][..],
        )
    } else if request.profile == ImageProfile::TravelJournal {
        (
            "journal_additions",
            "주 장면 밖의 종이·메모만 변경",
            if request.task_mode == ImageTaskMode::Edit {
                &[
                    "원본 정체성·사진 내부 구도",
                    "제공 문구·지명·날짜",
                    "읽기 순서와 여백",
                ][..]
            } else {
                &[
                    "주 장면·피사체 관계",
                    "제공 문구·지명·날짜",
                    "읽기 순서와 여백",
                ][..]
            },
        )
    } else if request.profile == ImageProfile::AppIcon {
        (
            "app_icon_master_concept",
            "핵심 은유·실루엣·32px 판독성만 변경",
            &[
                "제품 목적·핵심 은유",
                "중앙 컴포넌트 기하와 2–3개 시각 평면",
                "32px 실루엣 검사·opaque PNG master concept",
            ][..],
        )
    } else if request.profile == ImageProfile::LogoIdentity {
        (
            "brand_mark_identity",
            "마크 실루엣·간격·색 역할만 변경",
            &["형태·비율", "색·여백", "opaque PNG concept handoff"][..],
        )
    } else if matches!(
        request.profile,
        ImageProfile::AppWebUi | ImageProfile::InformationDesign
    ) {
        (
            "information_hierarchy",
            "정보 상태·읽기 순서만 변경",
            &["그리드와 위계", "컴포넌트·범례", "상태와 라벨"][..],
        )
    } else if matches!(
        request.profile,
        ImageProfile::CharacterPose | ImageProfile::PoomsaePose
    ) {
        (
            "character_pose_gate",
            "자세·관절·시선 게이트 하나만 교정",
            &["정체성·실루엣", "관절·체중", "손발·시선"][..],
        )
    } else {
        match primary {
            "C10" => (
                "panel_beat",
                "패널별 서사 비트만 변경",
                &["캐릭터 정체성", "잉킹·채색", "읽기 방향"][..],
            ),
            "C6" => (
                "information_state",
                "정보 단계·상태만 변경",
                &["그리드와 위계", "색 역할", "라벨 문법"][..],
            ),
            "C11" => (
                "world_focus",
                "세계의 초점만 변경",
                &["주인공 정체성", "환경 스케일", "화면비와 색조"][..],
            ),
            _ => (
                "single_output",
                "단일 결과물",
                &["피사체 사양", "구도·공간", "색·재질"][..],
            ),
        }
    };
    let recommended_detail = "high";
    let detail_reason = if pose_transfer {
        "전신 자세·관절 사슬·손발·방향·시선과 캐릭터 정체성을 동시에 유지"
    } else if cinematic_storyboard {
        "여러 패널의 인물 정체성·화면 방향·시선선·샷 순서를 동시에 유지"
    } else if request.profile == ImageProfile::AppIcon {
        "작은 크기에서도 하나의 핵심 은유와 중심 실루엣을 보존"
    } else if request.task_mode != ImageTaskMode::Generate {
        "참조 이미지의 정체성과 보존 불변조건을 유지"
    } else if !request.text_elements.is_empty() {
        "정확 문구의 글자 형태와 배치를 보존"
    } else if primary == "C10" {
        "여러 칸에서 정체성과 레이아웃을 유지"
    } else if primary == "C6" {
        "정보 위계와 읽기 순서를 유지"
    } else if primary == "C11" {
        "주인공과 환경 스케일의 일관성을 유지"
    } else {
        "단일 장면의 재질과 경계를 안정적으로 표현"
    };
    let text_target = if request.text_elements.is_empty() {
        "렌더 문구 없음"
    } else {
        "지정 문구·줄바꿈·위치 정확"
    };
    let qa_criteria = if pose_transfer {
        JsonValue::array([
            JsonValue::object([
                ("id", JsonValue::from("whole_pose")),
                ("label", JsonValue::from("전체 동작")),
                (
                    "target",
                    JsonValue::from("발·접지·체중·골반·몸통·전체 실루엣이 자세 원본과 일치"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("limb_detail")),
                ("label", JsonValue::from("팔다리·손발 디테일")),
                (
                    "target",
                    JsonValue::from("관절 사슬·손모양·발끝·겹침·원근 단축이 자세 원본과 일치"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("facing_gaze")),
                ("label", JsonValue::from("방향·시선")),
                (
                    "target",
                    JsonValue::from("좌우·몸 방향·머리 회전·눈동자·카메라 각도가 원본과 일치"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("character_identity")),
                ("label", JsonValue::from("캐릭터 일관성")),
                (
                    "target",
                    JsonValue::from("얼굴·표정·체형·머리·의상·화풍은 캐릭터 시트만 따름"),
                ),
            ]),
        ])
    } else if request.profile == ImageProfile::AppIcon {
        JsonValue::array([
            JsonValue::object([
                ("id", JsonValue::from("purpose_metaphor")),
                ("label", JsonValue::from("제품 목적·핵심 은유")),
                (
                    "target",
                    JsonValue::from("제품 목적이 하나의 지배적 시각 은유와 컴포넌트 기하로 읽힘"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("centered_planes")),
                ("label", JsonValue::from("중앙 구성·시각 평면")),
                (
                    "target",
                    JsonValue::from(
                        "중앙에 하나의 typed subject만 놓이고 단순한 2–3개 시각 평면으로 정리됨",
                    ),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("silhouette_32px")),
                ("label", JsonValue::from("32px 실루엣")),
                (
                    "target",
                    JsonValue::from("32px로 축소해도 문구·프레임 없이 핵심 실루엣과 대비가 판독됨"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("png_boundary")),
                ("label", JsonValue::from("PNG master concept 경계")),
                (
                    "target",
                    JsonValue::from(
                        "opaque 1024x1024 PNG 한 장만 약속하며 SVG·벡터·레이어·플랫폼 패키지는 주장하지 않음",
                    ),
                ),
            ]),
        ])
    } else if cinematic_storyboard {
        JsonValue::array([
            JsonValue::object([
                ("id", JsonValue::from("identity_anchors")),
                ("label", JsonValue::from("인물 정체성")),
                (
                    "target",
                    JsonValue::from("모든 패널에서 얼굴·머리·의상·소품·팔레트 역할이 동일"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("beat_progression")),
                ("label", JsonValue::from("스토리 비트")),
                (
                    "target",
                    JsonValue::from("패널마다 하나의 행동·감정 전환만 보이며 선언한 순서와 일치"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("screen_direction_eyeline")),
                ("label", JsonValue::from("화면 방향·시선선")),
                (
                    "target",
                    JsonValue::from("축을 넘지 않고 다음 패널의 시선 대상과 공간 관계가 연결"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("textless_handoff")),
                ("label", JsonValue::from("텍스트 없는 전달물")),
                (
                    "target",
                    JsonValue::from("패널 번호·캡션·말풍선·로고·워터마크·UI·의사문자 없음"),
                ),
            ]),
        ])
    } else {
        JsonValue::array([
            JsonValue::object([
                ("id", JsonValue::from("goal_fit")),
                ("label", JsonValue::from("목표 적합")),
                ("target", JsonValue::from("용도와 첫 시선이 브리프에 일치")),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("text_accuracy")),
                ("label", JsonValue::from("문구 정확")),
                ("target", JsonValue::from(text_target)),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("material_realism")),
                ("label", JsonValue::from("재질·매체")),
                (
                    "target",
                    JsonValue::from("선택한 매체의 표면과 빛 반응이 일관됨"),
                ),
            ]),
            JsonValue::object([
                ("id", JsonValue::from("layout")),
                ("label", JsonValue::from("레이아웃")),
                ("target", JsonValue::from("개수·위계·여백·읽기 순서 정확")),
            ]),
        ])
    };
    JsonValue::object([
        ("profile", JsonValue::from(profile)),
        ("consistency", consistency_plan(request)),
        ("task_mode", JsonValue::from(request.task_mode.as_str())),
        (
            "references",
            JsonValue::object([
                ("count", JsonValue::from(request.references.len() as u64)),
                (
                    "roles",
                    JsonValue::array(
                        request
                            .references
                            .iter()
                            .map(|reference| JsonValue::from(reference.role.as_str())),
                    ),
                ),
            ]),
        ),
        (
            "change_contract",
            JsonValue::object([
                (
                    "change_only",
                    JsonValue::array(
                        request
                            .change_contract
                            .change_only
                            .iter()
                            .cloned()
                            .map(JsonValue::from),
                    ),
                ),
                (
                    "preserve",
                    JsonValue::array(
                        request
                            .change_contract
                            .preserve
                            .iter()
                            .cloned()
                            .map(JsonValue::from),
                    ),
                ),
            ]),
        ),
        ("locks", JsonValue::strings(locks)),
        ("qa_criteria", qa_criteria),
        (
            "detail",
            JsonValue::object([
                ("current", JsonValue::from(request.output.detail.as_str())),
                ("reason", JsonValue::from(detail_reason)),
                ("recommended", JsonValue::from(recommended_detail)),
            ]),
        ),
        (
            "variant_axis",
            JsonValue::object([
                ("id", JsonValue::from(axis_id)),
                ("label", JsonValue::from(axis_label)),
                (
                    "rule",
                    JsonValue::from("비교 시 이 축 하나만 바꾸고 나머지 잠금은 유지"),
                ),
            ]),
        ),
    ])
}

fn consistency_plan(request: &ImagePromptRequest) -> JsonValue {
    let Some(consistency) = &request.consistency else {
        return JsonValue::Null;
    };
    let dimensions = JsonValue::array(
        consistency
            .dimensions
            .iter()
            .map(|dimension| JsonValue::from(dimension.as_str())),
    );
    let sequence = consistency.sequence.as_ref();
    JsonValue::object([
        ("dimensions", dimensions),
        (
            "shared_anchor_count",
            JsonValue::from(consistency.shared_anchors.len() as u64),
        ),
        (
            "sequence_step_count",
            JsonValue::from(sequence.map_or(0, |value| value.steps.len()) as u64),
        ),
        (
            "condition_on_previous",
            sequence.map_or(JsonValue::Null, |value| {
                JsonValue::from(value.condition_on_previous)
            }),
        ),
        (
            "qa_criteria",
            JsonValue::array(
                consistency
                    .dimensions
                    .iter()
                    .map(|dimension| JsonValue::object([
                        ("id", JsonValue::from(format!("consistency_{}", dimension.as_str()))),
                        ("label", JsonValue::from(dimension.as_str())),
                        (
                            "target",
                            JsonValue::from(match dimension {
                                ImageConsistencyDimension::MultiView =>
                                    "landmark geometry, scale, and camera relationship remain stable across views",
                                ImageConsistencyDimension::Character =>
                                    "face, hair, wardrobe, silhouette, and distinguishing features remain stable",
                                ImageConsistencyDimension::Temporal =>
                                    "ordered actions and state changes remain causal with no unexplained reset",
                                ImageConsistencyDimension::Semantic =>
                                    "object identity, spatial relations, layout hierarchy, and action meaning remain coherent",
                            }),
                        ),
                    ])),
            ),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn rendered_prompt_length_covers_fields_outside_the_partial_source_budget() {
        let mut request = example();
        request.constraints.required_elements = vec!["가".repeat(MAX_RENDERED_PROMPT_CHARS)];

        let outcome = compile_image_prompt(&request);

        assert!(outcome.prompt.is_some());
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "IMG_PROMPT_TOO_LONG")
        );
    }

    #[test]
    fn pose_transfer_generation_plan_exposes_ordered_visual_qa() {
        let request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-pose-transfer.json"
            )))
            .expect("JSON"),
        )
        .expect("request");

        let outcome = compile_image_prompt(&request);
        let metadata = outcome.metadata.to_pretty_string();
        assert!(outcome.prompt.is_some(), "{:#?}", outcome.diagnostics);
        for token in [
            "pose_fidelity_gate",
            "whole_pose",
            "limb_detail",
            "facing_gaze",
            "character_identity",
        ] {
            assert!(metadata.contains(token), "missing {token:?}\n{metadata}");
        }
    }

    #[test]
    fn cinematic_storyboard_generation_plan_exposes_continuity_qa() {
        let request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-cinematic-storyboard.json"
            )))
            .expect("JSON"),
        )
        .expect("request");

        let outcome = compile_image_prompt(&request);
        let metadata = outcome.metadata.to_pretty_string();
        assert!(outcome.prompt.is_some(), "{:#?}", outcome.diagnostics);
        for token in [
            "earliest_storyboard_gate",
            "\"consistency\"",
            "multi_view",
            "consistency_character",
            "identity_anchors",
            "beat_progression",
            "screen_direction_eyeline",
            "textless_handoff",
        ] {
            assert!(metadata.contains(token), "missing {token:?}\n{metadata}");
        }
    }

    #[test]
    fn app_icon_generation_plan_exposes_purpose_silhouette_and_png_boundary() {
        let request = ImagePromptRequest::from_json(
            parse(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/image-app-icon.json"
            )))
            .expect("JSON"),
        )
        .expect("app icon request");

        let outcome = compile_image_prompt(&request);
        let metadata = outcome.metadata.to_pretty_string();
        assert!(outcome.prompt.is_some(), "{:#?}", outcome.diagnostics);
        for token in [
            "app_icon_master_concept",
            "purpose_metaphor",
            "centered_planes",
            "silhouette_32px",
            "png_boundary",
            "opaque PNG master concept",
        ] {
            assert!(metadata.contains(token), "missing {token:?}\n{metadata}");
        }
    }
}
