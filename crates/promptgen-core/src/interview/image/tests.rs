use super::*;
use crate::image::VisualMedium;

fn complete_answers() -> BTreeMap<String, String> {
    [
        ("image.category", "C5"),
        ("image.medium", "photo"),
        (
            "image.campaign_plan",
            "스페셜티 커피를 찾는 직장인 대상, 무광 아이보리 컵 1개가 히어로, 상단 카피와 좌하단 CTA 안전 영역",
        ),
        (
            "image.subject",
            "무광 아이보리 도자기 컵 1개, 손잡이는 오른쪽",
        ),
        ("image.scene", "오전의 콘크리트 로스터리와 중간 회색 벽"),
        (
            "image.composition",
            "피사체는 높이 58%, 중심보다 3% 오른쪽, 상단 28% 여백",
        ),
        ("image.lighting", "soft_daylight"),
        ("image.palette", "#F3F0E8, #6B655F, #5B351F"),
        (
            "image.surface",
            "무광 도자기와 미세 유약 요철, 폭이 넓은 확산 반사",
        ),
        ("image.text_mode", "none"),
        ("image.lut", "warm_pastel_filmic"),
        ("image.aspect_ratio", "2:3"),
        ("image.detail", "high"),
        (
            "image.exclusions",
            "단독 주 피사체\n브랜드 표식이 없는 표면",
        ),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect()
}

#[test]
fn supported_aspect_ratios_map_to_exact_output_dimensions() {
    for preset in ASPECT_RATIO_PRESETS {
        let actual = dimensions(preset.id);
        assert_eq!(actual, (preset.width, preset.height), "{}", preset.id);
        assert_eq!(actual.0 % 16, 0, "{} width", preset.id);
        assert_eq!(actual.1 % 16, 0, "{} height", preset.id);
        let (ratio_width, ratio_height) = preset.id.split_once(':').expect("ratio id");
        let ratio_width = ratio_width.parse::<u64>().expect("ratio width");
        let ratio_height = ratio_height.parse::<u64>().expect("ratio height");
        assert_eq!(
            u64::from(actual.0) * ratio_height,
            u64::from(actual.1) * ratio_width,
            "{}",
            preset.id
        );
    }
}

#[test]
fn high_resolution_square_ratio_is_inferred_from_the_brief() {
    assert_eq!(
        find_ratio("고해상도 2048:2048 이미지"),
        Some("2048:2048".to_owned())
    );
}

#[test]
fn brief_infers_outcome_ratio_and_exact_text() {
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "4:5 봄 캠페인 포스터, 제목은 \"봄의 한 잔\"".to_owned(),
        answers: BTreeMap::new(),
    });
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.category")
            .map(String::as_str),
        Some("C5")
    );
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.aspect_ratio")
            .map(String::as_str),
        Some("4:5")
    );
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.text")
            .map(String::as_str),
        Some("봄의 한 잔")
    );
}

#[test]
fn six_representative_intents_map_to_six_outcomes() {
    for (brief, expected) in [
        ("자연광에서 일하는 성인 디자이너 라이프스타일 사진", "C1"),
        ("재사용 물병과 패키지 브랜드 제품 사진", "C4"),
        ("개인 기억 앱 아이콘 마스터 콘셉트", "C4"),
        ("신제품 출시 캠페인 광고 포스터", "C5"),
        ("에이전트 화면의 탐색과 상태 UI", "C6"),
        ("5단계 인포그래픽과 모바일 UI", "C6"),
        ("같은 여우 캐릭터의 4패널 일러스트 이야기", "C10"),
        ("거대한 고대 유적의 게임 컨셉 아트", "C11"),
    ] {
        assert_eq!(infer_category(brief), Some(expected), "{brief}");
    }
}

#[test]
fn category_choices_are_exactly_the_six_outcomes() {
    let ids = category_options()
        .into_iter()
        .map(|option| option.value)
        .collect::<Vec<_>>();
    assert_eq!(ids, ["C1", "C4", "C5", "C6", "C10", "C11"]);
}

#[test]
fn profile_inference_distinguishes_positive_logo_and_ui_intent() {
    assert_eq!(
        infer_profile("제품 로고와 브랜드 마크 시안", "C4"),
        Some("logo_identity")
    );
    assert_eq!(
        infer_profile("개인 기억 앱 아이콘 마스터 콘셉트", "C4"),
        Some("app_icon")
    );
    assert_eq!(infer_profile("아이콘", "C4"), None);
    assert_eq!(infer_category("아이콘"), None);
    assert_eq!(infer_category("아이콘 생성"), None);
    assert_eq!(infer_profile("패키지 로고 없이 제품 사진", "C4"), None);
    assert_eq!(
        infer_profile("모바일 앱 대시보드 UI 화면", "C6"),
        Some("app_web_ui")
    );
    assert_eq!(
        infer_profile("정보 디자인 플로차트", "C6"),
        Some("information_design")
    );
    assert_eq!(
        infer_profile("태권도 품새 앞굽이 동작", "C10"),
        Some("poomsae_pose")
    );
    assert_eq!(infer_category("에이전트 화면의 아이콘 버튼"), Some("C6"));
}

#[test]
fn every_outcome_routes_to_one_high_impact_decision() {
    for (category, expected_question) in [
        ("C1", "image.wardrobe"),
        ("C4", "image.product_guide"),
        ("C5", "image.campaign_plan"),
        ("C6", "image.information_plan"),
        ("C10", "image.panel_plan"),
        ("C11", "image.keyart_plan"),
    ] {
        let answers = [("image.category", category)]
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        let outcome = run(&InterviewRequest {
            kind: PromptKind::Image,
            brief: "선택한 결과물의 제작 계약".to_owned(),
            answers,
        });
        assert_eq!(outcome.status, super::super::InterviewStatus::NeedsInput);
        assert_eq!(
            outcome
                .questions
                .iter()
                .map(|question| question.id.as_str())
                .collect::<Vec<_>>(),
            [expected_question],
            "{category}"
        );
    }
}

