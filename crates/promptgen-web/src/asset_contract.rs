//! Design and safety contract for the embedded Studio assets.
//!
//! These assertions previously lived in an external Python script that read the asset files
//! from disk. Running them here instead checks the exact strings `include_str!` compiled into
//! the binary, so a build can never ship assets that differ from the ones a checker approved,
//! and the contract is enforced by `cargo test` rather than by remembering to run a script.
//!
//! Each rule cites the clause in `docs/DESIGN.md` it protects.

use super::{APP_CSS, APP_JS, INDEX_HTML};
use std::collections::BTreeSet;

/// Values of `id="..."` attributes in document order.
fn html_ids(html: &str) -> Vec<String> {
    attribute_values(html, "id=\"")
}

/// Values of any `name="value"` attribute, as raw strings.
fn attribute_values(html: &str, needle: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = html;
    while let Some(offset) = rest.find(needle) {
        let start = offset + needle.len();
        let Some(end) = rest[start..].find('"') else {
            break;
        };
        values.push(rest[start..start + end].to_owned());
        rest = &rest[start + end..];
    }
    values
}

#[test]
fn document_declares_korean_and_unique_control_ids() {
    assert!(
        INDEX_HTML.contains("lang=\"ko\""),
        "Studio document must declare lang=\"ko\""
    );

    let ids = html_ids(INDEX_HTML);
    let mut seen = BTreeSet::new();
    let duplicates = ids
        .iter()
        .filter(|id| !seen.insert((*id).clone()))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert!(
        duplicates.is_empty(),
        "duplicate HTML ids break label/aria wiring: {duplicates:?}"
    );
}

#[test]
fn studio_loads_no_asset_from_the_network() {
    // The Studio is loopback-only. An external stylesheet, script, or font would make a
    // local tool depend on a third party being reachable and able to observe usage.
    for value in attribute_values(INDEX_HTML, "src=\"")
        .into_iter()
        .chain(attribute_values(INDEX_HTML, "href=\""))
    {
        assert!(
            !(value.starts_with("http://")
                || value.starts_with("https://")
                || value.starts_with("//")),
            "external UI asset is forbidden: {value}"
        );
    }
}

#[test]
fn every_required_control_is_present() {
    const REQUIRED: &[&str] = &[
        "brief",
        "visual-shortcuts",
        "execution-note",
        "question-card",
        "question-control",
        "analyze",
        "execute",
        "edit-direction",
        "announcer",
        "prompt-output",
        "request-json",
        "inspector-panel",
        "knowledge-list",
        "diagnostic-list",
        "generated-image",
        "validation-badge",
        "route-summary",
        "route-medium-select",
        "route-aspect-select",
        "route-detail-select",
        "lut-setting",
        "lut-reason",
        "lut-grid",
        "catalog-search",
        "catalog-grid",
        "catalog-result-count",
        "preview-dialog",
        "preview-image",
        "preview-profile-select",
        "preview-select",
        "preview-close",
        "question-why",
        "theme-toggle",
        "stage-actions",
        "image-quick-options",
        "direction-examples",
        "result-overview",
        "result-locks-preview",
        "result-details",
    ];

    let ids = html_ids(INDEX_HTML).into_iter().collect::<BTreeSet<_>>();
    let missing = REQUIRED
        .iter()
        .filter(|id| !ids.contains(**id))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "required UI controls missing: {missing:?}"
    );
}

#[test]
fn script_avoids_dynamic_dom_and_code_evaluation() {
    // Prompt text and catalog metadata are attacker-influenced in the sense that they come
    // from files and model output. Building DOM from strings, or evaluating strings as code,
    // would turn that content into executable script.
    for primitive in ["innerHTML", "eval(", "new Function"] {
        assert!(
            !APP_JS.contains(primitive),
            "unsafe dynamic DOM/code primitive in app.js: {primitive}"
        );
    }
}

