use crate::materials;
use crate::storage::{self, HistoryItem, Settings};
use crate::AnalysisTask;
use serde_json::Value;
use std::time::Instant;

pub async fn run_analysis(task: AnalysisTask, settings: Settings) -> Result<HistoryItem, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();

    let (image_base64, mime_type) = get_image_data(&task, &settings).await?;

    save_thumbnail(&task.id, &image_base64);

    let result = if settings.provider_type == "gemini-native" {
        call_gemini(&image_base64, &mime_type, &settings).await?
    } else {
        let image_url = if task.source_type == "url" { task.image_url.clone() } else { None };
        call_openai_compatible(image_url.as_deref(), &image_base64, &mime_type, &settings).await?
    };

    let elapsed = start.elapsed().as_millis() as u64;

    let structured_prompt = build_structured_prompt(&result);
    let quality = compute_quality_from_json(&result);
    let gpt_prompt_en = model_prompt_text(&result, "gpt_image_2", "prompt_en");
    let gpt_prompt_zh = model_prompt_text(&result, "gpt_image_2", "prompt_zh");
    let nano_prompt_en = value_text(result.get("model_prompts").and_then(|v| v.get("nano_banana_pro")).and_then(|v| v.get("prompt_en")));
    let nano_prompt_zh = value_text(result.get("model_prompts").and_then(|v| v.get("nano_banana_pro")).and_then(|v| v.get("prompt_zh")));

    let item = HistoryItem {
        id: task.id.clone(),
        file_name: task.file_name.unwrap_or_default(),
        file_path: task.file_path.unwrap_or_default(),
        image_url: task.image_url.unwrap_or_default(),
        source_type: task.source_type.clone(),
        aspect_ratio: result.get("aspect_ratio").and_then(|v| v.as_str()).map(|s| s.to_string()),
        contains_people: result.get("contains_people").and_then(|v| v.as_bool()),
        reconstructed_prompt: Some(structured_prompt),
        reconstructed_prompt_zh: None,
        quality_notes: None,
        prompt_en: gpt_prompt_en.clone(),
        prompt_zh: gpt_prompt_zh.clone(),
        prompt_gpt_image_en: gpt_prompt_en.clone(),
        prompt_gpt_image_zh: gpt_prompt_zh.clone(),
        prompt_nano_banana_en: nano_prompt_en.clone(),
        prompt_nano_banana_zh: nano_prompt_zh.clone(),
        quality_score: quality.0,
        quality_label: quality.1.clone(),
        quality_breakdown: quality.2.clone(),
        quality_warnings: quality.3.clone(),
        model: settings.model.clone(),
        provider: settings.provider_type.clone(),
        elapsed_ms: elapsed,
        favorite: false,
        created_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    };

    let previous_history = storage::list_history_items();
    storage::add_history_item(item.clone())?;
    let current_history = storage::list_history_items();
    materials::sync_history_upserts(
        &previous_history,
        &current_history,
        std::slice::from_ref(&item),
    )
    .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("History was saved, but materials index sync failed: {error}"),
        )
        .into()
    })?;

    Ok(item)
}

async fn get_image_data(task: &AnalysisTask, settings: &Settings) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
    match task.source_type.as_str() {
        "file" => {
            let path = task.file_path.as_deref().ok_or("No file path")?;
            let data = std::fs::read(path)?;
            let ext = std::path::Path::new(path)
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
            Ok((b64, mime.to_string()))
        }
        "url" => {
            let url = task.image_url.as_deref().ok_or("No image URL")?;
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(settings.timeout_ms))
                .build()?;
            let resp = client.get(url).send().await?;
            let content_type = resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("image/jpeg")
                .split(';')
                .next()
                .unwrap_or("image/jpeg")
                .to_string();
            let bytes = resp.bytes().await?;
            let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
            let mime = if content_type.starts_with("image/") { content_type } else { "image/jpeg".to_string() };
            Ok((b64, mime))
        }
        "clipboard" => {
            let b64 = task.base64_data.as_deref().ok_or("No clipboard data")?;
            let mime = task.mime_type.as_deref().unwrap_or("image/png");
            Ok((b64.to_string(), mime.to_string()))
        }
        _ => Err("Invalid source type".into()),
    }
}

