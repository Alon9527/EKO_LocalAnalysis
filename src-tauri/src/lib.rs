mod storage;
mod analyzer;
mod exporter;
mod importer;
mod bridge;
mod materials;

use serde::Deserialize;
use storage::{Settings, HistoryItem, HistoryQuery};

#[derive(Debug, Deserialize)]
pub struct AnalysisTask {
    pub id: String,
    #[serde(rename = "sourceType")]
    pub source_type: String,
    #[serde(rename = "filePath")]
    pub file_path: Option<String>,
    #[serde(rename = "fileName")]
    pub file_name: Option<String>,
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "base64Data")]
    pub base64_data: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[tauri::command]
async fn get_settings() -> Result<Settings, String> {
    storage::get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_settings(data: serde_json::Value) -> Result<Settings, String> {
    storage::save_settings(data).map_err(|e| e.to_string())
}

#[tauri::command]
async fn analyze_image(task: AnalysisTask, settings: Settings) -> Result<HistoryItem, String> {
    analyzer::run_analysis(task, settings).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_history(query: HistoryQuery) -> Result<serde_json::Value, String> {
    storage::get_history(query).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_history(ids: Vec<String>) -> Result<(), String> {
    let previous_history = storage::list_history_items();
    storage::delete_history_items(&ids).map_err(|e| e.to_string())?;
    let current_history = storage::list_history_items();
    materials::sync_removed_history(&previous_history, &current_history, &ids).map_err(|error| {
        format!(
            "History was deleted, but materials index sync failed: {error}"
        )
    })
}

#[tauri::command]
async fn toggle_favorite(id: String) -> Result<bool, String> {
    storage::toggle_favorite(&id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history() -> Result<(), String> {
    let previous_history = storage::list_history_items();
    storage::clear_history().map_err(|e| e.to_string())?;
    materials::sync_cleared_history(&previous_history).map_err(|error| {
        format!(
            "History was cleared, but materials index sync failed: {error}"
        )
    })
}

#[tauri::command]
async fn export_items(ids: Vec<String>, format: String, output_path: String) -> Result<serde_json::Value, String> {
    exporter::export_items(&ids, &format, &output_path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_items(input_path: String) -> Result<serde_json::Value, String> {
    importer::import_items(&input_path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_materials(
    query: materials::MaterialQuery,
) -> Result<materials::MaterialListResponse, String> {
    let history = storage::list_history_items();
    let index = materials::ensure_index(&history).map_err(|error| error.to_string())?;
    Ok(materials::query_index(&index, &query))
}

#[tauri::command]
async fn get_history_materials(
    history_id: String,
) -> Result<Vec<materials::MaterialAsset>, String> {
    let history = storage::list_history_items();
    let index = materials::ensure_index(&history).map_err(|error| error.to_string())?;
    Ok(materials::history_assets(&index, &history_id))
}

#[tauri::command]
async fn rebuild_material_index() -> Result<materials::MaterialListResponse, String> {
    let history = storage::list_history_items();
    let index = materials::rebuild_index(&history).map_err(|error| error.to_string())?;
    Ok(materials::query_index(
        &index,
        &materials::MaterialQuery::default(),
    ))
}

#[tauri::command]
async fn update_material(
    id: String,
    patch: materials::MaterialPatch,
) -> Result<materials::MaterialAsset, String> {
    let history = storage::list_history_items();
    materials::ensure_index(&history).map_err(|error| error.to_string())?;
    materials::mutate_index(|index| materials::apply_patch(index, &id, patch))
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn merge_materials(
    ids: Vec<String>,
    display_name: Option<String>,
) -> Result<materials::MaterialAsset, String> {
    let history = storage::list_history_items();
    materials::ensure_index(&history).map_err(|error| error.to_string())?;
    materials::mutate_index(|index| {
        materials::merge_assets(index, &ids, display_name)
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn split_material(
    id: String,
    source_ids: Vec<String>,
    display_name: String,
) -> Result<Vec<materials::MaterialAsset>, String> {
    let history = storage::list_history_items();
    materials::ensure_index(&history).map_err(|error| error.to_string())?;
    materials::mutate_index(|index| {
        materials::split_asset(index, &id, &source_ids, display_name)
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn read_file_as_data_url(file_path: String) -> Result<String, String> {
    let data = std::fs::read(&file_path).map_err(|e| e.to_string())?;
    let ext = std::path::Path::new(&file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "image/jpeg",
    };
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
    Ok(format!("data:{};base64,{}", mime, b64))
}

#[tauri::command]
async fn read_thumbnail_as_data_url(id: String) -> Result<String, String> {
    let path = storage::thumbs_dir().join(format!("{}.jpg", id));
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let mime = detect_image_mime(&data);
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
    Ok(format!("data:{};base64,{}", mime, b64))
}

fn detect_image_mime(data: &[u8]) -> &'static str {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif"
    } else {
        "image/jpeg"
    }
}

#[tauri::command]
async fn scan_folder(folder_path: String) -> Result<Vec<String>, String> {
    let exts = ["jpg", "jpeg", "png", "webp", "bmp", "gif"];
    let mut files = Vec::new();
    let entries = std::fs::read_dir(&folder_path).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
            if exts.contains(&ext.to_lowercase().as_str()) {
                if let Some(p) = entry.path().to_str() {
                    files.push(p.to_string());
                }
            }
        }
    }
    Ok(files)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    storage::ensure_data_dir().expect("Failed to create data directory");

    tauri::Builder::default()
        .setup(|_| {
            bridge::spawn();
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            analyze_image,
            get_history,
            delete_history,
            toggle_favorite,
            clear_history,
            export_items,
            import_items,
            list_materials,
            get_history_materials,
            rebuild_material_index,
            update_material,
            merge_materials,
            split_material,
            read_file_as_data_url,
            read_thumbnail_as_data_url,
            scan_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
