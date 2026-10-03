use super::*;

pub(super) fn infer_category(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    // Medium words such as "illustration" describe how to render, while product
    // and brand words identify what is being made. Prefer the latter when both
    // are present ("제품 브랜드를 수채화 일러스트로").
    if !contains_any(
        &lower,
        &[
            "캠페인",
            "광고",
            "포스터",
            "키비주얼",
            "campaign",
            "advertisement",
            "poster",
        ],
    ) && contains_any(
        &lower,
        &[
            "앱 아이콘",
            "app icon",
            "application icon",
            "app-icon",
            "제품",
            "패키지",
            "브랜딩",
            "목업",
            "화장품",
            "뷰티",
            "제형",
            "분해도",
            "도감",
            "product",
            "packaging",
            "branding",
            "mockup",
        ],
    ) {
        return Some("C4");
    }
    for (category, terms) in [
        (
            "C10",
            &[
                "스티커",
                "이모지",
                "캐릭터 시트",
                "턴어라운드",
                "스토리보드",
                "키프레임",
                "만화",
                "웹툰",
                "코믹",
                "일러스트",
                "sprite",
                "sticker",
                "character sheet",
                "storyboard",
                "comic",
                "illustration",
            ][..],
        ),
        (
            "C6",
            &[
                "인포그래픽",
                "다이어그램",
                "플로우차트",
                "프레젠테이션",
                "슬라이드",
                "카드뉴스",
                "앱 화면",
                "웹 화면",
                "에이전트 화면",
                "app screen",
                "web screen",
                "agent screen",
                "dashboard",
                "ui 목업",
                "와이어프레임",
                "교육 도해",
                "infographic",
                "diagram",
                "presentation",
                "ui mockup",
            ][..],
        ),
        (
            "C5",
            &[
                "캠페인",
                "광고",
                "포스터",
                "키비주얼",
                "소셜 배너",
                "타이포 포스터",
                "campaign",
                "advertisement",
                "poster",
            ][..],
        ),
        (
            "C11",
            &[
                "키아트",
                "시네마틱",
                "콘셉트 아트",
                "컨셉 아트",
                "게임 아트",
                "판타지",
                "역사 장면",
                "cinematic",
                "key art",
                "concept art",
                "game art",
            ][..],
        ),
        (
            "C1",
            &[
                "인물",
                "패션",
                "화보",
                "룩북",
                "프로필",
                "라이프스타일",
                "에디토리얼",
                "portrait",
                "fashion",
                "lookbook",
                "lifestyle",
                "editorial",
            ][..],
        ),
    ] {
        if terms.iter().any(|term| lower.contains(term)) {
            return Some(category);
        }
    }
    None
}

pub(super) fn infer_medium(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    if contains_any(
        &lower,
        &[
            "사진",
            "photo",
            "photoreal",
            "촬영",
            "화보",
            "룩북",
            "film still",
        ],
    ) {
        Some("photo")
    } else if contains_any(&lower, &["3d", "렌더", "클레이", "아이소메트릭", "render"]) {
        Some("3d")
    } else if contains_any(
        &lower,
        &["일러스트", "illustration", "그림", "수채화", "웹툰", "만화"],
    ) {
        Some("illustration")
    } else if contains_any(
        &lower,
        &[
            "그래픽",
            "포스터",
            "타이포",
            "인포그래픽",
            "슬라이드",
            "카드뉴스",
        ],
    ) {
        Some("graphic_design")
    } else {
        None
    }
}

pub(super) fn infer_profile(value: &str, category: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    let category = category.to_ascii_uppercase();
    match category.as_str() {
        "C4" if contains_any(
            &lower,
            &["앱 아이콘", "app icon", "application icon", "app-icon"],
        ) =>
        {
            Some("app_icon")
        }
        "C4" if contains_any(
            &lower,
            &["로고", "브랜드 마크", "brand mark", "logo", "wordmark"],
        ) && !contains_any(
            &lower,
            &[
                "로고 없이",
                "로고 없음",
                "without logo",
                "without logos",
                "without text or logos",
                "no logo",
                "no logos",
                "no text or logos",
            ],
        ) =>
        {
            Some("logo_identity")
        }
        "C5" if contains_any(
            &lower,
            &[
                "여행 기록",
                "다이어리",
                "스크랩북",
                "travel journal",
                "scrapbook",
            ],
        ) =>
        {
            Some("travel_journal")
        }
        "C6" if contains_any(
            &lower,
            &[
                "앱",
                "웹",
                "ui",
                "ux",
                "app",
                "web",
                "dashboard",
                "wireframe",
                "화면",
                "screen",
            ],
        ) =>
        {
            Some("app_web_ui")
        }
        "C6" if contains_any(
            &lower,
            &[
                "인포그래픽",
                "정보 디자인",
                "information design",
                "diagram",
                "flowchart",
            ],
        ) =>
        {
            Some("information_design")
        }
        "C10" if contains_any(&lower, &["품새", "poomsae", "태권도", "taekwondo"]) => {
            Some("poomsae_pose")
        }
        "C10"
            if contains_any(
                &lower,
                &["캐릭터 자세", "character pose", "pose", "자세", "동작"],
            ) =>
        {
            Some("character_pose")
        }
        _ => None,
    }
}

pub(super) fn selected_text_requirement(answers: &BTreeMap<String, String>) -> TextRequirement {
    answer(answers, "image.category")
        .and_then(|category| find_entry(category).ok().flatten())
        .map_or(TextRequirement::Optional, |entry| entry.text_requirement)
}