#[test]
fn route_defaults_cover_the_six_outcomes_without_style_ids() {
    for (category, medium, ratio) in [
        ("C1", "photo", "2:3"),
        ("C4", "photo", "3:2"),
        ("C5", "photo", "2:3"),
        ("C6", "graphic_design", "16:9"),
        ("C10", "illustration", "2:3"),
        ("C11", "illustration", "16:9"),
    ] {
        let mut answers = BTreeMap::from([("image.category".to_owned(), category.to_owned())]);
        let mut inferences = Vec::new();
        infer_medium_from_route(&mut answers, &mut inferences);
        assert_eq!(answer(&answers, "image.medium"), Some(medium), "{category}");
        assert_eq!(default_aspect_ratio(category, ""), ratio, "{category}");
    }
}

#[test]
fn complete_image_interview_compiles() {
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "스페셜티 커피 캠페인 키비주얼".to_owned(),
        answers: complete_answers(),
    });
    assert_eq!(
        outcome.status,
        super::super::InterviewStatus::Ready,
        "{:?}",
        outcome.questions
    );
    let prompt = outcome.compilation.unwrap().prompt.unwrap();
    assert!(prompt.trim_end().ends_with("AR 2:3"));
    assert!(prompt.contains("warm_pastel_filmic"));
    assert!(!prompt.contains("보조 스타일"));
    assert!(!prompt.contains("exact_text_line_"));
}

#[test]
fn logo_profile_selects_a_compatible_default_medium() {
    let answers = BTreeMap::from([
        ("image.category".to_owned(), "C4".to_owned()),
        ("image.profile".to_owned(), "logo_identity".to_owned()),
        (
            "image.product_guide".to_owned(),
            "one opaque coffee brand mark concept with clear silhouette and spacing".to_owned(),
        ),
        (
            "image.subject".to_owned(),
            "one abstract coffee brand mark, no readable wordmark".to_owned(),
        ),
        (
            "image.scene".to_owned(),
            "neutral opaque brand presentation board".to_owned(),
        ),
        (
            "image.composition".to_owned(),
            "centered mark with generous clear space".to_owned(),
        ),
        ("image.lighting".to_owned(), "hard_graphic".to_owned()),
        (
            "image.palette".to_owned(),
            "#0B1633, #F3F5F2, #FF5A1F".to_owned(),
        ),
        (
            "image.surface".to_owned(),
            "opaque flat paper-like board and clean color planes".to_owned(),
        ),
        ("image.text_mode".to_owned(), "none".to_owned()),
        ("image.aspect_ratio".to_owned(), "1:1".to_owned()),
        ("image.detail".to_owned(), "high".to_owned()),
        (
            "image.exclusions".to_owned(),
            "transparent background\nSVG or vector final delivery".to_owned(),
        ),
    ]);
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "opaque PNG coffee brand mark concept".to_owned(),
        answers,
    });

    assert_eq!(
        outcome.status,
        super::super::InterviewStatus::Ready,
        "{:?}",
        outcome.questions
    );
    let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");
    assert_eq!(request.profile, crate::image::ImageProfile::LogoIdentity);
    assert_eq!(request.medium, VisualMedium::GraphicDesign);
    assert!(
        outcome
            .compilation
            .as_ref()
            .is_some_and(|compilation| compilation.is_valid())
    );
}

#[test]
fn app_icon_profile_gets_canonical_defaults_and_prompt_only_output() {
    let answers = BTreeMap::from([
        ("image.category".to_owned(), "C4".to_owned()),
        ("image.profile".to_owned(), "app_icon".to_owned()),
        (
            "image.app_icon_concept".to_owned(),
            "개인 기억 앱, 작은 별 안에 접힌 종이 형태를 넣은 단일 중심 마크".to_owned(),
        ),
        (
            "image.subject".to_owned(),
            "별 안에 접힌 종이 형태를 넣은 단일 앱 아이콘 마크 1개".to_owned(),
        ),
        (
            "image.scene".to_owned(),
            "opaque neutral canvas for one isolated centered app-icon mark".to_owned(),
        ),
        (
            "image.composition".to_owned(),
            "one dominant centered mark filling roughly 80% of the square".to_owned(),
        ),
        ("image.lighting".to_owned(), "hard_graphic".to_owned()),
        (
            "image.palette".to_owned(),
            "#0B1633, #F5D76E, #4E78C4".to_owned(),
        ),
        (
            "image.surface".to_owned(),
            "soft matte graphic artwork with crisp component edges".to_owned(),
        ),
        ("image.medium".to_owned(), "graphic_design".to_owned()),
        ("image.text_mode".to_owned(), "none".to_owned()),
        ("image.aspect_ratio".to_owned(), "1:1".to_owned()),
        ("image.detail".to_owned(), "high".to_owned()),
        (
            "image.exclusions".to_owned(),
            "text, letters, numerals, pseudo-writing, UI screenshot, device mockup, watermark"
                .to_owned(),
        ),
    ]);
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "개인 기억 앱 아이콘 마스터 콘셉트".to_owned(),
        answers,
    });

    assert_eq!(
        outcome.status,
        super::super::InterviewStatus::Ready,
        "{:?}",
        outcome.questions
    );
    let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");
    assert_eq!(request.profile, crate::image::ImageProfile::AppIcon);
    assert_eq!(request.medium, VisualMedium::GraphicDesign);
    assert_eq!(request.output.width, 1024);
    assert_eq!(request.output.height, 1024);
    assert!(request.text_elements.is_empty());
    assert_eq!(request.subjects.len(), 1);
    assert_eq!(
        request.subjects[0].placement,
        crate::image::CanvasPlacement::Zone(crate::image::CanvasZone::Center)
    );
    let compilation = outcome.compilation.expect("compilation");
    assert!(compilation.is_valid());
    assert!(
        compilation
            .prompt
            .unwrap()
            .contains("앱 아이콘 마스터 콘셉트 계약")
    );
}

#[test]
fn app_icon_exact_text_is_reasked_as_textless() {
    let answers = BTreeMap::from([
        ("image.category".to_owned(), "C4".to_owned()),
        ("image.profile".to_owned(), "app_icon".to_owned()),
        ("image.text_mode".to_owned(), "exact".to_owned()),
    ]);
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "개인 기억 앱 아이콘".to_owned(),
        answers,
    });
    let question = outcome
        .questions
        .iter()
        .find(|question| question.id == "image.text_mode")
        .expect("text mode question");
    assert_eq!(
        question
            .options
            .iter()
            .map(|option| option.value.as_str())
            .collect::<Vec<_>>(),
        ["none"]
    );
    assert!(
        question
            .validation_error
            .as_deref()
            .is_some_and(|message| message.contains("app_icon"))
    );
}

