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

    validate_model_prompt_contract(&result)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))?;

    let elapsed = start.elapsed().as_millis() as u64;

    let structured_prompt = build_structured_prompt(&result);
    let quality = compute_quality_from_json(&result);
    let gpt_prompt_en = model_prompt_text(&result, &["gpt_image_2"], "prompt_en");
    let gpt_prompt_zh = model_prompt_text(&result, &["gpt_image_2"], "prompt_zh");
    let nano_prompt_en = model_prompt_text(&result, &["nano_banana", "nano_banana_pro"], "prompt_en");
    let nano_prompt_zh = model_prompt_text(&result, &["nano_banana", "nano_banana_pro"], "prompt_zh");

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
        prompt_gpt_image_en: gpt_prompt_en,
        prompt_gpt_image_zh: gpt_prompt_zh,
        prompt_nano_banana_en: nano_prompt_en,
        prompt_nano_banana_zh: nano_prompt_zh,
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
            let resp = client.get(url).send().await.map_err(|error| transport_error(url, &error))?;
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
    include_str!("inference_prompt.txt").to_string()
}

async fn call_gemini(image_base64: &str, mime_type: &str, settings: &Settings) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let model = if settings.model.is_empty() { "gemini-2.5-flash" } else { &settings.model };
    let api_key = storage::normalize_api_key(&settings.api_key);
    if api_key.is_empty() {
        return Err("API Key 为空，请先在设置中心填写并保存密钥".into());
    }
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
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
            "responseMimeType": "application/json",
            "temperature": 0.1,
            "maxOutputTokens": 8192
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(settings.timeout_ms))
        .build()?;

    let resp = client.post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| transport_error(&url, &error))?;

    let status = resp.status();
    let text = resp.text().await?;

    if !status.is_success() {
        return Err(format!("Gemini API error {}: {}", status, text).into());
    }

    let data: Value = serde_json::from_str(&text)?;
    ensure_completion_not_truncated(&data)?;
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

fn normalized_openai_credentials(settings: &Settings) -> Result<(String, String), String> {
    let base_url = if settings.base_url.trim().is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        storage::normalize_base_url(&settings.base_url)
    };
    let api_key = storage::normalize_api_key(&settings.api_key);

    if api_key.is_empty() {
        return Err("API Key 为空，请先在设置中心填写并保存密钥".to_string());
    }
    if base_url.contains("api.apimart.ai") && !api_key.starts_with("sk-") {
        return Err("APIMart API Key 格式无效：请填写 APIMart 控制台生成、以 sk- 开头的密钥，不要包含 Bearer、引号或空格".to_string());
    }

    Ok((base_url, api_key))
}

fn unauthorized_message() -> &'static str {
    "API Key 无效或已失效：请到设置中心重新粘贴仅含密钥本身的内容；APIMart 密钥应以 sk- 开头，不要包含 Bearer、引号或首尾空格"
}

fn transport_error(url: &str, error: &reqwest::Error) -> std::io::Error {
    let safe_url = url.split('?').next().unwrap_or(url);
    let mut causes = Vec::new();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        let detail = cause.to_string();
        if !detail.is_empty() && !causes.contains(&detail) {
            causes.push(detail);
        }
        source = std::error::Error::source(cause);
    }

    let details = if causes.is_empty() {
        error.to_string()
    } else {
        causes.join(" -> ")
    };
    let lower = details.to_ascii_lowercase();
    let (kind, hint) = if error.is_timeout() || lower.contains("timed out") {
        ("请求超时", "检查该电脑的网络延迟、防火墙或服务商节点是否可达")
    } else if lower.contains("dns") || lower.contains("lookup") || lower.contains("name resolution") {
        ("DNS 解析失败", "尝试更换 DNS，并确认浏览器可打开 api.apimart.ai")
    } else if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        ("HTTPS/TLS 握手失败", "检查系统时间、根证书以及代理软件的 HTTPS 解密设置")
    } else if lower.contains("proxy") {
        ("代理连接失败", "检查系统代理地址，或让 api.apimart.ai 走直连规则")
    } else if error.is_connect() {
        ("无法建立连接", "检查防火墙、系统代理、DNS，并确认服务商在该网络可访问")
    } else {
        ("网络传输失败", "检查该电脑的网络、系统时间、防火墙和代理设置")
    };

    std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        format!(
            "连接 API 失败：{}\n类型：{}\n底层原因：{}\n建议：{}。请求尚未到达 API 鉴权阶段，因此这不是模型名或 API Key 校验错误。",
            safe_url, kind, details, hint
        ),
    )
}

