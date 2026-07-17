use crate::storage::{materials_index_backup_path, materials_index_path, HistoryItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialCategory {
    Element,
    Material,
    Color,
    Lighting,
    Camera,
    Composition,
    Style,
    Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialSourceVariant {
    pub id: String,
    pub history_id: String,
    pub thumbnail_id: String,
    pub field_path: String,
    pub prompt_zh: String,
    pub prompt_en: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct MaterialOverride {
    pub display_name: Option<String>,
    pub prompt_zh: Option<String>,
    pub prompt_en: Option<String>,
    pub aliases: Vec<String>,
    pub favorite: bool,
    pub manually_edited: bool,
    pub merged_into: Option<String>,
    pub split_from: Option<String>,
    pub split_source_ids: Vec<String>,
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialPatch {
    pub display_name: Option<String>,
    pub prompt_zh: Option<String>,
    pub prompt_en: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub favorite: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialAsset {
    pub id: String,
    pub category: MaterialCategory,
    pub generated_name: String,
    pub generated_explanation: String,
    pub generated_prompt_zh: String,
    pub generated_prompt_en: Option<String>,
    pub generated_aliases: Vec<String>,
    pub user_override: MaterialOverride,
    pub sources: Vec<MaterialSourceVariant>,
    pub created_at: u64,
    pub updated_at: u64,
}

pub fn extract_assets(item: &HistoryItem) -> Vec<MaterialAsset> {
    let Some(root) = item.reconstructed_prompt.as_ref() else {
        return Vec::new();
    };

    let mut output = Vec::new();
    extract_entities(item, root, &mut output);
    extract_scene(item, root, &mut output);
    extract_composition(item, root, &mut output);
    extract_environment(item, root, &mut output);
    extract_technical(item, root, &mut output);
    output
}

fn extract_entities(item: &HistoryItem, root: &Value, output: &mut Vec<MaterialAsset>) {
    let Some(entities) = root.get("entities").and_then(Value::as_array) else {
        return;
    };

    for (index, entity) in entities.iter().enumerate() {
        let prefix = format!("entities[{index}]");
        extract_value(item, MaterialCategory::Element, &format!("{prefix}.label"), entity.get("label"), output);
        extract_value(item, MaterialCategory::Material, &format!("{prefix}.appearance"), entity.get("appearance"), output);
        extract_value(item, MaterialCategory::Element, &format!("{prefix}.sub_elements"), entity.get("sub_elements"), output);
    }
}

fn extract_scene(item: &HistoryItem, root: &Value, output: &mut Vec<MaterialAsset>) {
    let Some(scene) = root.get("global_scene") else {
        return;
    };

    extract_value(item, MaterialCategory::Style, "global_scene.art_style", scene.get("art_style"), output);
    extract_value(item, MaterialCategory::Style, "global_scene.atmosphere", scene.get("atmosphere"), output);
    extract_value(item, MaterialCategory::Color, "global_scene.color_palette", scene.get("color_palette"), output);
    extract_value(item, MaterialCategory::Lighting, "global_scene.lighting", scene.get("lighting"), output);
}

fn extract_composition(item: &HistoryItem, root: &Value, output: &mut Vec<MaterialAsset>) {
    let Some(composition) = root.get("composition") else {
        return;
    };

    extract_value(item, MaterialCategory::Camera, "composition.camera_angle", composition.get("camera_angle"), output);
    extract_value(item, MaterialCategory::Camera, "composition.focal_length", composition.get("focal_length"), output);
    extract_value(item, MaterialCategory::Composition, "composition.framing", composition.get("framing"), output);
    extract_value(item, MaterialCategory::Composition, "composition.depth_of_field", composition.get("depth_of_field"), output);
}

fn extract_environment(item: &HistoryItem, root: &Value, output: &mut Vec<MaterialAsset>) {
    let Some(environment) = root.get("environment_details") else {
        return;
    };

    extract_value(item, MaterialCategory::Environment, "environment_details.foreground", environment.get("foreground"), output);
    extract_value(item, MaterialCategory::Environment, "environment_details.midground", environment.get("midground"), output);
    extract_value(item, MaterialCategory::Environment, "environment_details.background", environment.get("background"), output);
}

fn extract_technical(item: &HistoryItem, root: &Value, output: &mut Vec<MaterialAsset>) {
    let Some(technical) = root.get("technical_specs") else {
        return;
    };

    extract_value(item, MaterialCategory::Material, "technical_specs.texture_fidelity", technical.get("texture_fidelity"), output);
    extract_value(item, MaterialCategory::Style, "technical_specs.render_engine_style", technical.get("render_engine_style"), output);
    extract_value(item, MaterialCategory::Style, "technical_specs.vfx", technical.get("vfx"), output);
}

fn extract_value(item: &HistoryItem, category: MaterialCategory, field_path: &str, value: Option<&Value>, output: &mut Vec<MaterialAsset>) {
    let Some(value) = value else {
        return;
    };

    match value {
        Value::String(text) => {
            for fragment in split_list_like(text) {
                push_asset(item, category, field_path, &fragment, text, output);
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                if let Some(text) = value.as_str() {
                    push_asset(item, category, &format!("{field_path}[{index}]"), text.trim(), text, output);
                }
            }
        }
        _ => {}
    }
}

fn push_asset(item: &HistoryItem, category: MaterialCategory, field_path: &str, fragment: &str, source_prompt_zh: &str, output: &mut Vec<MaterialAsset>) {
    let normalized = normalize_key(fragment);
    if normalized.is_empty() {
        return;
    }

    let asset_id = stable_asset_id(category, &normalized);
    let source_prompt_en = safe_english_clause(item.prompt_en.as_deref(), category, fragment);
    let source = MaterialSourceVariant {
        id: stable_source_id(&item.id, field_path, &normalized),
        history_id: item.id.clone(),
        thumbnail_id: item.id.clone(),
        field_path: field_path.to_string(),
        prompt_zh: source_prompt_zh.to_string(),
        prompt_en: source_prompt_en.clone(),
        created_at: item.created_at,
    };

    if let Some(existing) = output.iter_mut().find(|asset| asset.category == category && normalize_key(&asset.generated_name) == normalized) {
        if !existing.sources.iter().any(|candidate| candidate.id == source.id) {
            existing.sources.push(source);
        }
        if existing.generated_prompt_en.is_none() {
            existing.generated_prompt_en = source_prompt_en;
        }
        existing.updated_at = existing.updated_at.max(item.created_at);
        return;
    }

    output.push(MaterialAsset {
        id: asset_id,
        category,
        generated_name: fragment.trim().to_string(),
        generated_explanation: format!("\u{6765}\u{6e90}\u{5b57}\u{6bb5}\u{ff1a}{field_path}"),
        generated_prompt_zh: fragment.trim().to_string(),
        generated_prompt_en: source_prompt_en,
        generated_aliases: Vec::new(),
        user_override: MaterialOverride::default(),
        sources: vec![source],
        created_at: item.created_at,
        updated_at: item.created_at,
    });
}

fn split_list_like(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let parts: Vec<_> = trimmed
        .split(is_list_punctuation)
        .map(str::trim)
        .collect();

    if parts.len() > 1 && parts.iter().all(|part| !part.is_empty()) {
        parts.into_iter().map(str::to_string).collect()
    } else {
        vec![trimmed.to_string()]
    }
}

fn is_list_punctuation(character: char) -> bool {
    matches!(character, '\u{3001}' | '\u{ff0c}' | ',' | '\u{ff1b}' | ';')
}

fn tokenize_english(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn phrase_spans(tokens: &[String], phrase: &str) -> Vec<(usize, usize)> {
    let phrase_tokens = tokenize_english(phrase);
    if phrase_tokens.is_empty() || phrase_tokens.len() > tokens.len() {
        return Vec::new();
    }

    tokens
        .windows(phrase_tokens.len())
        .enumerate()
        .filter(|(_, window)| *window == phrase_tokens.as_slice())
        .map(|(start, _)| (start, start + phrase_tokens.len()))
        .collect()
}

fn unambiguous_alias_phrase(phrases: Vec<&'static str>) -> Vec<&'static str> {
    let Some(first) = phrases.first().copied() else {
        return Vec::new();
    };

    if phrases.iter().all(|phrase| *phrase == first) {
        vec![first]
    } else {
        Vec::new()
    }
}

fn safe_chinese_alias_phrases(category: MaterialCategory, chinese_fragment: &str) -> Vec<&'static str> {
    let fragment = chinese_fragment.trim();
    let aliases = category_aliases(category);
    let exact: Vec<_> = aliases
        .iter()
        .filter(|(chinese, _)| fragment == *chinese)
        .map(|(_, english)| *english)
        .collect();
    if !exact.is_empty() {
        return unambiguous_alias_phrase(exact);
    }

    let longest_length = aliases
        .iter()
        .filter(|(chinese, _)| chinese.chars().count() >= 2 && fragment.contains(chinese))
        .map(|(chinese, _)| chinese.chars().count())
        .max();
    let Some(longest_length) = longest_length else {
        return Vec::new();
    };

    unambiguous_alias_phrase(aliases.iter().filter(|(chinese, _)| chinese.chars().count() == longest_length && fragment.contains(chinese)).map(|(_, english)| *english).collect())
}

fn safe_english_clause(prompt_en: Option<&str>, category: MaterialCategory, chinese_fragment: &str) -> Option<String> {
    let prompt_en = prompt_en?;

    prompt_en
        .split(is_list_punctuation)
        .map(str::trim)
        .find(|clause| {
            let tokens = tokenize_english(clause);
            let alias_spans: Vec<_> = safe_chinese_alias_phrases(category, chinese_fragment)
                .into_iter().flat_map(|alias| phrase_spans(&tokens, alias))
                .collect();
            let cue_spans: Vec<_> = category_cues(category)
                .iter()
                .flat_map(|cue| phrase_spans(&tokens, cue))
                .collect();

            alias_spans.iter().any(|alias| cue_spans.iter().any(|cue| alias.1 <= cue.0 || cue.1 <= alias.0))
        })
        .filter(|clause| !clause.is_empty())
        .map(str::to_string)
}

pub fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace() && !is_list_punctuation(*character))
        .flat_map(char::to_lowercase)
        .collect()
}

fn stable_asset_id(category: MaterialCategory, normalized: &str) -> String {
    format!("{}-{:016x}", category_key(category), fnv1a(normalized.as_bytes()))
}

fn stable_source_id(history_id: &str, field_path: &str, normalized: &str) -> String {
    format!("source-{:016x}", fnv1a(format!("{history_id}|{field_path}|{normalized}").as_bytes()))
}

fn category_key(category: MaterialCategory) -> &'static str {
    match category {
        MaterialCategory::Element => "element",
        MaterialCategory::Material => "material",
        MaterialCategory::Color => "color",
        MaterialCategory::Lighting => "lighting",
        MaterialCategory::Camera => "camera",
        MaterialCategory::Composition => "composition",
        MaterialCategory::Style => "style",
        MaterialCategory::Environment => "environment",
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn category_cues(category: MaterialCategory) -> &'static [&'static str] {
    match category {
        MaterialCategory::Element => &["lounge", "seating", "furniture", "arms"],
        MaterialCategory::Material => &["texture", "surface", "upholstery", "grain"],
        MaterialCategory::Color => &["color", "palette", "tone", "hue"],
        MaterialCategory::Lighting => &["window", "illumination", "source", "shadow"],
        MaterialCategory::Camera => &["lens", "camera", "shot", "eye-level"],
        MaterialCategory::Composition => &["composition", "framing", "balance", "focus"],
        MaterialCategory::Style => &["interior", "commercial", "style", "cinematic"],
        MaterialCategory::Environment => &["room", "setting", "background", "foreground"],
    }
}

fn category_aliases(category: MaterialCategory) -> &'static [(&'static str, &'static str)] {
    match category {
        MaterialCategory::Element => &[("\u{4f11}\u{95f2}\u{6905}", "chair"), ("\u{6905}", "chair"), ("\u{6c99}\u{53d1}", "sofa"), ("\u{684c}", "table"), ("\u{706f}", "lamp")],
        MaterialCategory::Material => &[("\u{76ae}\u{9769}", "leather"), ("\u{6728}", "wood"), ("\u{91d1}\u{5c5e}", "metal"), ("\u{7ec7}\u{7269}", "fabric"), ("\u{73bb}\u{7483}", "glass")],
        MaterialCategory::Color => &[("\u{6696}\u{767d}\u{8272}", "warm white"), ("\u{767d}\u{8272}", "white"), ("\u{80e1}\u{6843}\u{6728}\u{68d5}", "walnut brown"), ("\u{68d5}", "brown")],
        MaterialCategory::Lighting => &[("\u{7a97}", "window"), ("\u{5149}", "light"), ("\u{9634}\u{5f71}", "shadow")],
        MaterialCategory::Camera => &[("\u{5e73}\u{89c6}", "eye-level"), ("\u{6807}\u{51c6}\u{955c}\u{5934}", "standard-lens"), ("\u{5e7f}\u{89d2}", "wide-angle"), ("\u{957f}\u{7126}", "telephoto")],
        MaterialCategory::Composition => &[("\u{5c45}\u{4e2d}", "centered"), ("\u{666f}\u{6df1}", "depth"), ("\u{7126}\u{70b9}", "focus")],
        MaterialCategory::Style => &[("\u{6444}\u{5f71}", "photography"), ("\u{6e32}\u{67d3}", "render"), ("\u{63d2}\u{753b}", "illustration"), ("\u{7535}\u{5f71}\u{611f}", "cinematic")],
        MaterialCategory::Environment => &[("\u{5ba4}\u{5185}", "interior"), ("\u{5899}", "wall"), ("\u{5730}\u{9762}", "floor"), ("\u{80cc}\u{666f}", "background")],
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::HistoryItem;
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_index_paths(test_name: &str) -> (PathBuf, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "autoprompt-materials-{test_name}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        (
            directory.join("materials-index.json"),
            directory.join("materials-index.backup.json"),
        )
    }

    fn structured_history() -> HistoryItem {
        HistoryItem {
            id: "history-1".into(), file_name: "chair.jpg".into(), file_path: String::new(), image_url: String::new(), source_type: "file".into(),
            aspect_ratio: Some("4:3".into()), contains_people: Some(false),
            reconstructed_prompt: Some(serde_json::json!({
                "global_scene": { "art_style": "\u{5546}\u{4e1a}\u{5ba4}\u{5185}\u{6444}\u{5f71}", "atmosphere": "\u{5b89}\u{9759}\u{3001}\u{6e29}\u{6696}", "color_palette": ["\u{80e1}\u{6843}\u{6728}\u{68d5}", "\u{6696}\u{767d}\u{8272}"], "lighting": "\u{5de6}\u{4fa7}\u{7a97}\u{6237}\u{6295}\u{4e0b}\u{67d4}\u{548c}\u{6696}\u{5149}" },
                "composition": { "camera_angle": "\u{5e73}\u{89c6}", "focal_length": "\u{6807}\u{51c6}\u{955c}\u{5934}", "framing": "\u{4e3b}\u{4f53}\u{5c45}\u{4e2d}", "depth_of_field": "\u{4e2d}\u{7b49}\u{666f}\u{6df1}" },
                "entities": [{ "label": "\u{4f11}\u{95f2}\u{6905}", "appearance": "\u{7126}\u{7cd6}\u{8272}\u{76ae}\u{9769}\u{ff0c}\u{5f27}\u{5f62}\u{6276}\u{624b}", "sub_elements": ["\u{5706}\u{5f62}\u{5750}\u{57ab}", "\u{4f11}\u{95f2}\u{6905}"] }],
                "environment_details": { "foreground": "\u{6df1}\u{8272}\u{8fb9}\u{67dc}", "midground": "\u{4f11}\u{95f2}\u{6905}", "background": "\u{6728}\u{9970}\u{9762}\u{5899}" },
                "technical_specs": { "texture_fidelity": "\u{7ec6}\u{817b}\u{7684}\u{76ae}\u{9769}\u{7eb9}\u{7406}\u{ff0c}\u{6e05}\u{6670}\u{6728}\u{7eb9}", "render_engine_style": "\u{771f}\u{5b9e}\u{5546}\u{4e1a}\u{6444}\u{5f71}", "vfx": [] }
            })),
            reconstructed_prompt_zh: None, quality_notes: None,
            prompt_en: Some("A caramel leather lounge chair with curved arms, warm window light, eye-level standard-lens interior photography.".into()),
            prompt_zh: Some("\u{7126}\u{7cd6}\u{8272}\u{76ae}\u{9769}\u{4f11}\u{95f2}\u{6905}\u{ff0c}\u{5f27}\u{5f62}\u{6276}\u{624b}\u{ff0c}\u{6696}\u{8272}\u{7a97}\u{5149}\u{ff0c}\u{5e73}\u{89c6}\u{6807}\u{51c6}\u{955c}\u{5934}\u{5ba4}\u{5185}\u{6444}\u{5f71}\u{3002}".into()),
            quality_score: 84, quality_label: "\u{8f83}\u{5f3a}".into(), quality_breakdown: serde_json::json!({}), quality_warnings: vec![],
            model: "gpt-5.4".into(), provider: "openai-compatible".into(), elapsed_ms: 100, favorite: false, created_at: 1000,
        }
    }

    #[test]
    fn extracts_all_supported_categories_with_source_paths() {
        let assets = extract_assets(&structured_history());
        let categories = assets.iter().map(|a| a.category).collect::<std::collections::HashSet<_>>();
        assert_eq!(categories.len(), 8);
        assert!(assets.iter().any(|a| a.category == MaterialCategory::Element && a.generated_name == "\u{4f11}\u{95f2}\u{6905}"));
        assert!(assets.iter().any(|a| a.category == MaterialCategory::Material));
        assert!(assets.iter().all(|a| a.sources.iter().all(|s| s.history_id == "history-1")));
        assert!(assets.iter().any(|a| a.sources.iter().any(|s| s.field_path == "entities[0].label")));
    }

    #[test]
    fn normalization_merges_exact_values_but_not_fuzzy_values() {
        assert_eq!(normalize_key("  \u{6696}\u{767d}\u{8272}\u{ff0c} "), normalize_key("\u{6696}\u{767d}\u{8272}"));
        assert_ne!(normalize_key("\u{6696}\u{767d}\u{8272}"), normalize_key("\u{7c73}\u{767d}\u{8272}"));
    }

    #[test]
    fn full_prompt_only_history_produces_no_assets() {
        let mut item = structured_history();
        item.reconstructed_prompt = None;
        assert!(extract_assets(&item).is_empty());
    }

    #[test]
    fn english_fragment_is_kept_only_for_a_safe_clause_match() {
        let assets = extract_assets(&structured_history());
        let chair = assets.iter().find(|a| a.generated_name == "\u{4f11}\u{95f2}\u{6905}").unwrap();
        assert!(chair.generated_prompt_en.as_deref().unwrap_or("").contains("caramel leather lounge chair"));
        let color = assets.iter().find(|a| a.generated_name == "\u{6696}\u{767d}\u{8272}").unwrap();
        assert_eq!(color.generated_prompt_en, None);
    }

    #[test]
    fn merges_duplicate_elements_and_preserves_all_source_paths() {
        let assets = extract_assets(&structured_history());
        let chairs: Vec<_> = assets
            .iter()
            .filter(|asset| asset.category == MaterialCategory::Element && asset.generated_name == "\u{4f11}\u{95f2}\u{6905}")
            .collect();

        assert_eq!(chairs.len(), 1);
        let paths: std::collections::HashSet<_> = chairs[0]
            .sources
            .iter()
            .map(|source| source.field_path.as_str())
            .collect();
        assert_eq!(paths, std::collections::HashSet::from(["entities[0].label", "entities[0].sub_elements[1]"]));
    }

    #[test]
    fn normalization_removes_defined_internal_punctuation_and_whitespace() {
        assert_eq!(normalize_key(" Warm\u{ff0c} White; "), normalize_key("warm white"));
        assert_eq!(normalize_key("\u{6696} \u{767d}\u{8272}"), normalize_key("\u{6696}\u{ff0c}\u{767d}\u{ff1b}\u{8272}"));
        assert_ne!(normalize_key("warm-white"), normalize_key("warm white"));
    }

    #[test]
    fn english_matching_rejects_substrings_and_overlapping_alias_cues() {
        assert_eq!(safe_english_clause(Some("A chairman portrait"), MaterialCategory::Element, "\u{6905}"), None);
        assert_eq!(safe_english_clause(Some("A chair"), MaterialCategory::Element, "\u{6905}"), None);
    }

    #[test]
    fn chinese_alias_matching_rejects_short_aliases_embedded_in_larger_fragments() {
        assert_eq!(safe_english_clause(Some("A table furniture piece"), MaterialCategory::Element, "\u{684c}\u{9762}"), None);
    }

    #[test]
    fn chinese_alias_matching_accepts_an_exact_short_alias() {
        assert_eq!(
            safe_english_clause(Some("A table furniture piece"), MaterialCategory::Element, "\u{684c}"),
            Some("A table furniture piece".to_string())
        );
    }

    #[test]
    fn chinese_alias_matching_rejects_ambiguous_longest_matches() {
        assert_eq!(safe_english_clause(Some("Leather and metal texture"), MaterialCategory::Material, "\u{76ae}\u{9769}\u{91d1}\u{5c5e}"), None);
    }

    #[test]
    fn rebuild_preserves_manual_overrides_and_favorites() {
        let (primary, backup) = test_index_paths("rebuild-preserves-overrides");
        let original = structured_history();
        let mut index = rebuild_index_from(&[original.clone()], &primary, &backup).unwrap();
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        let chair = index.assets.iter_mut().find(|asset| asset.id == chair_id).unwrap();
        chair.user_override.display_name = Some("\u{6211}\u{7684}\u{4f11}\u{95f2}\u{6905}".into());
        chair.user_override.prompt_zh = Some("\u{4fdd}\u{7559}\u{8fd9}\u{6bb5}\u{624b}\u{52a8}\u{63d0}\u{793a}\u{8bcd}".into());
        chair.user_override.favorite = true;
        chair.user_override.manually_edited = true;
        save_index_to(&index, &primary, &backup).unwrap();

        let mut changed = original;
        changed.created_at = 2000;
        changed.reconstructed_prompt.as_mut().unwrap()["entities"][0]["label"] =
            serde_json::Value::String("\u{4f11} \u{95f2}\u{6905}".into());

        let rebuilt = rebuild_index_from(&[changed], &primary, &backup).unwrap();
        let chair = rebuilt.assets.iter().find(|asset| asset.id == chair_id).unwrap();
        assert_eq!(chair.generated_name, "\u{4f11} \u{95f2}\u{6905}");
        assert_eq!(chair.user_override.display_name.as_deref(), Some("\u{6211}\u{7684}\u{4f11}\u{95f2}\u{6905}"));
        assert_eq!(chair.user_override.prompt_zh.as_deref(), Some("\u{4fdd}\u{7559}\u{8fd9}\u{6bb5}\u{624b}\u{52a8}\u{63d0}\u{793a}\u{8bcd}"));
        assert!(chair.user_override.favorite);
        assert_eq!(load_index_from(&primary, &backup).unwrap(), rebuilt);
    }
    #[test]
    fn exact_assets_merge_sources_across_history_items() {
        let (primary, backup) = test_index_paths("exact-assets-merge-sources");
        let first = structured_history();
        let mut second = structured_history();
        second.id = "history-2".into();
        second.created_at = 2000;

        let index = rebuild_index_from(&[first, second], &primary, &backup).unwrap();
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        let chairs: Vec<_> = index.assets.iter().filter(|asset| asset.id == chair_id).collect();

        assert_eq!(chairs.len(), 1);
        assert_eq!(chairs[0].sources.len(), 4);
        assert_eq!(chairs[0].sources[0].history_id, "history-2");
        assert!(chairs[0].sources.windows(2).all(|pair| {
            pair[0].created_at > pair[1].created_at
                || (pair[0].created_at == pair[1].created_at && pair[0].id <= pair[1].id)
        }));
    }
    #[test]
    fn deleting_one_history_source_keeps_shared_asset() {
        let (primary, backup) = test_index_paths("delete-one-shared-source");
        let first = structured_history();
        let mut second = structured_history();
        second.id = "history-2".into();
        second.created_at = 2000;
        let mut index = rebuild_index_from(&[first, second], &primary, &backup).unwrap();

        remove_history_sources(&mut index, &["history-2".to_string()]);
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        let chair = index.assets.iter().find(|asset| asset.id == chair_id).unwrap();

        assert_eq!(chair.sources.len(), 2);
        assert!(chair.sources.iter().all(|source| source.history_id == "history-1"));
    }
    #[test]
    fn orphaned_manual_asset_survives_source_deletion() {
        let (primary, backup) = test_index_paths("orphaned-manual-asset");
        let mut index = rebuild_index_from(&[structured_history()], &primary, &backup).unwrap();
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));

        apply_patch(
            &mut index,
            &chair_id,
            MaterialPatch {
                display_name: Some("\u{4fdd}\u{7559}\u{7684}\u{6905}\u{5b50}".into()),
                favorite: Some(true),
                ..MaterialPatch::default()
            },
        )
        .unwrap();
        remove_history_sources(&mut index, &["history-1".to_string()]);

        let chair = index.assets.iter().find(|asset| asset.id == chair_id).unwrap();
        assert!(chair.sources.is_empty());
        assert_eq!(chair.user_override.display_name.as_deref(), Some("\u{4fdd}\u{7559}\u{7684}\u{6905}\u{5b50}"));
        assert!(chair.user_override.favorite);
        assert!(chair.user_override.manually_edited);
    }
    #[test]
    fn split_moves_selected_variants_to_a_user_asset() {
        let (primary, backup) = test_index_paths("split-selected-variants");
        let mut index = rebuild_index_from(&[structured_history()], &primary, &backup).unwrap();
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        let source_ids: Vec<_> = index
            .assets
            .iter()
            .find(|asset| asset.id == chair_id)
            .unwrap()
            .sources
            .iter()
            .map(|source| source.id.clone())
            .collect();

        assert!(split_asset(&mut index, &chair_id, &[], "\u{65e0}\u{6548}".into()).is_err());
        assert!(split_asset(&mut index, &chair_id, &["missing".into()], "\u{65e0}\u{6548}".into()).is_err());
        assert!(split_asset(&mut index, &chair_id, &source_ids, "\u{65e0}\u{6548}".into()).is_err());

        let changed = split_asset(
            &mut index,
            &chair_id,
            &[source_ids[0].clone()],
            "\u{6905}\u{80cc}\u{53d8}\u{4f53}".into(),
        )
        .unwrap();
        let original = changed.iter().find(|asset| asset.id == chair_id).unwrap();
        let split = changed.iter().find(|asset| asset.id != chair_id).unwrap();

        assert_eq!(original.sources.len(), 1);
        assert_eq!(split.sources.len(), 1);
        assert_eq!(split.sources[0].id, source_ids[0]);
        assert_eq!(split.user_override.display_name.as_deref(), Some("\u{6905}\u{80cc}\u{53d8}\u{4f53}"));
        assert_eq!(split.user_override.split_from.as_deref(), Some(chair_id.as_str()));
        assert!(split.user_override.manually_edited);
    }

    #[test]
    fn merge_assets_rejects_fewer_than_two_ids() {
        let mut index = build_index(&[structured_history()], None);
        let only_id = index.assets[0].id.clone();
        let before = index.clone();

        assert!(merge_assets(&mut index, &[only_id], None).is_err());
        assert_eq!(index, before);
    }

    #[test]
    fn merge_assets_combines_same_category_sources() {
        let mut index = build_index(&[structured_history()], None);
        let chair_id =
            stable_asset_id(MaterialCategory::Element, &normalize_key("休闲椅"));
        let cushion_id =
            stable_asset_id(MaterialCategory::Element, &normalize_key("圆形坐垫"));

        let merged = merge_assets(
            &mut index,
            &[chair_id.clone(), cushion_id.clone()],
            Some("Seating".into()),
        )
        .unwrap();

        assert_eq!(merged.id, chair_id);
        assert_eq!(merged.sources.len(), 3);
        assert_eq!(merged.user_override.display_name.as_deref(), Some("Seating"));
        assert!(merged.user_override.manually_edited);
        assert!(merged.sources.windows(2).all(|pair| {
            pair[0].created_at > pair[1].created_at
                || (pair[0].created_at == pair[1].created_at && pair[0].id <= pair[1].id)
        }));
        let cushion = index.assets.iter().find(|asset| asset.id == cushion_id).unwrap();
        assert_eq!(cushion.user_override.merged_into.as_deref(), Some(chair_id.as_str()));
    }
    #[test]
    fn merge_assets_rejects_mixed_categories_without_mutation() {
        let mut index = build_index(&[structured_history()], None);
        let element_id = index
            .assets
            .iter()
            .find(|asset| asset.category == MaterialCategory::Element)
            .unwrap()
            .id
            .clone();
        let material_id = index
            .assets
            .iter()
            .find(|asset| asset.category == MaterialCategory::Material)
            .unwrap()
            .id
            .clone();
        let before = index.clone();

        assert!(merge_assets(&mut index, &[element_id, material_id], None).is_err());
        assert_eq!(index, before);
    }
    #[test]
    fn merge_assets_rejects_blank_display_name_without_mutation() {
        let mut index = build_index(&[structured_history()], None);
        let ids: Vec<_> = index
            .assets
            .iter()
            .filter(|asset| asset.category == MaterialCategory::Element)
            .take(2)
            .map(|asset| asset.id.clone())
            .collect();
        let before = index.clone();

        assert!(merge_assets(&mut index, &ids, Some("   ".into())).is_err());
        assert_eq!(index, before);
    }
    #[test]
    fn invalid_primary_index_falls_back_to_last_valid_backup() {
        let (primary, backup) = test_index_paths("invalid-primary-fallback");
        let first = build_index(&[structured_history()], None);
        save_index_to(&first, &primary, &backup).unwrap();

        let mut second_history = structured_history();
        second_history.id = "history-2".into();
        let second = build_index(&[second_history], Some(&first));
        save_index_to(&second, &primary, &backup).unwrap();
        std::fs::write(&primary, b"not valid json").unwrap();

        let recovered = load_index_from(&primary, &backup).unwrap();

        assert_eq!(recovered.history_fingerprint, first.history_fingerprint);
        assert_eq!(recovered.assets, first.assets);
        assert_eq!(recovered.warnings.len(), first.warnings.len() + 1);
        assert!(recovered.warnings.last().unwrap().message.contains("backup"));
    }
    #[test]
    fn ensure_index_creates_and_persists_a_missing_index() {
        let (primary, backup) = test_index_paths("ensure-creates-index");

        let ensured =
            ensure_index_from(&[structured_history()], &primary, &backup).unwrap();

        assert!(primary.exists());
        assert_eq!(load_index_from(&primary, &backup).unwrap(), ensured);
    }
    #[test]
    fn failed_save_preserves_last_valid_primary_and_backup() {
        let (primary, backup) = test_index_paths("failed-save-preserves-pair");
        let first = build_index(&[structured_history()], None);
        save_index_to(&first, &primary, &backup).unwrap();

        let mut second_history = structured_history();
        second_history.id = "history-2".into();
        let second = build_index(&[second_history], Some(&first));
        save_index_to(&second, &primary, &backup).unwrap();
        let primary_before = std::fs::read(&primary).unwrap();
        let backup_before = std::fs::read(&backup).unwrap();

        let mut third_history = structured_history();
        third_history.id = "history-3".into();
        let third = build_index(&[third_history], Some(&second));
        let mut permissions = std::fs::metadata(&primary).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&primary, permissions).unwrap();

        assert!(save_index_to(&third, &primary, &backup).is_err());

        let mut permissions = std::fs::metadata(&primary).unwrap().permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(&primary, permissions).unwrap();
        assert_eq!(std::fs::read(&primary).unwrap(), primary_before);
        assert_eq!(std::fs::read(&backup).unwrap(), backup_before);
    }

    #[test]
    fn rebuild_preserves_merge_relationships_and_source_ownership() {
        let history = structured_history();
        let mut index = build_index(&[history.clone()], None);
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        let cushion_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{5706}\u{5f62}\u{5750}\u{57ab}"));
        merge_assets(
            &mut index,
            &[chair_id.clone(), cushion_id.clone()],
            Some("Seating".into()),
        )
        .unwrap();

        let rebuilt = build_index(&[history], Some(&index));
        let target = rebuilt.assets.iter().find(|asset| asset.id == chair_id).unwrap();
        let child = rebuilt.assets.iter().find(|asset| asset.id == cushion_id).unwrap();
        assert_eq!(target.sources.len(), 3);
        assert!(child.sources.is_empty());
        assert_eq!(child.user_override.merged_into.as_deref(), Some(target.id.as_str()));
    }

    #[test]
    fn rebuild_preserves_split_source_ownership_without_duplicates() {
        let history = structured_history();
        let mut index = build_index(&[history.clone()], None);
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        let moved_source_id = index.assets.iter().find(|asset| asset.id == chair_id).unwrap().sources[0].id.clone();
        let changed = split_asset(
            &mut index,
            &chair_id,
            &[moved_source_id.clone()],
            "Chair back".into(),
        )
        .unwrap();
        let split_id = changed.iter().find(|asset| asset.id != chair_id).unwrap().id.clone();

        let rebuilt = build_index(&[history], Some(&index));
        let original = rebuilt.assets.iter().find(|asset| asset.id == chair_id).unwrap();
        let split = rebuilt.assets.iter().find(|asset| asset.id == split_id).unwrap();
        let occurrences = rebuilt.assets.iter().flat_map(|asset| asset.sources.iter()).filter(|source| source.id == moved_source_id).count();
        assert_eq!(original.sources.len(), 1);
        assert_eq!(split.sources.len(), 1);
        assert_eq!(split.sources[0].id, moved_source_id);
        assert_eq!(occurrences, 1);
    }

    #[test]
    fn patched_index_can_be_saved_and_loaded_durably() {
        let (primary, backup) = test_index_paths("durable-patch");
        let mut index = build_index(&[structured_history()], None);
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        apply_patch(
            &mut index,
            &chair_id,
            MaterialPatch {
                display_name: Some("Edited chair".into()),
                favorite: Some(true),
                ..MaterialPatch::default()
            },
        )
        .unwrap();
        save_index_to(&index, &primary, &backup).unwrap();

        let loaded = load_index_from(&primary, &backup).unwrap();
        let chair = loaded.assets.iter().find(|asset| asset.id == chair_id).unwrap();
        assert_eq!(chair.user_override.display_name.as_deref(), Some("Edited chair"));
        assert!(chair.user_override.favorite);
    }

    #[test]
    fn incremental_upsert_and_clear_preserve_user_managed_assets() {
        let mut first = structured_history();
        first.id = "history-a".into();
        let mut second = structured_history();
        second.id = "history-b".into();
        second.created_at = 2000;
        let mut index = build_index(&[first], None);
        let chair_id = stable_asset_id(MaterialCategory::Element, &normalize_key("\u{4f11}\u{95f2}\u{6905}"));
        apply_patch(
            &mut index,
            &chair_id,
            MaterialPatch { favorite: Some(true), ..MaterialPatch::default() },
        )
        .unwrap();

        upsert_history_source(&mut index, &second);
        let chair = index.assets.iter().find(|asset| asset.id == chair_id).unwrap();
        assert_eq!(chair.sources.len(), 4);
        assert!(chair.user_override.favorite);
        clear_generated_sources(&mut index);
        assert!(index.assets.iter().all(|asset| asset.sources.is_empty()));
        assert_eq!(index.assets.len(), 1);
        assert_eq!(index.assets[0].id, chair_id);
    }

    #[test]
    fn equal_timestamps_produce_deterministic_generated_fields() {
        let mut first = structured_history();
        first.id = "history-a".into();
        let mut second = structured_history();
        second.id = "history-b".into();
        second.created_at = first.created_at;
        second.reconstructed_prompt.as_mut().unwrap()["global_scene"]["atmosphere"] =
            serde_json::Value::String("\u{5b89}\u{9759} \u{6e29}\u{6696}".into());

        let forward = build_index(&[first.clone(), second.clone()], None);
        let reverse = build_index(&[second, first], None);
        assert_eq!(forward.history_fingerprint, reverse.history_fingerprint);
        assert_eq!(forward.assets, reverse.assets);
    }

    #[test]
    fn successful_atomic_save_leaves_no_temporary_files() {
        let (primary, backup) = test_index_paths("atomic-cleanup");
        let index = build_index(&[structured_history()], None);
        save_index_to(&index, &primary, &backup).unwrap();
        let names: Vec<_> = std::fs::read_dir(primary.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|name| !name.contains(".tmp.")));
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialIndex {
    pub schema_version: u32,
    pub history_fingerprint: String,
    pub assets: Vec<MaterialAsset>,
    pub warnings: Vec<MaterialIndexWarning>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialIndexWarning {
    pub history_id: String,
    pub message: String,
}

const MATERIAL_INDEX_SCHEMA_VERSION: u32 = 1;

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn history_fingerprint(history: &[HistoryItem]) -> String {
    let mut entries: Vec<_> = history
        .iter()
        .map(|item| {
            (
                item.id.as_str(),
                item.created_at,
                serde_json::to_string(&item.reconstructed_prompt).unwrap_or_else(|_| "null".into()),
            )
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(right.0).then(left.1.cmp(&right.1)).then(left.2.cmp(&right.2)));

    let mut bytes = Vec::new();
    for (id, created_at, prompt) in entries {
        bytes.extend_from_slice(id.len().to_string().as_bytes());
        bytes.push(b':');
        bytes.extend_from_slice(id.as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(created_at.to_string().as_bytes());
        bytes.push(b'|');
        bytes.extend_from_slice(prompt.len().to_string().as_bytes());
        bytes.push(b':');
        bytes.extend_from_slice(prompt.as_bytes());
        bytes.push(b'\n');
    }
    format!("{:016x}", fnv1a(&bytes))
}

fn invalid_data(message: impl Into<String>) -> AnyError {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into()).into()
}

fn parse_index(path: &Path) -> Result<MaterialIndex, AnyError> {
    let data = fs::read_to_string(path)?;
    let index: MaterialIndex = serde_json::from_str(&data)?;
    if index.schema_version != MATERIAL_INDEX_SCHEMA_VERSION {
        return Err(invalid_data(format!(
            "unsupported materials index schema version {}",
            index.schema_version
        )));
    }
    Ok(index)
}

fn load_index_from(primary: &Path, backup: &Path) -> Result<MaterialIndex, AnyError> {
    match parse_index(primary) {
        Ok(index) => Ok(index),
        Err(primary_error) => {
            let mut index = parse_index(backup).map_err(|backup_error| {
                invalid_data(format!(
                    "failed to load materials index ({primary_error}); backup also failed ({backup_error})"
                ))
            })?;
            index.warnings.push(MaterialIndexWarning {
                history_id: String::new(),
                message: format!("Loaded the last valid materials index backup: {primary_error}"),
            });
            Ok(index)
        }
    }
}

fn temporary_path(path: &Path, label: &str) -> PathBuf {
    let name = path.file_name().and_then(|value| value.to_str()).unwrap_or("materials-index.json");
    path.with_file_name(format!("{name}.{label}.{}.{}", std::process::id(), current_timestamp()))
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), AnyError> {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let destination_wide: Vec<u16> = destination.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let result = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> Result<(), AnyError> {
    fs::rename(source, destination)?;
    Ok(())
}

fn write_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<(), AnyError> {
    let temporary = temporary_path(path, "restore");
    fs::write(&temporary, bytes)?;
    let result = atomic_replace(&temporary, path);
    let _ = fs::remove_file(&temporary);
    result
}

fn write_validated(index: &MaterialIndex, path: &Path) -> Result<(), AnyError> {
    fs::write(path, serde_json::to_vec_pretty(index)?)?;
    parse_index(path)?;
    Ok(())
}

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>, AnyError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn restore_file(path: &Path, original: Option<&[u8]>) -> Result<(), AnyError> {
    match original {
        Some(bytes) => {
            if fs::read(path).ok().as_deref() != Some(bytes) {
                write_bytes_atomically(path, bytes)?;
            }
        }
        None => match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        },
    }
    Ok(())
}

fn save_index_to(index: &MaterialIndex, primary: &Path, backup: &Path) -> Result<(), AnyError> {
    if let Some(parent) = primary.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }

    let primary_before = read_optional_file(primary)?;
    let backup_before = read_optional_file(backup)?;
    let previous = parse_index(primary).ok();
    let new_index_temp = temporary_path(primary, "tmp");
    if let Err(error) = write_validated(index, &new_index_temp) {
        let _ = fs::remove_file(&new_index_temp);
        return Err(error);
    }

    let backup_temp = previous
        .as_ref()
        .map(|_| temporary_path(backup, "tmp"));
    let result = (|| -> Result<(), AnyError> {
        if let (Some(previous), Some(backup_temp)) = (previous.as_ref(), backup_temp.as_ref()) {
            write_validated(previous, backup_temp)?;
            atomic_replace(backup_temp, backup)?;
            parse_index(backup)?;
        }

        atomic_replace(&new_index_temp, primary)?;
        parse_index(primary)?;
        Ok(())
    })();

    let _ = fs::remove_file(&new_index_temp);
    if let Some(path) = backup_temp.as_ref() {
        let _ = fs::remove_file(path);
    }

    if let Err(error) = result {
        let primary_restore = restore_file(primary, primary_before.as_deref());
        let backup_restore = restore_file(backup, backup_before.as_deref());
        if let Err(restore_error) = primary_restore.and(backup_restore) {
            return Err(invalid_data(format!(
                "failed to save materials index ({error}); rollback also failed ({restore_error})"
            )));
        }
        return Err(error);
    }

    Ok(())
}

pub fn save_index(index: &MaterialIndex) -> Result<(), AnyError> {
    save_index_to(index, &materials_index_path(), &materials_index_backup_path())
}
fn is_user_managed(asset: &MaterialAsset) -> bool {
    let user = &asset.user_override;
    user.favorite
        || user.manually_edited
        || user.merged_into.is_some()
        || user.split_from.is_some()
        || !user.split_source_ids.is_empty()
        || user.display_name.is_some()
        || user.prompt_zh.is_some()
        || user.prompt_en.is_some()
        || !user.aliases.is_empty()
}




pub fn apply_patch(index: &mut MaterialIndex, id: &str, patch: MaterialPatch) -> Result<MaterialAsset, AnyError> {
    for (label, value) in [
        ("display name", patch.display_name.as_deref()),
        ("Chinese prompt fragment", patch.prompt_zh.as_deref()),
        ("English prompt fragment", patch.prompt_en.as_deref()),
    ] {
        if value.is_some_and(|value| value.trim().is_empty()) {
            return Err(invalid_data(format!("{label} cannot be blank")));
        }
    }

    let asset = index
        .assets
        .iter_mut()
        .find(|asset| asset.id == id)
        .ok_or_else(|| invalid_data(format!("unknown material asset: {id}")))?;
    let has_manual_edit = patch.display_name.is_some()
        || patch.prompt_zh.is_some()
        || patch.prompt_en.is_some()
        || patch.aliases.is_some();

    if let Some(value) = patch.display_name {
        asset.user_override.display_name = Some(value.trim().to_string());
    }
    if let Some(value) = patch.prompt_zh {
        asset.user_override.prompt_zh = Some(value.trim().to_string());
    }
    if let Some(value) = patch.prompt_en {
        asset.user_override.prompt_en = Some(value.trim().to_string());
    }
    if let Some(values) = patch.aliases {
        asset.user_override.aliases = values
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
    }
    if let Some(value) = patch.favorite {
        asset.user_override.favorite = value;
    }
    asset.user_override.manually_edited |= has_manual_edit;
    asset.updated_at = current_timestamp();
    index.updated_at = asset.updated_at;
    Ok(asset.clone())
}

pub fn merge_assets(
    index: &mut MaterialIndex,
    ids: &[String],
    display_name: Option<String>,
) -> Result<MaterialAsset, AnyError> {
    if display_name
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(invalid_data("merge display name cannot be blank"));
    }
    let mut unique_ids = Vec::new();
    for id in ids {
        if !unique_ids.iter().any(|candidate| candidate == id) {
            unique_ids.push(id.clone());
        }
    }
    if unique_ids.len() < 2 {
        return Err(invalid_data("merge requires at least two material assets"));
    }

    let selected: Vec<_> = unique_ids
        .iter()
        .map(|id| {
            index
                .assets
                .iter()
                .find(|asset| &asset.id == id)
                .cloned()
                .ok_or_else(|| invalid_data(format!("unknown material asset: {id}")))
        })
        .collect::<Result<_, _>>()?;
    if selected
        .iter()
        .any(|asset| asset.category != selected[0].category)
    {
        return Err(invalid_data("merged material assets must share a category"));
    }
    let target_id = unique_ids[0].clone();
    let now = current_timestamp();
    let mut merged = selected[0].clone();

    for asset in selected.iter().skip(1) {
        for source in &asset.sources {
            if !merged.sources.iter().any(|current| current.id == source.id) {
                merged.sources.push(source.clone());
            }
        }
    }
    sort_sources(&mut merged.sources);
    if let Some(value) = display_name {
        merged.user_override.display_name = Some(value.trim().to_string());
    }
    merged.user_override.manually_edited = true;
    merged.user_override.merged_into = None;
    merged.updated_at = now;

    for asset in &mut index.assets {
        if asset.id == target_id {
            *asset = merged.clone();
        } else if unique_ids.iter().any(|id| id == &asset.id) {
            asset.user_override.merged_into = Some(target_id.clone());
            asset.user_override.manually_edited = true;
            asset.sources.clear();
            asset.updated_at = now;
        }
    }
    index.assets.sort_by(|left, right| left.id.cmp(&right.id));
    index.updated_at = now;
    Ok(merged)
}
pub fn split_asset(
    index: &mut MaterialIndex,
    id: &str,
    source_ids: &[String],
    display_name: String,
) -> Result<Vec<MaterialAsset>, AnyError> {
    if display_name.trim().is_empty() {
        return Err(invalid_data("split display name cannot be blank"));
    }
    if source_ids.is_empty() {
        return Err(invalid_data("split source set cannot be empty"));
    }

    let position = index
        .assets
        .iter()
        .position(|asset| asset.id == id)
        .ok_or_else(|| invalid_data(format!("unknown material asset: {id}")))?;
    let template = index.assets[position].clone();
    if source_ids
        .iter()
        .any(|source_id| !template.sources.iter().any(|source| &source.id == source_id))
    {
        return Err(invalid_data("split source set contains an unknown source"));
    }

    let mut selected: Vec<_> = template
        .sources
        .iter()
        .filter(|source| source_ids.iter().any(|source_id| source_id == &source.id))
        .cloned()
        .collect();
    if selected.len() == template.sources.len() {
        return Err(invalid_data("split source set cannot include every source"));
    }
    sort_sources(&mut selected);

    let mut identity = source_ids.to_vec();
    identity.sort();
    identity.dedup();
    let new_id = format!(
        "user-{}-{:016x}",
        category_key(template.category),
        fnv1a(format!("{}|{}|{}", id, identity.join("|"), display_name.trim()).as_bytes())
    );
    if index.assets.iter().any(|asset| asset.id == new_id) {
        return Err(invalid_data("split asset already exists"));
    }

    index.assets[position]
        .sources
        .retain(|source| !source_ids.iter().any(|source_id| source_id == &source.id));
    sort_sources(&mut index.assets[position].sources);
    index.assets[position].updated_at = current_timestamp();

    let now = current_timestamp();
    let mut split = MaterialAsset {
        id: new_id,
        category: template.category,
        generated_name: template.generated_name,
        generated_explanation: template.generated_explanation,
        generated_prompt_zh: template.generated_prompt_zh,
        generated_prompt_en: template.generated_prompt_en,
        generated_aliases: template.generated_aliases,
        user_override: MaterialOverride {
            display_name: Some(display_name.trim().to_string()),
            manually_edited: true,
            split_from: Some(id.to_string()),
            split_source_ids: identity,
            ..MaterialOverride::default()
        },
        created_at: selected.iter().map(|source| source.created_at).min().unwrap_or(now),
        updated_at: now,
        sources: selected,
    };
    sort_sources(&mut split.sources);

    let original = index.assets[position].clone();
    index.assets.push(split.clone());
    index.assets.sort_by(|left, right| left.id.cmp(&right.id));
    index.updated_at = now;
    Ok(vec![original, split])
}

pub fn upsert_history_source(index: &mut MaterialIndex, item: &HistoryItem) {
    let previous = index.clone();
    remove_history_sources(index, &[item.id.clone()]);
    for candidate in extract_assets(item) {
        merge_candidate(&mut index.assets, candidate);
    }
    reapply_user_relationships(&mut index.assets, &previous);
    index.assets.retain(|asset| !asset.sources.is_empty() || is_user_managed(asset));
    index.assets.sort_by(|left, right| left.id.cmp(&right.id));
    index.history_fingerprint.clear();
    index.updated_at = current_timestamp();
}

pub fn remove_history_sources(index: &mut MaterialIndex, ids: &[String]) {
    for asset in &mut index.assets {
        asset
            .sources
            .retain(|source| !ids.iter().any(|id| id == &source.history_id));
        sort_sources(&mut asset.sources);
        asset.updated_at = current_timestamp();
    }
    index
        .assets
        .retain(|asset| !asset.sources.is_empty() || is_user_managed(asset));
    index.history_fingerprint.clear();
    index.updated_at = current_timestamp();
}

pub fn clear_generated_sources(index: &mut MaterialIndex) {
    for asset in &mut index.assets {
        asset.sources.clear();
    }
    index.assets.retain(is_user_managed);
    index.history_fingerprint = history_fingerprint(&[]);
    index.updated_at = current_timestamp();
}
fn sort_sources(sources: &mut Vec<MaterialSourceVariant>) {
    sources.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then(left.id.cmp(&right.id))
    });
}

