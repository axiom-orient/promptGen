use super::*;

pub(super) fn push_missing(
    answers: &BTreeMap<String, String>,
    key: &str,
    question: InterviewQuestion,
    questions: &mut Vec<InterviewQuestion>,
) {
    if answer(answers, key).is_none() {
        questions.push(question);
    }
}

pub(super) fn option(value: &str, label: &str, description: &str) -> InterviewOption {
    InterviewOption::new(value, label, description)
}

pub(super) fn text_mode_question() -> InterviewQuestion {
    InterviewQuestion::choice(
        "image.text_mode",
        "이미지 안에 정확한 문구가 필요합니까?",
        "정확 문구가 있으면 입력한 줄바꿈과 위치를 보존하고, 다른 읽을 수 있는 문자는 만들지 않습니다.",
        vec![
            option("none", "문구 없음", "읽을 수 있는 문자를 생성하지 않음"),
            option(
                "exact",
                "정확 문구 있음",
                "지정한 문구와 줄바꿈을 그대로 렌더",
            ),
        ],
    )
}

pub(super) fn category_question() -> InterviewQuestion {
    InterviewQuestion::choice(
        "image.category",
        "무엇을 만들까요?",
        "결과물 카드에서 하나를 먼저 선택합니다.",
        category_options(),
    )
}

pub(super) fn profile_question(category: &str) -> InterviewQuestion {
    let mut options = vec![option(
        "standard",
        "기본 이미지",
        "카테고리의 일반적인 구조화 이미지 경로",
    )];
    match category.to_ascii_uppercase().as_str() {
        "C4" => {
            options.push(option(
                "logo_identity",
                "로고 아이덴티티",
                "불투명 PNG 콘셉트·브랜드 마크 방향",
            ));
            options.push(option(
                "app_icon",
                "앱 아이콘",
                "제품 목적·핵심 은유·32px 실루엣의 opaque PNG master concept",
            ));
        }
        "C5" => options.push(option(
            "travel_journal",
            "여행·다이어리",
            "사진·종이·제공 문구와 사실 보존",
        )),
        "C6" => {
            options.push(option(
                "app_web_ui",
                "앱·웹 UI",
                "앱·웹 화면과 컴포넌트 상태",
            ));
            options.push(option(
                "information_design",
                "정보 디자인",
                "정보 단위·위계·범례·읽기 순서",
            ));
        }
        "C10" => {
            options.push(option(
                "character_pose",
                "캐릭터·자세",
                "텍스트로 지정한 캐릭터와 동작",
            ));
            options.push(option(
                "poomsae_pose",
                "품새 자세",
                "텍스트로 지정한 품새·태권도 동작",
            ));
        }
        _ => {}
    }
    InterviewQuestion::choice(
        "image.profile",
        "이 결과물의 세부 이미지 프로필을 선택해 주세요.",
        "프로필은 여섯 카테고리 안에서만 선택되며, 카테고리를 바꾸면 호환되지 않는 이전 프로필은 제거합니다.",
        options,
    )
}

pub(super) fn is_primary_category(value: &str) -> bool {
    find_entry(value)
        .ok()
        .flatten()
        .is_some_and(|entry| entry.is_primary_output())
}

pub(super) fn medium_options() -> Vec<InterviewOption> {
    [
        ("photo", "사진", "카메라·렌즈·조명 결과 중심"),
        ("illustration", "일러스트", "그려진 형태와 선·면 중심"),
        ("3d", "3D", "입체 재질과 렌더링 중심"),
        (
            "graphic_design",
            "그래픽 디자인",
            "그리드·타이포·정보 구조 중심",
        ),
        (
            "mixed",
            "혼합 매체",
            "둘 이상의 시각 매체를 명시적으로 결합",
        ),
    ]
    .into_iter()
    .map(|(value, label, description)| option(value, label, description))
    .collect()
}

pub(super) fn text_position_options() -> Vec<InterviewOption> {
    [
        ("top_left", "좌상단"),
        ("top_center", "상단 중앙"),
        ("top_right", "우상단"),
        ("middle_left", "중앙 왼쪽"),
        ("center", "정중앙"),
        ("middle_right", "중앙 오른쪽"),
        ("bottom_left", "좌하단"),
        ("bottom_center", "하단 중앙"),
        ("bottom_right", "우하단"),
    ]
    .into_iter()
    .map(|(value, label)| option(value, label, "선택한 캔버스 영역에 정확 문구 배치"))
    .collect()
}

