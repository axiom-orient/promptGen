"use strict";

const state = {
  mode: "prompt-only",
  brief: "",
  explicitAnswers: {},
  normalizedAnswers: {},
  outcome: null,
  currentQuestion: null,
  busy: false,
  interviewRequestId: 0,
  generationRequestId: 0,
  generationBusy: false,
  artifactUrl: null,
  catalog: [],
  lutPresets: [],
  catalogQuery: "",
  assetRatios: {},
  previewEntry: null,
  previewProfile: "auto",
  previewReturnFocus: null,
  theme: "system",
  sampleBackup: null,
};

const HISTORY_KEY = "promptgen.recent.v5";
const HISTORY_SCHEMA_VERSION = 5;

const OUTPUT_PROFILES = {
  C1: { name: "인물·사진", medium: "photo", mediumLabel: "사진", aspect: "2:3", text: "선택", decision: "인물·행동·환경" },
  C4: { name: "제품·브랜드", medium: "photo", mediumLabel: "사진", aspect: "3:2", text: "선택", decision: "형태·재질·브랜드 적용" },
  C5: { name: "광고·포스터", medium: "photo", mediumLabel: "사진/그래픽", aspect: "2:3", text: "권장", decision: "메시지·히어로·CTA" },
  C6: { name: "정보·UI", medium: "graphic_design", mediumLabel: "그래픽", aspect: "16:9", text: "권장", decision: "정보 구조·상태·순서" },
  C10: { name: "일러스트·스토리", medium: "illustration", mediumLabel: "일러스트", aspect: "2:3", text: "선택", decision: "정체성·장면·흐름" },
  C11: { name: "컨셉·엔터테인먼트", medium: "illustration", mediumLabel: "컨셉 아트", aspect: "16:9", text: "선택", decision: "세계·초점·공간 서사" },
};

const CATEGORY_DEPENDENT_KEYS = [
  "image.profile",
  "image.wardrobe",
  "image.face",
  "image.hair",
  "image.adult_editorial_confirm",
  "image.variants",
  "image.formulation",
  "image.product_guide",
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
  "image.subject",
  "image.scene",
  "image.composition",
  "image.lighting",
  "image.surface",
  "image.palette",
  "image.aspect_ratio",
  "image.detail",
  "image.lut",
  "image.text_mode",
  "image.text",
  "image.text_position",
  "image.text_style",
  "image.exclusions",
];

const LUT_COPY = {
  clean_neutral: ["클린 뉴트럴", "제품·피부의 실제 색을 보존"],
  warm_pastel_filmic: ["웜 파스텔 필름", "따뜻한 하이라이트와 부드러운 블랙"],
  cool_steel: ["쿨 스틸", "차가운 중간톤과 선명한 재질 대비"],
  restrained_teal_orange: ["절제된 틸·오렌지", "청록 그림자와 따뜻한 하이라이트 분리"],
  bleach_bypass: ["블리치 바이패스", "낮은 채도와 단단한 영화식 대비"],
  tungsten_night: ["텅스텐 나이트", "따뜻한 실내광과 차가운 야간 그림자"],
  faded_print: ["페이디드 프린트", "바랜 인쇄물의 낮은 대비와 색층"],
  monochrome_high_contrast: ["고대비 흑백", "명암·표면·실루엣을 강하게 강조"],
};

const THEME_KEY = "promptgen.theme.v1";
const THEME_ORDER = ["system", "light", "dark"];
const THEME_LABEL = { system: "시스템 테마", light: "밝은 테마", dark: "어두운 테마" };
const THEME_GLYPH = { system: "◐", light: "☀", dark: "☾" };


// Catalog assets range from 1:1 to 16:9. Cards adopt the real shape so nothing is cropped away,
// clamped so a single extreme asset cannot distort the grid.
const MIN_CARD_RATIO = 0.62;
const MAX_CARD_RATIO = 1.9;

const MEDIUM_OPTIONS = [
  { value: "photo", label: "사진", hint: "렌즈·조명·재질 증명", token: "var(--photo)" },
  { value: "graphic_design", label: "그래픽 디자인", hint: "레이아웃·타이포 위계", token: "var(--graphic)" },
  { value: "illustration", label: "일러스트", hint: "선·채색·연출", token: "var(--illust)" },
  { value: "3d", label: "3D", hint: "형태·재질·스튜디오 광", token: "var(--three-d)" },
  { value: "mixed", label: "혼합 매체", hint: "둘 이상의 시각 매체 결합", token: "var(--text-faint)" },
];

const IMAGE_PROFILE_OPTIONS = {
  C1: [{ value: "standard", label: "기본 이미지", hint: "인물·라이프스타일" }],
  C4: [
    { value: "standard", label: "기본 이미지", hint: "제품·브랜드" },
    { value: "logo_identity", label: "로고 아이덴티티", hint: "opaque PNG 콘셉트" },
    { value: "app_icon", label: "앱 아이콘", hint: "opaque 1024² PNG master concept" },
  ],
  C5: [{ value: "standard", label: "기본 이미지", hint: "광고·포스터" }, { value: "travel_journal", label: "여행·다이어리", hint: "사진·종이·정확 문구" }],
  C6: [
    { value: "standard", label: "기본 이미지", hint: "정보·UI" },
    { value: "app_web_ui", label: "앱·웹 UI", hint: "화면·상태·컴포넌트" },
    { value: "information_design", label: "정보 디자인", hint: "위계·범례·읽기 순서" },
  ],
  C10: [
    { value: "standard", label: "기본 이미지", hint: "일러스트·스토리" },
    { value: "character_pose", label: "캐릭터·자세", hint: "텍스트 지정 동작" },
    { value: "poomsae_pose", label: "품새 자세", hint: "텍스트 지정 품새" },
  ],
  C11: [{ value: "standard", label: "기본 이미지", hint: "컨셉·엔터테인먼트" }],
};

const IMAGE_PROFILE_DEFAULTS = {
  travel_journal: { medium: "mixed", aspect: "4:5", text: "제공 문구만", decision: "주 장면·보존 조건·정확 문구" },
  app_icon: {
    medium: "graphic_design",
    aspect: "1:1",
    text: "없음",
    decision: "제품 목적·핵심 은유·32px 실루엣",
  },
};

const ASPECT_OPTIONS = [
  { value: "1:1", hint: "정사각형" },
  { value: "4:5", hint: "세로 피드" },
  { value: "3:4", hint: "세로 이미지" },
  { value: "2:3", hint: "세로 포스터" },
  { value: "3:2", hint: "가로 이미지" },
  { value: "4:3", hint: "가로 화면" },
  { value: "16:9", hint: "와이드" },
  { value: "9:16", hint: "세로 화면" },
];


const DETAIL_OPTIONS = [
  { value: "auto", label: "자동", hint: "요청에 맞춰" },
  { value: "low", label: "간결", hint: "구도만 확인" },
  { value: "medium", label: "표준", hint: "일반 작업" },
  { value: "high", label: "정밀", hint: "문구·재질 정밀" },
];

const TEXT_POSITION_ZONES = [
  ["top_left", "좌상단"], ["top_center", "상단 중앙"], ["top_right", "우상단"],
  ["middle_left", "가운데 왼쪽"], ["center", "정중앙"], ["middle_right", "가운데 오른쪽"],
  ["bottom_left", "좌하단"], ["bottom_center", "하단 중앙"], ["bottom_right", "우하단"],
];

const TEXT_STYLE_OPTIONS = [
  { value: "geometric", label: "기하 산세리프", hint: "중립·현대" },
  { value: "condensed", label: "좁고 강한 글자", hint: "강한 헤드라인" },
  { value: "didone", label: "고대비 세리프", hint: "고급·편집" },
  { value: "mono", label: "모노스페이스", hint: "기술·데이터" },
  { value: "brush", label: "한글 붓글씨", hint: "손글씨·정서" },
];

const QUESTION_CONTEXT = {
  "image.category": ["요청만으로 결과물을 확정할 수 없습니다", "이후 질문과 기본 비율·문구 계약이 모두 바뀝니다"],
  "image.subject": ["핵심 피사체와 수량이 아직 없습니다", "무엇을 몇 개 보여 줄지 결정합니다"],
  "image.scene": ["배경·시간·대기 정보가 아직 없습니다", "장면 묘사와 조명 해석이 달라집니다"],
  "image.wardrobe": ["화보 결과를 좌우하는 의상 정보입니다", "색·원단·핏이 실루엣과 재질 묘사가 됩니다"],
  "image.variants": ["세트 안에서 무엇이 달라지는지 정합니다", "각 변형의 표정·행동이 개별 지시가 됩니다"],
  "image.text_mode": ["문구를 넣을지 정합니다", "정확 문구를 고르면 배치·인상 설정이 열립니다"],
  "image.text": ["이미지에 그대로 렌더할 문구입니다", "입력한 줄과 순서를 그대로 지킵니다"],
  "image.palette": ["브랜드 색을 고정할지 정합니다", "색과 비중이 팔레트 계약이 됩니다"],
  "image.lut": ["사진 색 응답 방식을 고릅니다", "화이트밸런스·채도·대비 계약이 붙습니다"],
  "image.adult_editorial_confirm": ["성인 화보의 안전 조건을 확인합니다", "확인해야 화보 계약을 계속할 수 있습니다"],
};

const QUESTION_SUGGESTIONS = {
  "image.subject": ["무광 검정 알루미늄 캔 1개", "30세 성인 모델 1명", "세라믹 머그 3개 세트"],
  "image.scene": ["일몰 25분 전의 해안 테라스", "비 온 뒤 젖은 도심 골목", "무한대 배경의 흰 스튜디오"],
  "image.wardrobe": ["세이지색 리넨 재킷, 크림 이너, 와이드 팬츠, 여유로운 핏"],
  "image.palette": ["#9AA58B:35,#D8D1C4:25,#6F8490:25,#3B302B:15"],
};

function hasOwn(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key);
}

function ratioValue(aspect) {
  const [width, height] = String(aspect || "").split(":").map(Number);
  if (!Number.isFinite(width) || !Number.isFinite(height) || height <= 0) return 1;
  return width / height;
}

function clampRatio(value) {
  if (!Number.isFinite(value) || value <= 0) return 1;
  return Math.min(MAX_CARD_RATIO, Math.max(MIN_CARD_RATIO, value));
}