#[test]
fn incompatible_logo_medium_is_reasked_without_overwriting_the_explicit_value() {
    let brief = "product photo logo and brand mark concept for coffee";
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.to_owned(),
        answers: BTreeMap::new(),
    });

    assert_eq!(
        outcome.status,
        super::super::InterviewStatus::NeedsInput,
        "{:?}",
        outcome.questions
    );
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.medium")
            .map(String::as_str),
        Some("photo")
    );
    let question = outcome
        .questions
        .iter()
        .find(|question| question.id == "image.medium")
        .expect("profile-compatible medium question");
    assert_eq!(question.control, "choice");
    assert!(
        question
            .validation_error
            .as_deref()
            .is_some_and(|message| message.contains("logo_identity"))
    );
    assert_eq!(
        question
            .options
            .iter()
            .map(|option| option.value.as_str())
            .collect::<Vec<_>>(),
        ["graphic_design", "3d", "mixed"]
    );

    let mut recovered_answers = outcome.normalized_answers.clone();
    recovered_answers.insert("image.medium".to_owned(), "graphic_design".to_owned());
    let recovered = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.to_owned(),
        answers: recovered_answers,
    });
    assert_eq!(
        recovered.status,
        super::super::InterviewStatus::Ready,
        "{:?}",
        recovered.questions
    );
    let InterviewRequestValue::Image(request) = recovered.request_value.expect("typed request");
    assert_eq!(request.medium, VisualMedium::GraphicDesign);
}

#[test]
fn run_interview_rejects_unknown_answer_keys() {
    let mut answers = complete_answers();
    answers.insert("unsupported.answer".to_owned(), "value".to_owned());
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "제품 캠페인 이미지".to_owned(),
        answers,
    });
    assert_eq!(outcome.status, super::super::InterviewStatus::Invalid);
    let diagnostic = &outcome
        .compilation
        .expect("invalid compilation")
        .diagnostics[0];
    assert_eq!(diagnostic.code, "INTERVIEW_UNKNOWN_ANSWER");
    assert_eq!(diagnostic.path, "$.answers.unsupported.answer");
}

#[test]
fn invalid_category_is_reasked_before_route_specific_state_is_mutated() {
    let answers = [
        ("image.category", "banana"),
        ("image.panel_plan", "4패널 이야기"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "캐릭터 일러스트".to_owned(),
        answers,
    });

    assert_eq!(outcome.status, super::super::InterviewStatus::NeedsInput);
    for key in ["image.medium", "image.subject", "image.scene"] {
        assert!(!outcome.normalized_answers.contains_key(key), "{key}");
    }
    assert!(outcome.questions.iter().any(|question| {
        question.id == "image.category" && question.validation_error.is_some()
    }));
}

#[test]
fn invalid_control_values_are_reasked_instead_of_silently_defaulted() {
    for (key, value) in [
        ("language", "banana"),
        ("image.category", "banana"),
        ("image.medium", "banana"),
        ("image.aspect_ratio", "banana"),
        ("image.lighting", "banana"),
        ("image.detail", "banana"),
        ("image.text_mode", "banana"),
    ] {
        let mut answers = complete_answers();
        answers.insert(key.to_owned(), value.to_owned());
        let outcome = run(&InterviewRequest {
            kind: PromptKind::Image,
            brief: "제품 캠페인 이미지".to_owned(),
            answers,
        });
        assert_eq!(
            outcome.status,
            super::super::InterviewStatus::NeedsInput,
            "{key}"
        );
        assert!(outcome.compilation.is_none(), "{key}");
        assert!(
            outcome
                .questions
                .iter()
                .any(|question| question.id == key && question.validation_error.is_some()),
            "{key}: {:?}",
            outcome.questions
        );
    }
}

#[test]
fn invalid_palette_remains_a_question() {
    let mut answers = complete_answers();
    answers.insert("image.palette".to_owned(), "red, blue".to_owned());
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "커피 캠페인".to_owned(),
        answers,
    });
    assert!(
        outcome.questions.iter().any(|question| {
            question.id == "image.palette" && question.validation_error.is_some()
        })
    );
}

#[test]
fn explicit_medium_in_the_brief_outranks_the_route_default() {
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "제품 브랜드를 수채화 일러스트로 만들어 줘".to_owned(),
        answers: BTreeMap::new(),
    });
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.category")
            .map(String::as_str),
        Some("C4")
    );
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.medium")
            .map(String::as_str),
        Some("illustration")
    );
}

#[test]
fn editorial_brief_asks_only_for_specific_wardrobe_then_compiles() {
    let brief = "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.";
    let first = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.to_owned(),
        answers: BTreeMap::new(),
    });
    assert_eq!(
        first
            .questions
            .iter()
            .map(|question| question.id.as_str())
            .collect::<Vec<_>>(),
        ["image.wardrobe"]
    );

    let answers = [
        (
            "image.wardrobe",
            "세이지색 불투명 리넨 테일러드 재킷, 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏",
        ),
        ("image.adult_editorial_confirm", "yes"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    let completed = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.to_owned(),
        answers,
    });
    assert_eq!(
        completed.status,
        super::super::InterviewStatus::Ready,
        "{:?}",
        completed.questions
    );
    let prompt = completed.compilation.unwrap().prompt.unwrap();
    assert!(prompt.contains('\n'));
    assert!(prompt.contains("세이지색 불투명 리넨"));
    assert!(prompt.contains("2:3"));
}

#[test]
fn negative_constraints_do_not_become_positive_visual_inferences() {
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "문구와 로고 없이, 워터마크 없이 무광 검정 캔 1개 광고 포스터".to_owned(),
        answers: BTreeMap::new(),
    });
    let subject = outcome
        .normalized_answers
        .get("image.subject")
        .map(String::as_str)
        .unwrap_or_default();
    assert!(!subject.contains("워터마크"));
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.text_mode")
            .map(String::as_str),
        Some("none")
    );
}