#[test]
fn script_wires_the_documented_runtime_apis() {
    for (needle, requirement) in [
        ("\"/api/v3/interview\"", "guided interview API"),
        ("fetch(\"/api/v3/health\"", "health API"),
        ("fetch(\"/api/v3/lut-presets\"", "photographic LUT presets"),
        ("multi_choice", "multi-choice interview control"),
        ("setInspectorOpen", "responsive inspector lifecycle"),
        (
            "prompt_directives",
            "runtime visual directives in the inspector",
        ),
    ] {
        assert!(
            APP_JS.contains(needle),
            "{requirement} is not wired in app.js"
        );
    }
}

#[test]
fn stylesheet_keeps_the_responsive_and_accessible_contract() {
    for (needle, requirement) in [
        (
            "@media (max-width: 900px)",
            "responsive inspector breakpoint",
        ),
        (".inspector-panel.open", "responsive inspector state"),
        (":focus-visible", "keyboard focus style"),
        (
            "@media (prefers-color-scheme: dark)",
            "dark theme following the system setting",
        ),
        (
            ":root[data-theme=\"dark\"]",
            "dark theme following the explicit toggle",
        ),
        (
            "@media (prefers-reduced-motion: reduce)",
            "reduced-motion contract",
        ),
    ] {
        assert!(
            APP_CSS.contains(needle),
            "{requirement} is missing from app.css"
        );
    }
}

#[test]
fn stylesheet_respects_the_typographic_floor() {
    // Studio readability contract: 12px is the floor. Below it, text stops being information.
    let mut undersized = BTreeSet::new();
    for (index, _) in APP_CSS.match_indices("px") {
        let head = &APP_CSS[..index];
        let digits = head
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if digits.is_empty() {
            continue;
        }
        let declaration_start = head.rfind([';', '{', '\n']).unwrap_or(0);
        let declaration = &head[declaration_start..];
        if !(declaration.contains("font-size:") || declaration.contains("font:")) {
            continue;
        }
        let size = digits.chars().rev().collect::<String>();
        if let Ok(size) = size.parse::<u32>()
            && size < 12
        {
            undersized.insert(size);
        }
    }
    assert!(
        undersized.is_empty(),
        "app.css uses type below the Studio 12px floor: {undersized:?}px"
    );
}

#[test]
fn studio_exposes_one_clear_image_entry_flow() {
    for retired in [
        "kind-switch",
        "journey-nav",
        "catalog-tier-2-tabs",
        "catalog-density",
    ] {
        assert!(
            !INDEX_HTML.contains(retired),
            "retired chrome remains: {retired}"
        );
    }
    assert!(!APP_JS.contains("switchKind"));
    assert!(INDEX_HTML.contains("id=\"visual-shortcuts\""));
    assert!(APP_JS.contains("renderVisualShortcuts(data.visual_controls"));
    assert!(APP_JS.contains("selectCatalogEntry(entry, null, profileValue)"));
    assert!(APP_JS.contains("onSelect: (card) => openCatalogPreview(entry, card)"));
}

#[test]
fn result_exposes_the_actual_prompt_and_keeps_review_on_demand() {
    let prompt = INDEX_HTML.find("id=\"prompt-output\"").unwrap();
    let details = INDEX_HTML.find("id=\"result-details\"").unwrap();
    assert!(
        prompt < details,
        "the working prompt must not be inside collapsed review details"
    );
    assert!(INDEX_HTML.contains("id=\"result-locks-preview\""));
    assert!(APP_JS.contains("elements.stageActions.classList.remove(\"hidden\")"));
    assert!(APP_JS.contains("elements.execute.classList.toggle(\"hidden\", state.mode !== \"codex-imagegen\" || editing)"));
}

#[test]
fn obsolete_reverse_surface_is_absent() {
    for needle in [
        "catalog/imports",
        "data-entry-flow=",
        "reconstruction",
        "reverseSession",
        "import-stage",
    ] {
        assert!(
            !INDEX_HTML.contains(needle),
            "obsolete HTML surface remains: {needle}"
        );
        assert!(
            !APP_JS.contains(needle),
            "obsolete JavaScript surface remains: {needle}"
        );
    }
}