pub(super) fn detail_options() -> Vec<InterviewOption> {
    [
        ("auto", "자동", "요청의 목적과 내용에 맞는 디테일"),
        ("low", "간결", "구도와 핵심 형태를 중심으로 장식을 줄임"),
        ("medium", "중간", "일반 제작 기본값"),
        ("high", "높음", "정확 문구·다중 요소"),
    ]
    .into_iter()
    .map(|(value, label, description)| option(value, label, description))
    .collect()
}

pub(super) fn aspect_ratio_options() -> Vec<InterviewOption> {
    ASPECT_RATIO_PRESETS
        .iter()
        .map(|preset| option(preset.id, preset.label, preset.description))
        .collect()
}

pub(super) fn lighting_options() -> Vec<InterviewOption> {
    [
        (
            "soft_daylight",
            "부드러운 주광",
            "넓은 확산광과 자연스러운 재질 표현",
        ),
        (
            "hard_graphic",
            "하드 그래픽",
            "선명한 그림자와 강한 형태 대비",
        ),
        ("low_key", "로우 키", "제한된 키 라이트와 깊은 그림자"),
        ("golden_hour", "골든 아워", "낮은 각도의 따뜻한 측광"),
        (
            "neon_practical",
            "네온 프랙티컬",
            "분리된 청록·마젠타 실광원",
        ),
    ]
    .into_iter()
    .map(|(value, label, description)| option(value, label, description))
    .collect()
}

pub(super) fn text_style_options() -> Vec<InterviewOption> {
    [
        (
            "geometric",
            "기하학적 산세리프",
            "균일한 기준선과 명확한 획",
        ),
        ("condensed", "콘덴스드 산세리프", "높고 좁은 자형"),
        ("didone", "디돈 세리프", "굵은 세로획과 얇은 세리프 대비"),
        ("mono", "모노스페이스", "균일 폭과 타자기 인상"),
        ("brush", "통제된 붓글씨", "붓 가장자리와 또렷한 자모 골격"),
    ]
    .into_iter()
    .map(|(value, label, description)| option(value, label, description))
    .collect()
}

pub(super) fn category_options() -> Vec<InterviewOption> {
    catalog()
        .map(|catalog| {
            catalog
                .entries
                .iter()
                .filter(|entry| entry.is_primary_output())
                .map(|entry| {
                    let label = entry.tier_3.as_ref().map_or_else(
                        || entry.tier_2.clone(),
                        |tier_3| format!("{} · {tier_3}", entry.tier_2),
                    );
                    option(&entry.id, &label, &entry.intent)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn lut_options() -> Vec<InterviewOption> {
    [
        (
            "clean_neutral",
            "클린 뉴트럴",
            "색 편향을 줄이고 제품·피부의 실제 색을 보존",
        ),
        (
            "warm_pastel_filmic",
            "웜 파스텔 필름",
            "따뜻한 하이라이트와 부드럽게 뜬 블랙",
        ),
        ("cool_steel", "쿨 스틸", "차가운 중간톤과 선명한 재질 대비"),
        (
            "restrained_teal_orange",
            "절제된 틸·오렌지",
            "청록 그림자와 따뜻한 피부·하이라이트를 분리",
        ),
        (
            "bleach_bypass",
            "블리치 바이패스",
            "낮은 채도와 단단한 대비의 인쇄·영화 질감",
        ),
        (
            "tungsten_night",
            "텅스텐 나이트",
            "따뜻한 실내 광원과 차가운 야간 그림자",
        ),
        (
            "faded_print",
            "페이디드 프린트",
            "바랜 인쇄물처럼 낮은 대비와 부드러운 색층",
        ),
        (
            "monochrome_high_contrast",
            "고대비 흑백",
            "색을 제거하고 명암·표면·실루엣을 강조",
        ),
    ]
    .into_iter()
    .map(|(value, label, description)| option(value, label, description))
    .collect()
}