#[test]
fn reverse_style_brief_prefers_the_quantified_object_clause_over_the_genre() {
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "정사각형 비율의 프리미엄 스튜디오 제품 사진. 소형 데스크톱 스피커 1개와 무지 패키지 박스 1개를 함께 배치한다. 따뜻한 회베이지 배경과 부드러운 측상광.".to_owned(),
        answers: [
            ("image.category".to_owned(), "C4".to_owned()),
        ]
        .into_iter()
        .collect(),
    });
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.subject")
            .map(String::as_str),
        Some("소형 데스크톱 스피커 1개와 무지 패키지 박스 1개를 함께 배치한다")
    );
}

#[test]
fn product_brief_keeps_subject_scene_light_and_composition_in_their_own_slots() {
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "무광 흰 배경 위 검정 텀블러 1개, 왼쪽 위에서 들어오는 부드러운 창가광, 브랜드 표식과 문구 없이 세로 제품 사진".to_owned(),
        answers: BTreeMap::new(),
    });

    assert_eq!(outcome.status, super::super::InterviewStatus::Ready);
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.subject")
            .map(String::as_str),
        Some("무광 검정 텀블러 1개")
    );
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.scene")
            .map(String::as_str),
        Some("흰 배경")
    );
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.surface")
            .map(String::as_str),
        Some("무광")
    );
    assert_eq!(
        outcome
            .normalized_answers
            .get("image.aspect_ratio")
            .map(String::as_str),
        Some("2:3")
    );
    let composition = outcome
        .normalized_answers
        .get("image.composition")
        .expect("composition default");
    assert!(composition.contains("세로"), "{composition}");
    assert!(!composition.contains("창가광"), "{composition}");
    assert!(
        !composition.contains('%'),
        "an inferred composition must not invent percentages that compete with typed subject placement: {composition}"
    );

    let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");
    assert_eq!(request.subjects.len(), 1);
    assert_eq!(request.subjects[0].count, 1);
    assert_eq!(request.subjects[0].description, "검정 텀블러");
    assert_eq!(request.subjects[0].appearance.as_deref(), Some("무광"));
    assert_eq!(request.scene.background, "흰 배경");
    assert_eq!(request.scene.environment, "흰 배경");
    assert!(request.composition.framing.contains("세로"));
    assert!(!request.composition.framing.contains("창가광"));
    assert_ne!(
        request.composition.framing, request.composition.balance,
        "framing and balance must not repeat the same fact"
    );
    assert!(request.composition.balance.contains("프레이밍에 명시된"));
    assert!(
        !request.composition.framing.contains('%'),
        "subject scale and placement remain the only numeric geometry authorities"
    );
    assert!(
        !request.composition.balance.contains('%'),
        "balance must not introduce a second numeric geometry authority"
    );
    assert_eq!(
        request.subjects[0].scale,
        "주 피사체의 주요 실루엣은 캔버스 높이의 58%를 점유"
    );
    assert!(request.lighting.key_direction.contains("왼쪽 상단"));
    assert!(request.lighting.key_direction.contains("창가광"));
    assert!(request.lighting.key_quality.contains("부드러운"));
    assert_eq!(request.surfaces[0].finish, "무광");
    assert!(request.surfaces[0].material.contains("재료가 명시되지"));
    assert!(!request.surfaces[0].material.contains("배경"));
    assert!(request.output.height > request.output.width);
}

#[test]
fn nearby_product_phrases_keep_explicit_material_finish_and_window_direction() {
    for (brief, subject, material, finish, background, direction) in [
        (
            "유광 빨간 알루미늄 캔 1개, 밝은 회색 배경, 오른쪽 위의 부드러운 창문광, 문구 없는 세로 제품 사진",
            "빨간 알루미늄 캔",
            "알루미늄",
            "유광",
            "밝은 회색 배경",
            "오른쪽 상단",
        ),
        (
            "matte black stainless steel tumbler 1개, clean white background, soft window light from top left, vertical product photo without text or logos",
            "black stainless steel tumbler",
            "stainless steel",
            "matte",
            "clean white background",
            "왼쪽 상단",
        ),
    ] {
        let outcome = run(&InterviewRequest {
            kind: PromptKind::Image,
            brief: brief.to_owned(),
            answers: BTreeMap::new(),
        });
        assert_eq!(
            outcome.status,
            super::super::InterviewStatus::Ready,
            "{brief}: {:?}",
            outcome.questions
        );
        let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");
        assert_eq!(request.subjects[0].description, subject, "{brief}");
        assert_eq!(
            request.subjects[0].appearance.as_deref(),
            Some(finish),
            "{brief}"
        );
        assert_eq!(request.surfaces[0].material, material, "{brief}");
        assert_eq!(request.surfaces[0].finish, finish, "{brief}");
        assert_eq!(request.scene.background, background, "{brief}");
        assert!(
            request.lighting.key_direction.contains(direction),
            "{brief}: {}",
            request.lighting.key_direction
        );
        assert!(request.output.height > request.output.width, "{brief}");
    }
}

fn compiled_subjects(subject: &str) -> Vec<crate::image::SubjectSpec> {
    let mut answers = complete_answers();
    answers.insert("image.subject".to_owned(), subject.to_owned());
    let value = build_request_json(
        &InterviewRequest {
            kind: PromptKind::Image,
            brief: subject.to_owned(),
            answers: BTreeMap::new(),
        },
        &answers,
    );
    ImagePromptRequest::from_json(value)
        .expect("interview request")
        .subjects
}

#[test]
fn korean_compound_subjects_keep_independent_counts() {
    let subjects = compiled_subjects(
        "무광 아이보리 도자기 컵 1개와 중간 배전 원두 정확히 9알을 함께 배치한다",
    );

    assert_eq!(subjects.len(), 2);
    assert_eq!(subjects[0].count, 1);
    assert_eq!(subjects[1].count, 9);
    assert!(subjects[0].description.contains("컵"));
    assert!(subjects[1].description.contains("원두"));
    assert!(
        subjects
            .iter()
            .all(|subject| !subject.description.contains("함께 배치"))
    );
}

