use crate::json::JsonValue;

use super::model::PhotoLutSelection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPhotoLut {
    pub preset: String,
    pub strength_percent: u8,
    pub white_balance_kelvin: u16,
    pub tint: String,
    pub tone_curve: String,
    pub black_response: String,
    pub contrast: String,
    pub saturation_percent: u8,
    pub shadow_bias_hex: String,
    pub highlight_bias_hex: String,
    pub highlight_rolloff: String,
    pub skin_tone_policy: String,
    pub grain_size: String,
    pub grain_amount_percent: u8,
    pub halation_percent: u8,
    pub vignette_percent: u8,
}

impl ResolvedPhotoLut {
    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            (
                "black_response",
                JsonValue::from(self.black_response.clone()),
            ),
            ("contrast", JsonValue::from(self.contrast.clone())),
            (
                "grain_amount_percent",
                JsonValue::from(u64::from(self.grain_amount_percent)),
            ),
            ("grain_size", JsonValue::from(self.grain_size.clone())),
            (
                "halation_percent",
                JsonValue::from(u64::from(self.halation_percent)),
            ),
            (
                "highlight_bias_hex",
                JsonValue::from(self.highlight_bias_hex.clone()),
            ),
            (
                "highlight_rolloff",
                JsonValue::from(self.highlight_rolloff.clone()),
            ),
            ("preset", JsonValue::from(self.preset.clone())),
            (
                "saturation_percent",
                JsonValue::from(u64::from(self.saturation_percent)),
            ),
            (
                "shadow_bias_hex",
                JsonValue::from(self.shadow_bias_hex.clone()),
            ),
            (
                "skin_tone_policy",
                JsonValue::from(self.skin_tone_policy.clone()),
            ),
            (
                "strength_percent",
                JsonValue::from(u64::from(self.strength_percent)),
            ),
            ("tint", JsonValue::from(self.tint.clone())),
            ("tone_curve", JsonValue::from(self.tone_curve.clone())),
            (
                "vignette_percent",
                JsonValue::from(u64::from(self.vignette_percent)),
            ),
            (
                "white_balance_kelvin",
                JsonValue::from(u64::from(self.white_balance_kelvin)),
            ),
        ])
    }
}

pub const PRESET_NAMES: [&str; 8] = [
    "clean_neutral",
    "warm_pastel_filmic",
    "cool_steel",
    "restrained_teal_orange",
    "bleach_bypass",
    "tungsten_night",
    "faded_print",
    "monochrome_high_contrast",
];

pub fn available_lut_presets() -> JsonValue {
    JsonValue::array(
        PRESET_NAMES
            .into_iter()
            .filter_map(|name| base_preset(name).map(|preset| preset.to_json())),
    )
}

pub fn resolve_photo_lut(selection: &PhotoLutSelection) -> Option<ResolvedPhotoLut> {
    let mut resolved = base_preset(&selection.preset)?;
    resolved.strength_percent = selection.strength_percent;
    if let Some(value) = selection.white_balance_kelvin {
        resolved.white_balance_kelvin = value;
    }
    override_string(&mut resolved.tint, selection.tint.as_ref());
    override_string(&mut resolved.tone_curve, selection.tone_curve.as_ref());
    override_string(
        &mut resolved.black_response,
        selection.black_response.as_ref(),
    );
    override_string(&mut resolved.contrast, selection.contrast.as_ref());
    if let Some(value) = selection.saturation_percent {
        resolved.saturation_percent = value;
    }
    override_string(
        &mut resolved.shadow_bias_hex,
        selection.shadow_bias_hex.as_ref(),
    );
    override_string(
        &mut resolved.highlight_bias_hex,
        selection.highlight_bias_hex.as_ref(),
    );
    override_string(
        &mut resolved.highlight_rolloff,
        selection.highlight_rolloff.as_ref(),
    );
    override_string(
        &mut resolved.skin_tone_policy,
        selection.skin_tone_policy.as_ref(),
    );
    override_string(&mut resolved.grain_size, selection.grain_size.as_ref());
    if let Some(value) = selection.grain_amount_percent {
        resolved.grain_amount_percent = value;
    }
    if let Some(value) = selection.halation_percent {
        resolved.halation_percent = value;
    }
    if let Some(value) = selection.vignette_percent {
        resolved.vignette_percent = value;
    }
    Some(resolved)
}

fn override_string(target: &mut String, value: Option<&String>) {
    if let Some(value) = value {
        *target = value.clone();
    }
}