function effectiveAnswer(key) {
  if (hasOwn(state.explicitAnswers, key)) return state.explicitAnswers[key];
  return state.normalizedAnswers[key];
}

function invalidatePendingWork() {
  state.interviewRequestId += 1;
  state.generationRequestId += 1;
  state.generationBusy = false;
  if (state.busy) setBusy(false);
}

function clearArtifact() {
  state.artifactUrl = null;
  elements.imageFrame.classList.add("hidden");
  elements.generatedImage.removeAttribute("src");
}

function clearResolvedOutcome() {
  invalidatePendingWork();
  state.normalizedAnswers = {};
  state.outcome = null;
  state.currentQuestion = null;
  state.artifactUrl = null;
  elements.resultStage.classList.add("hidden");
  clearArtifact();
  elements.promptOutput.textContent = "";
  elements.requestJson.textContent = "{}";
  elements.copyPrompt.disabled = true;
  elements.copyRequest.disabled = true;
  elements.downloadRequest.disabled = true;
  elements.execute.disabled = true;
  elements.stageActions.classList.add("hidden");
  elements.resultDetails.open = false;
  elements.emptyStage.classList.remove("hidden");
  setBadge("대기", "neutral");
  updatePrimaryActionLabel();
}

function clearCategoryDependentAnswers() {
  for (const key of CATEGORY_DEPENDENT_KEYS) delete state.explicitAnswers[key];
  state.normalizedAnswers = {};
}

const $ = (id) => document.getElementById(id);
const elements = {
  brief: $("brief"),
  visualShortcuts: $("visual-shortcuts"),
  executionNote: $("execution-note"),
  briefCount: $("brief-count"),
  questionCard: $("question-card"),
  questionIndex: $("question-index"),
  questionLabel: $("question-label"),
  questionHelp: $("question-help"),
  questionControl: $("question-control"),
  questionError: $("question-error"),
  skipQuestion: $("skip-question"),
  answerQuestion: $("answer-question"),
  analyze: $("analyze"),
  execute: $("execute"),
  editDirection: $("edit-direction"),
  stageActions: $("stage-actions"),
  statusLine: $("status-line"),
  announcer: $("announcer"),
  emptyStage: $("empty-stage"),
  resultStage: $("result-stage"),
  generationPlan: $("generation-plan"),
  generationPlanGrid: $("generation-plan-grid"),
  promptOutput: $("prompt-output"),
  resultSummary: $("result-summary"),
  imageFrame: $("image-frame"),
  generatedImage: $("generated-image"),
  validationBadge: $("validation-badge"),
  inferenceList: $("inference-list"),
  diagnosticList: $("diagnostic-list"),
  knowledgeList: $("knowledge-list"),
  requestJson: $("request-json"),
  copyPrompt: $("copy-prompt"),
  copyRequest: $("copy-request"),
  downloadRequest: $("download-request"),
  runOptions: $("run-options"),
  version: $("version"),
  inspectorPanel: $("inspector-panel"),
  inspectorToggle: $("inspector-toggle"),
  inspectorClose: $("inspector-close"),
  inspectorBackdrop: $("inspector-backdrop"),
  toast: $("toast"),
  recentList: $("recent-list"),
  clearHistory: $("clear-history"),
  catalogSearch: $("catalog-search"),
  catalogResultCount: $("catalog-result-count"),
  catalogGrid: $("catalog-grid"),
  themeToggle: $("theme-toggle"),
  questionWhy: $("question-why"),
  previewDialog: $("preview-dialog"),
  previewImage: $("preview-image"),
  previewPath: $("preview-path"),
  previewTitle: $("preview-title"),
  previewFacts: $("preview-facts"),
  previewIntent: $("preview-intent"),
  previewDirectives: $("preview-directives"),
  previewProfileSelect: $("preview-profile-select"),
  previewSelect: $("preview-select"),
  previewClose: $("preview-close"),
  undoSample: $("undo-sample"),
  catalogCount: $("catalog-count"),
  routeSummary: $("route-summary"),
  routeCategory: $("route-category"),
  routeMedium: $("route-medium"),
  routeAspect: $("route-aspect"),
  routeText: $("route-text"),
  routeMediumSelect: $("route-medium-select"),
  routeProfileSelect: $("route-profile-select"),
  routeAspectSelect: $("route-aspect-select"),
  routeDetailSelect: $("route-detail-select"),
  routeAuto: $("route-auto"),
  routeTextSettings: $("route-text-settings"),
  routeTextPosition: $("route-text-position"),
  routeTextStyle: $("route-text-style"),
  lutSetting: $("lut-setting"),
  lutReason: $("lut-reason"),
  lutGrid: $("lut-grid"),
  textComposer: $("text-composer"),
  imageQuickOptions: $("image-quick-options"),
  directionExamples: $("direction-examples"),
  quickAspect: $("quick-aspect"),
  resultLocksPreview: $("result-locks-preview"),
  resultDetails: $("result-details"),
};

function userFacingApiError(data, status) {
  const raw = String(data?.error?.message || data?.reason || "").trim();
  if (/usage.?limit|rate.?limit|quota|사용.?한도/i.test(raw)) {
    return "Codex 사용 한도에 도달했습니다. 사용 가능 시점 이후 다시 시도해 주세요.";
  }
  if (/CODEX_[A-Z_]+|stderr\s*=|stdout\s*=|workdir:|command_execution|stack backtrace/i.test(raw)) {
    return "요청을 처리하지 못했습니다. Codex 상태를 확인한 뒤 다시 시도해 주세요.";
  }
  if (raw && raw.length <= 240) return raw;
  return `요청을 처리하지 못했습니다. 잠시 후 다시 시도해 주세요. (HTTP ${status})`;
}

async function api(path, payload) {
  const response = await fetch(path, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-PromptGen-Client": "web-v3",
    },
    body: JSON.stringify(payload),
  });
  let data;
  try {
    data = await response.json();
  } catch {
    throw new Error(`서버 응답을 읽지 못했습니다. 잠시 후 다시 시도해 주세요. (HTTP ${response.status})`);
  }
  if (!response.ok && !data.interview) {
    throw new Error(userFacingApiError(data, response.status));
  }
  return data;
}

function interviewPayload() {
  return {
    kind: "image",
    brief: state.brief,
    answers: { ...state.explicitAnswers },
  };
}

function catalogEntry(id) {
  return state.catalog.find((entry) => entry.id === id) || null;
}

function updatePrimaryActionLabel() {
  elements.analyze.firstElementChild.textContent = "프롬프트 만들기";
  const editing = state.outcome?.request?.task_mode === "edit";
  elements.execute.classList.toggle("hidden", state.mode !== "codex-imagegen" || editing);
  if (editing) {
    elements.executionNote.textContent = "이 프롬프트는 원본 사진 편집용입니다. 요청 JSON을 저장하고 CLI의 codex-imagegen --reference-image에 실제 원본 파일을 전달하세요.";
    elements.executionNote.classList.remove("hidden");
  } else {
    elements.executionNote.textContent = "Codex 로그인과 이미지 도구가 필요합니다. 검토와 생성은 외부 서비스를 호출하며 사용량을 소비합니다.";
    elements.executionNote.classList.toggle("hidden", state.mode !== "codex-imagegen");
  }

}

function catalogEntryIsSelected(entry) {
  return effectiveAnswer("image.category") === entry.id;
}

function mediumLabel(value) {
  return MEDIUM_OPTIONS.find((option) => option.value === value)?.label || value || "—";
}

function decorateMedium(tile, option) {
  const dot = document.createElement("span");
  dot.className = "medium-dot";
  dot.setAttribute("aria-hidden", "true");
  dot.style.setProperty("--medium-color", option.token);
  tile.appendChild(dot);
}

function decorateAspect(tile, option) {
  const glyph = document.createElement("span");
  glyph.className = "ratio-glyph";
  glyph.setAttribute("aria-hidden", "true");
  const box = document.createElement("i");
  const ratio = ratioValue(option.value);
  box.style.setProperty("--glyph-w", `${ratio >= 1 ? 22 : Math.round(22 * ratio)}px`);
  box.style.setProperty("--glyph-h", `${ratio >= 1 ? Math.round(22 / ratio) : 22}px`);
  glyph.appendChild(box);
  tile.appendChild(glyph);
}

function renderTileGroup(container, options, selected, decorate, onSelect) {
  container.replaceChildren();
  options.forEach((option, index) => {
    const tile = document.createElement("button");
    tile.type = "button";
    tile.className = "tile";
    tile.dataset.value = option.value;
    tile.setAttribute("role", "radio");
    tile.setAttribute("aria-checked", String(option.value === selected));
    tile.tabIndex = option.value === selected || (!selected && index === 0) ? 0 : -1;
    if (decorate) decorate(tile, option);
    const copy = document.createElement("span");
    const label = document.createElement("strong");
    label.textContent = option.label || option.value;
    copy.appendChild(label);
    if (option.hint) {
      const hint = document.createElement("small");
      hint.textContent = option.hint;
      copy.appendChild(hint);
    }
    tile.appendChild(copy);
    tile.addEventListener("click", () => onSelect(option.value));
    container.appendChild(tile);
  });
}

function renderZoneGrid(container, selected, onSelect) {
  container.replaceChildren();
  TEXT_POSITION_ZONES.forEach(([value, label], index) => {
    const cell = document.createElement("button");
    cell.type = "button";
    cell.className = "zone-cell";
    cell.dataset.value = value;
    cell.setAttribute("role", "radio");
    cell.setAttribute("aria-checked", String(value === selected));
    cell.tabIndex = value === selected || (!selected && index === 0) ? 0 : -1;
    cell.setAttribute("aria-label", label);
    cell.title = label;
    cell.addEventListener("click", () => onSelect(value));
    container.appendChild(cell);
  });
}

