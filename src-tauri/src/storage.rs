use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("autoprompt-data")
}

fn settings_path() -> PathBuf { data_dir().join("settings.json") }
fn history_path() -> PathBuf { data_dir().join("history.json") }
pub fn thumbs_dir() -> PathBuf { data_dir().join("thumbnails") }

pub fn ensure_data_dir() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(data_dir())?;
    fs::create_dir_all(thumbs_dir())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(rename = "providerType", default = "default_provider")]
    pub provider_type: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
    #[serde(rename = "baseUrl", default)]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(rename = "timeoutMs", default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(rename = "defaultLanguage", default = "default_lang")]
    pub default_language: String,
    #[serde(rename = "themeMode", default = "default_theme")]
    pub theme_mode: String,
    #[serde(default = "default_concurrency")]
    pub concurrency: u32,
}

fn default_provider() -> String { "gemini-native".into() }
fn default_model() -> String { "gemini-2.5-flash".into() }
fn default_timeout() -> u64 { 45000 }
fn default_lang() -> String { "zh".into() }
fn default_theme() -> String { "dark".into() }
fn default_concurrency() -> u32 { 2 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            provider_type: default_provider(),
            api_key: String::new(),
            base_url: String::new(),
            model: default_model(),
            timeout_ms: default_timeout(),
            default_language: default_lang(),
            theme_mode: default_theme(),
            concurrency: default_concurrency(),
        }
    }
}

pub fn get_settings() -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let path = settings_path();
    if path.exists() {
        let data = fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data).unwrap_or_default())
    } else {
        Ok(Settings::default())
    }
}

pub fn save_settings(data: serde_json::Value) -> Result<Settings, Box<dyn std::error::Error + Send + Sync>> {
    let mut current = get_settings()?;
    if let Some(obj) = data.as_object() {
        if let Some(v) = obj.get("providerType").and_then(|v| v.as_str()) { current.provider_type = v.to_string(); }
        if let Some(v) = obj.get("apiKey").and_then(|v| v.as_str()) { current.api_key = v.to_string(); }
        if let Some(v) = obj.get("baseUrl").and_then(|v| v.as_str()) { current.base_url = v.to_string(); }
        if let Some(v) = obj.get("model").and_then(|v| v.as_str()) { current.model = v.to_string(); }
        if let Some(v) = obj.get("timeoutMs").and_then(|v| v.as_u64()) { current.timeout_ms = v; }
        if let Some(v) = obj.get("defaultLanguage").and_then(|v| v.as_str()) { current.default_language = v.to_string(); }
        if let Some(v) = obj.get("themeMode").and_then(|v| v.as_str()) { current.theme_mode = v.to_string(); }
        if let Some(v) = obj.get("concurrency").and_then(|v| v.as_u64()) { current.concurrency = v as u32; }
    }
    fs::write(settings_path(), serde_json::to_string_pretty(&current)?)?;
    Ok(current)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    #[serde(rename = "fileName", default)]
    pub file_name: String,
    #[serde(rename = "filePath", default)]
    pub file_path: String,
    #[serde(rename = "imageUrl", default)]
    pub image_url: String,
    #[serde(rename = "sourceType", default)]
    pub source_type: String,
    pub aspect_ratio: Option<String>,
    pub contains_people: Option<bool>,
    pub reconstructed_prompt: Option<serde_json::Value>,
    #[serde(default)]
    pub reconstructed_prompt_zh: Option<serde_json::Value>,
    pub quality_notes: Option<Vec<String>>,
    pub prompt_en: Option<String>,
    pub prompt_zh: Option<String>,
    #[serde(rename = "qualityScore", default)]
    pub quality_score: u32,
    #[serde(rename = "qualityLabel", default)]
    pub quality_label: String,
    #[serde(rename = "qualityBreakdown", default)]
    pub quality_breakdown: serde_json::Value,
    #[serde(rename = "qualityWarnings", default)]
    pub quality_warnings: Vec<String>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub provider: String,
    #[serde(rename = "elapsedMs", default)]
    pub elapsed_ms: u64,
    #[serde(default)]
    pub favorite: bool,
    #[serde(rename = "createdAt", default)]
    pub created_at: u64,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub keyword: Option<String>,
    #[serde(rename = "minScore")]
    pub min_score: Option<u32>,
    #[serde(rename = "maxScore")]
    pub max_score: Option<u32>,
    pub favorite: Option<bool>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<usize>,
    pub page: Option<usize>,
}

fn read_history() -> Vec<HistoryItem> {
    let path = history_path();
    if !path.exists() { return vec![]; }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_history(items: &[HistoryItem]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fs::write(history_path(), serde_json::to_string_pretty(items)?)?;
    Ok(())
}

pub fn get_history(query: HistoryQuery) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut items = read_history();

    if let Some(ref kw) = query.keyword {
        let kw_lower = kw.to_lowercase();
        items.retain(|i| {
            i.prompt_en.as_deref().unwrap_or("").to_lowercase().contains(&kw_lower)
                || i.prompt_zh.as_deref().unwrap_or("").to_lowercase().contains(&kw_lower)
                || i.file_name.to_lowercase().contains(&kw_lower)
        });
    }
    if let Some(min) = query.min_score {
        items.retain(|i| i.quality_score >= min);
    }
    if let Some(max) = query.max_score {
        items.retain(|i| i.quality_score <= max);
    }
    if query.favorite == Some(true) {
        items.retain(|i| i.favorite);
    }

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = items.len();
    let page_size = query.page_size.unwrap_or(50);
    let page = query.page.unwrap_or(1);
    let start = (page - 1) * page_size;
    let paged: Vec<_> = items.into_iter().skip(start).take(page_size).collect();

    Ok(serde_json::json!({ "items": paged, "total": total }))
}

pub fn add_history_item(item: HistoryItem) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut all = read_history();
    all.insert(0, item);
    write_history(&all)
}

pub fn delete_history_items(ids: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let id_set: std::collections::HashSet<&str> = ids.iter().map(|s| s.as_str()).collect();
    let all = read_history();
    let filtered: Vec<_> = all.into_iter().filter(|i| !id_set.contains(i.id.as_str())).collect();
    write_history(&filtered)?;
    for id in ids {
        let thumb = thumbs_dir().join(format!("{}.jpg", id));
        let _ = fs::remove_file(thumb);
    }
    Ok(())
}

pub fn toggle_favorite(id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let mut all = read_history();
    let mut new_state = false;
    if let Some(item) = all.iter_mut().find(|i| i.id == id) {
        item.favorite = !item.favorite;
        new_state = item.favorite;
    }
    write_history(&all)?;
    Ok(new_state)
}

pub fn clear_history() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    write_history(&[])?;
    if let Ok(entries) = fs::read_dir(thumbs_dir()) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}
