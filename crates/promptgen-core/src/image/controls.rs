//! Provider-independent visual vocabulary. Slash tokens are authoring shorthand;
//! only the expanded, observable descriptions reach an image provider.
use super::{ImagePromptRequest, VisualMedium};
use crate::{common::PromptLanguage, diagnostic::Diagnostic};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisualControl {
    Isometric,
    TopDown,
    Macro,
    FullBody,
    CloseUp,
    WideShot,
    Headshot,
    LowAngle,
    HighAngle,
    ShallowDepth,
    DeepFocus,
    Cutaway,
    ExplodedView,
    Watercolor,
    Ink,
    Clay,
    Blueprint,
    SoftLight,
    GoldenHour,
    TravelJournal,
    Scrapbook,
    Risograph,
    Handwritten,
}

impl VisualControl {
    pub const ALL: [Self; 23] = [
        Self::TravelJournal,
        Self::Scrapbook,
        Self::Risograph,
        Self::Handwritten,
        Self::Isometric,
        Self::TopDown,
        Self::Macro,
        Self::FullBody,
        Self::CloseUp,
        Self::WideShot,
        Self::Headshot,
        Self::LowAngle,
        Self::HighAngle,
        Self::ShallowDepth,
        Self::DeepFocus,
        Self::Cutaway,
        Self::ExplodedView,
        Self::Watercolor,
        Self::Ink,
        Self::Clay,
        Self::Blueprint,
        Self::SoftLight,
        Self::GoldenHour,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Isometric => "isometric",
            Self::TopDown => "topdown",
            Self::Macro => "macro",
            Self::FullBody => "fullbody",
            Self::CloseUp => "closeup",
            Self::WideShot => "wideshot",
            Self::Headshot => "headshot",
            Self::LowAngle => "lowangle",
            Self::HighAngle => "highangle",
            Self::ShallowDepth => "shallowdepth",
            Self::DeepFocus => "deepfocus",
            Self::Cutaway => "cutaway",
            Self::ExplodedView => "explodedview",
            Self::Watercolor => "watercolor",
            Self::Ink => "ink",
            Self::Clay => "clay",
            Self::Blueprint => "blueprint",
            Self::SoftLight => "softlight",
            Self::GoldenHour => "goldenhour",
            Self::TravelJournal => "travel-journal",
            Self::Scrapbook => "scrapbook",
            Self::Risograph => "risograph",
            Self::Handwritten => "handwritten",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL.into_iter().find(|control| control.as_str() == value)
            .ok_or_else(|| format!("unknown visual control {value:?}; use an explicit visual description or a supported control"))
    }

    const fn axes(self) -> u8 {
        match self {
            Self::Isometric | Self::TopDown | Self::LowAngle | Self::HighAngle => 1,
            Self::Macro => 2 | 32,
            Self::FullBody | Self::CloseUp | Self::WideShot | Self::Headshot => 2,
            Self::Cutaway | Self::ExplodedView => 4,
            Self::Watercolor | Self::Ink | Self::Clay | Self::Blueprint | Self::Risograph => 8,
            Self::SoftLight | Self::GoldenHour => 16,
            Self::ShallowDepth | Self::DeepFocus => 32,
            Self::Scrapbook => 64,
            Self::TravelJournal => 128,
            Self::Handwritten => 0,
        }
    }

    pub const fn medium(self) -> Option<VisualMedium> {
        match self {
            Self::Watercolor | Self::Ink | Self::Risograph => Some(VisualMedium::Illustration),
            Self::Clay => Some(VisualMedium::ThreeD),
            Self::Blueprint => Some(VisualMedium::GraphicDesign),
            Self::Macro => Some(VisualMedium::Photo),
            _ => None,
        }
    }

    pub const fn controls_camera(self) -> bool {
        self.axes() & (1 | 2 | 32) != 0
    }