pub async fn test_api_connection(settings: Settings) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(settings.timeout_ms))
        .build()?;

    let (url, request) = if settings.provider_type == "gemini-native" {
        let api_key = storage::normalize_api_key(&settings.api_key);
        if api_key.is_empty() {
            return Err("API Key 为空，请先填写密钥".into());
        }
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key);
        let request = client.get(&url);
        (url, request)
    } else {
        let (base_url, api_key) = normalized_openai_credentials(&settings)
            .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
        let url = if base_url.contains("api.apimart.ai") {
            format!("{}/balance", base_url.trim_end_matches('/'))
        } else {
            format!("{}/models", base_url.trim_end_matches('/'))
        };
        let request = client.get(&url).header("Authorization", format!("Bearer {}", api_key));
        (url, request)
    };

    let response = request.send().await.map_err(|error| transport_error(&url, &error))?;
    let status = response.status();
    let text = response.text().await?;

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(unauthorized_message().into());
    }
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err("API Key 已过期、被禁用或没有访问权限，请在服务商控制台检查密钥状态".into());
    }
    if !status.is_success() {
        let snippet: String = text.chars().take(240).collect();
        return Err(format!("连接测试失败（HTTP {}）：{}", status, snippet).into());
    }

    if settings.base_url.contains("api.apimart.ai") {
        if let Ok(payload) = serde_json::from_str::<Value>(&text) {
            if payload.get("success").and_then(Value::as_bool) == Some(false) {
                let message = payload
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("APIMart 未确认该密钥有效");
                return Err(message.to_string().into());
            }
        }
    }

    Ok("API 连接成功，密钥有效".to_string())
}
async fn call_openai_compatible(image_url: Option<&str>, image_base64: &str, mime_type: &str, settings: &Settings) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let model = if settings.model.is_empty() { "gpt-4o" } else { &settings.model };
    let (base_url, api_key) = normalized_openai_credentials(settings)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let image_content = if let Some(img_url) = image_url {
        serde_json::json!({ "type": "image_url", "image_url": { "url": img_url } })
    } else {
        serde_json::json!({ "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", mime_type, image_base64) } })
    };

    let body = serde_json::json!({
        "model": model,
        "temperature": 0.1,
        "max_tokens": 8192,
        "stream": false,
        "response_format": { "type": "json_object" },
        "messages": [
            {
                "role": "system",
                "content": build_inference_instruction()
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": "Analyze this image now. Return the complete JSON contract, including both full model-specific reconstruction prompts."
                    },
                    image_content
                ]
            }
        ]
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(settings.timeout_ms))
        .build()?;

    let resp = client.post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|error| transport_error(&url, &error))?;

    let status = resp.status();
    let text = resp.text().await?;

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(unauthorized_message().into());
    }
    if !status.is_success() {
        return Err(format!("API error {}: {}", status, text).into());
    }

    let data: Value = serde_json::from_str(&text).map_err(|err| {
        let snippet: String = text.chars().take(300).collect();
        format!("API returned non-JSON response: {}. Raw response starts with: {}", err, snippet)
    })?;
    ensure_completion_not_truncated(&data)?;
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
    value
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn ensure_completion_not_truncated(data: &Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let openai_reason = data
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(Value::as_str);
    let gemini_reason = data
        .get("candidates")
        .and_then(|candidates| candidates.get(0))
        .and_then(|candidate| candidate.get("finishReason"))
        .and_then(Value::as_str);

    if openai_reason == Some("length") || gemini_reason == Some("MAX_TOKENS") {
        return Err("模型输出达到长度上限，完整双模型 Prompt 尚未生成。请提高模型输出上限或更换支持长 JSON 输出的视觉模型后重试。".into());
    }

    Ok(())
}