function renderRouteSummary() {
  const category = effectiveAnswer("image.category");
  if (!category || !OUTPUT_PROFILES[category]) {
    elements.routeSummary.classList.add("hidden");
    return;
  }

  const profile = OUTPUT_PROFILES[category];
  const entry = catalogEntry(category);
  const profileValue = effectiveAnswer("image.profile") || "standard";
  const profileDefaults = IMAGE_PROFILE_DEFAULTS[profileValue] || {};
  const medium = effectiveAnswer("image.medium") || profileDefaults.medium || profile.medium;
  const aspect = effectiveAnswer("image.aspect_ratio") || profileDefaults.aspect || profile.aspect;
  const detail = effectiveAnswer("image.detail") || "high";
  const textMode = effectiveAnswer("image.text_mode");
  const text = effectiveAnswer("image.text") || "";

  elements.routeSummary.classList.remove("hidden");
  elements.routeCategory.textContent = profileValue === "travel_journal" ? "여행·다이어리" : `${category} · ${entry?.name_ko || profile.name}`;
  elements.routeMedium.textContent = mediumLabel(medium);
  elements.routeAspect.textContent = aspect;
  elements.routeText.textContent = textMode === "exact"
    ? text.trim()
      ? `정확 문구 · ${text.split("\n").filter(Boolean).length}줄`
      : "정확 문구 입력 필요"
    : textMode === "none"
      ? "문구 없음"
      : profileDefaults.text || profile.text;

  renderTileGroup(
    elements.routeProfileSelect,
    IMAGE_PROFILE_OPTIONS[category] || IMAGE_PROFILE_OPTIONS.C5,
    profileValue,
    null,
    (value) => updateTechnicalAnswer("image.profile", value),
  );

  const mediumOptions = profileValue === "app_icon"
    ? MEDIUM_OPTIONS.filter((option) => ["graphic_design", "illustration", "3d", "mixed"].includes(option.value))
    : MEDIUM_OPTIONS;
  renderTileGroup(elements.routeMediumSelect, mediumOptions, medium, decorateMedium, (value) => {
    updateTechnicalAnswer("image.medium", value);
  });
  renderTileGroup(elements.routeAspectSelect, ASPECT_OPTIONS, aspect, decorateAspect, (value) => {
    updateTechnicalAnswer("image.aspect_ratio", value);
  });
  renderTileGroup(elements.routeDetailSelect, DETAIL_OPTIONS, detail, null, (value) => {
    updateTechnicalAnswer("image.detail", value);
  });
  const hasExactText = textMode === "exact" && profileValue !== "app_icon";
  elements.routeTextSettings.classList.toggle("hidden", !hasExactText);
  if (hasExactText) {
    renderZoneGrid(
      elements.routeTextPosition,
      effectiveAnswer("image.text_position") || "top_left",
      (value) => updateTechnicalAnswer("image.text_position", value),
    );
    renderTileGroup(
      elements.routeTextStyle,
      TEXT_STYLE_OPTIONS,
      effectiveAnswer("image.text_style") || "geometric",
      null,
      (value) => updateTechnicalAnswer("image.text_style", value),
    );
  }
  elements.lutSetting.classList.toggle("hidden", medium !== "photo");
  renderLutGrid(medium);
}

// Backend inference strings name presets by their internal id. docs/DESIGN.md §8 keeps those
// identifiers inside the inspector, so user-facing reasons use the Korean preset name.
function humanizeLutNames(text) {
  if (!text) return text;
  let result = text;
  for (const [preset, [label]] of Object.entries(LUT_COPY)) {
    result = result.replaceAll(preset, label);
  }
  return result;
}

function renderLutGrid(medium = effectiveAnswer("image.medium")) {
  elements.lutGrid.replaceChildren();
  if (medium !== "photo") return;
  const presets = state.lutPresets.length
    ? state.lutPresets
    : Object.keys(LUT_COPY).map((preset) => ({ preset }));
  const selected = effectiveAnswer("image.lut") || null;
  if (!selected) {
    elements.lutReason.textContent = "핵심 결정을 마치면 요청과 스타일에 맞춰 자동 선택합니다.";
  } else {
    const [selectedLabel] = LUT_COPY[selected] || [selected];
    const inferredReason = (state.outcome?.inferences || [])
      .find((item) => item.includes(selected) && item.includes("색감 프리셋"));
    elements.lutReason.textContent = hasOwn(state.explicitAnswers, "image.lut")
      ? `${selectedLabel} · 직접 선택`
      : humanizeLutNames(inferredReason) || `${selectedLabel} · 결과물과 요청에 따른 자동값`;
  }
  presets.forEach((preset, index) => {
    const name = preset.preset;
    const [label, description] = LUT_COPY[name] || [name, "사진 색 응답 프리셋"];
    const button = document.createElement("button");
    button.type = "button";
    button.className = "lut-card";
    button.dataset.lut = name;
    button.setAttribute("role", "radio");
    button.setAttribute("aria-checked", String(selected === name));
    button.tabIndex = selected === name || (!selected && index === 0) ? 0 : -1;
    button.classList.toggle("selected", selected === name);
    const swatch = document.createElement("span");
    swatch.className = `lut-swatch lut-${name.replaceAll("_", "-")}`;
    const copy = document.createElement("span");
    copy.className = "lut-card-copy";
    const title = document.createElement("strong");
    title.textContent = label;
    const detail = document.createElement("small");
    const technical = preset.white_balance_kelvin
      ? `${preset.white_balance_kelvin}K · 채도 ${preset.saturation_percent}% · 대비 ${preset.contrast}`
      : "";
    detail.textContent = [description, technical].filter(Boolean).join(" · ");
    copy.append(title, detail);
    button.append(swatch, copy);
    button.addEventListener("click", () => updateTechnicalAnswer("image.lut", name));
    elements.lutGrid.appendChild(button);
  });
}

function updateTechnicalAnswer(key, value) {
  state.explicitAnswers[key] = value;
  if (key === "image.medium" && value !== "photo") delete state.explicitAnswers["image.lut"];
  clearResolvedOutcome();
  renderQuickOptions();
  renderRouteSummary();
  if (state.brief.trim()) {
    runInterview({ routeFocusKey: key, routeFocusValue: value });
  } else {
    restoreRouteFocus(key, value);
  }
}

function renderQuickOptions() {
  const aspect = effectiveAnswer("image.aspect_ratio");
  const hasPreset = [...elements.quickAspect.querySelectorAll("[data-quick-aspect]")].some((button) => button.dataset.quickAspect === aspect);
  elements.quickAspect.querySelectorAll("[data-quick-aspect]").forEach((button, index) => {
    const active = button.dataset.quickAspect === aspect;
    button.setAttribute("aria-checked", String(active));
    button.tabIndex = active || (!hasPreset && index === 0) ? 0 : -1;
  });
}

function restoreRouteFocus(key, value) {
  const focusTargets = {
    "image.medium": [elements.routeMediumSelect, "value"],
    "image.profile": [elements.routeProfileSelect, "value"],
    "image.aspect_ratio": [elements.routeAspectSelect, "value"],
    "image.detail": [elements.routeDetailSelect, "value"],
    "image.text_position": [elements.routeTextPosition, "value"],
    "image.text_style": [elements.routeTextStyle, "value"],
    "image.lut": [elements.lutGrid, "lut"],
  };
  const [container, dataKey] = focusTargets[key] || [];
  requestAnimationFrame(() => {
    const target = [...(container?.querySelectorAll("[role='radio']") || [])]
      .find((item) => item.dataset[dataKey] === value);
    target?.focus({ preventScroll: true });
  });
}

async function loadLutPresets() {
  try {
    const response = await fetch("/api/v3/lut-presets");
    const data = await response.json();
    state.lutPresets = Array.isArray(data?.presets) ? data.presets : [];
  } catch {
    state.lutPresets = [];
  }
  renderRouteSummary();
}

let catalogImageObserver = null;

function observeCatalogImage(image) {
  if (typeof IntersectionObserver !== "function") {
    image.src = image.dataset.src;
    return;
  }
  if (!catalogImageObserver) {
    catalogImageObserver = new IntersectionObserver(
      (records, observer) => {
        for (const record of records) {
          if (!record.isIntersecting) continue;
          observer.unobserve(record.target);
          if (record.target.dataset.src) record.target.src = record.target.dataset.src;
        }
      },
      { rootMargin: "320px 0px" },
    );
  }
  catalogImageObserver.observe(image);
}

function releaseCatalogImages() {
  catalogImageObserver?.disconnect();
}

function catalogName(entry) {
  return entry.name_ko || entry.name_en || entry.id;
}

// The catalog card frame adopts the asset's own aspect so the reference is never cropped.
// Before the file loads the route default keeps the grid stable; the measured value replaces it once.
function cardRatio(entry) {
  if (state.assetRatios[entry.id]) return state.assetRatios[entry.id];
  const profile = OUTPUT_PROFILES[entry.id];
  if (profile) return clampRatio(ratioValue(profile.aspect));
  const suggested = (entry.suggested_aspect_ratios || [])[0];
  return suggested ? clampRatio(ratioValue(suggested)) : 1;
}

function catalogCardEffect(entry) {
  const profile = OUTPUT_PROFILES[entry.id];
  if (profile) return `핵심 결정 · ${profile.decision}`;
  if (entry.tier_3) return `${entry.tier_3} 구성 규칙을 결과물에 더합니다`;
  return "선택한 결과물에 시각 규칙을 더합니다";
}

function catalogCardFallback(entry) {
  const fallback = document.createElement("span");
  fallback.className = "catalog-card-fallback";
  const id = document.createElement("strong");
  id.textContent = entry.id;
  const detail = document.createElement("small");
  detail.textContent = `${entry.tier_2 || entry.tier_1 || "카탈로그"} · 예시 이미지를 불러오지 못했습니다`;
  fallback.append(id, detail);
  return fallback;
}

function catalogCardMedia(entry, assetUrl, altText) {
  const media = document.createElement("span");
  media.className = "catalog-card-media is-loading";
  media.style.setProperty("--card-ratio", String(cardRatio(entry)));
  const image = document.createElement("img");
  image.alt = altText;
  image.loading = "lazy";
  image.decoding = "async";
  image.dataset.src = assetUrl;
  image.addEventListener("load", () => {
    media.classList.remove("is-loading");
    image.classList.add("is-loaded");
    if (image.naturalWidth > 0 && image.naturalHeight > 0) {
      const measured = clampRatio(image.naturalWidth / image.naturalHeight);
      state.assetRatios[entry.id] = measured;
      media.style.setProperty("--card-ratio", String(measured));
    }
  });
  image.addEventListener("error", () => {
    media.classList.remove("is-loading");
    media.appendChild(catalogCardFallback(entry));
  });
  media.appendChild(image);
  observeCatalogImage(image);
  return media;
}

