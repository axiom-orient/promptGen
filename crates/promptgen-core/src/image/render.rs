use crate::catalog::{CatalogEntry, DirectiveSlot, text_guard_is_repetition_aware};
use crate::common::{PromptLanguage, heading, label, quote};

use super::lut::{ResolvedPhotoLut, resolve_photo_lut};
use super::model::{
    CanvasPlacement, CanvasZone, CinematicStoryboardSpec, ImageConsistencyDimension,
    ImageConsistencySpec, ImageProfile, ImagePromptRequest, ImageRenderProfile,
};

const TIER1_GUARD: &str = "All text appears once, perfectly legible — no duplicate text, no extra words, no invented glyphs, no watermark.";
const REPETITION_GUARD: &str =
    "The designated headline remains perfectly legible — no invented glyphs, no watermark.";
const TIER2_ASSERT: &str = "all depicted people are adults aged 25+, original fictional characters, non-nude fashion editorial styling, fully opaque garments, covered chest lines, composed upright editorial poses";
const TIER2_TAIL: &str = "no nudity, no nipple or genital exposure, no wardrobe malfunction, no extra people, no text, no watermark";
const TIER2_TAIL_WITH_EXACT_TEXT: &str = "no nudity, no nipple or genital exposure, no wardrobe malfunction, no extra people, no watermark";

pub(crate) fn render_image_prompt(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
) -> String {
    render_structured_prompt(request, selected)
}