    pub const fn lighting_key(self) -> Option<&'static str> {
        match self {
            Self::SoftLight => Some("soft_daylight"),
            Self::GoldenHour => Some("golden_hour"),
            _ => None,
        }
    }

    /// Expands into the existing domain slots. No parallel prompt section or
    /// provider command survives this authoring step.
    pub fn apply(self, request: &mut ImagePromptRequest) {
        let description = self.description(request.language).to_owned();
        match self {
            Self::Isometric | Self::TopDown | Self::LowAngle | Self::HighAngle => {
                request.composition.viewpoint = description.clone();
                request.composition.camera_angle = description;
                request.composition.framing = "Include the complete named subject with clear margins; preserve its declared placement".into();
                if let Some(camera) = &mut request.camera {
                    camera.perspective = request.composition.viewpoint.clone();
                }
            }
            Self::Macro => {
                request.composition.framing = description.clone();
                if let Some(camera) = &mut request.camera {
                    camera.field_of_view = description;
                    camera.depth_of_field = "Only the specified surface lies in the focal plane; the background is softly separated".into();
                    camera.focus = "The named subject's specified material surface".into();
                }
            }
            Self::FullBody | Self::CloseUp | Self::WideShot | Self::Headshot => {
                request.composition.framing = description.clone();
                if let Some(camera) = &mut request.camera {
                    camera.field_of_view = description;
                }
            }
            Self::ShallowDepth | Self::DeepFocus => {
                let camera = request.camera.get_or_insert_with(|| super::CameraSpec {
                    field_of_view: request.composition.framing.clone(),
                    perspective: request.composition.viewpoint.clone(),
                    depth_of_field: String::new(),
                    focus: request
                        .subjects
                        .iter()
                        .map(|subject| subject.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    motion_rendering: "still image".into(),
                });
                camera.depth_of_field = description;
            }
            Self::Watercolor | Self::Ink | Self::Clay | Self::Blueprint | Self::Risograph => {
                if request.task_mode == super::ImageTaskMode::Edit {
                    request.constraints.required_elements.push(format!("Apply only to declared additions outside the preserved source photograph: {description}"));
                } else {
                    for subject in &mut request.subjects {
                        subject.distinguishing_features.push(description.clone());
                    }
                }
            }
            Self::Cutaway | Self::ExplodedView => {
                request.constraints.required_elements.push(description)
            }
            Self::TravelJournal | Self::Scrapbook => {
                request.constraints.required_elements.push(description)
            }
            Self::Handwritten => {
                for text in &mut request.text_elements {
                    text.font_family = "legible pen handwriting".into();
                    text.weight = "regular".into();
                    text.treatment = description.clone();
                }
            }
            // Lighting is resolved into all its typed properties by the interview.
            Self::SoftLight | Self::GoldenHour => {}
        }
    }

    pub fn description(self, language: PromptLanguage) -> &'static str {
        let pair = match self {
            Self::Isometric => (
                "등각 투영. 평행한 모서리는 평행하게 유지하고 세 축의 축척을 같게 한다.",
                "Isometric projection; parallel edges remain parallel and all three axes use equal scale.",
            ),
            Self::TopDown => (
                "피사체 바로 위에서 수직으로 내려다본다. 기울어진 수평선 없이 평면 배치를 보인다.",
                "Direct overhead view perpendicular to the subject; show its planar arrangement without a tilted horizon.",
            ),
            Self::Macro => (
                "접사 제품 사진. 지정한 피사체와 재질의 미세한 표면을 초점면에 두고 배경은 부드럽게 분리한다.",
                "Macro photograph; place the named subject and its fine surface texture in the focal plane with a softly separated background.",
            ),
            Self::FullBody => (
                "전신과 양발을 프레임 안에 완전히 포함한다. 머리와 발끝을 자르지 않는다.",
                "Include the complete body and both feet inside the frame; leave headroom and space below the feet.",
            ),
            Self::CloseUp => (
                "지정한 피사체를 크게 담은 근접 구도. 판독할 핵심 부분을 프레임 안에 유지한다.",
                "Close-up framing of the named subject; keep the identifying details within the frame.",
            ),
            Self::WideShot => (
                "피사체와 주변 공간을 함께 보이는 넓은 구도. 전경·중경·배경의 위치 관계를 유지한다.",
                "Wide framing that includes the subject and its surroundings; preserve foreground, middle-ground and background relationships.",
            ),
            Self::Headshot => (
                "머리와 어깨가 중심인 구도. 얼굴 전체와 눈의 방향이 선명하게 보인다.",
                "Head-and-shoulders framing with the full face and gaze clearly visible.",
            ),
            Self::LowAngle => (
                "피사체 아래에서 위로 올려다보는 시점. 지평선과 수직선의 원근을 일관되게 한다.",
                "Low camera position looking up at the subject; keep the horizon and vertical perspective coherent.",
            ),
            Self::HighAngle => (
                "피사체 위에서 비스듬히 내려다보는 시점. 수직 탑뷰와 구분되는 원근을 유지한다.",
                "Elevated camera looking diagonally down at the subject, with perspective distinct from a perpendicular overhead view.",
            ),
            Self::ShallowDepth => (
                "주 피사체만 선명한 얕은 심도. 배경은 부드럽게 흐리고 피사체 경계에 번짐을 만들지 않는다.",
                "Shallow depth of field with the main subject sharp and the background softly blurred, without haloing subject edges.",
            ),
            Self::DeepFocus => (
                "전경부터 배경까지 식별 가능한 깊은 초점. 공간의 주요 요소와 연결 관계를 선명하게 유지한다.",
                "Deep focus keeping foreground through background readable; preserve clear spatial elements and their relationships.",
            ),
            Self::Cutaway => (
                "외피 일부를 절개해 지정한 내부 구조를 보인다. 남은 외형과 연결 관계를 유지하며 숨은 부품은 발명하지 않는다.",
                "Remove a section of the outer shell to expose the specified internal structure; preserve the remaining geometry and connections without inventing hidden components.",
            ),
            Self::ExplodedView => (
                "지정한 부품을 조립 축에 따라 분리해 배열한다. 부품 수, 순서, 상대 크기와 맞물리는 위치를 유지한다.",
                "Separate the specified components along their assembly axis; preserve component count, order, relative scale and mating positions.",
            ),
            Self::Watercolor => (
                "수채화. 종이 결 위에 투명한 색 번짐과 겹친 안료층, 부드러운 젖은 가장자리를 표현한다.",
                "Watercolor on textured paper with translucent pigment washes, layered color and soft wet edges.",
            ),
            Self::Ink => (
                "잉크 드로잉. 선 굵기와 해칭 밀도로 명암을 구분하고 채색으로 형태를 덮지 않는다.",
                "Ink drawing; distinguish form through line weight and hatching density without covering the structure with painted fills.",
            ),
            Self::Clay => (
                "점토 3D. 손으로 빚은 둥근 덩어리, 미세한 눌림과 무광 표면, 일관된 접촉 그림자를 표현한다.",
                "Clay 3D rendering with hand-shaped rounded volumes, subtle impressions, matte surfaces and coherent contact shadows.",
            ),
            Self::Blueprint => (
                "기술 청사진. 균일한 선 굵기와 정렬된 구조를 사용한다. 명시되지 않은 치수·라벨·부품은 추가하지 않는다.",
                "Technical blueprint with uniform line weight and aligned geometry; add no unspecified dimensions, labels or components.",
            ),
            Self::SoftLight => (
                "하나의 큰 확산 광원. 그림자 경계는 부드럽게 하고 빛 방향과 표면 반사를 일치시킨다.",
                "One large diffused light source; soften shadow edges and align surface highlights with the light direction.",
            ),
            Self::GoldenHour => (
                "낮은 각도의 따뜻한 태양광. 길어진 그림자와 림 하이라이트를 하나의 광원 방향에 맞춘다.",
                "Low-angle warm sunlight; align elongated shadows and rim highlights with one consistent sun direction.",
            ),
            Self::TravelJournal => (
                "여행 기록 한 장. 사용자가 제공한 장소·날짜·메모만 쓴다. 확인되지 않은 지명·일정·주소·이동시간은 발명하지 않는다. 본문에 확정한 문구가 없으면 장식에도 글자를 넣지 않는다.",
                "One travel journal page. Use only supplied places, dates and notes; invent no destinations, schedules, addresses or travel times. When no exact copy is supplied, keep decorative elements text-free.",
            ),
            Self::Scrapbook => (
                "중심 사진과 주변 종이 조각을 비대칭 콜라주로 배치한다. 얇은 테이프·종이 가장자리·가벼운 접촉 그림자를 사용한다. 인물의 얼굴과 주요 장면을 가리지 않으며 지정하지 않은 티켓·우표의 글자와 숫자는 넣지 않는다.",
                "Asymmetric collage around the main photograph with thin tape, paper edges and light contact shadows. Keep faces and key scene details unobscured; add no unspecified lettering or numbers to tickets or stamps.",
            ),
            Self::Risograph => (
                "리소그래프 인쇄 표현. 제한된 색판, 미세한 잉크 입자와 종이 섬유를 사용한다. 문구 주변의 대비와 읽기 쉬운 가장자리를 유지한다.",
                "Risograph print treatment with limited color plates, fine ink grain and paper fibers; preserve contrast and clear edges around the exact copy.",
            ),
            Self::Handwritten => (
                "사용자가 확정한 문구만 읽기 쉬운 펜 손글씨로 쓴다. 획과 간격은 자연스럽게 하되 철자·언어·줄바꿈을 바꾸지 않는다.",
                "Render only the approved copy as legible pen handwriting, with natural stroke and spacing variation while preserving spelling, language and line breaks.",
            ),
        };
        match language {
            PromptLanguage::Korean => pair.0,
            PromptLanguage::English => pair.1,
        }
    }
}