fn save_thumbnail(task_id: &str, base64_data: &str) {
    if let Ok(data) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, base64_data) {
        let path = storage::thumbs_dir().join(format!("{}.jpg", task_id));
        let _ = std::fs::write(path, data);
    }
}

fn build_inference_instruction() -> String {
    r#"You are an expert image-to-prompt analyst. Analyze the provided reference image and return only valid raw JSON. Do not include Markdown, explanations, comments, or code fences.

Core rules:
1. Describe only visible or highly credible visual information. Do not invent brand names, model names, hidden stories, unseen materials, or unsupported details.
2. Use fluent Chinese for structured analysis fields. Use English for prompt_en fields. Use faithful Chinese translations for prompt_zh fields.
3. Use positive visual language. Prefer preserve, match, keep, and recreate instead of negative exclusion phrasing.
4. aspect_ratio must be the closest value from: 1:1, 3:4, 4:3, 9:16, 16:9.
5. contains_people must be a JSON boolean.
6. If visible text exists, embedded_text must be exactly: with the text "..." in a typography. Keep quoted text under 25 characters. If no visible text exists, use an empty string.
7. Top-level prompt_en and prompt_zh must mirror model_prompts.gpt_image_2 for backward compatibility.

Model-specific prompt rules:
- GPT Image: write one dense production-ready descriptive prompt. Emphasize subject, environment, lighting, composition, materials, camera/lens, atmosphere, and technical finish. Keep it natural, compact, and directly usable for GPT Image. Keep it under 480 words.
- Nano Banana Pro / Gemini image generation: write a standalone text-to-image prompt that can recreate the analyzed image without needing the original image at generation time. Start with a direct scene goal such as: Tightly recreate this composition as a text-to-image generation. Do not write prompts that depend on uploading or viewing the reference image again. Explicitly lock the aspect ratio, camera angle, lens feel, perspective height, crop, subject identity, object count, exact spatial layout, foreground left, foreground right, midground left, midground right, central midground, background, lighting direction, shadow softness, color palette, material textures, visible text, reflections, and atmosphere. Describe all important objects by position, size relationship, color, shape, material, and relationship to neighboring objects. Use stable foreground-to-background ordering and precise geometric language so the prompt is self-contained. For nano_banana_pro.prompt_zh, keep a faithful Chinese version with clear sections such as 前景左侧, 前景右侧, 中景, 背景 when those spatial zones exist. Keep it under 620 words.

Return JSON with exactly this structure:
{
  "global_scene": { "art_style": "", "atmosphere": "", "color_palette": [], "lighting": "" },
  "composition": { "camera_angle": "", "focal_length": "", "framing": "", "depth_of_field": "" },
  "entities": [ { "label": "", "appearance": "", "pose": { "action_description": "", "body_language": "", "spatial_position": "" }, "sub_elements": [] } ],
  "environment_details": { "foreground": "", "midground": "", "background": "" },
  "technical_specs": { "texture_fidelity": "", "render_engine_style": "", "vfx": [] },
  "aspect_ratio": "1:1",
  "contains_people": false,
  "embedded_text": "",
  "model_prompts": {
    "gpt_image_2": { "prompt_en": "", "prompt_zh": "" },
    "nano_banana_pro": { "prompt_en": "", "prompt_zh": "" }
  },
  "prompt_en": "",
  "prompt_zh": ""
}

Validate before output: valid JSON, complete fields, correct arrays and booleans, no trailing commas."#.to_string()
}