pub(super) fn split_clauses(value: &str) -> Vec<String> {
    value
        .split(['\n', ',', ';', '.', '。', '!', '?'])
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn extract_subject_clause(value: &str) -> Option<String> {
    let markers = [
        "피사체",
        "모델",
        "인물",
        "사람",
        "남성",
        "여성",
        "고양이",
        "동물",
        "제품",
        "캔",
        "병",
        "컵",
        "텀블러",
        "레몬",
        "원두",
        "신발",
        "가방",
        "자동차",
        "의자",
        "시계",
        "화장품",
        "스피커",
        "패키지",
        "상자",
        "박스",
        "아이콘",
        "캐릭터",
        "마스코트",
        "스티커",
        "robot",
        "model",
        "person",
        "cat",
        "animal",
        "product",
        "bottle",
        "can",
        "cup",
        "tumbler",
        "lemon",
        "bean",
        "shoe",
        "bag",
        "car",
        "watch",
        "speaker",
        "package",
        "box",
        "mascot",
        "sticker",
    ];
    split_clauses(value)
        .into_iter()
        .filter_map(|clause| {
            let lower = clause.to_lowercase();
            let marker_score = markers
                .iter()
                .filter(|marker| lower.contains(**marker))
                .count();
            let count_score = count_candidates(&clause).len();
            // A quantified object clause carries a stronger subject contract than a
            // generic genre clause such as "제품 사진". Reverse-inferred briefs are
            // intentionally multi-sentence, so letting the genre marker win here
            // erases the exact objects before fidelity validation.
            let score = marker_score * 3 + count_score * 4;
            (score > 0).then_some((score, clause))
        })
        .max_by_key(|(score, clause)| (*score, clause.len()))
        .map(|(_, clause)| normalize_subject_clause(&clause))
        .filter(|clause| !clause.is_empty())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParsedSubject {
    pub id: String,
    pub count: u16,
    pub description: String,
    pub appearance: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct CountCandidate {
    start: usize,
    end: usize,
    count: u16,
}

const SUBJECT_MARKERS: &[&str] = &[
    "피사체",
    "모델",
    "인물",
    "사람",
    "고양이",
    "동물",
    "제품",
    "캔",
    "병",
    "컵",
    "텀블러",
    "레몬",
    "원두",
    "신발",
    "가방",
    "자동차",
    "의자",
    "시계",
    "화장품",
    "스피커",
    "패키지",
    "상자",
    "박스",
    "아이콘",
    "캐릭터",
    "마스코트",
    "스티커",
    "패널",
    "subject",
    "model",
    "person",
    "cat",
    "animal",
    "product",
    "bottle",
    "can",
    "cup",
    "tumbler",
    "lemon",
    "bean",
    "beans",
    "shoe",
    "shoes",
    "bag",
    "car",
    "watch",
    "speaker",
    "package",
    "box",
    "icon",
    "character",
    "mascot",
    "sticker",
    "stickers",
    "panel",
    "robot",
];

/// Converts the observable, count-bearing noun phrases in an interview subject
/// answer into independent typed subjects. The renderer and visual fidelity gate
/// treat `SubjectSpec::count` as authoritative, so keeping a compound phrase in a
/// single `count=1` subject would erase exactly the facts those layers verify.
pub(super) fn parse_subjects(value: &str) -> Vec<ParsedSubject> {
    let mut parsed = split_subject_segments(value)
        .into_iter()
        .filter_map(|segment| parse_subject_segment(&segment))
        .collect::<Vec<_>>();

    if parsed.is_empty() {
        let appearance = extract_finish(value);
        let description = clean_subject_description(value, &[]);
        parsed.push(ParsedSubject {
            id: subject_id(&description, 0),
            count: 1,
            description,
            appearance,
        });
    }

    let mut used_ids = BTreeSet::new();
    for (index, subject) in parsed.iter_mut().enumerate() {
        let base = subject_id(&subject.description, index);
        let mut id = base.clone();
        let mut suffix = 2_u16;
        while !used_ids.insert(id.clone()) {
            id = format!("{base}_{suffix}");
            suffix += 1;
        }
        subject.id = id;
    }
    parsed
}

/// Resolves the visible instance count for a C10 panel or sticker series.
///
/// This intentionally sits beside, rather than inside, `parse_subjects`: a
/// recurring identity shown in twelve stickers is one typed subject with twelve
/// visible instances, while ordinary compound requests still need independent
/// subject IDs and counts.
pub(super) fn repeated_panel_count(value: &str) -> Option<u16> {
    let lower = value.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "패널",
            "스티커",
            "그리드",
            "장면",
            "스토리보드",
            "panel",
            "sticker",
            "grid",
            "scene",
            "storyboard",
        ],
    ) {
        return None;
    }

    let explicit = series_unit_candidates(value)
        .into_iter()
        .chain(count_candidates(value))
        .rfind(|candidate| candidate.count > 1)
        .map(|candidate| candidate.count);
    explicit
        .or_else(|| grid_instance_count(value))
        .filter(|count| *count > 1)
}

/// Extracts the stable identity phrase for one subject repeated across a C10
/// panel plan without teaching the compound-subject parser that two
/// independently quantified objects are the same thing.
pub(super) fn repeated_panel_identity(value: &str, instance_count: u16) -> ParsedSubject {
    let identity_source = value
        .split([';', '\n'])
        .map(str::trim)
        .find(|segment| {
            let lower = segment.to_lowercase();
            contains_any(
                &lower,
                &["마스코트", "캐릭터", "여우", "mascot", "character", "fox"],
            )
        })
        .or_else(|| {
            value
                .split([';', '\n'])
                .map(str::trim)
                .find(|value| !value.is_empty())
        })
        .unwrap_or(value)
        .trim();
    let mut candidates = count_candidates(identity_source);
    candidates.extend(series_unit_candidates(identity_source));
    candidates.sort_by_key(|candidate| candidate.start);
    let series_start = candidates
        .iter()
        .find(|candidate| candidate.count == instance_count)
        .map(|candidate| candidate.start)
        .filter(|start| identity_source[..*start].trim().chars().count() >= 4);
    let identity_source = series_start.map_or(identity_source, |start| &identity_source[..start]);
    let counts = count_candidates(identity_source);
    let description = clean_subject_description(identity_source, &counts);
    ParsedSubject {
        id: subject_id(&description, 0),
        count: instance_count,
        appearance: extract_finish(identity_source),
        description,
    }
}

fn series_unit_candidates(value: &str) -> Vec<CountCandidate> {
    let bytes = value.as_bytes();
    let mut candidates = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let end = index;
        let suffix = value[end..]
            .trim_start()
            .trim_start_matches(['-', '_'])
            .to_lowercase();
        if [
            "패널",
            "컷",
            "장면",
            "스티커",
            "panel",
            "frame",
            "scene",
            "sticker",
        ]
        .iter()
        .any(|unit| suffix.starts_with(unit))
            && let Ok(count) = value[start..end].parse::<u16>()
        {
            candidates.push(CountCandidate { start, end, count });
        }
    }
    candidates
}