fn generated_priority(asset: &MaterialAsset) -> (u64, &str) {
    (
        asset.updated_at,
        asset.sources.first().map(|source| source.id.as_str()).unwrap_or(""),
    )
}

fn merge_candidate(assets: &mut Vec<MaterialAsset>, candidate: MaterialAsset) {
    if let Some(existing) = assets.iter_mut().find(|asset| asset.id == candidate.id) {
        let candidate_priority = generated_priority(&candidate);
        let existing_priority = generated_priority(existing);
        if candidate_priority.0 > existing_priority.0
            || (candidate_priority.0 == existing_priority.0
                && candidate_priority.1 < existing_priority.1)
        {
            existing.generated_name = candidate.generated_name.clone();
            existing.generated_explanation = candidate.generated_explanation.clone();
            existing.generated_prompt_zh = candidate.generated_prompt_zh.clone();
            existing.generated_prompt_en = candidate.generated_prompt_en.clone();
            existing.generated_aliases = candidate.generated_aliases.clone();
        }
        for source in candidate.sources {
            if !existing.sources.iter().any(|current| current.id == source.id) {
                existing.sources.push(source);
            }
        }
        existing.created_at = existing.created_at.min(candidate.created_at);
        existing.updated_at = existing.updated_at.max(candidate.updated_at);
        sort_sources(&mut existing.sources);
    } else {
        assets.push(candidate);
    }
}