pub fn validate_controls(controls: &[VisualControl], medium: VisualMedium) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (index, control) in controls.iter().enumerate() {
        let path = format!("$.visual_controls[{index}]");
        if controls[..index]
            .iter()
            .any(|previous| *previous == *control || previous.axes() & control.axes() != 0)
        {
            diagnostics.push(Diagnostic::error("IMG_CONTROL_CONFLICT", &path,
                "choose one visual control per axis; duplicate or competing controls are not combined"));
        }
        if control
            .medium()
            .is_some_and(|expected| expected != medium && medium != VisualMedium::Mixed)
        {
            diagnostics.push(Diagnostic::error(
                "IMG_CONTROL_MEDIUM",
                &path,
                "visual control conflicts with the declared medium",
            ));
        }
    }
    diagnostics
}

/// Only the leading slash block is shorthand. Quoted copy, URLs and body text
/// are preserved verbatim and never interpreted as commands.
pub fn split_visual_brief(brief: &str) -> Result<(Vec<VisualControl>, &str), String> {
    let mut rest = brief.trim();
    let mut controls = Vec::new();
    while rest.starts_with('/') {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        controls.push(VisualControl::parse(&rest[1..end].to_ascii_lowercase())?);
        rest = rest[end..].trim_start();
    }
    if !controls.is_empty() && rest.is_empty() {
        return Err("visual controls require a subject and intended result".into());
    }
    Ok((controls, rest))
}

