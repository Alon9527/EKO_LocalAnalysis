mod storage;
mod analyzer;
mod exporter;

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
    storage::delete_history_items(&ids).map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_favorite(id: String) -> Result<bool, String> {
    storage::toggle_favorite(&id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history() -> Result<(), String> {
    storage::clear_history().map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_items(ids: Vec<String>, format: String, output_path: String) -> Result<serde_json::Value, String> {
    exporter::export_items(&ids, &format, &output_path).map_err(|e| e.to_string())
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
            read_file_as_data_url,
            read_thumbnail_as_data_url,
            scan_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