fn merge_extracted_assets(history: &[HistoryItem]) -> Vec<MaterialAsset> {
    let mut candidates: Vec<_> = history.iter().flat_map(extract_assets).collect();
    candidates.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then(right.updated_at.cmp(&left.updated_at))
            .then_with(|| generated_priority(left).1.cmp(generated_priority(right).1))
    });
    let mut assets = Vec::new();
    for candidate in candidates {
        merge_candidate(&mut assets, candidate);
    }
    for asset in &mut assets {
        sort_sources(&mut asset.sources);
    }
    assets
}

fn take_sources(assets: &mut [MaterialAsset], source_ids: &[String]) -> Vec<MaterialSourceVariant> {
    let mut moved = Vec::new();
    for asset in assets {
        let mut retained = Vec::new();
        for source in std::mem::take(&mut asset.sources) {
            if source_ids.iter().any(|id| id == &source.id) {
                if !moved.iter().any(|current: &MaterialSourceVariant| current.id == source.id) {
                    moved.push(source);
                }
            } else {
                retained.push(source);
            }
        }
        asset.sources = retained;
    }
    sort_sources(&mut moved);
    moved
}

fn reapply_user_relationships(assets: &mut Vec<MaterialAsset>, previous: &MaterialIndex) {
    for old in &previous.assets {
        if let Some(current) = assets.iter_mut().find(|asset| asset.id == old.id) {
            current.user_override = old.user_override.clone();
            current.created_at = current.created_at.min(old.created_at);
        } else if is_user_managed(old) {
            let mut retained = old.clone();
            retained.sources.clear();
            assets.push(retained);
        }
    }

    let merges: Vec<_> = previous
        .assets
        .iter()
        .filter_map(|asset| {
            asset.user_override.merged_into.as_ref().map(|target| (asset.id.clone(), target.clone()))
        })
        .collect();
    for (child_id, target_id) in merges {
        let moved = if let Some(child) = assets.iter_mut().find(|asset| asset.id == child_id) {
            std::mem::take(&mut child.sources)
        } else {
            Vec::new()
        };
        if let Some(target) = assets.iter_mut().find(|asset| asset.id == target_id) {
            for source in moved {
                if !target.sources.iter().any(|current| current.id == source.id) {
                    target.sources.push(source);
                }
            }
            sort_sources(&mut target.sources);
        }
    }

    let splits: Vec<_> = previous
        .assets
        .iter()
        .filter(|asset| asset.user_override.split_from.is_some())
        .map(|asset| {
            let ids = if asset.user_override.split_source_ids.is_empty() {
                asset.sources.iter().map(|source| source.id.clone()).collect()
            } else {
                asset.user_override.split_source_ids.clone()
            };
            (asset.id.clone(), ids)
        })
        .collect();
    for (split_id, source_ids) in splits {
        let moved = take_sources(assets, &source_ids);
        if let Some(split) = assets.iter_mut().find(|asset| asset.id == split_id) {
            split.sources = moved;
            sort_sources(&mut split.sources);
        }
    }
}