function renderCatalogCard(entry, options = {}) {
  const {
    selected = false,
    input = null,
    blocked = false,
    onSelect = null,
    onPreview = null,
  } = options;
  const wrap = document.createElement("div");
  wrap.className = "catalog-card-wrap";

  const card = document.createElement(input ? "label" : "button");
  card.className = "catalog-card";
  card.dataset.catalogId = entry.id;
  card.classList.toggle("selected", selected);
  card.classList.toggle("limit-reached", blocked);
  if (!input) {
    card.type = "button";
    if (onPreview) {
      card.setAttribute("aria-label", `${catalogName(entry)} 프리뷰 열기`);
    } else {
      card.setAttribute("aria-pressed", String(selected));
      card.setAttribute("aria-label", `${catalogName(entry)} ${selected ? "선택 해제" : "선택"}`);
    }
  }

  const media = catalogCardMedia(
    entry,
    `/api/v3/catalog/assets/${encodeURIComponent(entry.id)}`,
    `${catalogName(entry)} 결과 예시`,
  );
  const copy = document.createElement("span");
  copy.className = "catalog-card-copy";
  const title = document.createElement("strong");
  title.textContent = catalogName(entry);
  const detail = document.createElement("small");
  detail.textContent = catalogCardEffect(entry);
  copy.append(title, detail);

  if (input) card.append(input);
  card.append(media, copy);
  if (onSelect) card.addEventListener("click", () => onSelect(card));
  wrap.appendChild(card);

  return wrap;
}

function restoreCatalogFocus(entryId, anchorTop = null) {
  requestAnimationFrame(() => {
    const card = [...elements.catalogGrid.querySelectorAll("[data-catalog-id]")]
      .find((candidate) => candidate.dataset.catalogId === entryId);
    if (card && Number.isFinite(anchorTop)) {
      const offset = card.getBoundingClientRect().top - anchorTop;
      if (Math.abs(offset) > 1) window.scrollBy({ top: offset, behavior: "auto" });
    }
    card?.focus({ preventScroll: true });
  });
}

function selectCatalogEntry(entry, anchorTop = null, profileValue = null) {
  let selected = true;
  const currentProfile = state.explicitAnswers["image.profile"] || "auto";
  const profileMatches = profileValue === null || profileValue === currentProfile;
  if (state.explicitAnswers["image.category"] === entry.id && profileMatches) {
    delete state.explicitAnswers["image.category"];
    clearCategoryDependentAnswers();
    selected = false;
  } else {
    const categoryChanged = effectiveAnswer("image.category") !== entry.id;
    const profileChanged = profileValue !== null && profileValue !== currentProfile;
    if (categoryChanged || profileChanged) clearCategoryDependentAnswers();
    state.explicitAnswers["image.category"] = entry.id;
    if (profileValue === "auto") delete state.explicitAnswers["image.profile"];
    else if (profileValue !== null) state.explicitAnswers["image.profile"] = profileValue;
  }
  clearResolvedOutcome();
  renderQuickOptions();
  renderCatalogExplorer();
  renderRouteSummary();
  if (state.brief.trim()) {
    runInterview({
      focusQuestion: true,
      catalogAnchorTop: anchorTop,
    });
  }
  else {
    setStatus(`${entry.name_ko || entry.id} 결과물을 ${selected ? "골랐습니다" : "해제했습니다"}. 요청은 준비되면 위 입력란에 작성하세요.`);
    restoreCatalogFocus(entry.id, anchorTop);
  }
}


// Representative outcomes are grouped by their canonical tier-2 purpose.
function matchesQuery(entry, query) {
  if (!query) return true;
  return [
    entry.id,
    entry.name_ko,
    entry.name_en,
    entry.tier_1,
    entry.tier_2,
    entry.tier_3,
    catalogCardEffect(entry),
  ]
    .filter(Boolean)
    .some((field) => String(field).toLowerCase().includes(query));
}

function catalogEmptyState(message, action) {
  const empty = document.createElement("div");
  empty.className = "catalog-empty";
  const title = document.createElement("strong");
  title.textContent = "조건에 맞는 카드가 없습니다";
  const copy = document.createElement("span");
  copy.className = "empty-copy";
  copy.textContent = message;
  empty.append(title, copy);
  if (action) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "quiet-button";
    button.textContent = action.label;
    button.addEventListener("click", action.run);
    empty.appendChild(button);
  }
  return empty;
}

function renderCatalogExplorer() {
  if (!state.catalog.length || !elements.catalogGrid) return;

  releaseCatalogImages();
  const query = state.catalogQuery.trim().toLowerCase();
  const scoped = state.catalog.filter((entry) => entry.tier_1 === "결과물");
  const searched = scoped.filter((entry) => matchesQuery(entry, query));

  const entries = searched;

  elements.catalogGrid.replaceChildren();
  if (!entries.length) {
    elements.catalogGrid.appendChild(
      catalogEmptyState(
        query
          ? `"${state.catalogQuery.trim()}" 검색에 맞는 결과물가 없습니다.`
          : "선택할 결과물가 없습니다.",
        {
          label: "필터 지우기",
          run: () => {
            state.catalogQuery = "";
            elements.catalogSearch.value = "";
            renderCatalogExplorer();
          },
        },
      ),
    );
  }

  for (const entry of entries) {
    const selected = catalogEntryIsSelected(entry);
    elements.catalogGrid.appendChild(
      renderCatalogCard(entry, {
        selected,
        blocked: false,
        onSelect: (card) => openCatalogPreview(entry, card),
        onPreview: (trigger) => openCatalogPreview(entry, trigger),
      }),
    );
  }
  updateCatalogResultCount(entries.length, scoped.length);
}

function updateCatalogResultCount(shown, total) {
  if (!elements.catalogResultCount) return;
  const filters = [];
  if (state.catalogQuery.trim()) filters.push(`검색 "${state.catalogQuery.trim()}"`);
  elements.catalogResultCount.textContent = filters.length
    ? `대표 결과물 ${total}개 중 ${shown}개 표시 · ${filters.join(" · ")}`
    : `대표 결과물 ${shown}개를 모두 보고 있습니다.`;
}

function catalogCards() {
  return [...elements.catalogGrid.querySelectorAll(".catalog-card[data-catalog-id]")];
}

function moveCatalogFocus(current, delta) {
  const cards = catalogCards();
  const index = cards.indexOf(current);
  if (index < 0) return;
  const next = cards[Math.min(cards.length - 1, Math.max(0, index + delta))];
  next?.focus();
}

function catalogGridKeydown(event) {
  const card = event.target.closest?.(".catalog-card[data-catalog-id]");
  if (!card) return;
  const columns = Math.max(
    1,
    Math.round(elements.catalogGrid.clientWidth / (card.getBoundingClientRect().width || 1)),
  );
  const moves = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: columns, ArrowUp: -columns };
  if (event.key in moves) {
    event.preventDefault();
    moveCatalogFocus(card, moves[event.key]);
    return;
  }
  const cards = catalogCards();
  if (event.key === "Home") {
    event.preventDefault();
    cards[0]?.focus();
  } else if (event.key === "End") {
    event.preventDefault();
    cards[cards.length - 1]?.focus();
  }
}


function previewFact(label, value) {
  const row = document.createElement("div");
  const key = document.createElement("span");
  key.textContent = label;
  const detail = document.createElement("strong");
  detail.textContent = value;
  row.append(key, detail);
  return row;
}

function openCatalogPreview(entry, trigger) {
  state.previewEntry = entry;
  const allowedProfiles = IMAGE_PROFILE_OPTIONS[entry.id] || [];
  const explicitProfile = state.explicitAnswers["image.profile"];
  state.previewProfile = allowedProfiles.some((option) => option.value === explicitProfile)
    ? explicitProfile
    : "auto";
  state.previewReturnFocus = trigger || null;
  elements.previewImage.src = `/api/v3/catalog/assets/${encodeURIComponent(entry.id)}`;
  elements.previewImage.alt = `${catalogName(entry)} 결과 예시 전체 이미지`;
  elements.previewPath.textContent = [entry.tier_1, entry.tier_2, entry.tier_3].filter(Boolean).join(" › ");
  elements.previewTitle.textContent = `${entry.id} · ${catalogName(entry)}`;

  const profile = OUTPUT_PROFILES[entry.id];
  const selectedProfileDefaults = IMAGE_PROFILE_DEFAULTS[state.previewProfile] || {};
  const facts = [
    ["매체", selectedProfileDefaults.medium
      ? mediumLabel(selectedProfileDefaults.medium)
      : profile?.mediumLabel || "사용자 지정"],
    ["기본 비율", selectedProfileDefaults.aspect
      || profile?.aspect || (entry.suggested_aspect_ratios || []).join(" · ")],
    ["문구 계약", selectedProfileDefaults.text || profile?.text || "선택"],
    ["핵심 결정", selectedProfileDefaults.decision || profile?.decision || "피사체·장면·구도"],
  ];
  elements.previewFacts.replaceChildren();
  for (const [label, value] of facts) elements.previewFacts.appendChild(previewFact(label, value));

  elements.previewIntent.textContent = entry.intent || "";
  elements.previewDirectives.replaceChildren();
  for (const directive of (entry.prompt_directives || []).slice(0, 4)) {
    const item = document.createElement("li");
    const slot = document.createElement("b");
    slot.textContent = directive.slot;
    item.append(slot, document.createTextNode(directive.text));
    elements.previewDirectives.appendChild(item);
  }

  renderPreviewProfileOptions(entry);
  renderPreviewSelectLabel(entry);
  elements.previewDialog.classList.remove("hidden");
  requestAnimationFrame(() => elements.previewSelect.focus());
}

function previewProfileOptions(entry) {
  return [
    { value: "auto", label: "자동 추천", hint: "요청 내용을 보고 가장 맞는 세부 경로를 정합니다." },
    ...(IMAGE_PROFILE_OPTIONS[entry.id] || []),
  ];
}

function renderPreviewSelectLabel(entry) {
  const currentProfile = state.explicitAnswers["image.profile"] || "auto";
  const matchesCurrentRoute = state.explicitAnswers["image.category"] === entry.id
    && currentProfile === state.previewProfile;
  elements.previewSelect.textContent = matchesCurrentRoute ? "이 경로 해제" : "이 프리뷰로 시작";
}