async fn call_gemini(image_base64: &str, mime_type: &str, settings: &Settings) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let model = if settings.model.is_empty() { "gemini-2.5-flash" } else { &settings.model };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, settings.api_key
    );

    let body = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [
                { "text": build_inference_instruction() },
                { "inline_data": { "mimeType": mime_type, "data": image_base64 } }
            ]
        }],
        "generationConfig": {
            "responseMimeType": "application/json"
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(settings.timeout_ms))
        .build()?;

    let resp = client.post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        return Err(format!("Gemini API error {}: {}", status, text).into());
    }

    let data: Value = serde_json::from_str(&text)?;
    let content_text = data["candidates"][0]["content"]["parts"]
        .as_array()
        .map(|parts| {
            parts.iter()
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    parse_json_response(&content_text)
}

async fn call_openai_compatible(image_url: Option<&str>, image_base64: &str, mime_type: &str, settings: &Settings) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let model = if settings.model.is_empty() { "gpt-4o" } else { &settings.model };
    let base_url = if settings.base_url.is_empty() { "https://api.openai.com/v1" } else { &settings.base_url };
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let image_content = if let Some(img_url) = image_url {
        serde_json::json!({ "type": "image_url", "image_url": { "url": img_url } })
    } else {
        serde_json::json!({ "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", mime_type, image_base64) } })
    };

    let body = serde_json::json!({
        "model": model,
        "temperature": 0.2,
        "response_format": { "type": "json_object" },
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": build_inference_instruction() },
                image_content
            ]
        }]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(settings.timeout_ms))
        .build()?;

    let resp = client.post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", settings.api_key))
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        return Err(format!("API error {}: {}", status, text).into());
    }

    let data: Value = serde_json::from_str(&text)?;
    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");

    parse_json_response(content)
}

fn parse_json_response(text: &str) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let trimmed = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return Ok(v);
    }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if let Ok(v) = serde_json::from_str::<Value>(&trimmed[start..=end]) {
                return Ok(v);
            }
        }
    }
    Err("Failed to parse model response as JSON".into())
}

fn value_text(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| v.as_str()).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn model_prompt_text(result: &Value, model_key: &str, field: &str) -> Option<String> {
    value_text(result.get("model_prompts").and_then(|v| v.get(model_key)).and_then(|v| v.get(field)))
        .or_else(|| value_text(result.get(field)))
}

fn build_structured_prompt(result: &Value) -> Value {
    if result.get("global_scene").is_some()
        || result.get("composition").is_some()
        || result.get("entities").is_some()
        || result.get("environment_details").is_some()
        || result.get("technical_specs").is_some()
    {
        serde_json::json!({
            "global_scene": result.get("global_scene").cloned().unwrap_or_else(|| serde_json::json!({})),
            "composition": result.get("composition").cloned().unwrap_or_else(|| serde_json::json!({})),
            "entities": result.get("entities").cloned().unwrap_or_else(|| serde_json::json!([])),
            "environment_details": result.get("environment_details").cloned().unwrap_or_else(|| serde_json::json!({})),
            "technical_specs": result.get("technical_specs").cloned().unwrap_or_else(|| serde_json::json!({})),
            "embedded_text": result.get("embedded_text").cloned().unwrap_or_else(|| serde_json::json!("")),
            "model_prompts": result.get("model_prompts").cloned().unwrap_or_else(|| serde_json::json!({}))
        })
    } else {
        result.get("reconstructed_prompt").cloned().unwrap_or_else(|| serde_json::json!({}))
    }
}

fn collect_json_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(collect_json_text).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "),
        Value::Object(map) => map.values().map(collect_json_text).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", "),
        _ => String::new(),
    }
}