#[test]
fn english_compound_subjects_keep_independent_counts() {
    let subjects = compiled_subjects(
        "one portable speaker and one matching unprinted package box placed together",
    );

    assert_eq!(subjects.len(), 2);
    assert_eq!(subjects[0].count, 1);
    assert_eq!(subjects[1].count, 1);
    assert!(subjects[0].description.contains("speaker"));
    assert!(subjects[1].description.contains("package box"));
    assert!(
        subjects
            .iter()
            .all(|subject| !subject.description.contains("together"))
    );
}

#[test]
fn sticker_grid_counts_visible_stickers_not_the_source_character() {
    let subjects = compiled_subjects(
        "원본 파랑 후드 마스코트 1명의 기쁨과 놀람을 담은 12종 표정 스티커 팩, 3×4 그리드",
    );

    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0].count, 12);
    assert!(subjects[0].description.contains("스티커"));
}

#[test]
fn a_single_quantified_subject_remains_one_subject() {
    let subjects = compiled_subjects("무광 검정 알루미늄 캔 1개");

    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0].count, 1);
    assert!(subjects[0].description.contains("캔"));
}

#[test]
fn uncounted_compound_noun_is_preserved_as_one_visible_subject() {
    let subjects = compiled_subjects("무광 검정 텀블러와 작은 레몬 한 개");

    assert_eq!(subjects.len(), 2);
    assert_eq!(subjects[0].count, 1);
    assert_eq!(subjects[1].count, 1);
    assert!(subjects[0].description.contains("텀블러"));
    assert!(subjects[1].description.contains("레몬"));
}

#[test]
fn plus_separated_product_pair_keeps_both_explicit_counts() {
    let subjects = compiled_subjects("스피커 1개 + 패키지 박스 1개");

    assert_eq!(subjects.len(), 2);
    assert_eq!(subjects[0].count, 1);
    assert_eq!(subjects[1].count, 1);
    assert_eq!(subjects[0].id, "speaker");
    assert_eq!(subjects[1].id, "package_box");
}

#[test]
fn product_model_number_is_not_mistaken_for_a_subject_count() {
    let subjects = compiled_subjects("iPhone 16 product box");

    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0].count, 1);
    assert!(subjects[0].description.contains("iPhone 16"));
}

#[test]
fn adult_age_decade_is_preserved_while_an_object_counter_remains_a_count() {
    let person = compiled_subjects("30대 성인 모델 1명");
    assert_eq!(person.len(), 1);
    assert_eq!(person[0].count, 1);
    assert!(person[0].description.contains("30대"));

    let cars = compiled_subjects("자동차 30대");
    assert_eq!(cars.len(), 1);
    assert_eq!(cars[0].count, 30);
    assert!(cars[0].description.contains("자동차"));
}

#[test]
fn english_color_conjunction_stays_inside_one_product_description() {
    let subjects = compiled_subjects("black and white product package 1개");

    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0].count, 1);
    assert!(subjects[0].description.contains("black and white"));
}

#[test]
fn explicit_mascot_and_sticker_counts_both_survive() {
    let subjects = compiled_subjects("파란 마스코트 1명과 표정 스티커 12장");

    assert_eq!(subjects.len(), 2);
    assert_eq!(subjects[0].id, "mascot");
    assert_eq!(subjects[0].count, 1);
    assert_eq!(subjects[1].id, "stickers");
    assert_eq!(subjects[1].count, 12);
}

#[test]
fn c10_sticker_sheet_compiles_one_repeated_identity_and_flat_panel_contract() {
    let panel_plan = "3×4 스티커 12개: 기쁨(양팔 들기), 놀람(양손 볼), 칭찬(엄지), 걱정(땀방울), 분노(주먹), 집중(책 읽기), 축하(종이 조각), 자신감(브이), 수줍음(하트), 졸림(앉기), 감사(두 손 모으기), 인사(손 흔들기)";
    let answers = [
        ("image.category", "C10"),
        ("image.medium", "illustration"),
        (
            "image.subject",
            "파랑 후드와 주황 스카프, 둥근 크림색 얼굴을 가진 원본 마스코트 1명; 기쁨·놀람·칭찬·걱정·분노·집중·축하를 포함한 서로 다른 12개 표정과 제스처",
        ),
        (
            "image.scene",
            "따뜻한 아이보리 종이 배경 위 3×4 균일 그리드, 모든 스티커는 흰 다이컷 테두리와 동일한 여백을 유지",
        ),
        ("image.panel_plan", panel_plan),
        ("image.text_mode", "none"),
        ("image.aspect_ratio", "2:3"),
        (
            "image.palette",
            "#2459B3, #E96E50, #F4C44E, #F7F1E6, #25262A",
        ),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief:
            "원본 파랑 후드 마스코트 1명의 12종 표정 스티커 팩, 3×4 균일 그리드, 텍스트 없음, 2:3."
                .to_owned(),
        answers,
    });
    assert_eq!(outcome.status, super::super::InterviewStatus::Ready);
    let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");

    assert_eq!(request.subjects.len(), 1);
    let mascot = &request.subjects[0];
    assert_eq!(mascot.id, "mascot");
    assert_eq!(mascot.count, 12);
    for anchor in ["파랑 후드", "주황 스카프", "둥근 크림색 얼굴"] {
        assert!(
            mascot.description.contains(anchor),
            "{}",
            mascot.description
        );
        assert!(
            mascot
                .distinguishing_features
                .iter()
                .any(|feature| feature.contains(anchor)),
            "{anchor}: {:?}",
            mascot.distinguishing_features
        );
    }
    assert!(mascot.action.contains(panel_plan), "{}", mascot.action);
    assert!(request.composition.framing.contains("3×4"));
    assert!(request.composition.framing.contains("균일"));
    assert!(!request.composition.framing.contains("가장 큰 패널"));
    assert!(request.composition.viewpoint.contains("직교"));
    assert!(request.composition.negative_space.is_empty());
    let camera = request
        .camera
        .as_ref()
        .expect("flat graphic camera contract");
    assert!(camera.perspective.contains("직교"));
    assert!(request.lighting.key_temperature_kelvin.is_none());
    assert!(request.lighting.key_to_fill_ratio.is_none());
    assert!(request.lighting.rim_description.is_none());
    assert!(request.surfaces[0].finish.contains("잉크 또는 브러시 질감"));
    assert!(
        !request.surfaces[0]
            .material
            .contains("특정 재료가 명시되지")
    );
    assert_ne!(request.scene.environment, request.scene.background);

    let prompt = outcome
        .compilation
        .expect("compilation")
        .prompt
        .expect("compiled prompt");
    assert!(!prompt.contains("그림의 장면 수와 이야기 흐름을 정해 주세요."));
    assert_eq!(prompt.matches(panel_plan).count(), 1);
}