function renderPreviewProfileOptions(entry) {
  renderTileGroup(
    elements.previewProfileSelect,
    previewProfileOptions(entry),
    state.previewProfile,
    null,
    (value) => {
      state.previewProfile = value;
      renderPreviewProfileOptions(entry);
      renderPreviewSelectLabel(entry);
    },
  );
}

function closeCatalogPreview() {
  if (elements.previewDialog.classList.contains("hidden")) return;
  elements.previewDialog.classList.add("hidden");
  elements.previewImage.removeAttribute("src");
  const trigger = state.previewReturnFocus;
  state.previewEntry = null;
  state.previewProfile = "auto";
  state.previewReturnFocus = null;
  trigger?.focus?.();
}

function trapPreviewFocus(event) {
  if (event.key !== "Tab") return;
  const focusable = [...elements.previewDialog.querySelectorAll("button")].filter((node) => !node.disabled);
  if (!focusable.length) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

function applyTheme(theme) {
  state.theme = theme;
  if (theme === "system") document.documentElement.removeAttribute("data-theme");
  else document.documentElement.setAttribute("data-theme", theme);
  elements.themeToggle.textContent = THEME_GLYPH[theme];
  elements.themeToggle.title = THEME_LABEL[theme];
  elements.themeToggle.setAttribute("aria-label", `화면 테마 전환 · 현재 ${THEME_LABEL[theme]}`);
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Theme preference is optional; the system setting remains the fallback.
  }
}

function initTheme() {
  let stored = "system";
  try {
    stored = localStorage.getItem(THEME_KEY) || "system";
  } catch {
    stored = "system";
  }
  applyTheme(THEME_ORDER.includes(stored) ? stored : "system");
}

function renderVisualShortcuts(controls) {
  elements.visualShortcuts.replaceChildren();
  for (const control of controls) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "shortcut-button";
    button.dataset.visualControl = control.id;
    button.textContent = `/${control.id}`;
    button.title = control.description;
    button.setAttribute("aria-label", `/${control.id}: ${control.description}`);
    button.addEventListener("click", () => {
      const match = state.brief.match(/^((?:\/[a-z-]+(?:\s+|$))*)/i);
      const prefix = (match?.[0] || "").trim().split(/\s+/).filter(Boolean).map((value) => value.toLowerCase());
      const token = `/${control.id}`;
      const next = prefix.includes(token) ? prefix.filter((value) => value !== token) : [...prefix, token];
      state.brief = [...next, state.brief.slice(match?.[0].length || 0)].filter(Boolean).join(" ");
      elements.brief.value = state.brief;
      elements.briefCount.textContent = String(state.brief.length);
      invalidateInterviewForBriefChange();
      elements.brief.focus();
    });
    elements.visualShortcuts.appendChild(button);
  }
}

async function loadCatalog() {
  try {
    const response = await fetch("/api/v3/catalog");
    const data = await response.json();
    state.catalog = Array.isArray(data?.catalog?.entries) ? data.catalog.entries : [];
    renderVisualShortcuts(data.visual_controls || []);
    const outputCount = state.catalog.filter((entry) => entry.tier_1 === "결과물").length;
    elements.catalogCount.textContent = `대표 결과물 ${outputCount || 6}`;
    renderCatalogExplorer();
  } catch {
    elements.catalogGrid.textContent = "시각 카탈로그를 불러오지 못했습니다. 요청을 입력해 계속할 수 있습니다.";
  }
}


async function runInterview(options = {}) {
  const {
    focusQuestion = true,
    catalogFocusId = null,
    catalogAnchorTop = null,
    routeFocusKey = null,
    routeFocusValue = null,
  } = options;
  if (!state.brief.trim()) {
    setStatus("요청을 먼저 입력하세요.", true);
    if (catalogFocusId) restoreCatalogFocus(catalogFocusId, catalogAnchorTop);
    else elements.brief.focus();
    return;
  }
  const requestId = ++state.interviewRequestId;
  const payload = interviewPayload();
  setBusy(true, "구조를 분석하고 있습니다.");
  try {
    const data = await api("/api/v3/interview", payload);
    if (requestId !== state.interviewRequestId) return;
    applyOutcome(data.interview, { focusQuestion });
  } catch (error) {
    if (requestId !== state.interviewRequestId) return;
    setStatus(error.message, true);
    setBadge("오류", "error");
  } finally {
    if (requestId === state.interviewRequestId) {
      setBusy(false);
      if (catalogFocusId) restoreCatalogFocus(catalogFocusId, catalogAnchorTop);
      if (routeFocusKey) restoreRouteFocus(routeFocusKey, routeFocusValue);
    }
  }
}

function applyOutcome(outcome, options = {}) {
  const { save = true, focusQuestion = true } = options;
  state.outcome = outcome;
  state.normalizedAnswers = { ...(outcome.normalized_answers || {}) };
  renderQuickOptions();
  renderCatalogExplorer();
  renderRouteSummary();
  renderInspector(outcome);
  if (outcome.status === "needs_input") {
    state.currentQuestion = outcome.questions[0] || null;
    renderQuestion(state.currentQuestion, outcome.questions.length, { focus: focusQuestion });
    const catalogQuestion = state.currentQuestion?.id === "image.category";
    elements.emptyStage.classList.toggle("hidden", !catalogQuestion);
    if (catalogQuestion) elements.directionExamples.open = false;
    elements.resultStage.classList.add("hidden");

    setStatus(`현재 필요한 결정 ${outcome.questions.length}개 · ${state.currentQuestion?.label || "질문을 확인해 주세요."}`);
    setBadge("질문", "warn");
    return;
  }
  state.currentQuestion = null;
  elements.questionCard.classList.add("hidden");
  const compilation = outcome.compilation || {};
  if (outcome.status === "ready" && compilation.prompt) {
    renderResult(compilation.prompt, outcome.request, compilation);
    if (save) saveRecent(outcome);

    setStatus("프롬프트가 검증되었습니다.");
    setBadge(compilation.status === "valid_with_warnings" ? "경고 있음" : "유효", compilation.status === "valid_with_warnings" ? "warn" : "ok");
  } else {
    elements.emptyStage.classList.remove("hidden");
    elements.resultStage.classList.add("hidden");
    setStatus("입력 계약을 충족하지 못했습니다.", true);
    setBadge("무효", "error");
  }
}

function renderQuestion(question, remaining, options = {}) {
  const { focus = true } = options;
  if (!question) {
    elements.questionCard.classList.add("hidden");
    elements.questionCard.removeAttribute("data-question-id");
    return;
  }
  elements.questionCard.classList.remove("hidden");
  elements.questionCard.dataset.questionId = question.id;
  elements.questionIndex.textContent = `핵심 결정 · 현재 확인 ${remaining}개`;
  elements.questionLabel.textContent = question.label;
  elements.questionHelp.textContent = question.help || "";
  elements.questionError.textContent = question.validation_error || "";
  elements.questionError.classList.toggle("hidden", !question.validation_error);
  elements.skipQuestion.classList.toggle("hidden", question.required);
  elements.questionControl.replaceChildren();
  renderQuestionContext(question);

  let control;
  const optionEntries = (question.options || [])
    .map((option) => ({ option, entry: catalogEntry(option.value) }))
    .filter((item) => item.entry);
  if ((question.control === "choice" || question.control === "multi_choice") && optionEntries.length) {
    control = renderCatalogChoiceControl(question, optionEntries);
  } else if (question.id === "image.text" && question.control === "multiline") {
    control = renderExactTextControl(question);
  } else if (question.control === "choice") {
    control = document.createElement("fieldset");
    control.id = "question-input";
    control.className = "choice-grid";
    control.setAttribute("aria-label", question.label);
    for (const option of question.options || []) {
      const label = document.createElement("label");
      label.className = "choice-option";
      const radio = document.createElement("input");
      radio.type = "radio";
      radio.name = "question-option";
      radio.value = option.value;
      radio.checked = effectiveAnswer(question.id) === option.value;
      const copy = document.createElement("span");
      const title = document.createElement("strong");
      title.textContent = option.label;
      const description = document.createElement("small");
      description.textContent = option.description || "";
      copy.append(title, description);
      label.append(radio, copy);
      control.appendChild(label);
    }
  } else if (question.control === "multi_choice") {
    control = document.createElement("fieldset");
    control.id = "question-input";
    control.className = "choice-grid";
    control.setAttribute("aria-label", question.label);
    const selected = new Set(
      (effectiveAnswer(question.id) || "")
        .split(",")
        .map((value) => value.trim())
        .filter((value) => value && value !== "none"),
    );
    for (const option of question.options || []) {
      const label = document.createElement("label");
      label.className = "choice-option";
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.name = "question-option";
      checkbox.value = option.value;
      checkbox.checked = selected.has(option.value);
      const copy = document.createElement("span");
      const title = document.createElement("strong");
      title.textContent = option.label;
      const description = document.createElement("small");
      description.textContent = option.description || "";
      copy.append(title, description);
      label.append(checkbox, copy);
      control.appendChild(label);
    }
  } else if (question.control === "multiline") {
    control = document.createElement("textarea");
    control.rows = 4;
    control.id = "question-input";
    control.placeholder = question.placeholder || "";
    control.value = effectiveAnswer(question.id) || "";
  } else {
    control = document.createElement("input");
    control.type = "text";
    control.id = "question-input";
    control.placeholder = question.placeholder || "";
    control.value = effectiveAnswer(question.id) || "";
  }
  control.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") submitAnswer();
  });
  if (question.validation_error) control.setAttribute("aria-describedby", "question-error");
  elements.questionControl.appendChild(control);
  renderQuestionSuggestions(question, control);
  if (focus) {
    requestAnimationFrame(() => {
      const focusTarget = control.querySelector?.("input, button") || control;
      focusTarget?.focus();
    });
  }
}

// The question card states which output the decision belongs to, why it is being asked,
// and what it changes, so a required answer never reads as an arbitrary form field.
function renderQuestionContext(question) {
  if (!elements.questionWhy) return;
  elements.questionWhy.replaceChildren();
  const category = effectiveAnswer("image.category");
  const profile = OUTPUT_PROFILES[category];
  const entry = catalogEntry(category);
  const [reason, impact] = QUESTION_CONTEXT[question.id] || [];
  const rows = [
    ["결과물", profile ? `${category} · ${entry?.name_ko || profile.name}` : "아직 정하지 않음"],
    ["묻는 이유", reason || "이 값이 없으면 프롬프트 계약을 완성할 수 없습니다"],
    ["바뀌는 것", impact || "최종 프롬프트의 해당 절이 결정됩니다"],
  ];
  for (const [label, value] of rows) {
    const row = document.createElement("div");
    const key = document.createElement("span");
    key.textContent = label;
    const detail = document.createElement("strong");
    detail.textContent = value;
    row.append(key, detail);
    elements.questionWhy.appendChild(row);
  }
}