pub(super) fn repeated_identity_anchors(description: &str) -> Vec<String> {
    description
        .split([',', ';'])
        .map(str::trim)
        .filter(|anchor| !anchor.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn subject_relationship(value: &str) -> Option<String> {
    let lower = value.to_lowercase();
    if contains_any(
        &lower,
        &[
            "그리드",
            "grid",
            "패널",
            "panel",
            "스티커 팩",
            "sticker pack",
        ],
    ) {
        let layout = extract_grid(value).unwrap_or_else(|| "지정된 반복 레이아웃".to_owned());
        return Some(if value.chars().any(is_hangul) {
            format!("모든 인스턴스를 {layout} 안에 서로 겹치지 않게 배치")
        } else {
            format!("Arrange every instance without overlap in the {layout}")
        });
    }
    if contains_any(
        &lower,
        &[
            "함께",
            "나란히",
            "옆에",
            "주위",
            "둘러",
            "together",
            "side by side",
            "beside",
            "next to",
            "around",
        ],
    ) {
        return Some(if value.chars().any(is_hangul) {
            "모든 피사체를 요청에 명시된 상대 위치로 함께 배치".to_owned()
        } else {
            "Arrange all subjects together in the stated relative positions".to_owned()
        });
    }
    None
}

fn split_subject_segments(value: &str) -> Vec<String> {
    let mut normalized = value.replace(['\n', ';', '。'], ",");
    for (from, to) in [
        ("개와 ", "개|"),
        ("알과 ", "알|"),
        ("명과 ", "명|"),
        ("마리와 ", "마리|"),
        ("대와 ", "대|"),
        ("병과 ", "병|"),
        ("개 및 ", "개|"),
        ("알 및 ", "알|"),
        ("명 및 ", "명|"),
        (" & ", "|"),
        (" + ", "|"),
    ] {
        normalized = replace_ascii_case_insensitive(&normalized, from, to);
    }
    normalized = split_english_subject_conjunctions(&normalized);
    normalized = split_korean_subject_conjunctions(&normalized);
    normalized
        .split([',', '|'])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_subject_segment(segment: &str) -> Option<ParsedSubject> {
    let candidates = count_candidates(segment);
    let selected = select_subject_count(segment, &candidates);
    if selected.is_none() && !contains_subject_marker(segment) {
        return None;
    }
    let appearance = extract_finish(segment);
    let description = clean_subject_description(segment, &candidates);
    (!description.is_empty()).then(|| ParsedSubject {
        id: String::new(),
        count: selected.map_or(1, |candidate| candidate.count),
        description,
        appearance,
    })
}

fn select_subject_count<'a>(
    segment: &str,
    candidates: &'a [CountCandidate],
) -> Option<&'a CountCandidate> {
    if candidates.is_empty() {
        return None;
    }
    let lower = segment.to_lowercase();
    let repeated_unit = ["스티커", "sticker", "패널", "panel", "컷", "frame"]
        .into_iter()
        .filter_map(|marker| lower.find(marker))
        .min();
    repeated_unit
        .and_then(|unit| {
            candidates
                .iter()
                .filter(|candidate| candidate.start <= unit)
                .min_by_key(|candidate| unit - candidate.start)
        })
        .or_else(|| candidates.last())
}

fn count_candidates(value: &str) -> Vec<CountCandidate> {
    let mut candidates = Vec::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let digit_end = index;
        let before = value[..start].chars().next_back();
        let after = value[digit_end..].chars().next();
        if before.is_some_and(|ch| matches!(ch, ':' | 'x' | 'X' | '×'))
            || after.is_some_and(|ch| matches!(ch, ':' | 'x' | 'X' | '×' | '%' | '°'))
        {
            continue;
        }
        let Ok(count) = value[start..digit_end].parse::<u16>() else {
            continue;
        };
        let suffix = &value[digit_end..];
        let counter = korean_counter_after(suffix);
        if counter
            .is_some_and(|(length, counter)| korean_age_decade(value, digit_end, length, counter))
        {
            continue;
        }
        if counter.is_none() && !is_english_numeric_count(value, start, digit_end, count) {
            continue;
        }
        let mut end = digit_end + counter.map_or(0, |(length, _)| length);
        if value[end..].starts_with(['을', '를', '의']) {
            end += value[end..].chars().next().map_or(0, char::len_utf8);
        }
        candidates.push(CountCandidate { start, end, count });
    }

    let lower = value.to_lowercase();
    for (word, count) in [
        ("one", 1),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
    ] {
        let mut offset = 0;
        while let Some(relative) = lower[offset..].find(word) {
            let start = offset + relative;
            let end = start + word.len();
            let boundary_before =
                start == 0 || !lower.as_bytes()[start - 1].is_ascii_alphanumeric();
            let boundary_after =
                end == lower.len() || !lower.as_bytes()[end].is_ascii_alphanumeric();
            if boundary_before && boundary_after && contains_subject_marker(value) {
                candidates.push(CountCandidate { start, end, count });
            }
            offset = end;
        }
    }
    for (word, count) in [
        ("한", 1),
        ("하나", 1),
        ("두", 2),
        ("둘", 2),
        ("세", 3),
        ("셋", 3),
        ("네", 4),
        ("넷", 4),
        ("다섯", 5),
        ("여섯", 6),
        ("일곱", 7),
        ("여덟", 8),
        ("아홉", 9),
        ("열", 10),
        ("열한", 11),
        ("열두", 12),
    ] {
        for counter in [
            "마리", "켤레", "개", "알", "명", "종", "대", "병", "장", "점", "권", "벌",
        ] {
            for separator in ["", " "] {
                let phrase = format!("{word}{separator}{counter}");
                let mut offset = 0;
                while let Some(relative) = value[offset..].find(&phrase) {
                    let start = offset + relative;
                    let mut end = start + phrase.len();
                    let boundary_before = value[..start]
                        .chars()
                        .next_back()
                        .is_none_or(|ch| !ch.is_alphanumeric());
                    if boundary_before {
                        if value[end..].starts_with(['을', '를', '의']) {
                            end += value[end..].chars().next().map_or(0, char::len_utf8);
                        }
                        candidates.push(CountCandidate { start, end, count });
                    }
                    offset = start + phrase.len();
                }
            }
        }
    }
    candidates.sort_by_key(|candidate| candidate.start);
    candidates.dedup_by_key(|candidate| (candidate.start, candidate.end));
    candidates
}

fn clean_subject_description(value: &str, counts: &[CountCandidate]) -> String {
    let mut description = value.trim().to_owned();
    for candidate in counts.iter().rev() {
        if candidate.end <= description.len() {
            description.replace_range(candidate.start..candidate.end, "");
        }
    }
    for phrase in [
        "정확히 ",
        "함께 배치한다",
        "함께 배치",
        "나란히 배치한다",
        "나란히",
        "placed together",
        "arranged together",
        "together",
        "exactly ",
    ] {
        description = replace_ascii_case_insensitive(&description, phrase, "");
    }
    description = remove_finish_terms(&description);
    description = description
        .trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '.' | '·' | '-'))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if description.is_empty() {
        "지정된 시각 피사체".to_owned()
    } else {
        description
    }
}

pub(super) fn contains_subject_marker(value: &str) -> bool {
    contains_any(&value.to_lowercase(), SUBJECT_MARKERS)
}

fn subject_id(description: &str, index: usize) -> String {
    let lower = description.to_lowercase();
    for (id, markers) in [
        ("beans", &["원두", "coffee bean", "beans"][..]),
        ("cup", &["컵", " cup"][..]),
        ("tumbler", &["텀블러", "tumbler"][..]),
        ("lemon", &["레몬", "lemon"][..]),
        ("speaker", &["스피커", "speaker"][..]),
        ("package_box", &["패키지 박스", "package box"][..]),
        ("stickers", &["스티커", "sticker"][..]),
        ("mascot", &["마스코트", "mascot"][..]),
        ("character", &["캐릭터", "character"][..]),
        ("person", &["모델", "인물", "사람", "person", "model"][..]),
        ("bottle", &["병", "bottle"][..]),
        ("can", &["캔", " can"][..]),
        ("box", &["상자", "박스", " box"][..]),
    ] {
        if markers.iter().any(|marker| lower.contains(marker)) {
            return id.to_owned();
        }
    }
    format!("subject_{}", index + 1)
}