#[test]
fn c10_storyboard_infers_one_identity_repeated_across_ordered_panels() {
    let panel_plan = "같은 붉은 목도리 여우가 등장하는 4패널, 좌→우 읽기. 씨앗 발견→심기→비 기다리기→새싹과 인사, 문구 없음.";
    let answers = [("image.category", "C10"), ("image.panel_plan", panel_plan)]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "붉은 목도리 여우의 4패널 일러스트, 문구 없음.".to_owned(),
        answers,
    });
    assert_eq!(
        outcome.status,
        super::super::InterviewStatus::Ready,
        "{:?}",
        outcome.questions
    );
    let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");

    assert_eq!(request.subjects.len(), 1);
    assert_eq!(request.subjects[0].count, 4);
    assert!(request.subjects[0].description.contains("붉은 목도리 여우"));
    assert!(request.subjects[0].action.contains(panel_plan));
    assert!(request.composition.framing.contains("균일"));
    assert!(!request.composition.framing.contains("가장 큰 패널"));
    assert!(
        request
            .camera
            .as_ref()
            .expect("flat graphic camera")
            .perspective
            .contains("직교")
    );
}

#[test]
fn c10_english_series_contract_does_not_add_korean_execution_copy() {
    let answers = [
        ("language", "en"),
        ("image.category", "C10"),
        ("image.medium", "illustration"),
        (
            "image.subject",
            "original mascot with a blue hood, orange scarf, and round cream face; twelve distinct expressions and gestures",
        ),
        (
            "image.scene",
            "warm ivory background with a uniform 3x4 grid and equal white die-cut borders",
        ),
        (
            "image.panel_plan",
            "3x4 sticker grid with 12 stickers: joy, surprise, praise, worry, anger, focus, celebration, confidence, shyness, sleepiness, gratitude, greeting",
        ),
        ("image.surface", "consistent ink lines and crisp flat color"),
        ("image.text_mode", "none"),
        ("image.aspect_ratio", "2:3"),
        ("image.palette", "#2459B3, #E96E50, #F4C44E, #F7F1E6"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "Create a 12-sticker mascot sheet in a uniform 3x4 grid with no text.".to_owned(),
        answers,
    });
    assert_eq!(outcome.status, super::super::InterviewStatus::Ready);
    let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");
    let subject = &request.subjects[0];
    let camera = request.camera.as_ref().expect("flat graphic camera");
    let c10_execution_copy = [
        subject.action.as_str(),
        subject.gaze.as_str(),
        subject.pose.as_str(),
        subject.scale.as_str(),
        request.composition.framing.as_str(),
        request.composition.viewpoint.as_str(),
        request.composition.camera_angle.as_str(),
        request.composition.balance.as_str(),
        camera.field_of_view.as_str(),
        camera.perspective.as_str(),
        camera.depth_of_field.as_str(),
        camera.focus.as_str(),
        camera.motion_rendering.as_str(),
        request.lighting.key_direction.as_str(),
        request.lighting.key_quality.as_str(),
        request.lighting.fill_description.as_str(),
        request.lighting.shadow_character.as_str(),
        request.lighting.exposure.as_str(),
        request.surfaces[0].material.as_str(),
        request.surfaces[0].finish.as_str(),
        request.surfaces[0].micro_detail.as_str(),
        request.surfaces[0].light_response.as_str(),
        request.scene.environment.as_str(),
        request.scene.atmosphere.as_str(),
    ];
    for value in c10_execution_copy {
        assert!(
            !value.chars().any(is_hangul),
            "unexpected Korean C10 execution copy: {value}"
        );
    }
    assert!(
        request
            .constraints
            .required_elements
            .iter()
            .all(|value| !value.chars().any(is_hangul))
    );
}

#[test]
fn exact_text_position_is_the_composition_negative_space() {
    for position in SUPPORTED_TEXT_POSITIONS {
        let mut answers = complete_answers();
        answers.insert("image.text_mode".to_owned(), "exact".to_owned());
        answers.insert("image.text".to_owned(), "URBAN\nSIGNAL".to_owned());
        answers.insert("image.text_position".to_owned(), (*position).to_owned());
        answers.insert("image.text_style".to_owned(), "geometric".to_owned());
        let value = build_request_json(
            &InterviewRequest {
                kind: PromptKind::Image,
                brief: "도시 캠페인 포스터".to_owned(),
                answers: BTreeMap::new(),
            },
            &answers,
        );
        let request = ImagePromptRequest::from_json(value).expect("interview request");
        assert_eq!(
            request
                .composition
                .negative_space
                .iter()
                .map(|zone| zone.as_str())
                .collect::<Vec<_>>(),
            [*position],
            "{position}"
        );
    }
}

#[test]
fn quoted_exact_copy_is_rendered_only_by_the_authoritative_text_section() {
    let mut answers = complete_answers();
    answers.insert("image.text_mode".to_owned(), "exact".to_owned());
    answers.insert("image.text".to_owned(), "URBAN\nSIGNAL".to_owned());
    answers.insert("image.text_position".to_owned(), "top_left".to_owned());
    answers.insert("image.text_style".to_owned(), "condensed".to_owned());
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "도시 포스터 제목은 \"URBAN\"과 \"SIGNAL\"".to_owned(),
        answers,
    });
    let compilation = outcome.compilation.expect("compilation");
    assert!(
        compilation.prompt.is_some(),
        "{:#?}",
        compilation.diagnostics
    );
    let prompt = compilation.prompt.expect("prompt");

    assert_eq!(prompt.matches("\"URBAN\"").count(), 1);
    assert_eq!(prompt.matches("\"SIGNAL\"").count(), 1);
    assert!(!prompt.contains("text_elements"));
}