#[test]
fn raw_adapter_failures_are_mapped_to_safe_ui_copy() {
    for needle in [
        "function userFacingApiError(data, status)",
        "CODEX_[A-Z_]+",
        "stderr\\s*=",
        "Codex 상태를 확인한 뒤 다시 시도해 주세요.",
    ] {
        assert!(
            APP_JS.contains(needle),
            "safe API error mapping missing: {needle}"
        );
    }
}

#[test]
fn component_styles_go_through_semantic_colour_tokens() {
    // docs/DESIGN.md §2.1: light and dark are both first-class, so component rules must not
    // hardcode a colour that only works in one of them.
    for token in [
        "--surface-sunken",
        "--text-muted",
        "--accent-soft",
        "--skeleton",
    ] {
        assert!(
            APP_CSS.matches(token).count() >= 2,
            "semantic colour token {token} is not defined for both themes"
        );
    }

    let component_css = APP_CSS
        .split_once("/* ---------- base ---------- */")
        .map_or(APP_CSS, |(_, rest)| rest);
    let mut raw = BTreeSet::new();
    for (index, _) in component_css.match_indices('#') {
        let hex = component_css[index + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect::<String>();
        if matches!(hex.len(), 3 | 4 | 6 | 8) {
            raw.insert(format!("#{hex}"));
        }
    }
    assert!(
        raw.is_empty(),
        "component CSS bypasses semantic colour tokens: {raw:?}"
    );
}

#[test]
fn desktop_workbench_layout_is_declared() {
    assert!(
        APP_CSS.contains("grid-template-columns: var(--sidebar) minmax(0, 1fr)"),
        "desktop workbench/canvas layout is missing from app.css"
    );
}

#[test]
fn assistive_technology_contract_is_intact() {
    // docs/DESIGN.md §6.4/§7: exactly one live region speaks, and state is exposed via ARIA
    // rather than only visually.
    assert_eq!(
        INDEX_HTML.matches("aria-live=\"polite\"").count(),
        1,
        "Studio must route announcements through exactly one polite live region"
    );
    assert!(
        INDEX_HTML.contains("id=\"announcer\""),
        "the polite live region must be the announcer element"
    );
    for (needle, source, requirement) in [
        (
            "data-mode=\"prompt-only\" aria-pressed=\"true\"",
            "index.html",
            "generation mode selected state",
        ),
        (
            "button.setAttribute(\"aria-pressed\"",
            "app.js",
            "generation mode updates",
        ),
        (
            "activateFromKeyboard",
            "app.js",
            "roving keyboard activation",
        ),
        (
            "tile.tabIndex = option.value === selected || (!selected && index === 0) ? 0 : -1",
            "app.js",
            "roving tabindex",
        ),
        (
            "role=\"radiogroup\"",
            "index.html",
            "accessible tile radiogroups",
        ),
    ] {
        let text = if source == "index.html" {
            INDEX_HTML
        } else {
            APP_JS
        };
        assert!(
            text.contains(needle),
            "{requirement} is missing from {source}"
        );
    }
}

#[test]
fn catalog_cards_size_and_load_from_the_real_asset() {
    // Cards adopt each representative asset's real aspect so nothing is cropped away.
    for (needle, requirement) in [
        ("naturalWidth", "measured asset dimensions"),
        ("clampRatio", "ratio correction"),
        ("IntersectionObserver", "viewport-driven asset loading"),
        ("catalog-card-fallback", "failed-asset surface"),
        ("catalog-card-media is-loading", "loading surface"),
    ] {
        assert!(
            APP_JS.contains(needle),
            "{requirement} is missing from app.js"
        );
    }
    assert!(
        APP_CSS.contains("--card-ratio") && APP_JS.contains("--card-ratio"),
        "catalog cards must size their media frame from the asset aspect ratio"
    );
}

#[test]
fn catalog_scope_is_reachable_without_drilling_down() {
    for needle in [
        "const entries = searched",
        "openCatalogPreview",
        "trapPreviewFocus",
        "catalogGridKeydown",
        "focus({ preventScroll: true })",
    ] {
        assert!(
            APP_JS.contains(needle),
            "catalog interaction missing: {needle}"
        );
    }
}

#[test]
fn route_settings_expose_every_typed_value() {
    // docs/DESIGN.md §5: promoting a control to a richer component must not silently drop
    // any value the typed request accepts.
    for ratio in ["1:1", "2:3", "3:2", "3:4", "4:5", "4:3", "16:9", "9:16"] {
        assert!(
            APP_JS.contains(&format!("{{ value: \"{ratio}\"")),
            "valid image aspect ratio {ratio} is missing from route settings"
        );
    }
    for detail in ["auto", "low", "medium", "high"] {
        assert!(
            APP_JS.contains(&format!("{{ value: \"{detail}\"")),
            "supported explicit image detail {detail} is missing from route settings"
        );
    }
    for (needle, requirement) in [
        ("renderZoneGrid", "3x3 text placement zone selector"),
        ("TEXT_POSITION_ZONES", "text placement zones"),
        ("renderTileGroup", "tile radiogroup rendering"),
    ] {
        assert!(
            APP_JS.contains(needle),
            "{requirement} is missing from app.js"
        );
    }
    assert!(
        APP_JS.matches("name: \"").count() >= 6
            && ["C1:", "C4:", "C5:", "C6:", "C10:", "C11:"]
                .iter()
                .all(|id| APP_JS.contains(id)),
        "all six representative output profiles must be represented in the Studio"
    );
}

#[test]
fn explicit_answers_never_absorb_backend_normalization() {
    // A backend-resolved answer presented as something the user typed would make the
    // interview appear to have captured a decision it never asked about.
    assert!(
        APP_JS.contains("explicitAnswers") && APP_JS.contains("normalizedAnswers"),
        "explicit user answers and backend-resolved answers must have separate state"
    );
    for assignment in [
        "state.explicitAnswers = { ...outcome.normalized_answers",
        "state.answers = { ...outcome.normalized_answers",
    ] {
        assert!(
            !APP_JS.contains(assignment),
            "backend normalized answers must never be copied into explicit user answers"
        );
    }
    assert!(
        APP_JS.contains("interviewRequestId")
            && APP_JS.contains("requestId !== state.interviewRequestId")
            && APP_JS.contains("state.outcome || state.currentQuestion || state.busy"),
        "stale interview responses must be discarded even when the first interview is still pending"
    );
    assert!(
        APP_JS.contains("generationRequestId")
            && APP_JS.contains("requestId !== state.generationRequestId")
            && APP_JS.contains("generationBusy")
            && APP_JS.contains("function invalidatePendingWork()")
            && APP_JS.contains("invalidatePendingWork();"),
        "stale generation completions must not mutate newer UI state, including saved-result restore"
    );
}

#[test]
fn studio_state_helpers_are_singly_defined() {
    assert_eq!(
        APP_JS.matches("function switchMode(mode)").count(),
        1,
        "image generation mode switch must have exactly one implementation"
    );
    for (needle, requirement) in [
        ("promptgen.recent.v5", "local recent-prompt strip"),
        ("localStorage", "local recent-prompt storage"),
        ("dataset.questionId", "guided question identity"),
    ] {
        assert!(
            APP_JS.contains(needle),
            "{requirement} is missing from app.js"
        );
    }
}

#[test]
fn embedded_assets_are_not_truncated() {
    // A truncated `include_str!` still compiles and still serves; balanced delimiters are a
    // cheap structural check that the whole asset made it into the binary.
    for (name, text, open, close) in [
        ("app.css", APP_CSS, '{', '}'),
        ("app.js", APP_JS, '{', '}'),
        ("app.js", APP_JS, '(', ')'),
        ("app.js", APP_JS, '[', ']'),
    ] {
        assert_eq!(
            text.matches(open).count(),
            text.matches(close).count(),
            "unbalanced {open}{close} in {name}"
        );
    }
}
