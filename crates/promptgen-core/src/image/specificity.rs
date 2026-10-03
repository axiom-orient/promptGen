use crate::diagnostic::Diagnostic;

const AMBIGUOUS_TERMS: &[&str] = &[
    "알아서",
    "적당히",
    "대충",
    "아무렇게나",
    "원하는 대로",
    "멋지게",
    "감성적으로",
    "고급스럽게",
    "세련되게",
    "예쁘게",
    "whatever",
    "somehow",
    "as appropriate",
    "beautiful",
    "stunning",
    "premium looking",
];

const SD_ERA_TERMS: &[&str] = &[
    "masterpiece",
    "best detail",
    "8k",
    "4k",
    "uhd",
    "trending on artstation",
    "ultra-detailed",
    "highly detailed",
    "sharp focus",
    "--ar",
    "--v",
];

const NEGATIVE_STYLE_TERMS: &[&str] = &["negative prompt", "negative:", "네거티브 프롬프트"];

const YOUTH_CODED_TERMS: &[&str] = &[
    "교복",
    "학생",
    "소녀",
    "schoolgirl",
    "school boy",
    "schoolboy",
    "teen girl",
    "underage",
    "minor",
];

pub fn inspect_description(path: &str, value: &str, diagnostics: &mut Vec<Diagnostic>) {
    let normalized = value.to_lowercase();
    // Every term table is authored lowercase, so the haystack is lowered once and the
    // needles are compared as-is; `term_tables_are_lowercase` keeps that true.
    for term in AMBIGUOUS_TERMS {
        if normalized.contains(term) {
            diagnostics.push(
                Diagnostic::error(
                    "IMG_AMBIGUOUS_TERM",
                    path,
                    format!("ambiguous instruction {term:?} is not observable"),
                )
                .with_hint(
                    "replace it with visible geometry, placement, material, light, color, or measured hierarchy",
                ),
            );
        }
    }
    for term in SD_ERA_TERMS {
        if normalized.contains(term) {
            diagnostics.push(
                Diagnostic::error(
                    "IMG_DEPRECATED_PROMPT_TOKEN",
                    path,
                    format!("deprecated non-visual prompt token {term:?}"),
                )
                .with_hint("describe the visible result instead of model-era detail tags"),
            );
        }
    }
    for term in NEGATIVE_STYLE_TERMS {
        if normalized.contains(term) {
            diagnostics.push(Diagnostic::error(
                "IMG_NEGATIVE_SECTION",
                path,
                format!("negative-prompt section marker {term:?} is not allowed"),
            ));
        }
    }
    if contains_weight_syntax(value) {
        diagnostics.push(Diagnostic::error(
            "IMG_WEIGHT_SYNTAX",
            path,
            "Stable Diffusion-style weighted token syntax is not allowed",
        ));
    }
    if contains_placeholder(value) {
        diagnostics.push(
            Diagnostic::error(
                "IMG_PLACEHOLDER",
                path,
                "unresolved placeholder or slot token remains in the request",
            )
            .with_hint(
                "replace brackets, braces, and angle-bracket slots with final visual content",
            ),
        );
    }
}

pub fn inspect_exact_text(path: &str, value: &str, diagnostics: &mut Vec<Diagnostic>) {
    if contains_placeholder(value) {
        diagnostics.push(Diagnostic::error(
            "IMG_TEXT_PLACEHOLDER",
            path,
            "exact image text contains an unresolved placeholder",
        ));
    }
    if value.contains('\n') || value.contains('\r') {
        diagnostics.push(Diagnostic::error(
            "IMG_TEXT_EMBEDDED_NEWLINE",
            path,
            "each exact text line must be a separate array element",
        ));
    }
}

pub fn inspect_tier2_youth_terms(path: &str, value: &str, diagnostics: &mut Vec<Diagnostic>) {
    let normalized = value.to_lowercase();
    for term in YOUTH_CODED_TERMS {
        if normalized.contains(term) {
            diagnostics.push(Diagnostic::error(
                "IMG_TIER2_YOUTH_CODED",
                path,
                format!("youth-coded term {term:?} is forbidden in the adult editorial lane"),
            ));
        }
    }
}

pub fn contains_hangul_and_latin(value: &str) -> bool {
    let hangul = value.chars().any(
        |character| matches!(character as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF),
    );
    let latin = value
        .chars()
        .any(|character| character.is_ascii_alphabetic());
    hangul && latin
}

fn contains_weight_syntax(value: &str) -> bool {
    let bytes = value.as_bytes();
    for window in bytes.windows(4) {
        if window[0] == b'(' && window[2] == b':' && window[3].is_ascii_digit() {
            return true;
        }
    }
    false
}

fn contains_placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.contains("[PERSONA_LOCK]")
        || trimmed.contains("[TITLE]")
        || trimmed.contains("{상품명}")
        || trimmed.contains("<prompt>")
    {
        return true;
    }
    has_balanced_slot(trimmed, '[', ']')
        || has_balanced_slot(trimmed, '{', '}')
        || has_balanced_slot(trimmed, '<', '>')
}

fn has_balanced_slot(value: &str, open: char, close: char) -> bool {
    if let Some(start) = value.find(open)
        && let Some(end) = value[start + open.len_utf8()..].find(close)
    {
        let content = &value[start + open.len_utf8()..start + open.len_utf8() + end];
        return !content.trim().is_empty() && content.chars().count() <= 64;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguity_and_placeholders_are_detected() {
        let mut diagnostics = Vec::new();
        inspect_description("$.x", "알아서 고급스럽게 [TITLE]", &mut diagnostics);
        assert!(diagnostics.len() >= 3);
    }

    #[test]
    fn language_mix_detection_is_line_scoped() {
        assert!(contains_hangul_and_latin("겨울 SALE"));
        assert!(!contains_hangul_and_latin("겨울 세일"));
        assert!(!contains_hangul_and_latin("WINTER SALE"));
    }
}