fn model_prompt_text(result: &Value, model_keys: &[&str], field: &str) -> Option<String> {
    for model_key in model_keys {
        if let Some(value) = value_text(
            result
                .get("model_prompts")
                .and_then(|v| v.get(*model_key))
                .and_then(|v| v.get(field)),
        ) {
            return Some(value);
        }
    }
    None
}

fn validate_model_prompt_contract(result: &Value) -> Result<(), String> {
    const GPT_SECTIONS: &[&str] = &[
        "OUTPUT FRAME",
        "CAMERA",
        "FIXED LAYOUT",
        "APPEARANCE",
        "LIGHTING",
        "INVARIANTS",
    ];
    const NANO_SECTIONS: &[&str] = &[
        "FRAME AND CAMERA",
        "EXACT SPATIAL LAYOUT",
        "MATERIALS AND COLOR",
        "LIGHT AND ATMOSPHERE",
        "LOCKED CONDITIONS",
    ];
    const GPT_ZH_SECTIONS: &[&str] = &["输出画幅", "相机", "固定布局", "外观", "光线", "不变量"];
    const NANO_ZH_SECTIONS: &[&str] = &["画幅与相机", "精确空间布局", "材质与色彩", "光线与氛围", "锁定条件"];

    let gpt_en = model_prompt_text(result, &["gpt_image_2"], "prompt_en");
    let gpt_zh = model_prompt_text(result, &["gpt_image_2"], "prompt_zh");
    let nano_en = model_prompt_text(result, &["nano_banana", "nano_banana_pro"], "prompt_en");
    let nano_zh = model_prompt_text(result, &["nano_banana", "nano_banana_pro"], "prompt_zh");
    let mut issues = Vec::new();

    if gpt_en.is_none() {
        issues.push("missing model_prompts.gpt_image_2.prompt_en".to_string());
    }
    if gpt_zh.is_none() {
        issues.push("missing model_prompts.gpt_image_2.prompt_zh".to_string());
    }
    if nano_en.is_none() {
        issues.push("missing model_prompts.nano_banana.prompt_en".to_string());
    }
    if nano_zh.is_none() {
        issues.push("missing model_prompts.nano_banana.prompt_zh".to_string());
    }

    if let Some(prompt) = gpt_en.as_deref() {
        let missing = GPT_SECTIONS
            .iter()
            .filter(|section| !prompt.contains(**section))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            issues.push(format!("gpt_image_2 missing sections: {}", missing.join(", ")));
        }
    }

    if let Some(prompt) = nano_en.as_deref() {
        let missing = NANO_SECTIONS
            .iter()
            .filter(|section| !prompt.contains(**section))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            issues.push(format!("nano_banana missing sections: {}", missing.join(", ")));
        }
    }

    if let Some(prompt) = gpt_en.as_deref() {
        if prompt.split_whitespace().count() < 70 {
            issues.push("GPT English prompt is too short (minimum 70 words)".to_string());
        }
    }
    if let Some(prompt) = nano_en.as_deref() {
        if prompt.split_whitespace().count() < 70 {
            issues.push("Nano Banana English prompt is too short (minimum 70 words)".to_string());
        }
    }
    if let Some(prompt) = gpt_zh.as_deref() {
        let missing = GPT_ZH_SECTIONS
            .iter()
            .filter(|section| !prompt.contains(**section))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            issues.push(format!("GPT 中文提示词缺少结构段落: {}", missing.join("、")));
        }
        if prompt.chars().filter(|character| !character.is_whitespace()).count() < 140 {
            issues.push("GPT 中文提示词过短（至少 140 个非空白字符）".to_string());
        }
    }
    if let Some(prompt) = nano_zh.as_deref() {
        let missing = NANO_ZH_SECTIONS
            .iter()
            .filter(|section| !prompt.contains(**section))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            issues.push(format!("Nano Banana 中文提示词缺少结构段落: {}", missing.join("、")));
        }
        if prompt.chars().filter(|character| !character.is_whitespace()).count() < 140 {
            issues.push("Nano Banana 中文提示词过短（至少 140 个非空白字符）".to_string());
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "模型返回了旧版或不完整的提示词结构，结果未保存。请重新分析；若持续出现，请更换支持视觉识别和长 JSON 输出的模型。{}",
            issues.join("; ")
        ))
    }
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
            "reconstruction_blueprint": result.get("reconstruction_blueprint").cloned().unwrap_or_else(|| serde_json::json!({})),
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
    fn inference_instruction_requires_two_model_prompts() {
        let instruction = build_inference_instruction();
        assert!(instruction.contains("model_prompts"));
        assert!(instruction.contains("gpt_image_2"));
        assert!(instruction.contains("nano_banana"));
        assert!(instruction.contains("EXACT SPATIAL LAYOUT"));
        assert!(instruction.contains("输出画幅"));
        assert!(instruction.contains("精确空间布局"));
        assert!(instruction.contains("reconstruction_blueprint"));
    }

    #[test]
    fn model_prompt_parser_keeps_model_outputs_distinct() {
        let result = serde_json::json!({
            "prompt_en": "legacy gpt prompt",
            "model_prompts": {
                "gpt_image_2": { "prompt_en": "geometry-first gpt prompt" },
                "nano_banana": { "prompt_en": "standalone nano reconstruction" }
            }
        });

        assert_eq!(
            model_prompt_text(&result, &["gpt_image_2"], "prompt_en").as_deref(),
            Some("geometry-first gpt prompt")
        );
        assert_eq!(
            model_prompt_text(&result, &["nano_banana", "nano_banana_pro"], "prompt_en").as_deref(),
            Some("standalone nano reconstruction")
        );
    }

    #[test]
    fn model_prompt_contract_rejects_legacy_summary_fallback() {
        let result = serde_json::json!({
            "prompt_en": "A bright kitchen with a person holding a trash bag.",
            "prompt_zh": "一个明亮厨房里有人拿着垃圾袋。"
        });

        let error = validate_model_prompt_contract(&result).unwrap_err();

        assert!(error.contains("model_prompts.gpt_image_2"));
        assert!(error.contains("model_prompts.nano_banana"));
    }

    #[test]
    fn model_prompt_contract_requires_model_specific_sections() {
        let result = serde_json::json!({
            "model_prompts": {
                "gpt_image_2": {
                    "prompt_en": "A generic reconstruction prompt.",
                    "prompt_zh": "一段通用重建提示词。"
                },
                "nano_banana": {
                    "prompt_en": "Another generic reconstruction prompt.",
                    "prompt_zh": "另一段通用重建提示词。"
                }
            }
        });

        let error = validate_model_prompt_contract(&result).unwrap_err();

        assert!(error.contains("OUTPUT FRAME"));
        assert!(error.contains("EXACT SPATIAL LAYOUT"));
    }

    #[test]
    fn model_prompt_contract_rejects_short_chinese_summaries() {
        let result = serde_json::json!({
            "model_prompts": {
                "gpt_image_2": {
                    "prompt_en": "OUTPUT FRAME: 16:9. CAMERA: eye level. FIXED LAYOUT: foreground and background. APPEARANCE: realistic. LIGHTING: daylight. INVARIANTS: one subject.",
                    "prompt_zh": "一个明亮的厨房场景。"
                },
                "nano_banana": {
                    "prompt_en": "FRAME AND CAMERA: 16:9 eye level. EXACT SPATIAL LAYOUT: foreground and background. MATERIALS AND COLOR: realistic. LIGHT AND ATMOSPHERE: daylight. LOCKED CONDITIONS: one subject.",
                    "prompt_zh": "一个明亮的厨房场景。"
                }
            }
        });

        let error = validate_model_prompt_contract(&result).unwrap_err();

        assert!(error.contains("GPT 中文提示词"));
        assert!(error.contains("Nano Banana 中文提示词"));
    }

    #[test]
    fn model_prompt_contract_accepts_complete_model_sections() {
        let gpt_en = format!("OUTPUT FRAME: 16:9 landscape frame. CAMERA: eye-level wide-angle view with controlled perspective and exact crop. FIXED LAYOUT: foreground, midground, and background objects keep their count, screen position, scale, overlap, and spacing. APPEARANCE: photorealistic surfaces with accurate colors, forms, and texture. LIGHTING: soft directional daylight with balanced highlights and shadows. INVARIANTS: preserve the described subject count, open areas, boundaries, proportions, and uncluttered surfaces. {}", "Detailed spatial reconstruction wording. ".repeat(8));
        let gpt_zh = format!("输出画幅：横向16:9画幅并锁定准确裁切。相机：平视广角机位，保持透视、地平线与取景范围。固定布局：逐区描述前景、中景和背景中物体的数量、位置、尺度、遮挡与间距。外观：准确描述可见颜色、形状、材质和纹理。光线：柔和定向自然光，保留高光、反射和阴影关系。不变量：锁定主体数量、开放区域、边界、比例与简洁表面。{}", "继续保留每个可见物体的准确空间关系和真实表面细节。".repeat(5));
        let nano_en = format!("FRAME AND CAMERA: generate a 16:9 landscape image from an eye-level wide-angle camera with exact crop and perspective. EXACT SPATIAL LAYOUT: reconstruct foreground, midground, and background object counts, positions, scale, orientation, overlap, spacing, and boundaries. MATERIALS AND COLOR: reproduce visible materials, colors, forms, and fine textures accurately. LIGHT AND ATMOSPHERE: use soft directional daylight with balanced reflections, highlights, and shadows. LOCKED CONDITIONS: keep the specified subjects, open regions, proportions, edges, and uncluttered surfaces fixed. {}", "Precise standalone text-to-image reconstruction detail. ".repeat(8));
        let nano_zh = format!("画幅与相机：生成横向16:9画面，采用平视广角机位并锁定准确裁切和透视。精确空间布局：逐区重建前景、中景和背景中物体的数量、位置、尺度、朝向、遮挡、间距与边界。材质与色彩：准确还原可见材质、颜色、形状和细密纹理。光线与氛围：使用柔和定向自然光，平衡反射、高光与阴影。锁定条件：固定指定主体、开放区域、比例、边缘与简洁表面。{}", "仅凭文字继续锁定每个可见物体的准确空间关系和真实表面细节。".repeat(5));
        let result = serde_json::json!({
            "model_prompts": {
                "gpt_image_2": { "prompt_en": gpt_en, "prompt_zh": gpt_zh },
                "nano_banana": { "prompt_en": nano_en, "prompt_zh": nano_zh }
            }
        });

        assert!(validate_model_prompt_contract(&result).is_ok());
    }

    #[test]
    fn structured_prompt_preserves_reconstruction_blueprint_and_model_prompts() {
        let result = serde_json::json!({
            "global_scene": { "art_style": "photorealistic" },
            "reconstruction_blueprint": { "frame": "16:9 landscape" },
            "model_prompts": {
                "gpt_image_2": { "prompt_en": "gpt" },
                "nano_banana": { "prompt_en": "nano" }
            }
        });

        let structured = build_structured_prompt(&result);
        assert_eq!(structured["reconstruction_blueprint"]["frame"], "16:9 landscape");
        assert_eq!(structured["model_prompts"]["gpt_image_2"]["prompt_en"], "gpt");
        assert_eq!(structured["model_prompts"]["nano_banana"]["prompt_en"], "nano");
    }

    #[test]
    fn openai_credentials_normalize_apimart_input() {
        let mut settings = Settings::default();
        settings.base_url = " \"https://api.apimart.ai/v1/chat/completions/\" ".to_string();
        settings.api_key = " Bearer sk-test-key\r\n".to_string();

        let (base_url, api_key) = normalized_openai_credentials(&settings).unwrap();

        assert_eq!(base_url, "https://api.apimart.ai/v1");
        assert_eq!(api_key, "sk-test-key");
    }

    #[test]
    fn openai_credentials_reject_non_apimart_token() {
        let mut settings = Settings::default();
        settings.base_url = "https://api.apimart.ai/v1".to_string();
        settings.api_key = "not-an-apimart-key".to_string();

        let error = normalized_openai_credentials(&settings).unwrap_err();

        assert!(error.contains("以 sk- 开头"));
    }
}