fn render_structured_prompt(request: &ImagePromptRequest, selected: &[&CatalogEntry]) -> String {
    let language = request.language;
    let mut sections = Vec::new();
    sections.push(format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}",
        heading(language, "목적", "PURPOSE"),
        label(language, "용도", "Use case"),
        visual(&request.use_case),
        label(language, "작업 모드", "Task mode"),
        request.task_mode.as_str(),
        label(language, "매체", "Medium"),
        request.medium.as_str()
    ));
    if request.profile != ImageProfile::Standard {
        sections.push(render_category_profile_contract(request));
    }

    if !request.references.is_empty() {
        let references = request
            .references
            .iter()
            .map(|reference| {
                format!(
                    "- image_{}: role={}; description={}; use_only_for=[{}]",
                    reference.index,
                    reference.role.as_str(),
                    quote(&reference.description),
                    reference
                        .use_for
                        .iter()
                        .map(|value| quote(value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "{}\n{}",
            heading(language, "입력 참조 이미지", "INPUT REFERENCES"),
            references
        ));
    }

    if !request.change_contract.change_only.is_empty()
        || !request.change_contract.preserve.is_empty()
    {
        let mut contract = Vec::new();
        if !request.change_contract.change_only.is_empty() {
            contract.push(format!(
                "- {}:",
                label(language, "다음 항목만 변경", "Change only")
            ));
            contract.extend(
                request
                    .change_contract
                    .change_only
                    .iter()
                    .map(|value| format!("  - {}", visual(value))),
            );
        }
        if !request.change_contract.preserve.is_empty() {
            contract.push(format!("- {}:", label(language, "보존 조건", "Preserve")));
            contract.extend(
                request
                    .change_contract
                    .preserve
                    .iter()
                    .map(|value| format!("  - {}", visual(value))),
            );
        }
        contract.push(format!(
            "- {}",
            label(
                language,
                "명시한 변경 외에는 아무것도 바꾸지 않는다. 이후 반복에서도 이 보존 목록을 그대로 적용한다.",
                "Change nothing outside the declared edits. Reapply this complete preserve list in every later iteration."
            )
        ));
        sections.push(format!(
            "{}\n{}",
            heading(language, "변경 계약", "CHANGE CONTRACT"),
            contract.join("\n")
        ));
    }

    if request.render_profile == ImageRenderProfile::PoseTransfer {
        sections.push(render_pose_fidelity_contract(request));
    }
    if request.render_profile == ImageRenderProfile::CinematicStoryboard
        && let Some(storyboard) = &request.storyboard
    {
        sections.push(render_cinematic_storyboard_contract(storyboard, language));
    }
    if let Some(consistency) = &request.consistency {
        sections.push(render_consistency_contract(consistency, language));
    }

    if !selected.is_empty()
        && !matches!(
            request.profile,
            ImageProfile::AppIcon | ImageProfile::TravelJournal
        )
    {
        let textless_ui =
            request.profile == ImageProfile::AppWebUi && request.text_elements.is_empty();
        let mut lines = Vec::new();
        for entry in selected {
            for directive in &entry.prompt_directives {
                if textless_ui
                    && matches!(
                        directive.slot,
                        crate::catalog::DirectiveSlot::Typography
                            | crate::catalog::DirectiveSlot::Constraint
                    )
                {
                    continue;
                }
                lines.push(format!(
                    "- {}: {}",
                    directive_slot_label(language, directive.slot),
                    visual(&directive.text)
                ));
            }
        }
        sections.push(format!(
            "{}\n{}",
            heading(language, "선택한 시각 시스템", "SELECTED VISUAL SYSTEM"),
            lines.join("\n")
        ));
    }

    let mut scene = Vec::new();
    if request.scene.environment.trim() == request.scene.background.trim() {
        scene.push(format!(
            "- {}: {}",
            label(language, "환경 및 배경", "Environment and background"),
            visual(&request.scene.environment)
        ));
    } else {
        scene.extend([
            format!(
                "- {}: {}",
                label(language, "환경", "Environment"),
                visual(&request.scene.environment)
            ),
            format!(
                "- {}: {}",
                label(language, "배경", "Background"),
                visual(&request.scene.background)
            ),
        ]);
    }
    scene.push(format!(
        "- {}: {}",
        label(language, "분위기", "Atmosphere"),
        visual(&request.scene.atmosphere)
    ));
    if let Some(value) = &request.scene.time_of_day {
        scene.push(format!(
            "- {}: {}",
            label(language, "시간대", "Time of day"),
            visual(value)
        ));
    }
    if let Some(value) = &request.scene.weather {
        scene.push(format!(
            "- {}: {}",
            label(language, "날씨", "Weather"),
            visual(value)
        ));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "장면", "SCENE"),
        scene.join("\n")
    ));

    let mut subjects = Vec::new();
    for subject in &request.subjects {
        let mut fields = vec![
            format!("id={}", quote(&subject.id)),
            format!("count={}", subject.count),
            format!("description={}", quote(&subject.description)),
            format!("placement={}", placement(&subject.placement)),
            format!("scale={}", quote(&subject.scale)),
            format!("pose={}", quote(&subject.pose)),
            format!("action={}", quote(&subject.action)),
        ];
        if !subject.gaze.starts_with("시선 해당 없음")
            && !subject
                .gaze
                .to_ascii_lowercase()
                .starts_with("not applicable")
        {
            fields.push(format!("gaze={}", quote(&subject.gaze)));
        }
        if let Some(value) = &subject.face {
            fields.push(format!("face={}", quote(value)));
        }
        if let Some(value) = &subject.hair {
            fields.push(format!("hair={}", quote(value)));
        }
        if let Some(value) = &subject.appearance {
            fields.push(format!("appearance={}", quote(value)));
        }
        if !subject.distinguishing_features.is_empty() {
            fields.push(format!(
                "distinguishing_features=[{}]",
                subject
                    .distinguishing_features
                    .iter()
                    .map(|value| quote(value))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        subjects.push(format!("- {}", fields.join("; ")));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "피사체", "SUBJECTS"),
        subjects.join("\n")
    ));

    let negative_space = if request.composition.negative_space.is_empty() {
        label(language, "지정 없음", "not specified").to_owned()
    } else {
        request
            .composition
            .negative_space
            .iter()
            .map(|zone| zone.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    sections.push(format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}",
        heading(language, "구도 및 공간", "COMPOSITION AND SPACE"),
        label(language, "프레이밍", "Framing"),
        visual(&request.composition.framing),
        label(language, "시점", "Viewpoint"),
        visual(&request.composition.viewpoint),
        label(language, "카메라 각도", "Camera angle"),
        visual(&request.composition.camera_angle),
        label(language, "균형", "Balance"),
        visual(&request.composition.balance),
        label(language, "시각 계층", "Visual hierarchy"),
        request
            .composition
            .visual_hierarchy
            .iter()
            .map(|id| quote(id))
            .collect::<Vec<_>>()
            .join(" > "),
        label(language, "깊이 레이어", "Depth layers"),
        request
            .composition
            .depth_layers
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | "),
        label(language, "의도된 여백 영역", "Negative-space zones"),
        negative_space
    ));

    if let Some(text_section) = render_exact_text_section(request, selected) {
        sections.push(text_section);
    }

    if let Some(camera) = &request.camera {
        sections.push(format!(
            "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}",
            heading(language, "카메라 결과", "CAMERA OUTCOME"),
            label(language, "시야", "Field of view"),
            visual(&camera.field_of_view),
            label(language, "원근", "Perspective"),
            visual(&camera.perspective),
            label(language, "심도", "Depth of field"),
            visual(&camera.depth_of_field),
            label(language, "초점", "Focus"),
            visual(&camera.focus),
            label(language, "움직임 표현", "Motion rendering"),
            visual(&camera.motion_rendering)
        ));
    }

    let mut lighting = vec![
        format!(
            "- {}: {}",
            label(language, "키 방향", "Key direction"),
            visual(&request.lighting.key_direction)
        ),
        format!(
            "- {}: {}",
            label(language, "키 광질", "Key quality"),
            visual(&request.lighting.key_quality)
        ),
    ];
    if let Some(value) = request.lighting.key_temperature_kelvin {
        lighting.push(format!(
            "- {}: {} K",
            label(language, "키 색온도", "Key temperature"),
            value
        ));
    }
    if let Some(value) = &request.lighting.key_color_hex {
        lighting.push(format!(
            "- {}: {}",
            label(language, "키 색상", "Key color"),
            value.to_ascii_uppercase()
        ));
    }
    if let Some(value) = request.lighting.key_to_fill_ratio {
        lighting.push(format!(
            "- {}: {:.2}:1",
            label(language, "키 대 필 비율", "Key-to-fill ratio"),
            value
        ));
    }
    lighting.extend([
        format!(
            "- {}: {}",
            label(language, "필", "Fill"),
            visual(&request.lighting.fill_description)
        ),
        format!(
            "- {}: {}",
            label(language, "그림자", "Shadow character"),
            visual(&request.lighting.shadow_character)
        ),
        format!(
            "- {}: {}",
            label(language, "노출", "Exposure"),
            visual(&request.lighting.exposure)
        ),
    ]);
    if let Some(value) = &request.lighting.rim_description {
        lighting.push(format!(
            "- {}: {}",
            label(language, "림 라이트", "Rim light"),
            visual(value)
        ));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "조명", "LIGHTING"),
        lighting.join("\n")
    ));

    let palette = request
        .color
        .palette
        .iter()
        .map(|entry| {
            format!(
                "{} {}% ({})",
                entry.hex.to_ascii_uppercase(),
                entry.proportion_percent,
                visual(&entry.usage)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let mut color = vec![
        format!("- {}: {}", label(language, "팔레트", "Palette"), palette),
        format!(
            "- {}: {}",
            label(language, "조화", "Harmony"),
            visual(&request.color.harmony)
        ),
        format!(
            "- {}: {}",
            label(language, "대비", "Contrast"),
            visual(&request.color.contrast)
        ),
        format!(
            "- {}: {}",
            label(language, "채도", "Saturation"),
            visual(&request.color.saturation)
        ),
    ];
    if let Some(selection) = &request.color.photo_lut
        && let Some(resolved) = resolve_photo_lut(selection)
    {
        color.extend(render_lut(&resolved, language));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "색 및 사진 LUT", "COLOR AND PHOTO LUT"),
        color.join("\n")
    ));

    if !request.surfaces.is_empty() {
        let surfaces = request
            .surfaces
            .iter()
            .map(|surface| {
                format!(
                    "- id={}; material={}; finish={}; micro_detail={}; light_response={}",
                    quote(&surface.id),
                    quote(&surface.material),
                    quote(&surface.finish),
                    quote(&surface.micro_detail),
                    quote(&surface.light_response)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            "{}\n{}",
            heading(language, "재질 및 표면", "MATERIALS AND SURFACES"),
            surfaces
        ));
    }

    let required = if request.constraints.required_elements.is_empty() {
        label(language, "없음", "none").to_owned()
    } else {
        request
            .constraints
            .required_elements
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let omitted = if request.constraints.excluded_elements.is_empty() {
        label(language, "없음", "none").to_owned()
    } else {
        request
            .constraints
            .excluded_elements
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let mut constraints = vec![
        format!(
            "- {}: {}",
            label(language, "필수 요소", "Required elements"),
            required
        ),
        format!(
            "- {}: {}",
            label(
                language,
                "프레임 밖으로 유지할 범주",
                "Categories kept outside the frame"
            ),
            omitted
        ),
        format!(
            "- {}: {}",
            label(language, "안전 티어", "Safety tier"),
            request.constraints.safety_tier
        ),
        format!(
            "- {}: {}",
            label(language, "프레임 경계", "Frame boundary"),
            label(
                language,
                "지정한 피사체·필수 요소·표면만 화면에 존재하며, 배경은 구조화된 장면 설명을 그대로 따른다.",
                "Only the specified subjects, required elements, and surfaces occupy the frame; the background follows the structured scene description exactly."
            )
        ),
    ];
    if request.constraints.original_characters_only {
        constraints.push(format!(
            "- {}",
            label(
                language,
                "인물은 실존 인물과 무관한 가상 오리지널 캐릭터다.",
                "Every person is an original fictional character unrelated to any real person."
            )
        ));
    }
    if request.constraints.clean_unbranded_finish {
        constraints.push(format!(
            "- {}",
            label(
                language,
                "브랜드 표식이 없는 깨끗하고 무표식인 마감.",
                "Clean, unbranded, unmarked finish."
            )
        ));
    }
    if request.constraints.safety_tier == 2 {
        constraints.push(format!("- SAFETY_ASSERT: {TIER2_ASSERT}"));
        constraints.push(format!(
            "- COMPLIANCE_TAIL: {}",
            tier_two_tail(!request.text_elements.is_empty())
        ));
    }
    sections.push(format!(
        "{}\n{}",
        heading(language, "제약 및 안전", "CONSTRAINTS AND SAFETY"),
        constraints.join("\n")
    ));

    let detail_intent = match request.output.detail {
        super::model::ImageDetail::Auto => label(
            language,
            "요청의 목적과 내용에 맞는 세부 묘사",
            "Visual detail appropriate to the stated purpose and content",
        ),
        super::model::ImageDetail::Low => label(
            language,
            "정확 문구·수량·구도를 보존하며 불필요한 표면 장식을 줄임",
            "Preserve exact text, counts and composition while reducing incidental surface decoration",
        ),
        super::model::ImageDetail::Medium => label(
            language,
            "주요 재질과 윤곽을 명확하게 표현",
            "Depict the main materials and contours clearly",
        ),
        super::model::ImageDetail::High => label(
            language,
            "정확 문구·재질의 미세 표면·또렷한 윤곽을 정밀하게 표현",
            "Depict exact text, fine material surfaces and crisp contours precisely",
        ),
    };
    sections.push(format!(
        "- {}: {}",
        label(language, "디테일 의도", "Detail intent"),
        detail_intent
    ));

    sections.push(format!(
        "{}\n- backend={}\n- width={}\n- height={}\n- detail={}\n- background={}\n- format={}\n- AR {}",
        heading(language, "납품·디테일 계약", "DELIVERY CONTRACT"),
        request.output.backend,
        request.output.width,
        request.output.height,
        request.output.detail.as_str(),
        request.output.background.as_str(),
        request.output.format,
        request.output.aspect_ratio_label()
    ));

    sections.join("\n\n")
}

fn render_category_profile_contract(request: &ImagePromptRequest) -> String {
    let language = request.language;
    if request.profile == ImageProfile::AppIcon {
        return render_app_icon_profile_contract(request);
    }
    let (title_ko, title_en, directive_ko, directive_en) = match request.profile {
        ImageProfile::Standard => return String::new(),
        ImageProfile::TravelJournal => (
            "여행·다이어리",
            "TRAVEL JOURNAL",
            "중심 장면 또는 제공한 사진을 주인공으로 두고 종이·메모는 주변에 배치한다. 문구·지명·날짜는 제공한 값만 사용한다. 편집에서는 원본 사진 내부의 얼굴·옷·자세·구도·빛·색을 바꾸지 않으며, 인쇄 질감은 선언한 추가 영역에만 적용한다.",
            "Keep the main scene or supplied photograph dominant and place paper and notes around it. Use only supplied copy, places and dates. For edits, preserve faces, clothing, poses, internal composition, light and color of the base photograph; restrict print treatment to the declared added regions.",
        ),
        ImageProfile::LogoIdentity => (
            "로고 아이덴티티 프로필",
            "LOGO IDENTITY PROFILE",
            "불투명 PNG 콘셉트·브랜드 마크 방향으로만 렌더한다. 형태·비율·획·여백·색 역할과 사용 맥락을 명확히 하되, SVG·벡터·투명 최종 납품물이라고 주장하지 않는다.",
            "Render as an opaque PNG concept and brand-mark direction only. Clarify silhouette, proportion, stroke, spacing, color roles, and usage context; do not claim an SVG, vector, or transparent final asset.",
        ),
        ImageProfile::AppIcon => unreachable!("app_icon is rendered by its dedicated contract"),
        ImageProfile::AppWebUi => (
            "앱·웹 UI 프로필",
            "APP / WEB UI PROFILE",
            if request.text_elements.is_empty() {
                "앱·웹·에이전트 화면의 시각 구조, 탐색, 상태, 컴포넌트 관계와 반응형 안전 영역만 고정한다. 제목·라벨·데이터·차트용 타이포그래피 지시는 생략하고 읽을 수 있는 문자·숫자·의사문자를 만들지 않는다."
            } else {
                "앱 또는 웹 화면의 정보 구조, 상태, 그리드, 컴포넌트 경계, 읽기 순서와 반응형 안전 영역을 관찰 가능한 화면으로 고정한다."
            },
            if request.text_elements.is_empty() {
                "Lock only the visual structure, navigation, state, component relationships, and responsive safe areas of an app, web, or agent screen. Omit title, label, data, and chart typography directives; render no readable text, numerals, or pseudo-writing."
            } else {
                "Lock the app or web screen as an observable interface: information architecture, state, grid, component boundaries, reading order, and responsive safe areas."
            },
        ),
        ImageProfile::InformationDesign => (
            "정보 디자인 프로필",
            "INFORMATION DESIGN PROFILE",
            "정보 단위·위계·범례·단계·수치·연결선을 한 가지 읽기 순서로 조직하고, 장식이 데이터 의미를 가리지 않게 한다.",
            "Organize information units, hierarchy, legends, steps, values, and connectors into one reading order; decoration must not obscure data meaning.",
        ),
        ImageProfile::CharacterPose => (
            "캐릭터·자세 프로필",
            "CHARACTER / POSE PROFILE",
            "텍스트로 지정한 캐릭터 정체성, 관절 사슬, 무게 중심, 실루엣, 손·발·시선과 행동을 먼저 고정하고 과장된 해부학 오류를 금지한다.",
            "Prioritize the text-directed character identity, joint chain, weight, silhouette, hands, feet, gaze, and action; reject distorted anatomy and ambiguous contact.",
        ),
        ImageProfile::PoomsaePose => (
            "품새 자세 프로필",
            "POOMSAE POSE PROFILE",
            "품새 동작을 텍스트로 지시한 단일 인물 시안으로 렌더한다. 발 위치·체중·골반·몸통 회전·팔 각도·손 모양·시선과 도복 실루엣을 관찰 가능한 순서로 고정한다.",
            "Render a single-character, text-directed poomsae concept. Lock foot placement, weight, pelvis, torso rotation, arm angles, hand shape, gaze, and dobok silhouette in observable order.",
        ),
    };
    format!(
        "{}\n- {}: {}\n- {}: {}",
        heading(language, title_ko, title_en),
        label(language, "프로필", "Profile"),
        request.profile.as_str(),
        label(language, "구성 원칙", "Direction"),
        visual(if language == PromptLanguage::Korean {
            directive_ko
        } else {
            directive_en
        })
    )
}

fn render_app_icon_profile_contract(request: &ImagePromptRequest) -> String {
    let language = request.language;
    let subject = request.subjects.first();
    let metaphor = subject
        .map(|value| visual(&value.description))
        .unwrap_or_else(|| label(language, "단일 typed subject", "one typed subject").to_owned());
    let geometry = subject
        .map(|value| {
            format!(
                "{}; {}; {}",
                visual(&value.pose),
                visual(&value.action),
                value
                    .appearance
                    .as_deref()
                    .map(visual)
                    .unwrap_or_else(|| label(language, "간결한 형태", "simple geometry").to_owned())
            )
        })
        .unwrap_or_else(|| label(language, "간결한 중심 기하", "simple core geometry").to_owned());
    let planes = if request.composition.depth_layers.is_empty() {
        label(
            language,
            "단순한 2–3개 시각 평면",
            "simple 2–3 visual planes",
        )
        .to_owned()
    } else {
        request
            .composition
            .depth_layers
            .iter()
            .map(|value| visual(value))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}",
        heading(
            language,
            "앱 아이콘 마스터 콘셉트 계약",
            "APP ICON MASTER CONCEPT CONTRACT"
        ),
        label(language, "프로필", "Profile"),
        "app_icon",
        label(language, "제품 목적", "Product purpose"),
        visual(&request.use_case),
        label(language, "핵심 은유", "Core metaphor"),
        metaphor,
        label(language, "컴포넌트 기하", "Component geometry"),
        geometry,
        label(language, "중앙 구성", "Centered composition"),
        label(
            language,
            "하나의 지배적 중심 상징인 단일 typed subject를 정중앙에 배치하고 캔버스를 자연스럽게 채운다",
            "Place the single typed subject as one dominant symbol at the exact center and let the artwork naturally fill the square canvas",
        ),
        label(language, "시각 평면", "Visual planes"),
        planes,
        label(language, "32px 실루엣 검사", "32px silhouette check"),
        label(
            language,
            "32px로 축소해도 하나의 핵심 은유와 강한 실루엣이 즉시 판독되어야 한다",
            "At 32px, one core metaphor and a strong silhouette must remain immediately recognizable",
        ),
        label(language, "금지", "Prohibitions"),
        label(
            language,
            "텍스트·문자·숫자·의사문자·UI 스크린샷·디바이스 목업·둥근 사각형 마스크·앱 아이콘 프레임·워터마크를 렌더하지 않는다",
            "No text, letters, numerals, pseudo-writing, UI screenshot, device mockup, rounded-square mask, app-icon frame, or watermark",
        ),
        label(language, "납품 경계", "Delivery boundary"),
        label(
            language,
            "opaque 1024×1024 PNG 마스터 콘셉트 한 장만 약속한다. SVG·벡터·편집 가능한 레이어·플랫폼 패키지는 주장하지 않는다",
            "Promise one opaque 1024x1024 PNG master concept only; do not claim SVG, vector, editable layers, or a platform package",
        )
    )
}

fn render_cinematic_storyboard_contract(
    storyboard: &CinematicStoryboardSpec,
    language: PromptLanguage,
) -> String {
    let panel_lines = storyboard
        .panels
        .iter()
        .enumerate()
        .map(|(index, panel)| {
            format!(
                "- panel_{} id={}; shot_size={}; camera_angle={}; camera_move={}; action={}; body_facing={}; gaze_target={}; emotional_beat={}",
                index + 1,
                quote(&panel.id),
                panel.shot_size,
                quote(&panel.camera_angle),
                quote(&panel.camera_move),
                quote(&panel.action),
                panel.body_facing,
                panel.gaze_target,
                quote(&panel.emotional_beat),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let anchors = storyboard
        .identity_anchors
        .iter()
        .map(|anchor| quote(anchor))
        .collect::<Vec<_>>()
        .join(", ");
    let reading = match storyboard.screen_direction.as_str() {
        "left_to_right" => label(language, "좌→우", "left to right"),
        "right_to_left" => label(language, "우→좌", "right to left"),
        _ => storyboard.screen_direction.as_str(),
    };
    format!(
        "{}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n{}",
        heading(
            language,
            "텍스트 없는 시네마틱 스토리보드 계약",
            "TEXTLESS CINEMATIC STORYBOARD CONTRACT"
        ),
        label(language, "납품 형태", "Delivery"),
        label(
            language,
            "패널 테두리만 있는 하나의 완성된 스토리보드 시트; 패널 수·순서·간격은 layout과 정확히 일치",
            "one finished storyboard sheet with panel borders only; panel count, order, and gutters exactly match layout"
        ),
        label(language, "레이아웃", "Layout"),
        storyboard.layout,
        label(language, "읽기·화면 방향", "Reading and screen direction"),
        reading,
        label(language, "반복 정체성 앵커", "Recurring identity anchors"),
        anchors,
        label(language, "텍스트 금지", "Text prohibition"),
        label(
            language,
            "패널 번호·캡션·말풍선·대사·로고·워터마크·UI·의사문자를 절대 렌더하지 않는다.",
            "Never render panel numbers, captions, speech balloons, dialogue, logos, watermarks, UI, or pseudo-text."
        ),
        label(language, "연속성", "Continuity"),
        label(
            language,
            "같은 인물의 얼굴 비율·머리·의상·소품·팔레트 역할을 유지한다. 중립 샷 없이 화면 방향을 뒤집지 않으며, 다음 패널의 시선 대상은 앞 패널의 시선선과 공간적으로 연결한다.",
            "Keep the same character face proportions, hair, wardrobe, prop, and palette roles. Do not reverse screen direction without a neutral re-establishing shot; connect each following gaze target spatially to the prior eyeline."
        ),
        panel_lines,
    )
}

fn render_consistency_contract(
    consistency: &ImageConsistencySpec,
    language: PromptLanguage,
) -> String {
    let dimensions = consistency
        .dimensions
        .iter()
        .map(|dimension| dimension.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let anchors = consistency
        .shared_anchors
        .iter()
        .map(|anchor| quote(anchor))
        .collect::<Vec<_>>()
        .join(", ");
    let mut lines = vec![
        format!(
            "- {}: {}",
            label(language, "일관성 축", "Consistency dimensions"),
            dimensions
        ),
        format!(
            "- {}: {}",
            label(language, "공유 앵커", "Shared anchors"),
            anchors
        ),
    ];
    if let Some(sequence) = &consistency.sequence {
        lines.push(format!(
            "- {}: {}",
            label(language, "순서", "Sequence"),
            sequence
                .steps
                .iter()
                .enumerate()
                .map(|(index, step)| format!("{}={}", index + 1, quote(step)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        lines.push(format!(
            "- {}: {}",
            label(language, "이전 결과 조건화", "Condition on previous output"),
            if sequence.condition_on_previous {
                label(language, "예", "yes")
            } else {
                label(language, "아니오", "no")
            }
        ));
    }
    for dimension in &consistency.dimensions {
        let (ko, en) = match dimension {
            ImageConsistencyDimension::MultiView => (
                "멀티뷰: 시점이 달라도 랜드마크 위치·비율·세계 좌표와 카메라 관계를 유지한다.",
                "Multi-view: keep landmark placement, scale, world coordinates, and the declared camera relationship stable across views.",
            ),
            ImageConsistencyDimension::Character => (
                "캐릭터: 얼굴 비율·머리·의상·실루엣·구별 특징과 팔레트 역할을 모든 결과에서 유지한다.",
                "Character: keep face proportions, hair, wardrobe, silhouette, distinguishing features, and palette roles stable in every output.",
            ),
            ImageConsistencyDimension::Temporal => (
                "시간: 선언한 순서대로 행동·감정·소품 상태가 인과적으로 이어지며 상태가 설명 없이 초기화되지 않는다.",
                "Temporal: make action, emotion, and prop state progress causally in the declared order; never reset state without an observable cause.",
            ),
            ImageConsistencyDimension::Semantic => (
                "의미: 객체 정체성·공간 관계·레이아웃 위계·행동 의미를 결과 사이에서 보존한다.",
                "Semantic: preserve object identity, spatial relations, layout hierarchy, and action meaning across outputs.",
            ),
        };
        lines.push(format!("- {}", label(language, ko, en)));
    }
    format!(
        "{}\n{}",
        heading(
            language,
            "멀티 이미지 일관성 계약",
            "MULTI-IMAGE CONSISTENCY CONTRACT"
        ),
        lines.join("\n")
    )
}

fn render_pose_fidelity_contract(request: &ImagePromptRequest) -> String {
    let language = request.language;
    let pose_index = request
        .references
        .iter()
        .find(|reference| reference.role.as_str() == "pose")
        .map(|reference| reference.index)
        .unwrap_or(0);
    let identity_index = request
        .references
        .iter()
        .find(|reference| reference.role.as_str() == "subject")
        .map(|reference| reference.index)
        .unwrap_or(0);
    let lines = [
        format!(
            "- {}",
            label(
                language,
                &format!(
                    "역할을 섞지 않는다. image_{pose_index}는 전신 자세·관절 기하·체중·접지·몸통/머리 방향·시선·카메라·가림만 지배한다. image_{identity_index}는 캐릭터 정체성·얼굴·표정·체형·머리·의상·화풍만 지배한다."
                ),
                &format!(
                    "Keep reference roles disjoint. image_{pose_index} controls only full-body pose, joint geometry, weight, foot contact, torso/head facing, gaze, camera, and occlusion. image_{identity_index} controls only character identity, face, expression, proportions, hair, wardrobe, and rendering style."
                )
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "자세 설명과 자세 원본의 실제 픽셀이 충돌하면 자세 원본의 보이는 기하가 최종 권위다. 캐릭터 시트의 기본 자세로 정규화하지 않는다.",
                "If the written pose description conflicts with visible geometry in the pose reference, the pose reference pixels are final authority. Do not normalize toward the character sheet's default pose."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "좌우 반전 금지. 인물 기준 왼쪽/오른쪽, 카메라에 가까운 팔다리, 보이는 신체 측면을 그대로 유지한다.",
                "Never mirror. Preserve character-local left/right, the limbs nearest the camera, and the visible side of the body."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "정확도 순서 1: 발 위치·접지·서기 폭과 깊이·체중 중심·골반/몸통 각도·전체 실루엣을 맞춘다.",
                "Fidelity gate 1: match foot placement and contact, stance width/depth, weight center, pelvis/torso angle, and the whole-body silhouette."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "정확도 순서 2: 어깨-팔꿈치-손목-손과 골반-무릎-발목-발의 관절 사슬, 손모양, 발끝 방향, 겹침과 원근 단축을 맞춘다.",
                "Fidelity gate 2: match shoulder-elbow-wrist-hand and hip-knee-ankle-foot chains, hand shape, toe direction, overlap, and foreshortening."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "정확도 순서 3: 몸이 바라보는 방향·머리 회전·눈동자 시선·카메라 각도를 맞춘다. 앞 단계가 맞아도 이 단계가 다르면 실패다.",
                "Fidelity gate 3: match body facing, head rotation, eye gaze, and camera angle. A result that passes the earlier gates but fails this gate is still incorrect."
            )
        ),
        format!(
            "- {}",
            label(
                language,
                "교정 시 실패한 가장 이른 단계 하나만 수정하고 이미 맞은 자세·디테일·정체성·출력 잠금은 유지한다.",
                "During correction, change only the earliest failing gate and preserve every already-correct pose, detail, identity, and output lock."
            )
        ),
    ];
    format!(
        "{}\n{}",
        heading(language, "자세 충실도 계약", "POSE FIDELITY CONTRACT"),
        lines.join("\n")
    )
}

fn render_exact_text_section(
    request: &ImagePromptRequest,
    selected: &[&CatalogEntry],
) -> Option<String> {
    if request.text_elements.is_empty() {
        return None;
    }

    let language = request.language;
    let mut text = Vec::new();
    for (element_index, element) in request.text_elements.iter().enumerate() {
        let separator_count = element.separator_count.map_or_else(String::new, |count| {
            format!(
                "; separator_count_per_line={count} (exactly {count} distinct separation gaps across each complete line; each gap traverses the glyph run and is visible on solid letter stems; count gaps, not resulting fragments; three gaps create four fragments)"
            )
        });
        text.push(format!(
            "- text_element_{}: id={}; role={}; language={}; placement={}; font_family={}; weight={}; alignment={}; size_percent={}%; color={}; letter_spacing={}; treatment={}{}",
            element_index + 1,
            quote(&element.id),
            element.role.as_str(),
            quote(&element.language),
            placement(&element.placement),
            quote(&element.font_family),
            quote(&element.weight),
            quote(&element.alignment),
            element.size_percent,
            element.color_hex.to_ascii_uppercase(),
            quote(&element.letter_spacing),
            quote(&element.treatment),
            separator_count
        ));
        text.push(format!(
            "  - {}",
            label(
                language,
                "다음 인용문만 지정된 순서와 줄바꿈으로 정확히 한 번 렌더링한다:",
                "Render only these quoted lines, exactly once, in this order and with these line breaks:"
            )
        ));
        for (index, line) in element.lines.iter().enumerate() {
            text.push(format!("    {}. {}", index + 1, quote(line)));
        }
        if let Some(spelling_hint) = &element.spelling_hint {
            text.push(format!(
                "  - spelling_hint={} ({})",
                quote(spelling_hint),
                label(
                    language,
                    "철자 안내 전용이며 별도 문구로 렌더링하지 않는다",
                    "guidance only; do not render it as additional copy"
                )
            ));
        }
    }
    text.push(format!(
        "- {}",
        label(
            language,
            "인용된 정확 문구만 렌더링한다. text_element 메타데이터, 번호, 레이블은 이미지에 표시하지 않는다.",
            "Render only the quoted exact copy. Do not render text_element metadata, numbering, or labels."
        )
    ));
    text.push(format!(
        "- {}",
        if text_guard_is_repetition_aware(selected) {
            REPETITION_GUARD
        } else {
            TIER1_GUARD
        }
    ));
    Some(format!(
        "{}\n{}",
        heading(language, "이미지 안 정확 문구", "EXACT TEXT IN IMAGE"),
        text.join("\n")
    ))
}

fn tier_two_tail(has_exact_text: bool) -> &'static str {
    if has_exact_text {
        // The original editorial contract explicitly permits a
        // sequence-preserving subset of NEGATIVE_TAIL. Exact-text work removes
        // only `no text`; all remaining clauses retain their canonical order.
        TIER2_TAIL_WITH_EXACT_TEXT
    } else {
        TIER2_TAIL
    }
}

fn visual(value: &str) -> String {
    crate::common::prompt_text(value)
}

const fn directive_slot_label(language: PromptLanguage, slot: DirectiveSlot) -> &'static str {
    match (language, slot) {
        (PromptLanguage::Korean, DirectiveSlot::Subject) => "피사체 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Scene) => "장면 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Composition) => "구도 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Camera) => "카메라 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Lighting) => "조명 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Color) => "색상 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Material) => "재질 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Typography) => "타이포그래피 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Motion) => "움직임 규칙",
        (PromptLanguage::Korean, DirectiveSlot::Constraint) => "제약 규칙",
        (PromptLanguage::English, DirectiveSlot::Subject) => "Subject rule",
        (PromptLanguage::English, DirectiveSlot::Scene) => "Scene rule",
        (PromptLanguage::English, DirectiveSlot::Composition) => "Composition rule",
        (PromptLanguage::English, DirectiveSlot::Camera) => "Camera rule",
        (PromptLanguage::English, DirectiveSlot::Lighting) => "Lighting rule",
        (PromptLanguage::English, DirectiveSlot::Color) => "Color rule",
        (PromptLanguage::English, DirectiveSlot::Material) => "Material rule",
        (PromptLanguage::English, DirectiveSlot::Typography) => "Typography rule",
        (PromptLanguage::English, DirectiveSlot::Motion) => "Motion rule",
        (PromptLanguage::English, DirectiveSlot::Constraint) => "Constraint rule",
    }
}

fn placement(value: &CanvasPlacement) -> String {
    match value {
        CanvasPlacement::Zone(zone) => zone_name(*zone).to_owned(),
        CanvasPlacement::Custom {
            x_percent,
            y_percent,
            width_percent,
            height_percent,
        } => format!(
            "custom_box(x={}%, y={}%, width={}%, height={}%)",
            x_percent, y_percent, width_percent, height_percent
        ),
    }
}

const fn zone_name(zone: CanvasZone) -> &'static str {
    zone.as_str()
}

fn render_lut(lut: &ResolvedPhotoLut, language: PromptLanguage) -> Vec<String> {
    vec![
        format!(
            "- {}: {} at {}% strength",
            label(language, "LUT 프리셋", "LUT preset"),
            lut.preset,
            lut.strength_percent
        ),
        format!(
            "- {}: {} K; tint={}",
            label(language, "LUT 화이트 밸런스", "LUT white balance"),
            lut.white_balance_kelvin,
            lut.tint
        ),
        format!(
            "- {}: tone_curve={}; black_response={}; contrast={}; highlight_rolloff={}",
            label(language, "LUT 톤 응답", "LUT tone response"),
            lut.tone_curve,
            lut.black_response,
            lut.contrast,
            lut.highlight_rolloff
        ),
        format!(
            "- {}: saturation={}%; shadow_bias={}; highlight_bias={}; skin_tone_policy={}",
            label(language, "LUT 색 분리", "LUT color separation"),
            lut.saturation_percent,
            lut.shadow_bias_hex,
            lut.highlight_bias_hex,
            lut.skin_tone_policy
        ),
        format!(
            "- {}: grain={} at {}%; halation={}%; vignette={}%",
            label(language, "LUT 사진 질감", "LUT photographic texture"),
            lut.grain_size,
            lut.grain_amount_percent,
            lut.halation_percent,
            lut.vignette_percent
        ),
    ]
}

#[cfg(test)]
#[path = "render/tests.rs"]
mod tests;