fn compute_quality_from_json(result: &Value) -> (u32, String, Value, Vec<String>) {
    let structured = build_structured_prompt(result);

    let score_text = |text: &str, min_words: usize| -> u32 {
        let clean = text.trim();
        if clean.is_empty() { return 0; }
        let words: Vec<&str> = clean.split_whitespace().collect();
        let mut score: u32 = 30;
        score += std::cmp::min(35, ((words.len() as f64 / min_words as f64) * 35.0) as u32);
        if clean.contains(',') || clean.contains(':') { score += 5; }
        score += 10;
        std::cmp::min(100, score)
    };

    let subject_text = if structured.get("entities").is_some() {
        collect_json_text(&structured["entities"])
    } else {
        structured["subject"].as_str().unwrap_or("").to_string()
    };
    let context_text = if structured.get("environment_details").is_some() {
        collect_json_text(&structured["environment_details"])
    } else {
        structured["context_and_background"].as_str().unwrap_or("").to_string()
    };
    let lighting_text = structured["global_scene"]["lighting"]
        .as_str()
        .unwrap_or_else(|| structured["lighting"].as_str().unwrap_or(""))
        .to_string();
    let camera_text = if structured.get("composition").is_some() {
        collect_json_text(&structured["composition"])
    } else {
        structured["camera_and_composition"].as_str().unwrap_or("").to_string()
    };

    let subject = score_text(&subject_text, 12);
    let context = score_text(&context_text, 14);
    let lighting = score_text(&lighting_text, 10);
    let camera = score_text(&camera_text, 10);
    let text_score: u32 = {
        let et = structured["embedded_text"]
            .as_str()
            .unwrap_or_else(|| structured["embedded_text_syntax"].as_str().unwrap_or(""))
            .trim();
        if et.is_empty() { 92 } else if et.contains("with the text") { 100 } else { 50 }
    };

    let prompt_en = result["prompt_en"].as_str().unwrap_or("");
    let word_count = prompt_en.split_whitespace().count();
    let imagen: u32 = {
        let mut s: u32 = 0;
        if result["aspect_ratio"].as_str().is_some() { s += 20; }
        if result["contains_people"].as_bool().is_some() { s += 10; }
        if !prompt_en.is_empty() { s += 20; }
        if word_count >= 30 && word_count <= 220 { s += 20; } else { s += 8; }
        if collect_json_text(&structured["global_scene"]).len() > 5 || structured["style_prefix"].as_str().unwrap_or("").len() > 5 { s += 15; }
        s += 10;
        std::cmp::min(100, s)
    };

    let weighted = subject as f64 * 0.24 + context as f64 * 0.18 + lighting as f64 * 0.16
        + camera as f64 * 0.16 + text_score as f64 * 0.08 + imagen as f64 * 0.18;
    let total = std::cmp::max(1, std::cmp::min(100, weighted.round() as u32));

    let label = if total >= 90 { "很高" } else if total >= 78 { "较强" } else if total >= 64 { "可用" } else if total >= 45 { "偏弱" } else { "较低" };

    let mut warnings = Vec::new();
    if subject < 70 { warnings.push("主体细节偏弱".to_string()); }
    if context < 70 { warnings.push("空间层次不足".to_string()); }
    if lighting < 68 { warnings.push("光影描述偏弱".to_string()); }
    if total >= 86 { warnings.clear(); }

    let breakdown = serde_json::json!({
        "subject": subject,
        "context": context,
        "lighting": lighting,
        "camera": camera,
        "text": text_score,
        "imagen": imagen,
    });

    (total, label.to_string(), breakdown, warnings)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_prompt_preserves_model_specific_prompts() {
        let result = serde_json::json!({
            "global_scene": { "art_style": "commercial photography" },
            "composition": { "camera_angle": "eye level" },
            "entities": [],
            "environment_details": {},
            "technical_specs": {},
            "embedded_text": "",
            "model_prompts": {
                "gpt_image_2": { "prompt_en": "A compact descriptive prompt for GPT Image.", "prompt_zh": "GPT Image prompt zh." },
                "nano_banana_pro": { "prompt_en": "Recreate the reference image closely and preserve the same camera angle.", "prompt_zh": "Nano Banana prompt zh." }
            }
        });

        let structured = build_structured_prompt(&result);
        assert_eq!(structured["model_prompts"]["nano_banana_pro"]["prompt_en"], "Recreate the reference image closely and preserve the same camera angle.");
    }

    #[test]
    fn model_prompt_text_prefers_specific_model_then_legacy_prompt() {
        let result = serde_json::json!({
            "prompt_en": "Legacy prompt",
            "model_prompts": { "gpt_image_2": { "prompt_en": "GPT prompt" } }
        });

        assert_eq!(model_prompt_text(&result, "gpt_image_2", "prompt_en"), Some("GPT prompt".to_string()));
        assert_eq!(model_prompt_text(&result, "nano_banana_pro", "prompt_en"), Some("Legacy prompt".to_string()));
    }
    #[test]
    fn nano_instruction_requires_standalone_text_to_image_reconstruction() {
        let instruction = build_inference_instruction();

        assert!(instruction.contains("standalone text-to-image prompt"));
        assert!(instruction.contains("without needing the original image"));
        assert!(instruction.contains("foreground left"));
        assert!(instruction.contains("foreground right"));
        assert!(instruction.contains("midground"));
        assert!(instruction.contains("background"));
        assert!(instruction.contains("object count"));
        assert!(instruction.contains("spatial layout"));
        assert!(instruction.contains("aspect ratio"));
    }
}