fn split_korean_subject_conjunctions(value: &str) -> String {
    let mut output = value.to_owned();
    let mut search_from = 0;
    loop {
        let next = ["와 ", "과 "]
            .into_iter()
            .filter_map(|separator| {
                output[search_from..]
                    .find(separator)
                    .map(|relative| (search_from + relative, separator))
            })
            .min_by_key(|(index, _)| *index);
        let Some((index, separator)) = next else {
            break;
        };
        let left_start = output[..index]
            .rfind([',', '|'])
            .map_or(0, |boundary| boundary + 1);
        let right_start = index + separator.len();
        let right_end = output[right_start..]
            .find([',', '|'])
            .map_or(output.len(), |boundary| right_start + boundary);
        let left = &output[left_start..index];
        let right = &output[right_start..right_end];
        if ends_with_subject_marker(left) && contains_subject_marker(right) {
            output.replace_range(index..right_start, "|");
            search_from = index + 1;
        } else {
            search_from = right_start;
        }
    }
    output
}

fn split_english_subject_conjunctions(value: &str) -> String {
    let mut output = value.to_owned();
    let mut search_from = 0;
    loop {
        let lower = output.to_lowercase();
        let Some(relative) = lower[search_from..].find(" and ") else {
            break;
        };
        let index = search_from + relative;
        let right_start = index + " and ".len();
        let left_start = output[..index]
            .rfind([',', '|'])
            .map_or(0, |boundary| boundary + 1);
        let right_end = output[right_start..]
            .find([',', '|'])
            .map_or(output.len(), |boundary| right_start + boundary);
        let left = &output[left_start..index];
        let right = &output[right_start..right_end];
        if ends_with_subject_marker(left) && contains_subject_marker(right) {
            output.replace_range(index..right_start, "|");
            search_from = index + 1;
        } else {
            search_from = right_start;
        }
    }
    output
}

fn ends_with_subject_marker(value: &str) -> bool {
    let normalized = value
        .trim()
        .trim_end_matches(|character: char| {
            character.is_ascii_digit()
                || character.is_whitespace()
                || matches!(character, '.' | ',' | ':' | ';' | '-' | '_')
        })
        .to_lowercase();
    SUBJECT_MARKERS
        .iter()
        .copied()
        .any(|marker| normalized.ends_with(marker))
}

fn korean_counter_after(value: &str) -> Option<(usize, &'static str)> {
    let whitespace = value.len() - value.trim_start().len();
    let suffix = &value[whitespace..];
    [
        "마리", "켤레", "개", "알", "명", "종", "대", "병", "장", "점", "권", "벌",
    ]
    .into_iter()
    .find(|counter| suffix.starts_with(counter))
    .map(|counter| (whitespace + counter.len(), counter))
}

fn korean_age_decade(value: &str, digit_end: usize, counter_length: usize, counter: &str) -> bool {
    if counter != "대" {
        return false;
    }
    let following = value[digit_end + counter_length..].trim_start();
    [
        "초반", "중반", "후반", "성인", "남성", "여성", "모델", "인물", "사람",
    ]
    .iter()
    .any(|marker| following.starts_with(marker))
}

fn is_english_numeric_count(value: &str, start: usize, end: usize, count: u16) -> bool {
    let previous_words = ascii_words(&value[..start]);
    let following_words = ascii_words(&value[end..]);
    let suffix_count = previous_words
        .last()
        .is_some_and(|word| is_english_subject_noun(word));
    if suffix_count {
        return true;
    }

    let prefix_allowed = previous_words.last().is_none_or(|word| {
        matches!(
            word.as_str(),
            "exactly" | "about" | "add" | "show" | "place" | "render" | "include" | "with"
        )
    });
    if !prefix_allowed {
        return false;
    }
    following_words.iter().take(4).any(|word| {
        is_english_subject_noun(word) && (count == 1 || is_english_plural_count_noun(word))
    })
}

fn ascii_words(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn is_english_subject_noun(word: &str) -> bool {
    matches!(
        word,
        "subject"
            | "subjects"
            | "model"
            | "models"
            | "person"
            | "people"
            | "cat"
            | "cats"
            | "animal"
            | "animals"
            | "product"
            | "products"
            | "bottle"
            | "bottles"
            | "can"
            | "cans"
            | "cup"
            | "cups"
            | "tumbler"
            | "tumblers"
            | "lemon"
            | "lemons"
            | "bean"
            | "beans"
            | "shoe"
            | "shoes"
            | "bag"
            | "bags"
            | "car"
            | "cars"
            | "watch"
            | "watches"
            | "speaker"
            | "speakers"
            | "package"
            | "packages"
            | "box"
            | "boxes"
            | "icon"
            | "icons"
            | "character"
            | "characters"
            | "mascot"
            | "mascots"
            | "sticker"
            | "stickers"
            | "panel"
            | "panels"
            | "robot"
            | "robots"
    )
}

fn is_english_plural_count_noun(word: &str) -> bool {
    word == "people" || word.ends_with('s')
}

fn extract_grid(value: &str) -> Option<String> {
    let chars = value.char_indices().collect::<Vec<_>>();
    for (index, (_, ch)) in chars.iter().enumerate() {
        if !matches!(ch, 'x' | 'X' | '×') || index == 0 || index + 1 >= chars.len() {
            continue;
        }
        let (left_start, _) = chars[..index]
            .iter()
            .rev()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .copied()?;
        let right_start = chars[index + 1].0;
        let right_end = chars[index + 1..]
            .iter()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .map_or(right_start, |(offset, ch)| offset + ch.len_utf8());
        if left_start < chars[index].0 && right_end > right_start {
            return Some(format!("{} grid", &value[left_start..right_end]));
        }
    }
    None
}

fn grid_instance_count(value: &str) -> Option<u16> {
    let chars = value.char_indices().collect::<Vec<_>>();
    for (index, (_, ch)) in chars.iter().enumerate() {
        if !matches!(ch, 'x' | 'X' | '×') || index == 0 || index + 1 >= chars.len() {
            continue;
        }
        let (left_start, _) = chars[..index]
            .iter()
            .rev()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .copied()?;
        let separator_start = chars[index].0;
        let right_start = chars[index + 1].0;
        let right_end = chars[index + 1..]
            .iter()
            .take_while(|(_, ch)| ch.is_ascii_digit())
            .last()
            .map_or(right_start, |(offset, ch)| offset + ch.len_utf8());
        if left_start >= separator_start || right_end <= right_start {
            continue;
        }
        let rows = value[left_start..separator_start].parse::<u16>().ok()?;
        let columns = value[right_start..right_end].parse::<u16>().ok()?;
        return rows.checked_mul(columns);
    }
    None
}

fn replace_ascii_case_insensitive(value: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return value.to_owned();
    }
    let mut output = value.to_owned();
    loop {
        let lower = output.to_lowercase();
        let Some(index) = lower.find(&needle.to_lowercase()) else {
            break;
        };
        output.replace_range(index..index + needle.len(), replacement);
    }
    output
}