fn base_preset(name: &str) -> Option<ResolvedPhotoLut> {
    let value = match name {
        "clean_neutral" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 5500,
            tint: "neutral".to_owned(),
            tone_curve: "gentle S-curve with preserved midtone separation".to_owned(),
            black_response: "neutral black floor with visible near-black detail".to_owned(),
            contrast: "medium".to_owned(),
            saturation_percent: 92,
            shadow_bias_hex: "#252A30".to_owned(),
            highlight_bias_hex: "#F6F3EC".to_owned(),
            highlight_rolloff: "soft".to_owned(),
            skin_tone_policy: "neutral, preserve natural undertone".to_owned(),
            grain_size: "fine".to_owned(),
            grain_amount_percent: 4,
            halation_percent: 0,
            vignette_percent: 0,
        },
        "warm_pastel_filmic" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 5100,
            tint: "slight magenta bias".to_owned(),
            tone_curve: "gentle S-curve with softened upper mids".to_owned(),
            black_response: "slightly lifted matte blacks".to_owned(),
            contrast: "low".to_owned(),
            saturation_percent: 84,
            shadow_bias_hex: "#66727A".to_owned(),
            highlight_bias_hex: "#E7B982".to_owned(),
            highlight_rolloff: "very soft".to_owned(),
            skin_tone_policy: "warm but not orange; retain natural red variation".to_owned(),
            grain_size: "fine".to_owned(),
            grain_amount_percent: 9,
            halation_percent: 3,
            vignette_percent: 2,
        },
        "cool_steel" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 6200,
            tint: "slight green-neutral bias".to_owned(),
            tone_curve: "firm S-curve with crisp midtone separation".to_owned(),
            black_response: "deep clean blacks".to_owned(),
            contrast: "medium-high".to_owned(),
            saturation_percent: 72,
            shadow_bias_hex: "#344A5E".to_owned(),
            highlight_bias_hex: "#DDE7EC".to_owned(),
            highlight_rolloff: "medium".to_owned(),
            skin_tone_policy: "protect skin from cyan contamination".to_owned(),
            grain_size: "fine".to_owned(),
            grain_amount_percent: 5,
            halation_percent: 0,
            vignette_percent: 3,
        },
        "restrained_teal_orange" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 5600,
            tint: "neutral".to_owned(),
            tone_curve: "cinema-style S-curve with open faces".to_owned(),
            black_response: "deep blue-neutral blacks".to_owned(),
            contrast: "medium-high".to_owned(),
            saturation_percent: 88,
            shadow_bias_hex: "#1F5961".to_owned(),
            highlight_bias_hex: "#E3A16F".to_owned(),
            highlight_rolloff: "soft".to_owned(),
            skin_tone_policy: "warm skin separated from restrained teal shadows".to_owned(),
            grain_size: "fine".to_owned(),
            grain_amount_percent: 6,
            halation_percent: 2,
            vignette_percent: 4,
        },
        "bleach_bypass" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 5600,
            tint: "neutral".to_owned(),
            tone_curve: "steep S-curve with compressed color separation".to_owned(),
            black_response: "dense crushed blacks with retained contour".to_owned(),
            contrast: "high".to_owned(),
            saturation_percent: 48,
            shadow_bias_hex: "#303338".to_owned(),
            highlight_bias_hex: "#D7D4CB".to_owned(),
            highlight_rolloff: "firm".to_owned(),
            skin_tone_policy: "desaturated but recognizably natural".to_owned(),
            grain_size: "medium".to_owned(),
            grain_amount_percent: 18,
            halation_percent: 1,
            vignette_percent: 5,
        },
        "tungsten_night" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 3200,
            tint: "slight magenta bias".to_owned(),
            tone_curve: "low-key curve with protected practical lights".to_owned(),
            black_response: "deep navy-black floor".to_owned(),
            contrast: "medium-high".to_owned(),
            saturation_percent: 86,
            shadow_bias_hex: "#172A46".to_owned(),
            highlight_bias_hex: "#F0A45D".to_owned(),
            highlight_rolloff: "soft around practical lights".to_owned(),
            skin_tone_policy: "warm practical-light skin, avoid red clipping".to_owned(),
            grain_size: "medium".to_owned(),
            grain_amount_percent: 16,
            halation_percent: 7,
            vignette_percent: 5,
        },
        "faded_print" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 5000,
            tint: "slight yellow-magenta paper bias".to_owned(),
            tone_curve: "flattened print curve with compressed highlights".to_owned(),
            black_response: "lifted charcoal blacks".to_owned(),
            contrast: "low-medium".to_owned(),
            saturation_percent: 68,
            shadow_bias_hex: "#756B62".to_owned(),
            highlight_bias_hex: "#E8DCC4".to_owned(),
            highlight_rolloff: "flat print-like".to_owned(),
            skin_tone_policy: "muted warm skin with visible texture".to_owned(),
            grain_size: "medium".to_owned(),
            grain_amount_percent: 20,
            halation_percent: 0,
            vignette_percent: 7,
        },
        "monochrome_high_contrast" => ResolvedPhotoLut {
            preset: name.to_owned(),
            strength_percent: 100,
            white_balance_kelvin: 5500,
            tint: "neutral".to_owned(),
            tone_curve: "steep monochrome S-curve".to_owned(),
            black_response: "crushed black with preserved silhouette edges".to_owned(),
            contrast: "high".to_owned(),
            saturation_percent: 0,
            shadow_bias_hex: "#111111".to_owned(),
            highlight_bias_hex: "#F1F1ED".to_owned(),
            highlight_rolloff: "medium".to_owned(),
            skin_tone_policy: "not applicable; preserve luminance texture".to_owned(),
            grain_size: "medium".to_owned(),
            grain_amount_percent: 22,
            halation_percent: 2,
            vignette_percent: 9,
        },
        _ => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::model::PhotoLutSelection;

    #[test]
    fn all_presets_resolve() {
        for name in PRESET_NAMES {
            let selection = PhotoLutSelection {
                preset: name.to_owned(),
                strength_percent: 100,
                white_balance_kelvin: None,
                tint: None,
                tone_curve: None,
                black_response: None,
                contrast: None,
                saturation_percent: None,
                shadow_bias_hex: None,
                highlight_bias_hex: None,
                highlight_rolloff: None,
                skin_tone_policy: None,
                grain_size: None,
                grain_amount_percent: None,
                halation_percent: None,
                vignette_percent: None,
            };
            assert!(resolve_photo_lut(&selection).is_some(), "{name}");
        }
    }
}
