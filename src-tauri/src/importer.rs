use crate::materials;
use crate::storage::{self, HistoryItem};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::Path;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub(crate) struct ImportedArchive {
    pub items: Vec<HistoryItem>,
    pub thumbnails: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportSummary {
    pub imported: usize,
    pub renamed: usize,
    pub skipped: usize,
    pub total: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportMergeOutcome {
    pub items: Vec<HistoryItem>,
    pub thumbnail_writes: Vec<(String, Vec<u8>)>,
    pub summary: ImportSummary,
}

pub fn import_items(input_path: &str) -> Result<Value, AnyError> {
    let archive = read_import_archive(input_path)?;
    let current = storage::list_history_items();
    let previous_history = current.clone();
    let outcome = prepare_import_merge(current, archive);

    storage::replace_history_items(&outcome.items)?;
    for (id, data) in &outcome.thumbnail_writes {
        storage::write_thumbnail(id, data)?;
    }
    let imported_items = &outcome.items[..outcome.summary.imported.min(outcome.items.len())];
    materials::sync_history_upserts(
        &previous_history,
        &outcome.items,
        imported_items,
    )
    .map_err(|error| {
        boxed_error(format!(
            "History was imported, but materials index sync failed: {error}"
        ))
    })?;

    Ok(serde_json::json!({
        "imported": outcome.summary.imported,
        "renamed": outcome.summary.renamed,
        "skipped": outcome.summary.skipped,
        "total": outcome.summary.total
    }))
}

pub(crate) fn sanitize_for_export(item: &HistoryItem) -> HistoryItem {
    let mut sanitized = item.clone();
    sanitized.file_path.clear();
    sanitized
}

fn read_import_archive(input_path: &str) -> Result<ImportedArchive, AnyError> {
    let ext = Path::new(input_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "json" => read_json_archive(input_path),
        "zip" => read_zip_archive(input_path),
        _ => Err(boxed_error("仅支持导入 JSON 或 ZIP 结果文件")),
    }
}

fn read_json_archive(input_path: &str) -> Result<ImportedArchive, AnyError> {
    let text = fs::read_to_string(input_path)?;
    let value: Value = serde_json::from_str(&text)?;
    Ok(ImportedArchive {
        items: parse_items_value(value)?,
        thumbnails: HashMap::new(),
    })
}

fn read_zip_archive(input_path: &str) -> Result<ImportedArchive, AnyError> {
    let file = fs::File::open(input_path)?;
    let mut zip = zip::ZipArchive::new(file)?;

    let data_json = {
        let mut data_file = zip
            .by_name("data.json")
            .map_err(|_| boxed_error("ZIP 中缺少 data.json"))?;
        let mut text = String::new();
        data_file.read_to_string(&mut text)?;
        text
    };

    let value: Value = serde_json::from_str(&data_json)?;
    let mut thumbnails = HashMap::new();

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        if !name.starts_with("thumbnails/") {
            continue;
        }
        let Some(stem) = Path::new(&name).file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        thumbnails.insert(stem.to_string(), data);
    }

    Ok(ImportedArchive {
        items: parse_items_value(value)?,
        thumbnails,
    })
}

fn parse_items_value(value: Value) -> Result<Vec<HistoryItem>, AnyError> {
    if value.is_array() {
        return Ok(serde_json::from_value(value)?);
    }

    for key in ["items", "data", "results"] {
        if let Some(items) = value.get(key) {
            return Ok(serde_json::from_value(items.clone())?);
        }
    }

    Err(boxed_error("导入文件中没有可识别的历史结果数组"))
}

pub(crate) fn prepare_import_merge(
    existing: Vec<HistoryItem>,
    archive: ImportedArchive,
) -> ImportMergeOutcome {
    let mut known_ids: HashSet<String> = existing.iter().map(|item| item.id.clone()).collect();
    let mut imported_items = Vec::new();
    let mut thumbnail_writes = Vec::new();
    let mut renamed = 0;
    let mut skipped = 0;

    for mut item in archive.items {
        if is_empty_history_item(&item) {
            skipped += 1;
            continue;
        }

        let original_id = item.id.clone();
        if item.id.trim().is_empty() || known_ids.contains(&item.id) {
            item.id = uuid::Uuid::new_v4().to_string();
            renamed += 1;
        }
        known_ids.insert(item.id.clone());

        item.file_path.clear();
        if item.source_type.trim().is_empty() {
            item.source_type = "imported".to_string();
        }
        if item.created_at == 0 {
            item.created_at = now_millis();
        }

        if let Some(data) = find_thumbnail(&archive.thumbnails, &original_id, &item.file_name) {
            thumbnail_writes.push((item.id.clone(), data.clone()));
        }

        imported_items.push(item);
    }

    let imported = imported_items.len();
    let mut items = imported_items;
    items.extend(existing);

    ImportMergeOutcome {
        items,
        thumbnail_writes,
        summary: ImportSummary {
            imported,
            renamed,
            skipped,
            total: imported + skipped,
        },
    }
}

fn is_empty_history_item(item: &HistoryItem) -> bool {
    item.id.trim().is_empty()
        && item.prompt_en.as_deref().unwrap_or("").trim().is_empty()
        && item.prompt_zh.as_deref().unwrap_or("").trim().is_empty()
        && item.reconstructed_prompt.is_none()
}

fn find_thumbnail<'a>(
    thumbnails: &'a HashMap<String, Vec<u8>>,
    original_id: &str,
    file_name: &str,
) -> Option<&'a Vec<u8>> {
    if let Some(data) = thumbnails.get(original_id) {
        return Some(data);
    }
    let stem = Path::new(file_name).file_stem().and_then(|s| s.to_str())?;
    thumbnails.get(stem)
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn boxed_error(message: impl Into<String>) -> AnyError {
    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn history_item(id: &str, prompt: &str) -> HistoryItem {
        HistoryItem {
            id: id.to_string(),
            file_name: format!("{id}.jpg"),
            file_path: String::from("C:\\private\\source.jpg"),
            image_url: String::new(),
            source_type: String::from("file"),
            aspect_ratio: Some(String::from("1:1")),
            contains_people: Some(false),
            reconstructed_prompt: None,
            reconstructed_prompt_zh: None,
            quality_notes: None,
            prompt_en: Some(prompt.to_string()),
            prompt_zh: Some(prompt.to_string()),
            quality_score: 80,
            quality_label: String::from("较强"),
            quality_breakdown: serde_json::json!({}),
            quality_warnings: vec![],
            model: String::from("gemini-2.5-flash"),
            provider: String::from("gemini-native"),
            elapsed_ms: 10,
            favorite: false,
            created_at: 1,
        }
    }

    #[test]
    fn merge_imported_items_renames_duplicates_and_clears_file_paths() {
        let existing = vec![history_item("same-id", "existing")];
        let archive = ImportedArchive {
            items: vec![history_item("same-id", "incoming")],
            thumbnails: HashMap::new(),
        };

        let outcome = prepare_import_merge(existing, archive);

        assert_eq!(outcome.summary.imported, 1);
        assert_eq!(outcome.summary.renamed, 1);
        assert_eq!(outcome.items.len(), 2);
        assert_ne!(outcome.items[0].id, "same-id");
        assert_eq!(outcome.items[0].file_path, "");
        assert_eq!(outcome.items[1].prompt_en.as_deref(), Some("existing"));
    }

    #[test]
    fn export_items_are_sanitized_before_sharing() {
        let mut item = history_item("item-1", "prompt");
        item.image_url = String::from("https://example.com/image.jpg");

        let sanitized = sanitize_for_export(&item);

        assert_eq!(sanitized.file_path, "");
        assert_eq!(sanitized.image_url, "https://example.com/image.jpg");
        assert_eq!(sanitized.id, "item-1");
    }
}