#[test]
fn c4_product_guide_does_not_trigger_follow_up_subject_or_scene_questions() {
    let answers = [
        ("image.category", "C4"),
        (
            "image.product_guide",
            "무광 휴대용 스피커 1개와 무지 패키지 박스 1개, 3/4 외관, 로고·문구 없음",
        ),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "제품 브랜드 이미지".to_owned(),
        answers,
    });

    assert!(
        outcome
            .questions
            .iter()
            .all(|question| question.id != "image.subject" && question.id != "image.scene")
    );
}

#[test]
fn c4_compound_brief_does_not_repeat_product_subject_or_scene_questions() {
    let answers = [("image.category", "C4")]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "무광 검정 텀블러와 작은 레몬 한 개".to_owned(),
        answers,
    });

    assert_eq!(outcome.status, super::super::InterviewStatus::Ready);
    let InterviewRequestValue::Image(request) = outcome.request_value.expect("typed request");
    assert_eq!(request.subjects.len(), 2);
    assert_eq!(request.subjects[0].count, 1);
    assert_eq!(request.subjects[1].count, 1);
}

#[test]
fn user_facing_lut_names_cover_all_presets() {
    for (brief, expected) in [
        ("클린 뉴트럴", "clean_neutral"),
        ("웜 파스텔 필름", "warm_pastel_filmic"),
        ("쿨 스틸", "cool_steel"),
        ("틸 오렌지", "restrained_teal_orange"),
        ("블리치 바이패스", "bleach_bypass"),
        ("밤의 텅스텐 조명", "tungsten_night"),
        ("바랜 인쇄", "faded_print"),
        ("고대비 흑백", "monochrome_high_contrast"),
    ] {
        assert_eq!(default_lut(brief).0, expected, "{brief}");
    }
}

#[test]
fn visual_camera_controls_replace_inferred_camera_slots_without_duplicate_directions() {
    for (prefix, expected) in [("/topDown", "수직"), ("/isometric", "등각")] {
        let outcome = run(&InterviewRequest {
            kind: PromptKind::Image,
            brief: format!("{prefix} 무광 황동 시계 1개의 제품 사진. 문구 없음."),
            answers: BTreeMap::from([("image.category".into(), "C4".into())]),
        });
        let ready = outcome.ready().expect("ready camera request");
        let super::super::InterviewRequestValue::Image(image) = ready.request;
        assert!(image.composition.camera_angle.contains(expected));
        assert!(image.composition.viewpoint.contains(expected));
        let prompt = ready.compilation.prompt.as_deref().unwrap();
        assert!(!prompt.contains("3/4 각도"));
        assert!(!prompt.contains(prefix));
        assert_eq!(image.output.detail, crate::image::ImageDetail::High);
        assert_eq!(image.output.backend, crate::image::IMAGE_BACKEND);
    }
}

#[test]
fn controls_reject_explicit_medium_camera_and_light_conflicts() {
    for (brief, key, value) in [
        ("/watercolor 황동 시계 1개", "image.medium", "photo"),
        (
            "/topdown 황동 시계 1개",
            "image.composition",
            "낮은 위치에서 올려다보는 시점",
        ),
        (
            "/goldenhour 황동 시계 1개",
            "image.lighting",
            "neon_practical",
        ),
        ("/macro /deepfocus 황동 시계 1개", "image.medium", "photo"),
    ] {
        let outcome = run(&InterviewRequest {
            kind: PromptKind::Image,
            brief: brief.into(),
            answers: BTreeMap::from([
                ("image.category".into(), "C4".into()),
                (key.into(), value.into()),
            ]),
        });
        assert_eq!(
            outcome.status,
            super::super::InterviewStatus::Invalid,
            "{brief}: {:?}",
            outcome.questions
        );
        assert!(outcome.compilation.unwrap().prompt.is_none());
    }
}

#[test]
fn journal_preserves_exact_copy_and_does_not_treat_style_as_a_provider_command() {
    let outcome = run(&InterviewRequest { kind: PromptKind::Image,
        brief: "/travel-journal /scrapbook /handwritten 제주 해변 여행 기록 한 장. 제목은 ‘바람을 따라’".into(),
        answers: BTreeMap::new(),
    });
    let ready = outcome.ready().unwrap_or_else(|| {
        panic!(
            "status={:?} questions={:?} compilation={:?}",
            outcome.status, outcome.questions, outcome.compilation
        )
    });
    let super::super::InterviewRequestValue::Image(image) = ready.request;
    assert_eq!(image.text_elements[0].lines, ["바람을 따라"]);
    assert_eq!(image.text_elements[0].color_hex, "#252423");
    assert_eq!(image.profile, crate::image::ImageProfile::TravelJournal);
    assert_eq!(
        image.text_elements[0].font_family,
        "legible pen handwriting"
    );
    let prompt = ready.compilation.prompt.as_deref().unwrap();
    assert!(prompt.contains("확인되지 않은 지명"));
    assert!(prompt.contains("사진") && prompt.contains("가리지"));
    assert!(!prompt.contains("/travel-journal"));
    assert!(!prompt.contains("/scrapbook"));
}

#[test]
fn photo_journal_requires_source_binding_and_preserves_source_light_and_color() {
    let brief = "/travel-journal /scrapbook 원본 사진에 종이 여백과 메모를 추가. 문구 없음.";
    let initial = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.into(),
        answers: BTreeMap::new(),
    });
    assert_eq!(initial.status, super::super::InterviewStatus::NeedsInput);
    assert_eq!(initial.questions[0].id, "image.preserve");
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.into(),
        answers: BTreeMap::from([
            ("image.task_mode".into(), "edit".into()),
            (
                "image.reference_description".into(),
                "원본 해안 여행 사진: 흰 셔츠의 성인 1명, 바다를 보는 뒷모습".into(),
            ),
            (
                "image.preserve".into(),
                "얼굴과 정체성\n옷과 자세\n사진 내부 구도와 장소".into(),
            ),
            (
                "image.change_only".into(),
                "사진 바깥 여백에 크림색 종이 추가".into(),
            ),
        ]),
    });
    let ready = outcome.ready().unwrap_or_else(|| {
        panic!(
            "status={:?} questions={:?} compilation={:?}",
            outcome.status, outcome.questions, outcome.compilation
        )
    });
    let super::super::InterviewRequestValue::Image(image) = ready.request;
    assert_eq!(image.task_mode, crate::image::ImageTaskMode::Edit);
    assert_eq!(image.references.len(), 1);
    assert_eq!(
        image.references[0].role,
        crate::image::ImageReferenceRole::Base
    );
    assert_eq!(image.change_contract.preserve.len(), 3);
    assert!(image.lighting.key_temperature_kelvin.is_none());
    assert!(image.color.photo_lut.is_none());
    assert!(
        image
            .color
            .palette
            .iter()
            .all(|color| color.usage.starts_with("Decoration only:"))
    );
    assert!(image.camera.is_none());
    let canonical = crate::compile_image_prompt(image);
    assert_eq!(&canonical, ready.compilation);
}

