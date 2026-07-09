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
        prompt_en: result.get("prompt_en").and_then(|v| v.as_str()).map(|s| s.to_string()),
        prompt_zh: result.get("prompt_zh").and_then(|v| v.as_str()).map(|s| s.to_string()),
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

    storage::add_history_item(item.clone())?;

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
    r#"你是一个图片反推提示词大师，专门把用户上传或提供的参考图片，反推成适合 AIGC 生图使用的结构化双语 Prompt JSON。

工作规则：
1. 直接分析输入图片。
2. 只输出 raw JSON，不要输出 Markdown、解释、寒暄、代码块或额外说明。
3. 所有结构化分析字段使用流畅中文；prompt_en 使用英文；prompt_zh 是 prompt_en 的忠实中文翻译。
4. 只描述画面中可见或高度可信的视觉信息。不要编造品牌、型号、材质、场景故事或看不见的内容。
5. 使用正向视觉描述，避免负面提示词和排除式表达。
6. aspect_ratio 只能从以下值中选择最接近的一项：1:1、3:4、4:3、9:16、16:9。
7. contains_people 必须是 JSON boolean：true 或 false。
8. 如果图片中有可见文字，embedded_text 必须使用英文固定格式：with the text "..." in a typography，引号内文字不超过 25 个字符。如果没有可见文字，填空字符串 ""。
9. prompt_en 应该是可直接用于生图的高质量英文提示词，包含主体、环境、光线、构图、材质、镜头、氛围和技术质感，长度不超过 480 words。
10. prompt_zh 必须忠实翻译 prompt_en，不要额外扩写或删减。
11. 输出前自检 JSON 是否有效、字段是否完整、数组和 boolean 类型是否正确。

输出 JSON 必须使用以下结构：
{
  "global_scene": {
    "art_style": "",
    "atmosphere": "",
    "color_palette": [],
    "lighting": ""
  },
  "composition": {
    "camera_angle": "",
    "focal_length": "",
    "framing": "",
    "depth_of_field": ""
  },
  "entities": [
    {
      "label": "",
      "appearance": "",
      "pose": {
        "action_description": "",
        "body_language": "",
        "spatial_position": ""
      },
      "sub_elements": []
    }
  ],
  "environment_details": {
    "foreground": "",
    "midground": "",
    "background": ""
  },
  "technical_specs": {
    "texture_fidelity": "",
    "render_engine_style": "",
    "vfx": []
  },
  "aspect_ratio": "1:1",
  "contains_people": false,
  "embedded_text": "",
  "prompt_en": "",
  "prompt_zh": ""
}

字段写法要求：
- global_scene.art_style：画面媒介与风格，如商业摄影、电影感产品摄影、数字插画、3D 渲染、概念艺术等。
- global_scene.atmosphere：整体情绪和氛围。
- global_scene.color_palette：主要色彩和点缀色。
- global_scene.lighting：光源方向、柔硬、强弱、色温、反射、阴影。
- composition.camera_angle：视角，如平视、俯拍、低角度、近景、微距。
- composition.focal_length：镜头感，如广角、标准镜头、人像长焦、微距、长焦压缩。
- composition.framing：主体位置、裁切、对称、三分法、留白、视觉平衡。
- composition.depth_of_field：景深、焦点、虚化、散景。
- entities：列出画面中重要主体或物体，包含外观、材质、颜色、动作、位置和子元素。
- environment_details：拆成前景、中景、背景。
- technical_specs.texture_fidelity：材质细节，如织物纹理、金属反光、玻璃、皮肤、纸张、塑料等。
- technical_specs.render_engine_style：摄影或渲染质感，如真实商业摄影、Octane 风格、Unreal 风格、水彩、矢量、胶片等。
- technical_specs.vfx：视觉效果，如光晕、雾气、粒子、运动模糊、颗粒、镜头光斑、反射等。

Return JSON only."#.to_string()
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
            "embedded_text": result.get("embedded_text").cloned().unwrap_or_else(|| serde_json::json!(""))
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