function renderQuestionSuggestions(question, control) {
  const suggestions = QUESTION_SUGGESTIONS[question.id];
  if (!suggestions || !("value" in control)) return;
  const row = document.createElement("div");
  row.className = "suggestion-row";
  const label = document.createElement("span");
  label.textContent = "예시 채우기";
  row.appendChild(label);
  for (const suggestion of suggestions) {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "suggestion-chip";
    chip.textContent = suggestion;
    // Suggestions fill the field only. Submitting stays an explicit user decision.
    chip.addEventListener("click", () => {
      control.value = suggestion;
      control.dispatchEvent(new Event("input", { bubbles: true }));
      control.focus();
    });
    row.appendChild(chip);
  }
  elements.questionControl.appendChild(row);
}

function renderExactTextControl(question) {
  const wrap = document.createElement("div");
  wrap.className = "text-editor";
  const area = document.createElement("textarea");
  area.rows = 5;
  area.id = "question-input";
  area.placeholder = question.placeholder || "";
  area.value = effectiveAnswer(question.id) || "";
  const meta = document.createElement("div");
  meta.className = "text-editor-meta";
  const lines = document.createElement("span");
  const chars = document.createElement("span");
  const update = () => {
    const rows = area.value.split("\n").filter((line) => line.trim());
    lines.textContent = `${rows.length}줄이 그대로 렌더됩니다`;
    chars.textContent = `${area.value.length}자`;
  };
  area.addEventListener("input", update);
  update();
  meta.append(lines, chars);
  wrap.append(area, meta);
  // `currentQuestionValue` reads `control.value`, so the wrapper forwards it to the textarea.
  Object.defineProperty(wrap, "value", {
    get: () => area.value,
    set: (next) => {
      area.value = next;
      update();
    },
  });
  wrap.focus = () => area.focus();
  return wrap;
}

function currentQuestionValue(question, control) {
  if (question.control === "choice" && control?.dataset.value) return control.dataset.value;
  if (question.control === "choice") {
    return control?.querySelector('input[name="question-option"]:checked')?.value || "";
  }
  if (question.control === "multi_choice") {
    return Array.from(control?.querySelectorAll('input[name="question-option"]:checked') || [])
      .map((input) => input.value)
      .join(",");
  }
  return control?.value?.trim() || "";
}

function renderCatalogChoiceControl(question, optionEntries) {
  const control = document.createElement("fieldset");
  control.id = "question-input";
  control.className = "choice-grid catalog-choice-grid";
  control.setAttribute("aria-label", question.label);
  const selected = new Set(
    (effectiveAnswer(question.id) || "")
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean),
  );
  for (const { option, entry } of optionEntries) {
    const input = document.createElement("input");
    input.type = question.control === "multi_choice" ? "checkbox" : "radio";
    input.name = "question-option";
    input.value = option.value;
    input.checked = selected.has(option.value);
    input.className = "visually-hidden";
    const wrap = renderCatalogCard(entry, {
      input,
      selected: input.checked,
      onPreview: (trigger) => openCatalogPreview(entry, trigger),
    });
    const card = wrap.querySelector(".catalog-card");
    input.addEventListener("change", () => {
      card.classList.toggle("selected", input.checked);
      if (question.control === "choice" && input.checked) {
        control.querySelectorAll(".catalog-card").forEach((item) => {
          if (item !== card) item.classList.remove("selected");
        });
      }
    });
    control.appendChild(wrap);
  }
  return control;
}

async function submitAnswer(valueOverride) {
  const question = state.currentQuestion;
  if (!question) return;
  const control = $("question-input");
  const value = valueOverride ?? currentQuestionValue(question, control);
  if (!value && question.required) {
    elements.questionError.textContent = "이 항목은 프롬프트 계약에 필요합니다.";
    elements.questionError.classList.remove("hidden");
    const focusTarget = control?.querySelector?.("input, button") || control;
    focusTarget?.focus();
    return;
  }
  state.explicitAnswers[question.id] = value || "none";
  await runInterview();
}

function renderResult(prompt, request, compilation) {
  clearArtifact();
  elements.emptyStage.classList.add("hidden");
  elements.resultStage.classList.remove("hidden");
  elements.stageActions.classList.remove("hidden");
  elements.resultDetails.open = false;
  elements.promptOutput.textContent = prompt;
  elements.requestJson.textContent = JSON.stringify(request || {}, null, 2);
  elements.copyPrompt.disabled = false;
  elements.copyRequest.disabled = false;
  elements.downloadRequest.disabled = false;
  elements.execute.disabled = false;
  elements.execute.textContent = "이미지 생성";
  updatePrimaryActionLabel();
  const diagnosticCount = compilation?.diagnostics?.length || 0;
  const lines = prompt.split("\n").length;
  elements.resultSummary.textContent = `${Array.from(prompt).length}자 · ${lines}줄 · 진단 ${diagnosticCount}개`;
  const plan = compilation?.metadata?.generation_plan;
  renderResultOverview(plan);
  renderGenerationPlan(plan);
}

function renderResultOverview(plan) {
  elements.resultLocksPreview.replaceChildren();
  for (const lock of (plan?.locks || []).slice(0, 3)) {
    const chip = document.createElement("span");
    chip.textContent = lock;
    elements.resultLocksPreview.appendChild(chip);
  }
}

function renderGenerationPlan(plan) {
  elements.generationPlanGrid.replaceChildren();
  if (!plan) {
    elements.generationPlan.classList.add("hidden");
    return;
  }
  elements.generationPlan.classList.remove("hidden");
  const detail = plan.detail || {};
  const detailState = detail.current === detail.recommended
    ? `${detail.current || "—"} · 권장값 적용`
    : `${detail.current || "—"} · 권장 ${detail.recommended || "—"}`;
  elements.generationPlanGrid.appendChild(
    planCard("디테일", detailState, detail.reason || "출력 복잡도에 맞춰 결정"),
  );
  elements.generationPlanGrid.appendChild(
    planCard("잠금", (plan.locks || []).join(" · ") || "기본 계약", "생성에서도 동일하게 유지"),
  );
  elements.generationPlanGrid.appendChild(
    planCard("변주 축", plan.variant_axis?.label || "단일 결과물", plan.variant_axis?.rule || "한 번에 한 축만 변경"),
  );
  const qa = (plan.qa_criteria || [])
    .map((criterion) => criterion.label)
    .filter(Boolean)
    .join(" · ");
  elements.generationPlanGrid.appendChild(
    planCard("생성 후 확인", qa || "목표 · 재질 · 레이아웃", "요청 조건별로 실제 결과를 확인"),
  );
}

function planCard(kicker, title, copy) {
  const card = document.createElement("article");
  card.className = "generation-plan-card";
  const label = document.createElement("span");
  label.textContent = kicker;
  const strong = document.createElement("strong");
  strong.textContent = title;
  const body = document.createElement("small");
  body.textContent = copy;
  card.append(label, strong, body);
  return card;
}

function renderInspector(outcome) {
  const inferences = outcome?.inferences || [];
  elements.inferenceList.replaceChildren();
  if (!inferences.length) {
    elements.inferenceList.textContent = "명시한 입력을 우선해 자동 추론을 하지 않았습니다.";
  } else {
    for (const item of inferences) elements.inferenceList.appendChild(listItem("추론", item));
  }
  const catalogEntries = outcome?.compilation?.metadata?.catalog_entries || [];
  elements.knowledgeList.replaceChildren();
  if (catalogEntries.length) {
    for (const entry of catalogEntries) {
      const title = `${entry.id} · ${entry.name_ko || entry.name_en || entry.intent}`;
      const directives = (entry.prompt_directives || [])
        .slice(0, 3)
        .map((directive) => `${directive.slot}: ${directive.text}`)
        .join(" · ");
      const textContract = entry.text_requirement && entry.text_requirement !== "optional"
        ? ` · text=${entry.text_requirement}`
        : "";
      elements.knowledgeList.appendChild(
        listItem(title, `${directives || entry.intent || "구조화 규칙 적용"}${textContract}`),
      );
    }
  } else {
    const normalized = outcome?.normalized_answers || {};
    const pendingIds = [normalized["image.category"]].filter(Boolean);
    if (pendingIds.length) {
      for (const id of pendingIds) elements.knowledgeList.appendChild(listItem(id, "질문 완료 후 실행 directive가 확정됩니다."));
    } else {
      elements.knowledgeList.textContent = "선택한 시각 방향이 없습니다.";
    }
  }

  const diagnostics = outcome?.compilation?.diagnostics || [];
  elements.diagnosticList.replaceChildren();
  if (!diagnostics.length) {
    elements.diagnosticList.textContent = outcome?.status === "ready" ? "오류나 경고가 없습니다." : "컴파일하면 진단이 표시됩니다.";
  } else {
    for (const item of diagnostics) {
      elements.diagnosticList.appendChild(listItem(`${item.severity || "info"} · ${item.code || ""}`, `${item.path || "$"} — ${item.message || ""}`));
    }
  }
  elements.requestJson.textContent = JSON.stringify(outcome?.request || {}, null, 2);
}

function listItem(title, copy) {
  const row = document.createElement("div");
  row.className = "list-item";
  const strong = document.createElement("strong");
  strong.textContent = title;
  const body = document.createElement("span");
  body.textContent = copy;
  row.append(strong, body);
  return row;
}