#[test]
fn normalized_camera_defaults_can_be_resubmitted_after_a_required_correction() {
    let brief = "/topdown 무광 황동 시계 1개의 제품 사진. 문구 없음.";
    let first = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.into(),
        answers: BTreeMap::from([
            ("image.category".into(), "C4".into()),
            ("image.palette".into(), "invalid".into()),
        ]),
    });
    assert_eq!(first.status, super::super::InterviewStatus::NeedsInput);
    assert!(first.normalized_answers.contains_key("image.composition"));
    let mut answers = first.normalized_answers;
    answers.insert("image.palette".into(), "#F0E8D8, #84715C, #252423".into());
    let completed = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.into(),
        answers,
    });
    assert_eq!(completed.status, super::super::InterviewStatus::Ready);
    let prompt = completed.compilation.unwrap().prompt.unwrap();
    assert!(!prompt.contains("3/4 구도"));
    assert!(!prompt.contains("3/4 각도"));
}

#[test]
fn source_photo_style_is_scoped_to_additions_and_macro_focus_is_coherent() {
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief:
            "/travel-journal /scrapbook /risograph 원본 사진 바깥의 종이 여백만 장식. 문구 없음."
                .into(),
        answers: BTreeMap::from([
            ("image.task_mode".into(), "edit".into()),
            (
                "image.reference_description".into(),
                "성인 한 명이 바다를 바라보는 제공된 사진".into(),
            ),
            ("image.preserve".into(), "원본 사진 전체".into()),
            (
                "image.change_only".into(),
                "바깥 종이에 지정한 인쇄 질감 추가".into(),
            ),
        ]),
    });
    let ready = outcome.ready().expect("scoped source edit");
    let super::super::InterviewRequestValue::Image(image) = ready.request;
    assert!(image.subjects.iter().all(|subject| {
        subject
            .distinguishing_features
            .iter()
            .all(|feature| !feature.contains("리소그래프"))
    }));
    assert!(
        image
            .constraints
            .required_elements
            .iter()
            .any(|value| value.contains("only to declared additions")
                && value.contains("리소그래프"))
    );
    let outcome = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: "/macro 황동 시계 1개의 제품 사진. 문구 없음.".into(),
        answers: BTreeMap::from([("image.category".into(), "C4".into())]),
    });
    let ready = outcome.ready().expect("macro request");
    let super::super::InterviewRequestValue::Image(image) = ready.request;
    let camera = image.camera.as_ref().expect("camera");
    assert!(camera.field_of_view.contains("접사"));
    assert!(camera.depth_of_field.contains("focal plane"));
    assert!(
        !ready
            .compilation
            .prompt
            .as_ref()
            .unwrap()
            .contains("주 피사체 전체는 선명")
    );
}

#[test]
fn mixed_quote_styles_preserve_the_authored_copy_order() {
    assert_eq!(
        extract_quoted_texts("제목은 ‘바람을 따라’, 부제는 \"ON THE COAST\", 메모는 「海边」"),
        ["바람을 따라", "ON THE COAST", "海边"]
    );
}

#[test]
fn complete_photo_annotation_brief_does_not_reask_declared_edits() {
    let outcome = run(&InterviewRequest {kind: PromptKind::Image,
        brief: "/scrapbook 원본 사진에 크림색 종이 여백과 한국어 메모만 추가. 얼굴·옷·자세·장소와 사진 내부 구도를 유지. 문구는 \"오늘의 작은 기록\"만 사용.".into(),
        answers: BTreeMap::new()});
    let ready = outcome
        .ready()
        .unwrap_or_else(|| panic!("{:?} {:?}", outcome.questions, outcome.compilation));
    let super::super::InterviewRequestValue::Image(image) = ready.request;
    assert_eq!(image.task_mode, crate::image::ImageTaskMode::Edit);
    assert!(
        image.references[0]
            .description
            .contains("to be supplied as image 1 at execution")
    );
    assert!(image.change_contract.preserve[0].contains("얼굴"));
    assert!(image.change_contract.change_only[0].contains("메모만 추가"));
    assert_eq!(image.text_elements[0].lines, ["오늘의 작은 기록"]);
}

#[test]
fn edit_brief_keeps_all_declared_clauses_and_respects_explicit_answers() {
    let brief = "/scrapbook 원본 사진. 얼굴을 유지. 배경 장소를 보존. 여백에 종이만 추가. 모서리에 지정 문구만 추가. 문구는 \"여름 기록\".";
    let inferred = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.into(),
        answers: BTreeMap::new(),
    });
    let ready = inferred.ready().expect("multi-clause edit");
    let super::super::InterviewRequestValue::Image(image) = ready.request;
    assert_eq!(
        image.change_contract.preserve,
        ["얼굴을 유지", "배경 장소를 보존"]
    );
    assert_eq!(
        image.change_contract.change_only,
        ["여백에 종이만 추가", "모서리에 지정 문구만 추가"]
    );
    let explicit = run(&InterviewRequest {
        kind: PromptKind::Image,
        brief: brief.into(),
        answers: BTreeMap::from([("image.preserve".into(), "원본 사진 전체".into())]),
    });
    let super::super::InterviewRequestValue::Image(image) = explicit.ready().unwrap().request;
    assert_eq!(image.change_contract.preserve, ["원본 사진 전체"]);
}