pub fn controls_json() -> crate::json::JsonValue {
    use crate::json::JsonValue;
    JsonValue::array(VisualControl::ALL.into_iter().map(|control| {
        JsonValue::object([
            ("id", JsonValue::from(control.as_str())),
            (
                "description",
                JsonValue::from(control.description(PromptLanguage::Korean)),
            ),
        ])
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn syntax_does_not_reinterpret_copy_or_urls() {
        let (controls, body) = split_visual_brief("/isometric /cutaway 온실의 관수 구조").unwrap();
        assert_eq!(controls, [VisualControl::Isometric, VisualControl::Cutaway]);
        assert_eq!(body, "온실의 관수 구조");
        for value in [
            "문구는 \"/macro\"",
            "https://example.com/a",
            "주제 /isometric",
        ] {
            assert_eq!(split_visual_brief(value).unwrap(), (vec![], value));
        }
        assert!(split_visual_brief("/unknown 시계").is_err());
        assert!(split_visual_brief("/macro").is_err());
    }
    #[test]
    fn incompatible_controls_fail_closed() {
        assert!(
            !validate_controls(
                &[VisualControl::Macro, VisualControl::FullBody],
                VisualMedium::Photo
            )
            .is_empty()
        );
        assert!(!validate_controls(&[VisualControl::Watercolor], VisualMedium::Photo).is_empty());
        assert!(
            validate_controls(
                &[VisualControl::Isometric, VisualControl::Cutaway],
                VisualMedium::Illustration
            )
            .is_empty()
        );
    }
}