async function execute() {
  if (!state.outcome || state.outcome.status !== "ready") return;
  if (state.mode === "prompt-only") {
    return;
  }
  const requestId = ++state.generationRequestId;
  const generationInput = {
    interview: interviewPayload(),
    mode: state.mode,
    output_name: `promptgen-${Date.now()}.png`,
  };
  state.generationBusy = true;
  setBusy(true, "Luna 프롬프트 검토 후 Codex 이미지 생성 중");
  elements.execute.disabled = true;
  try {
    const data = await api("/api/v3/generate", generationInput);
    if (requestId !== state.generationRequestId) return;
    if (!data.ok) throw new Error(userFacingApiError(data, 200));
    const refinement = data.receipt?.prompt_refinement;
    if (typeof refinement?.prompt === "string") {
      elements.promptOutput.textContent = refinement.prompt;
    }
    const artifactSha256 = data.receipt?.sha256 || "";
    if (!/^[0-9a-f]{64}$/i.test(artifactSha256)) {
      throw new Error("생성 receipt의 SHA-256이 유효하지 않습니다.");
    }
    const expectedArtifactUrl = `/artifacts/${generationInput.output_name}?sha256=${artifactSha256.toLowerCase()}`;
    if (data.artifact_url !== expectedArtifactUrl) {
      throw new Error("생성 artifact URL이 receipt의 SHA-256과 일치하지 않습니다.");
    }
    state.artifactUrl = data.artifact_url;
    elements.generatedImage.src = state.artifactUrl;
    elements.imageFrame.classList.remove("hidden");
    const dimensionNote = data.receipt.dimensions_normalized
      ? ` · provider ${data.receipt.source_width}×${data.receipt.source_height} → 정규화`
      : "";
    const refinementNote = refinement
      ? ` · ${refinement.model} 검토 · 보강 ${refinement.additions?.length || 0}개`
      : "";
    elements.resultSummary.textContent = `${data.receipt.width}×${data.receipt.height}${dimensionNote}${refinementNote} · SHA-256 ${data.receipt.sha256.slice(0, 12)}…`;
    setStatus("Luna가 검토한 프롬프트로 이미지 생성과 PNG 검증을 완료했습니다.");
    showToast("Luna 검토와 이미지 생성이 완료되었습니다.");
  } catch (error) {
    if (requestId !== state.generationRequestId) return;
    setStatus(error.message, true);
    showToast(error.message);
  } finally {
    if (requestId === state.generationRequestId) {
      state.generationBusy = false;
      setBusy(false);
      elements.execute.disabled = false;
    }
  }
}

function switchMode(mode) {
  if (mode !== state.mode && state.generationBusy) {
    state.generationRequestId += 1;
    state.generationBusy = false;
    setBusy(false);
  }
  state.mode = mode;
  document.querySelectorAll("[data-mode]").forEach((button) => {
    const active = button.dataset.mode === mode;
    button.classList.toggle("active", active);
    button.setAttribute("aria-pressed", String(active));
  });
  updatePrimaryActionLabel();
  if (state.outcome?.status === "ready") {
    elements.execute.textContent = "이미지 생성";
  }
}

function resetOutput(clearBrief = true) {
  if (clearBrief) {
    state.brief = "";
    elements.brief.value = "";
    elements.briefCount.textContent = "0";
  }
  state.sampleBackup = null;
  elements.undoSample?.classList.add("hidden");
  state.explicitAnswers = {};
  clearResolvedOutcome();
  elements.questionCard.classList.add("hidden");
  elements.resultStage.classList.add("hidden");
  elements.textComposer.classList.remove("hidden");
  elements.emptyStage.classList.remove("hidden");
  elements.directionExamples.open = false;
  clearArtifact();
  elements.promptOutput.textContent = "";
  elements.requestJson.textContent = "{}";
  elements.inferenceList.textContent = "아직 추론된 값이 없습니다.";
  elements.diagnosticList.textContent = "컴파일하면 진단이 표시됩니다.";
  elements.knowledgeList.textContent = "선택한 시각 지식이 없습니다.";
  elements.copyPrompt.disabled = true;
  elements.copyRequest.disabled = true;
  elements.downloadRequest.disabled = true;
  elements.execute.disabled = true;
  updatePrimaryActionLabel();
  renderQuickOptions();
  renderCatalogExplorer();
  renderRouteSummary();

  setBadge("대기", "neutral");
  setStatus("입력 대기");
}

function invalidateInterviewForBriefChange() {
  const preserved = {};
  for (const key of ["image.category"]) {
    if (hasOwn(state.explicitAnswers, key)) preserved[key] = state.explicitAnswers[key];
  }
  state.explicitAnswers = preserved;
  clearResolvedOutcome();
  elements.questionCard.classList.add("hidden");
  elements.resultStage.classList.add("hidden");
  elements.generationPlan.classList.add("hidden");
  elements.emptyStage.classList.remove("hidden");
  clearArtifact();
  elements.promptOutput.textContent = "";
  elements.requestJson.textContent = "{}";
  elements.inferenceList.textContent = "새 요청을 분석하면 추론이 표시됩니다.";
  elements.diagnosticList.textContent = "컴파일하면 진단이 표시됩니다.";
  elements.knowledgeList.textContent = "새 요청에서 시각 방향을 다시 선택할 수 있습니다.";
  elements.copyPrompt.disabled = true;
  elements.copyRequest.disabled = true;
  elements.downloadRequest.disabled = true;
  elements.execute.disabled = true;
  renderQuickOptions();
  renderCatalogExplorer();
  renderRouteSummary();

  setBadge("대기", "neutral");
  setStatus("요청이 바뀌어 이전 상세 결정과 결과를 초기화했습니다.");
}

function setInspectorOpen(open) {
  elements.inspectorPanel.classList.toggle("open", open);
  elements.inspectorPanel.setAttribute("aria-hidden", String(!open));
  elements.inspectorToggle.setAttribute("aria-expanded", String(open));
  elements.inspectorBackdrop.classList.toggle("hidden", !open);
  document.querySelector(".workspace").inert = open;
  document.querySelector(".workbench").inert = open;
  if (open) elements.inspectorClose.focus();
  else elements.inspectorToggle.focus();
}

function readRecent() {
  try {
    const value = JSON.parse(localStorage.getItem(HISTORY_KEY) || "[]");
    return Array.isArray(value)
      ? value.filter((item) => (
        item?.schema_version === HISTORY_SCHEMA_VERSION
        && typeof item.brief === "string"
        && item.outcome?.status === "ready"
      ))
      : [];
  } catch {
    return [];
  }
}

function writeRecent(items) {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(items.slice(0, 6)));
  } catch {
    // Browser storage is optional. Prompt compilation remains fully functional.
  }
}

function saveRecent(outcome) {
  if (!state.brief.trim() || outcome?.status !== "ready") return;
  const current = readRecent().filter((item) => !(item.brief === state.brief));
  current.unshift({
    schema_version: HISTORY_SCHEMA_VERSION,
    kind: "image",
    mode: state.mode,
    brief: state.brief,
    explicit_answers: { ...state.explicitAnswers },
    normalized_answers: { ...state.normalizedAnswers },
    outcome: JSON.parse(JSON.stringify(outcome)),
    created_at: new Date().toISOString(),
  });
  writeRecent(current);
  renderRecent();
}

function renderRecent() {
  const items = readRecent();
  elements.recentList.replaceChildren();
  if (!items.length) {
    const empty = document.createElement("p");
    empty.className = "empty-copy";
    empty.textContent = "완성한 프롬프트가 이 기기에만 저장됩니다.";
    elements.recentList.appendChild(empty);
    return;
  }
  for (const item of items) {
    const row = document.createElement("article");
    row.className = "recent-item";
    const restore = document.createElement("button");
    restore.className = "recent-restore";
    restore.type = "button";
    restore.title = "저장된 결과 그대로 열기";
    const icon = document.createElement("span");
    icon.className = "recent-icon";
    icon.textContent = "↗";
    const copy = document.createElement("span");
    copy.className = "recent-copy";
    const title = document.createElement("strong");
    title.textContent = item.brief;
    const meta = document.createElement("small");
    const time = new Date(item.created_at);
    const category = item.normalized_answers?.["image.category"];
    meta.textContent = [
      category,
      Number.isNaN(time.getTime()) ? "local" : time.toLocaleString("ko-KR"),
    ].filter(Boolean).join(" · ");
    copy.append(title, meta);
    restore.append(icon, copy);
    restore.addEventListener("click", () => {
      invalidatePendingWork();
      state.mode = item.mode === "codex-imagegen" ? "codex-imagegen" : "prompt-only";
      switchMode(state.mode);
      state.brief = item.brief;
      elements.brief.value = item.brief;
      elements.briefCount.textContent = String(item.brief.length);
      state.explicitAnswers = { ...(item.explicit_answers || {}) };
      state.normalizedAnswers = { ...(item.normalized_answers || {}) };
      applyOutcome(JSON.parse(JSON.stringify(item.outcome)), { save: false });
      setStatus("저장된 프롬프트 결과를 그대로 열었습니다.");
    });
    const reanalyze = document.createElement("button");
    reanalyze.type = "button";
    reanalyze.className = "recent-reanalyze";
    reanalyze.textContent = "다시 분석";
    reanalyze.title = "현재 엔진으로 요청을 다시 분석";
    reanalyze.addEventListener("click", () => {
      state.mode = item.mode === "codex-imagegen" ? "codex-imagegen" : "prompt-only";
      switchMode(state.mode);
      state.brief = item.brief;
      elements.brief.value = item.brief;
      elements.briefCount.textContent = String(item.brief.length);
      state.explicitAnswers = { ...(item.explicit_answers || {}) };
      clearResolvedOutcome();
      runInterview();
    });
    row.append(restore, reanalyze);
    elements.recentList.appendChild(row);
  }
}

function setBusy(busy, message) {
  state.busy = busy;
  elements.analyze.disabled = busy;
  elements.answerQuestion.disabled = busy;
  elements.textComposer.setAttribute("aria-busy", String(busy));
  if (message) setStatus(message);
}

function setStatus(message, isError = false) {
  elements.statusLine.textContent = message;
  elements.statusLine.style.color = isError ? "var(--danger)" : "";
  announce(message);
}

function setBadge(label, type) {
  elements.validationBadge.textContent = label;
  elements.validationBadge.className = `status-badge ${type}`;
}

async function copyText(text, label) {
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    showToast(`${label} 복사 완료`);
  } catch {
    showToast("클립보드 접근에 실패했습니다.");
  }
}