fn finish_terms(value: &str) -> Vec<(&'static str, &'static str)> {
    let lower = value.to_lowercase();
    let mut terms = Vec::new();
    for (canonical, aliases) in [
        ("무광", &["무광택", "무광"][..]),
        ("유광", &["고광택", "유광", "광택"][..]),
        ("새틴", &["새틴"][..]),
        ("브러시드", &["브러시드", "헤어라인"][..]),
        ("matte", &["matte", "matt finish"][..]),
        ("glossy", &["high-gloss", "high gloss", "glossy"][..]),
        ("satin", &["satin finish", "satin"][..]),
        ("brushed", &["brushed finish", "brushed"][..]),
    ] {
        if let Some(alias) = aliases.iter().find(|alias| lower.contains(**alias)) {
            terms.push((canonical, *alias));
        }
    }
    terms
}

fn extract_finish(value: &str) -> Option<String> {
    let values = finish_terms(value)
        .into_iter()
        .map(|(canonical, _)| canonical)
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join(", "))
}

fn remove_finish_terms(value: &str) -> String {
    let mut output = value.to_owned();
    for (_, alias) in finish_terms(value) {
        output = replace_ascii_case_insensitive(&output, alias, "");
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn material_terms(value: &str) -> Vec<&'static str> {
    let lower = value.to_lowercase();
    let mut materials = Vec::new();
    for (canonical, aliases) in [
        ("스테인리스 스틸", &["스테인리스 스틸", "스테인리스"][..]),
        ("알루미늄", &["알루미늄"][..]),
        ("도자기", &["도자기", "세라믹"][..]),
        ("유리", &["유리"][..]),
        ("가죽", &["가죽"][..]),
        ("종이", &["재생지", "종이"][..]),
        ("플라스틱", &["플라스틱"][..]),
        ("실리콘", &["실리콘"][..]),
        ("고무", &["고무"][..]),
        ("목재", &["목재", "나무"][..]),
        ("금속", &["금속"][..]),
        ("stainless steel", &["stainless steel"][..]),
        ("aluminum", &["aluminium", "aluminum"][..]),
        ("ceramic", &["ceramic", "porcelain"][..]),
        ("glass", &["glass"][..]),
        ("leather", &["leather"][..]),
        ("paper", &["recycled paper", "paper"][..]),
        ("plastic", &["plastic"][..]),
        ("silicone", &["silicone"][..]),
        ("rubber", &["rubber"][..]),
        ("wood", &["wooden", "wood"][..]),
        ("metal", &["metal"][..]),
    ] {
        if aliases.iter().any(|alias| lower.contains(*alias)) && !materials.contains(&canonical) {
            materials.push(canonical);
        }
    }
    materials
}

fn subject_after_background(value: &str) -> Option<String> {
    for marker in ["배경", "background"] {
        let lower = value.to_lowercase();
        let Some(marker_start) = lower.find(marker) else {
            continue;
        };
        let marker_end = marker_start + marker.len();
        let tail = &value[marker_end..];
        for separator in [
            " 위에 ",
            " 위 ",
            " 앞에 ",
            "에 놓인 ",
            "에 ",
            "에서 ",
            " with ",
            " featuring ",
        ] {
            if let Some(index) = tail.to_lowercase().find(separator) {
                let candidate = tail[index + separator.len()..].trim();
                if contains_subject_marker(candidate) {
                    let finish = extract_finish(&value[..marker_start]);
                    return Some(match finish {
                        Some(finish) => format!("{finish} {candidate}"),
                        None => candidate.to_owned(),
                    });
                }
            }
        }
    }
    None
}

pub(super) fn normalize_subject_clause(value: &str) -> String {
    let mut subject = value.trim().to_owned();
    if let Some(extracted) = subject_after_background(&subject) {
        subject = extracted;
    }
    for separator in ["없이 ", "without "] {
        if let Some(index) = subject.to_lowercase().rfind(separator) {
            subject = subject[index + separator.len()..].trim_start().to_owned();
        }
    }
    let lower = subject.to_lowercase();
    if (lower.contains("모델")
        || lower.contains("인물")
        || lower.contains("person")
        || lower.contains("model"))
        && let Some(index) = subject.find("의 ")
    {
        let suffix = subject[index + "의 ".len()..].to_lowercase();
        if contains_any(
            &suffix,
            &[
                "코트",
                "재킷",
                "블레이저",
                "수트",
                "니트",
                "셔츠",
                "드레스",
                "로브",
                "팬츠",
                "스커트",
                "리조트웨어",
                "라운지웨어",
                "wardrobe",
                "wearing",
            ],
        ) {
            subject.truncate(index);
        }
    }
    for suffix in [
        " 세로 화보",
        " 가로 화보",
        " 화보",
        " 룩북",
        " 에디토리얼",
        " 포스터",
        " editorial",
        " lookbook",
        " poster",
    ] {
        if subject.to_lowercase().ends_with(suffix) {
            let keep = subject.len().saturating_sub(suffix.len());
            subject.truncate(keep);
            subject = subject.trim_end().to_owned();
        }
    }
    subject
}

fn normalize_scene_clause(value: &str) -> String {
    let lower = value.to_lowercase();
    for marker in ["배경", "background"] {
        let Some(start) = lower.find(marker) else {
            continue;
        };
        let end = start + marker.len();
        if start > 0 {
            let mut scene = value[..end].trim().to_owned();
            if contains_subject_marker(&value[end..]) {
                scene = remove_finish_terms(&scene);
            }
            return scene;
        }
    }
    value.trim().to_owned()
}

pub(super) fn extract_scene_clause(value: &str) -> Option<String> {
    let markers = [
        "배경",
        "스튜디오",
        "골목",
        "거리",
        "도시",
        "실내",
        "야외",
        "해안",
        "테라스",
        "카페",
        "로스터리",
        "무대",
        "숲",
        "바다",
        "밤",
        "낮",
        "아침",
        "저녁",
        "새벽",
        "비",
        "눈",
        "안개",
        "background",
        "studio",
        "street",
        "alley",
        "city",
        "indoor",
        "outdoor",
        "night",
        "morning",
        "rain",
        "fog",
        "coast",
        "terrace",
    ];
    let matches = split_clauses(value)
        .into_iter()
        .filter(|clause| {
            let lower = clause.to_lowercase();
            markers.iter().any(|marker| lower.contains(marker))
        })
        .map(|clause| normalize_scene_clause(&clause))
        .filter(|clause| !clause.is_empty())
        .take(2)
        .collect::<Vec<_>>();
    (!matches.is_empty()).then(|| matches.join(", "))
}

pub(super) fn extract_composition_clause(value: &str) -> Option<String> {
    let strong_markers = [
        "구도",
        "여백",
        "중앙",
        "배치",
        "전신",
        "반신",
        "클로즈업",
        "아이레벨",
        "로우앵글",
        "하이앵글",
        "%",
        "composition",
        "negative space",
        "centered",
        "close-up",
        "full body",
        "eye level",
    ];
    split_clauses(value).into_iter().find(|clause| {
        let lower = clause.to_lowercase();
        let strong = strong_markers.iter().any(|marker| lower.contains(marker));
        let directional_placement = contains_any(
            &lower,
            &[
                "왼쪽",
                "오른쪽",
                "상단",
                "하단",
                "left",
                "right",
                "top",
                "bottom",
            ],
        ) && contains_any(
            &lower,
            &[
                "피사체",
                "제품",
                "인물",
                "배치",
                "위치",
                "놓",
                "subject",
                "product",
                "placed",
                "positioned",
            ],
        );
        let lighting_clause = contains_any(
            &lower,
            &[
                "광원",
                "광선",
                "일광",
                "창가",
                "창문광",
                "확산광",
                "측광",
                "역광",
                "조명",
                "빛",
                "그림자",
                "light",
                "lighting",
                "shadow",
                "window",
            ],
        );
        strong || (directional_placement && !lighting_clause)
    })
}

pub(super) fn extract_surface_clause(value: &str) -> Option<String> {
    split_clauses(value).into_iter().find_map(|clause| {
        let lower = clause.to_lowercase();
        let materials = material_terms(&clause);
        let finish = extract_finish(&clause);
        let explicit_surface = contains_any(
            &lower,
            &[
                "질감", "표면", "원단", "마감", "texture", "surface", "fabric", "finish",
            ],
        );
        if finish.is_none() && materials.is_empty() && !explicit_surface {
            return None;
        }
        // A finish-only background describes the set, not the product. Only
        // promote it to the primary surface when the same clause names a subject.
        if materials.is_empty()
            && !explicit_surface
            && !contains_subject_marker(&clause)
            && contains_any(&lower, &["배경", "background"])
        {
            return None;
        }
        let mut observations = materials
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if let Some(finish) = finish {
            observations.push(finish);
        }
        (!observations.is_empty()).then(|| observations.join(", "))
    })
}

pub(super) fn extract_lighting_clause(value: &str) -> Option<String> {
    split_clauses(value).into_iter().find(|clause| {
        let lower = clause.to_lowercase();
        contains_any(
            &lower,
            &[
                "광원",
                "광선",
                "일광",
                "창가",
                "창문광",
                "확산광",
                "측광",
                "역광",
                "조명",
                "빛",
                "그림자",
                "light",
                "lighting",
                "shadow",
                "window",
                "golden hour",
            ],
        ) && !contains_any(
            &lower,
            &[
                "없", "않", "금지", "제외", "without", "avoid", "exclude", "no light",
            ],
        )
    })
}

pub(super) fn surface_fields(value: &str, category: &str) -> (String, String) {
    let materials = material_terms(value);
    let material = if materials.is_empty() || materials.len() > 2 {
        match category {
            "C1" => "요청에 특정 재료가 명시되지 않음; 피부와 의상 표면을 자연스럽게 구분",
            "C4" => "요청에 특정 재료가 명시되지 않은 일관된 제품 소재",
            _ => "요청에 특정 재료가 명시되지 않은 일관된 주 표면",
        }
        .to_owned()
    } else {
        materials.join(", ")
    };
    let finish =
        extract_finish(value).unwrap_or_else(|| "요청된 재료의 고유한 표면 마감".to_owned());
    (material, finish)
}

pub(super) fn extract_wardrobe_clause(value: &str) -> Option<String> {
    let markers = [
        "코트",
        "재킷",
        "블레이저",
        "수트",
        "니트",
        "셔츠",
        "드레스",
        "로브",
        "팬츠",
        "스커트",
        "리조트웨어",
        "라운지웨어",
        "의상",
        "착용",
        "coat",
        "jacket",
        "blazer",
        "suit",
        "dress",
        "robe",
        "pants",
        "skirt",
        "wardrobe",
        "wearing",
    ];
    split_clauses(value).into_iter().find(|clause| {
        let lower = clause.to_lowercase();
        markers.iter().any(|marker| lower.contains(marker))
    })
}

pub(super) fn wardrobe_is_specific(value: &str) -> bool {
    let lower = value.to_lowercase();
    let has_garment = contains_any(
        &lower,
        &[
            "코트",
            "재킷",
            "블레이저",
            "수트",
            "니트",
            "셔츠",
            "드레스",
            "로브",
            "팬츠",
            "슬랙스",
            "스커트",
            "터틀넥",
            "보디스",
            "리조트웨어",
            "라운지웨어",
            "coat",
            "jacket",
            "blazer",
            "suit",
            "knit",
            "shirt",
            "dress",
            "robe",
            "pants",
            "slacks",
            "skirt",
            "bodice",
        ],
    );
    let axes = [
        contains_any(
            &lower,
            &[
                "아이보리",
                "크림",
                "흰",
                "검정",
                "블랙",
                "네이비",
                "갈색",
                "브라운",
                "세이지",
                "베이지",
                "회색",
                "그레이",
                "빨강",
                "레드",
                "파랑",
                "블루",
                "초록",
                "그린",
                "ivory",
                "cream",
                "black",
                "navy",
                "brown",
                "sage",
                "beige",
                "gray",
                "grey",
                "red",
                "blue",
                "green",
                "#",
            ],
        ),
        contains_any(
            &lower,
            &[
                "울",
                "니트",
                "리넨",
                "린넨",
                "새틴",
                "실크",
                "면",
                "데님",
                "가죽",
                "트위드",
                "코튼",
                "wool",
                "linen",
                "satin",
                "silk",
                "cotton",
                "denim",
                "leather",
                "tweed",
            ],
        ),
        contains_any(
            &lower,
            &[
                "오버사이즈",
                "여유로운",
                "슬림",
                "테일러드",
                "구조적",
                "하이웨이스트",
                "발목 길이",
                "무릎 길이",
                "롱",
                "와이드",
                "핏",
                "재단",
                "oversized",
                "relaxed",
                "slim",
                "tailored",
                "structured",
                "high-waist",
                "ankle-length",
                "wide-leg",
                "fit",
                "cut",
            ],
        ),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    has_garment && axes >= 2
}

pub(super) fn infer_lighting(value: &str) -> Option<&'static str> {
    let lower = split_clauses(value)
        .into_iter()
        .map(|clause| clause.to_lowercase())
        .filter(|clause| {
            !contains_any(
                clause,
                &[
                    "없",
                    "않",
                    "금지",
                    "제외",
                    "피하",
                    "과도",
                    "no ",
                    "not ",
                    "without",
                    "avoid",
                    "exclude",
                    "excessive",
                ],
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    if contains_any(&lower, &["네온", "청록", "마젠타", "neon"]) {
        Some("neon_practical")
    } else if contains_any(
        &lower,
        &["골든아워", "석양", "해질녘", "golden hour", "sunset"],
    ) {
        Some("golden_hour")
    } else if contains_any(
        &lower,
        &["저조도", "로우키", "어두운 조명", "low key", "noir"],
    ) {
        Some("low_key")
    } else if contains_any(&lower, &["하드 라이트", "날카로운 그림자", "hard light"]) {
        Some("hard_graphic")
    } else if contains_any(&lower, &["확산광", "창가", "부드러운 빛", "soft daylight"]) {
        Some("soft_daylight")
    } else {
        None
    }
}

pub(super) fn infer_text_position(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    for (zone, terms) in [
        ("top_left", &["좌상", "왼쪽 상단", "top left"][..]),
        ("top_center", &["상단 중앙", "top center"][..]),
        ("top_right", &["우상", "오른쪽 상단", "top right"][..]),
        ("middle_left", &["가운데 왼쪽", "middle left"][..]),
        ("center", &["정중앙", "중앙 배치", "centered text"][..]),
        ("middle_right", &["가운데 오른쪽", "middle right"][..]),
        ("bottom_left", &["좌하", "왼쪽 하단", "bottom left"][..]),
        ("bottom_center", &["하단 중앙", "bottom center"][..]),
        ("bottom_right", &["우하", "오른쪽 하단", "bottom right"][..]),
    ] {
        if terms.iter().any(|term| lower.contains(term)) {
            return Some(zone);
        }
    }
    None
}

pub(super) fn infer_text_style(value: &str) -> Option<&'static str> {
    let lower = value.to_lowercase();
    if contains_any(&lower, &["콘덴스드", "좁은 산세리프", "condensed"]) {
        Some("condensed")
    } else if contains_any(&lower, &["디돈", "고대비 세리프", "didone"]) {
        Some("didone")
    } else if contains_any(&lower, &["모노", "타자기", "monospace"]) {
        Some("mono")
    } else if contains_any(&lower, &["붓글씨", "브러시", "brush"]) {
        Some("brush")
    } else if contains_any(&lower, &["산세리프", "기하", "sans", "geometric"]) {
        Some("geometric")
    } else {
        None
    }
}

pub(super) fn extract_hex_palette(value: &str) -> Option<Vec<String>> {
    let bytes = value.as_bytes();
    let mut colors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut index = 0;
    while index + 7 <= bytes.len() {
        if bytes[index] == b'#'
            && bytes[index + 1..index + 7]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            let color = value[index..index + 7].to_ascii_uppercase();
            if seen.insert(color.clone()) {
                colors.push(color);
            }
            index += 7;
        } else {
            index += 1;
        }
    }
    (3..=5).contains(&colors.len()).then_some(colors)
}

pub(super) fn extract_quoted_texts(value: &str) -> Vec<String> {
    let mut texts = Vec::new();
    let mut seen = BTreeSet::new();
    let mut active: Option<(char, usize)> = None;
    for (index, character) in value.char_indices() {
        if let Some((close, start)) = active {
            if character == close {
                let text = value[start..index].trim();
                if !text.is_empty() && seen.insert(text.to_owned()) {
                    texts.push(text.to_owned());
                }
                active = None;
            }
        } else {
            let close = match character {
                '"' => Some('"'),
                '“' => Some('”'),
                '‘' => Some('’'),
                '「' => Some('」'),
                '『' => Some('』'),
                _ => None,
            };
            if let Some(close) = close {
                active = Some((close, index + character.len_utf8()));
            }
        }
    }
    texts
}

pub(super) fn default_composition(
    category: &str,
    has_text: bool,
    brief: &str,
    language: &str,
) -> String {
    let lower = brief.to_lowercase();
    if lower.contains("클로즈업") || lower.contains("close-up") {
        return "주 피사체 얼굴 또는 제품 전면이 캔버스 높이의 72%, 시선·기능 면에 초점, 후경은 18% 이하 정보 밀도".to_owned();
    }
    if category == "C4" && contains_any(&lower, &["세로", "vertical", "portrait orientation"]) {
        return "세로 캔버스 중앙의 단일 제품 3/4 히어로 구도, 전면과 측면 재질이 함께 보이며 상하 여백을 균형 있게 유지"
            .to_owned();
    }
    if category == "C10"
        && contains_any(
            &lower,
            &[
                "패널",
                "스티커",
                "그리드",
                "스토리보드",
                "panel",
                "sticker",
                "grid",
                "storyboard",
            ],
        )
    {
        let layout = extract_grid(brief).unwrap_or_else(|| "지정된 패널 grid".to_owned());
        return if language == "en" {
            format!(
                "Arrange every cell in the {layout} at equal size with uniform gutters; assign exactly one action to each cell in reading order"
            )
        } else {
            format!(
                "{layout}의 모든 셀을 균일한 크기와 동일한 여백으로 배치하고, 각 셀에는 읽기 순서대로 한 가지 행동만 배정"
            )
        };
    }
    let base = match category {
        "C1" => {
            "단독 인물 전신 또는 3/4 신체가 캔버스 높이의 68%, 아이레벨 세로 구도, 상하 여백 합계 18%, 실루엣과 의상선이 겹치지 않음"
        }
        "C4" => {
            "제품 또는 패키지가 중앙 52%, 재질과 기능 면이 동시에 보이는 3/4 구도, 좌우 18%는 브랜드 적용 또는 콜아웃 안전 영역"
        }
        "C5" => {
            "헤드라인이 1순위, 히어로가 2순위인 비대칭 캠페인 그리드, 바깥 여백 8%, 문구와 피사체 경계 4% 이상 분리"
        }
        "C6" => {
            "12열 그리드, 바깥 여백 6%, 제목 1개와 핵심 정보 블록 3개 이하, 좌상단에서 우하단으로 읽는 단일 위계"
        }
        "C10" if language == "en" => {
            "Separate the main subject and background in a flat front-facing illustration"
        }
        "C10" => "단일 일러스트의 주 피사체와 배경을 정면 평면 구도로 분리",
        "C11" => {
            "주 피사체가 높이의 64%, 중심보다 6% 한쪽으로 이동, 반대쪽 28%는 타이틀 또는 환경 서사를 위한 네거티브 스페이스"
        }
        _ if has_text => {
            "주 피사체가 높이의 56%, 중심보다 8% 오른쪽, 좌상단 30%는 정확 문구용 네거티브 스페이스, 읽기 순서는 문구→피사체→보조 정보"
        }
        _ => {
            "주 피사체가 높이의 62%, 중심보다 3% 오른쪽, 상단 22%는 비우고 배경 정보 밀도는 피사체의 절반 이하"
        }
    };
    base.to_owned()
}

pub(super) fn default_lighting(category: &str, brief: &str) -> String {
    if let Some(value) = infer_lighting(brief) {
        return value.to_owned();
    }
    match category {
        "C5" | "C6" => "hard_graphic".to_owned(),
        "C11" => "low_key".to_owned(),
        "C10" => "hard_graphic".to_owned(),
        _ => "soft_daylight".to_owned(),
    }
}

pub(super) fn default_surface(category: &str, language: &str) -> String {
    let base = match category {
        "C1" => "자연스러운 피부 결, 보이는 모공, 의상 원단의 직조와 재단선, 미세한 필름 그레인",
        "C4" => {
            "제품과 패키지 재료별 표면 차이, 단면 가장자리와 조립 틈, 하이라이트 폭으로 구분되는 금속·플라스틱·종이·유리"
        }
        "C5" | "C6" => {
            "인쇄 또는 화면 그래픽의 평면 마감, 레이어 경계와 정보 위계를 분리하는 균일한 잉크·픽셀 가장자리"
        }
        "C10" if language == "en" => {
            "consistent line weight with ink or brush texture, crisp color-plane boundaries, and stable tone across panels"
        }
        "C10" => "선 두께가 일정한 잉크 또는 브러시 질감, 면색 경계가 또렷하고 패널 간 톤이 일관됨",
        "C11" => {
            "피부·직물·금속·환경 표면이 서로 다른 반사 폭을 가지며, 어두운 영역에도 미세 재질층이 남음"
        }
        _ => {
            "주 피사체 재료를 식별할 수 있는 미세 요철, 가장자리 마감, 곡률에 맞는 연속 하이라이트와 접점 그림자"
        }
    };
    base.to_owned()
}

pub(super) fn fallback_palette(category: &str) -> Vec<String> {
    let values: &[&str] = match category {
        "C1" => &["#F5F0E8", "#D8CBB8", "#8A7A66", "#2E2A26"],
        "C4" => &["#F4F5F6", "#7C8794", "#2D333A", "#D18B47"],
        "C5" => &["#F1EAD8", "#7A8450", "#111111", "#D8442E"],
        "C6" => &["#F7F8FA", "#243447", "#4F7CAC", "#E2A93B"],
        "C10" => &["#FFF7E8", "#E85D4A", "#2F6FA3", "#20242A"],
        "C11" => &["#0E1420", "#D9A566", "#4A6670", "#ECE7DD"],
        _ => &["#F2F2EF", "#6B655F", "#1A1A1A"],
    };
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(super) fn default_aspect_ratio(category: &str, brief: &str) -> String {
    if let Some(ratio) = find_ratio(brief) {
        return ratio;
    }
    let lower = brief.to_lowercase();
    if contains_any(&lower, &["세로", "vertical", "portrait orientation"]) {
        return "2:3".to_owned();
    }
    if contains_any(&lower, &["가로", "horizontal", "landscape orientation"]) {
        return "3:2".to_owned();
    }
    if category == "C1" && contains_any(&lower, &["화보", "에디토리얼", "룩북", "editorial"])
    {
        return "2:3".to_owned();
    }
    match category {
        "C1" | "C5" | "C10" => "2:3".to_owned(),
        "C4" => "3:2".to_owned(),
        "C6" | "C11" => "16:9".to_owned(),
        _ => "1:1".to_owned(),
    }
}

pub(super) fn default_lut(brief: &str) -> (&'static str, &'static str) {
    let lower = brief.to_lowercase();
    if contains_any(
        &lower,
        &[
            "모노크롬 하이 콘트라스트",
            "고대비 흑백",
            "monochrome high contrast",
            "black and white",
            "흑백",
        ],
    ) {
        return ("monochrome_high_contrast", "요청의 고대비 흑백 표현");
    }
    if contains_any(&lower, &["블리치 바이패스", "bleach bypass"]) {
        return ("bleach_bypass", "요청에 명시된 저채도 고대비 영화 색감");
    }
    if contains_any(
        &lower,
        &[
            "절제된 틸 오렌지",
            "틸 오렌지",
            "restrained teal orange",
            "teal orange",
        ],
    ) {
        return (
            "restrained_teal_orange",
            "요청에 명시된 청록 그림자·따뜻한 하이라이트 분리",
        );
    }
    if contains_any(&lower, &["쿨 스틸", "cool steel"]) {
        return ("cool_steel", "요청에 명시된 차가운 기술 색감");
    }
    if contains_any(&lower, &["페이디드 프린트", "바랜 인쇄", "faded print"]) {
        return ("faded_print", "요청에 명시된 바랜 인쇄 색감");
    }
    if contains_any(&lower, &["웜 파스텔 필름", "따뜻한 파스텔", "warm pastel"]) {
        return (
            "warm_pastel_filmic",
            "요청에 명시된 따뜻한 파스텔 필름 색감",
        );
    }
    if contains_any(&lower, &["클린 뉴트럴", "clean neutral"]) {
        return ("clean_neutral", "요청에 명시된 중립 색 재현");
    }
    if contains_any(&lower, &["텅스텐", "야간", "야경", "밤", "전구", "night"]) {
        return ("tungsten_night", "요청의 야간·텅스텐 조명");
    }
    ("clean_neutral", "제품·재질의 중립 색을 보존하는 기본값")
}

pub(super) fn default_exclusions(has_text: bool) -> String {
    if has_text {
        "지정한 주 피사체와 필수 소품만 화면에 존재\n브랜드 표식이 없는 표면\n읽을 수 있는 문자는 사용자가 지정한 정확 문구의 각 줄만 순서대로 한 번씩 존재\n배경은 구조화된 장면의 요소만 유지"
            .to_owned()
    } else {
        "지정한 주 피사체와 필수 소품만 화면에 존재\n브랜드 표식이 없는 표면\n읽을 수 있는 문자가 없는 깨끗한 이미지\n배경은 구조화된 장면의 요소만 유지"
            .to_owned()
    }
}

pub(super) fn find_ratio(value: &str) -> Option<String> {
    for token in
        value.split(|c: char| c.is_whitespace() || matches!(c, ',' | '/' | '(' | ')' | '[' | ']'))
    {
        if aspect_ratio_preset(token).is_some() {
            return Some(token.to_owned());
        }
    }
    None
}

pub(super) fn selected_safety_tier(category: &str, has_text: bool) -> u8 {
    let minimum = find_entry(category)
        .ok()
        .flatten()
        .map_or(0, |entry| entry.default_safety_tier);
    minimum.max(u8::from(has_text))
}

pub(super) fn parse_hex_palette(value: &str) -> Option<Vec<String>> {
    let mut colors = Vec::new();
    let mut unique = BTreeSet::new();
    for token in value
        .split([',', ' ', '\n'])
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let normalized = token.to_ascii_uppercase();
        if normalized.len() != 7
            || !normalized.starts_with('#')
            || !normalized[1..]
                .chars()
                .all(|character| character.is_ascii_hexdigit())
            || !unique.insert(normalized.clone())
        {
            return None;
        }
        colors.push(normalized);
    }
    (3..=5).contains(&colors.len()).then_some(colors)
}

pub(super) fn dimensions(value: &str) -> (u32, u32) {
    aspect_ratio_preset(value)
        .map(|preset| (preset.width, preset.height))
        .unwrap_or((1024, 1024))
}

pub(super) fn aspect_ratio_preset(value: &str) -> Option<&'static AspectRatioPreset> {
    ASPECT_RATIO_PRESETS
        .iter()
        .find(|preset| preset.id == value)
}

pub(super) fn proportions(count: usize) -> Vec<u8> {
    match count {
        3 => vec![45, 35, 20],
        4 => vec![35, 30, 20, 15],
        5 => vec![30, 25, 20, 15, 10],
        _ => Vec::new(),
    }
}

pub(super) fn string_array(value: &str) -> JsonValue {
    JsonValue::array(split_nonempty(value).into_iter().map(JsonValue::from))
}

pub(super) fn split_nonempty(value: &str) -> Vec<String> {
    value
        .split(['\n', ';'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn contains_any(value: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| value.contains(term))
}

pub(super) fn is_hangul(character: char) -> bool {
    matches!(character as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF)
}