fn build_index(history: &[HistoryItem], previous: Option<&MaterialIndex>) -> MaterialIndex {
    let mut assets = merge_extracted_assets(history);
    if let Some(previous) = previous {
        reapply_user_relationships(&mut assets, previous);
    }
    assets.retain(|asset| !asset.sources.is_empty() || is_user_managed(asset));
    assets.sort_by(|left, right| left.id.cmp(&right.id));

    MaterialIndex {
        schema_version: MATERIAL_INDEX_SCHEMA_VERSION,
        history_fingerprint: history_fingerprint(history),
        assets,
        warnings: Vec::new(),
        updated_at: current_timestamp(),
    }
}

fn rebuild_index_from(history: &[HistoryItem], primary: &Path, backup: &Path) -> Result<MaterialIndex, AnyError> {
    let previous = load_index_from(primary, backup).ok();
    let index = build_index(history, previous.as_ref());
    save_index_to(&index, primary, backup)?;
    Ok(index)
}

fn ensure_index_from(
    history: &[HistoryItem],
    primary: &Path,
    backup: &Path,
) -> Result<MaterialIndex, AnyError> {
    let expected_fingerprint = history_fingerprint(history);
    match parse_index(primary) {
        Ok(index) if index.history_fingerprint == expected_fingerprint => Ok(index),
        _ => rebuild_index_from(history, primary, backup),
    }
}
pub fn load_index() -> Result<MaterialIndex, AnyError> {
    load_index_from(&materials_index_path(), &materials_index_backup_path())
}

pub fn ensure_index(history: &[HistoryItem]) -> Result<MaterialIndex, AnyError> {
    ensure_index_from(
        history,
        &materials_index_path(),
        &materials_index_backup_path(),
    )
}
pub fn rebuild_index(history: &[HistoryItem]) -> Result<MaterialIndex, AnyError> {
    rebuild_index_from(history, &materials_index_path(), &materials_index_backup_path())
}