function downloadRequest() {
  const request = state.outcome?.request;
  if (!request) return;
  const blob = new Blob([`${JSON.stringify(request, null, 2)}\n`], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "promptgen-image-request.json";
  link.click();
  URL.revokeObjectURL(url);
}

let toastTimer;
function showToast(message) {
  clearTimeout(toastTimer);
  elements.toast.textContent = message;
  elements.toast.classList.remove("hidden");
  announce(message);
  toastTimer = setTimeout(() => elements.toast.classList.add("hidden"), 2400);
}

let announcementFrame;
function announce(message) {
  if (!message || elements.announcer.textContent === message) return;
  cancelAnimationFrame(announcementFrame);
  elements.announcer.textContent = "";
  announcementFrame = requestAnimationFrame(() => {
    elements.announcer.textContent = message;
  });
}

async function loadHealth() {
  try {
    const response = await fetch("/api/v3/health");
    const data = await response.json();
    elements.version.textContent = `v${data.version}`;
  } catch {
    elements.version.textContent = "offline";
  }
}


function keyboardIndex(event, current, length) {
  if (event.key === "Home") return 0;
  if (event.key === "End") return length - 1;
  if (event.key === "ArrowRight" || event.key === "ArrowDown") return (current + 1) % length;
  if (event.key === "ArrowLeft" || event.key === "ArrowUp") return (current - 1 + length) % length;
  return null;
}

function activateFromKeyboard(event, container, selector, dataKey) {
  const items = [...container.querySelectorAll(selector)];
  const current = items.indexOf(event.target);
  if (current < 0 || !items.length) return;
  const nextIndex = keyboardIndex(event, current, items.length);
  if (nextIndex === null) return;
  event.preventDefault();
  const next = items[nextIndex];
  const value = next.dataset[dataKey];
  next.click();
  requestAnimationFrame(() => {
    const restored = [...container.querySelectorAll(selector)]
      .find((item) => item.dataset[dataKey] === value);
    restored?.focus({ preventScroll: true });
  });
}

elements.brief.addEventListener("input", () => {
  const nextBrief = elements.brief.value;
  const invalidatesPreviousInterview =
    nextBrief !== state.brief && Boolean(state.outcome || state.currentQuestion || state.busy);
  state.brief = nextBrief;
  elements.briefCount.textContent = String(state.brief.length);
  if (invalidatesPreviousInterview) invalidateInterviewForBriefChange();
});
elements.analyze.addEventListener("click", runInterview);
elements.generatedImage.addEventListener("error", () => {
  if (!state.artifactUrl || elements.generatedImage.getAttribute("src") !== state.artifactUrl) return;
  elements.imageFrame.classList.add("hidden");
  setStatus("생성 파일을 확인하지 못했습니다. 파일이 바뀌었거나 표시할 수 없습니다. 실행 기록의 해시와 실제 파일을 확인하세요.", true);
});
elements.answerQuestion.addEventListener("click", () => submitAnswer());
elements.skipQuestion.addEventListener("click", () => submitAnswer("none"));
elements.routeMediumSelect.addEventListener("change", () => {
  updateTechnicalAnswer("image.medium", elements.routeMediumSelect.value);
});
elements.routeAspectSelect.addEventListener("change", () => {
  updateTechnicalAnswer("image.aspect_ratio", elements.routeAspectSelect.value);
});
elements.routeDetailSelect.addEventListener("change", () => {
  updateTechnicalAnswer("image.detail", elements.routeDetailSelect.value);
});
elements.routeTextPosition.addEventListener("change", () => {
  updateTechnicalAnswer("image.text_position", elements.routeTextPosition.value);
});
elements.routeTextStyle.addEventListener("change", () => {
  updateTechnicalAnswer("image.text_style", elements.routeTextStyle.value);
});
elements.routeAuto.addEventListener("click", () => {
  for (const key of [
    "image.medium",
    "image.profile",
    "image.aspect_ratio",
    "image.detail",
    "image.lut",
    "image.text_position",
    "image.text_style",
  ]) {
    delete state.explicitAnswers[key];
  }
  clearResolvedOutcome();
  renderRouteSummary();
  if (state.brief.trim()) runInterview();
});
elements.execute.addEventListener("click", execute);
elements.editDirection.addEventListener("click", () => {
  elements.emptyStage.classList.remove("hidden");
  elements.directionExamples.open = true;

  setStatus("현재 결과를 유지한 채 대표 결과물을 다시 고를 수 있습니다.");
  elements.emptyStage.scrollIntoView({ behavior: "smooth", block: "start" });
  requestAnimationFrame(() => {
    const selected = elements.catalogGrid.querySelector(".catalog-card.selected");
    (selected || elements.catalogGrid.querySelector(".catalog-card"))?.focus({ preventScroll: true });
  });
});
elements.copyPrompt.addEventListener("click", () => copyText(elements.promptOutput.textContent, "프롬프트"));
elements.copyRequest.addEventListener("click", () => copyText(JSON.stringify(state.outcome?.request || {}, null, 2), "요청 JSON"));
elements.downloadRequest.addEventListener("click", downloadRequest);
elements.inspectorToggle.addEventListener("click", () => {
  setInspectorOpen(!elements.inspectorPanel.classList.contains("open"));
});
elements.inspectorPanel.addEventListener("keydown", (event) => {
  if (event.key !== "Tab") return;
  const focusable = [...elements.inspectorPanel.querySelectorAll("button:not(:disabled), input, textarea, select, a[href], [tabindex='0']")];
  const first = focusable[0], last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
  else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
});
elements.inspectorClose.addEventListener("click", () => setInspectorOpen(false));
elements.inspectorBackdrop.addEventListener("click", () => setInspectorOpen(false));
elements.clearHistory.addEventListener("click", () => {
  writeRecent([]);
  renderRecent();
});
elements.catalogSearch.addEventListener("input", () => {
  state.catalogQuery = elements.catalogSearch.value;
  renderCatalogExplorer();
});
elements.catalogSearch.addEventListener("keydown", (event) => {
  if (event.key !== "Escape" || !elements.catalogSearch.value) return;
  event.stopPropagation();
  elements.catalogSearch.value = "";
  state.catalogQuery = "";
  renderCatalogExplorer();
});
elements.catalogGrid.addEventListener("keydown", catalogGridKeydown);
elements.runOptions.addEventListener("keydown", (event) => {
  activateFromKeyboard(event, elements.runOptions, "[data-mode]", "mode");
});
for (const group of [
  elements.routeMediumSelect,
  elements.routeProfileSelect,
  elements.routeAspectSelect,
  elements.routeDetailSelect,
  elements.routeTextPosition,
  elements.routeTextStyle,
  elements.lutGrid,
]) {
  group.addEventListener("keydown", (event) => {
    const dataKey = event.target?.dataset?.lut ? "lut" : "value";
    activateFromKeyboard(event, group, '[role="radio"]', dataKey);
  });
}
elements.themeToggle.addEventListener("click", () => {
  const next = THEME_ORDER[(THEME_ORDER.indexOf(state.theme) + 1) % THEME_ORDER.length];
  applyTheme(next);
  showToast(THEME_LABEL[next]);
});
elements.previewClose.addEventListener("click", closeCatalogPreview);
elements.previewDialog.addEventListener("click", (event) => {
  if (event.target === elements.previewDialog) closeCatalogPreview();
});
elements.previewDialog.addEventListener("keydown", trapPreviewFocus);
elements.previewSelect.addEventListener("click", () => {
  const entry = state.previewEntry;
  if (!entry) return;
  const profileValue = state.previewProfile;
  closeCatalogPreview();
  selectCatalogEntry(entry, null, profileValue);
});
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (!elements.previewDialog.classList.contains("hidden")) {
    closeCatalogPreview();
    return;
  }
  if (elements.inspectorPanel.classList.contains("open")) {
    setInspectorOpen(false);
    elements.inspectorToggle.focus();
  }
});
$("reset-all").addEventListener("click", () => resetOutput(true));
document.querySelectorAll("[data-mode]").forEach((button) => button.addEventListener("click", () => switchMode(button.dataset.mode)));
document.querySelectorAll("[data-sample]").forEach((button) => button.addEventListener("click", () => {
  const sample = button.dataset.sample || "";
  const backup = {
    kind: "image",
    mode: state.mode,
    brief: state.brief,
    explicitAnswers: { ...state.explicitAnswers },
    normalizedAnswers: { ...state.normalizedAnswers },
    outcome: state.outcome ? JSON.parse(JSON.stringify(state.outcome)) : null,
    artifactUrl: state.artifactUrl,
  };
  resetOutput(false);
  state.sampleBackup = backup;
  state.brief = sample;
  elements.brief.value = sample;
  elements.briefCount.textContent = String(sample.length);
  elements.undoSample.classList.remove("hidden");
  setStatus(`${button.dataset.sampleLabel || "예시 요청"} 예시를 입력란에 넣었습니다. 필요하면 방금 예시를 취소할 수 있습니다.`);
  elements.brief.focus();
}));
elements.undoSample.addEventListener("click", () => {
  const backup = state.sampleBackup;
  if (!backup) return;
  state.mode = backup.mode;
  switchMode(backup.mode);
  state.brief = backup.brief;
  state.explicitAnswers = { ...backup.explicitAnswers };
  state.normalizedAnswers = { ...backup.normalizedAnswers };
  state.outcome = backup.outcome;
  state.currentQuestion = backup.outcome?.questions?.[0] || null;
  state.artifactUrl = backup.artifactUrl;
  elements.brief.value = backup.brief;
  elements.briefCount.textContent = String(backup.brief.length);
  state.sampleBackup = null;
  elements.undoSample.classList.add("hidden");
  if (backup.outcome) {
    applyOutcome(backup.outcome, { save: false });
    if (backup.artifactUrl) {
      state.artifactUrl = backup.artifactUrl;
      elements.generatedImage.src = backup.artifactUrl;
      elements.imageFrame.classList.remove("hidden");
    }
    setStatus("예시를 적용하기 전 결과와 답변을 그대로 복원했습니다.");
  } else {
    elements.questionCard.classList.add("hidden");
    elements.resultStage.classList.add("hidden");
    elements.emptyStage.classList.remove("hidden");
    renderQuickOptions();
    renderCatalogExplorer();
    renderRouteSummary();
    setStatus("예시를 적용하기 전 요청으로 돌아갔습니다.");
  }
  elements.brief.focus();
});

elements.brief.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter") runInterview();
});

initTheme();
renderQuickOptions();
loadHealth();
loadCatalog();
loadLutPresets();
renderRecent();
