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
    const FORBIDDEN_HEADINGS: &[&str] = &[
        "OUTPUT FRAME",
        "FIXED LAYOUT",
        "LOCKED CONDITIONS",
        "SCENE & PURPOSE",
        "RENDERING INTENT",
        "输出画幅：",
        "固定布局：",
        "锁定条件：",
        "场景与用途：",
        "渲染意图：",
    ];
    const REFERENCE_DEPENDENCIES: &[&str] = &[
        "reference image",
        "based on the reference",
        "refer to the image",
        "参考图",
        "保持原图",
        "与原图一致",
        "如图",
    ];

    let gpt_en = model_prompt_text(result, &["gpt_image_2"], "prompt_en");
    let gpt_zh = model_prompt_text(result, &["gpt_image_2"], "prompt_zh");
    let nano_en = model_prompt_text(result, &["nano_banana", "nano_banana_pro"], "prompt_en");
    let nano_zh = model_prompt_text(result, &["nano_banana", "nano_banana_pro"], "prompt_zh");
    let mut issues = Vec::new();

    for (field, prompt) in [
        ("model_prompts.gpt_image_2.prompt_en", gpt_en.as_deref()),
        ("model_prompts.gpt_image_2.prompt_zh", gpt_zh.as_deref()),
        ("model_prompts.nano_banana.prompt_en", nano_en.as_deref()),
        ("model_prompts.nano_banana.prompt_zh", nano_zh.as_deref()),
    ] {
        if prompt.is_none() {
            issues.push(format!("missing {field}"));
        }
    }

    if let Some(prompt) = gpt_en.as_deref() {
        let words = prompt.split_whitespace().count();
        if words < 100 {
            issues.push(format!("GPT English prompt is summary-level ({words}/100 words)"));
        }
    }
    if let Some(prompt) = nano_en.as_deref() {
        let words = prompt.split_whitespace().count();
        if words < 120 {
            issues.push(format!("Nano Banana English prompt is summary-level ({words}/120 words)"));
        }
        let start = prompt.trim_start().to_ascii_lowercase();
        if !start.starts_with("create ") && !start.starts_with("generate ") && !start.starts_with("produce ") {
            issues.push("Nano Banana prompt must begin with Create, Generate, or Produce".to_string());
        }
    }
    if let Some(prompt) = gpt_zh.as_deref() {
        let chars = prompt.chars().filter(|character| !character.is_whitespace()).count();
        if chars < 180 {
            issues.push(format!("GPT 中文提示词仍是摘要（{chars}/180 个非空白字符）"));
        }
    }
    if let Some(prompt) = nano_zh.as_deref() {
        let chars = prompt.chars().filter(|character| !character.is_whitespace()).count();
        if chars < 220 {
            issues.push(format!("Nano Banana 中文提示词仍是摘要（{chars}/220 个非空白字符）"));
        }
    }

    for prompt in [gpt_en.as_deref(), gpt_zh.as_deref(), nano_en.as_deref(), nano_zh.as_deref()]
        .into_iter()
        .flatten()
    {
        let upper = prompt.to_uppercase();
        let lower = prompt.to_lowercase();
        let visible_headings = FORBIDDEN_HEADINGS
            .iter()
            .filter(|heading| upper.contains(&heading.to_uppercase()))
            .copied()
            .collect::<Vec<_>>();
        if !visible_headings.is_empty() {
            issues.push(format!("final prompt contains visible template headings: {}", visible_headings.join(", ")));
        }
        let dependencies = REFERENCE_DEPENDENCIES
            .iter()
            .filter(|phrase| lower.contains(&phrase.to_lowercase()))
            .copied()
            .collect::<Vec<_>>();
        if !dependencies.is_empty() {
            issues.push(format!("prompt depends on an unavailable reference: {}", dependencies.join(", ")));
        }
    }

    if gpt_en.as_deref() == nano_en.as_deref() || gpt_zh.as_deref() == nano_zh.as_deref() {
        issues.push("GPT and Nano Banana prompts must use distinct model-native wording".to_string());
    }

    validate_reconstruction_blueprint(result, &mut issues);

    if issues.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "模型只返回了摘要级或旧版反推结果，结果未保存。请重新分析；若持续出现，请将识图模型从 mini/nano 级别换成更强的视觉模型。{}",
            issues.join("; ")
        ))
    }
}

fn validate_reconstruction_blueprint(result: &Value, issues: &mut Vec<String>) {
    let blueprint = result.get("reconstruction_blueprint");
    for field in ["frame", "camera", "surface_and_light"] {
        let value = blueprint
            .and_then(|item| item.get(field))
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if value.chars().filter(|character| !character.is_whitespace()).count() < 8 {
            issues.push(format!("reconstruction_blueprint.{field} is missing or too vague"));
        }
    }

    for (field, minimum) in [("fixed_layout", 3usize), ("spatial_relationships", 2), ("scene_invariants", 2)] {
        let count = blueprint
            .and_then(|item| item.get(field))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if count < minimum {
            issues.push(format!("reconstruction_blueprint.{field} needs at least {minimum} entries"));
        }
    }

    let entity_text = result
        .get("entities")
        .map(collect_json_text)
        .unwrap_or_default();
    if result.get("entities").and_then(Value::as_array).map(Vec::len).unwrap_or(0) < 1
        || entity_text.chars().filter(|character| !character.is_whitespace()).count() < 40
    {
        issues.push("entities does not contain a detailed visual inventory".to_string());
    }

    for field in ["foreground", "midground", "background"] {
        let value = result
            .get("environment_details")
            .and_then(|item| item.get(field))
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if value.chars().filter(|character| !character.is_whitespace()).count() < 6 {
            issues.push(format!("environment_details.{field} is missing or too vague"));
        }
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
        if word_count >= 100 && word_count <= 320 { s += 20; } else if word_count >= 70 && word_count <= 380 { s += 12; } else { s += 8; }
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

    fn complete_model_prompt_result() -> Value {
        let gpt_en = "A photorealistic commercial interior photograph of a calm modern kitchen intended for an architectural portfolio. The scene contains one central cooking island wrapped in veined white stone, one parallel sink island behind it, and a continuous wall of warm walnut cabinetry with two integrated steel ovens. Use wide landscape framing from an eye-level viewpoint, with the cooking island occupying the center-right foreground, the sink island offset behind it toward the left, and the window wall extending across the background. Show crisp stone veining, fine wood grain, brushed metal, clear glass, clean counter edges, and controlled natural reflections. Bright side daylight creates soft-edged shadows, cool white highlights, warm brown midtones, and a serene open atmosphere. Keep the exact two-island arrangement, open walkway, cabinet geometry, window boundaries, and uncluttered counters; do not add furniture, text, logos, or a watermark.";
        let gpt_zh = "一幅用于建筑作品集的高真实感现代厨房商业室内摄影。场景中有一座包覆白色纹理石材的中央烹饪岛、一座位于其后并与其平行的水槽岛，以及沿墙连续排列的暖色胡桃木橱柜和两台嵌入式钢制烤箱。采用横向宽幅构图和平视视点，烹饪岛占据前景中央偏右，水槽岛向左错位位于其后，窗墙横跨背景。清晰呈现石材纹理、细密木纹、拉丝金属、透明玻璃、整洁台面边缘和受控自然反射。明亮侧向日光形成柔边阴影、冷白高光、暖棕中间调和宁静开放的氛围。保持两座岛台的准确排列、开放通道、橱柜几何、窗墙边界与简洁台面，不添加家具、文字、标志或水印。";
        let nano_en = "Create a photorealistic 16:9 architectural portfolio image of a calm high-end modern kitchen with a broad eye-level field of view, straight vertical lines, and deep focus. Show exactly one rectangular cooking island in the center-right foreground, wrapped on the top and both visible ends in white stone with long gray veins; integrate one brushed-steel gas range into its rear half. Place exactly one parallel sink island behind it toward the left, leaving a clearly visible walkway between the two islands. Across the right background, build one continuous wall of warm walnut cabinetry with two vertically stacked steel ovens and a matching stone backsplash. Extend a floor-to-ceiling window wall across the left and rear background, keeping its dark frames evenly spaced. Render crisp stone veining, directional wood grain, brushed metal, clear glazing, and subtle surface reflections. Use bright natural side light with soft shadow edges, balanced warm brown and cool white tones, and a quiet airy mood. Keep the scene limited to these two islands and the fixed cabinetry, maintain the exact offsets and open floor area, keep the counters clean, and preserve continuous architectural boundaries.";
        let nano_zh = "生成一幅用于建筑作品集的高真实感16:9高端现代厨房画面，采用宽广的平视取景、端正垂直线和深景深。前景中央偏右准确放置一座长方形烹饪岛，台面和两个可见端面包覆带长灰色纹理的白色石材，在岛台后半部嵌入一台拉丝钢燃气灶。其后偏左准确放置一座与之平行的水槽岛，两座岛台之间保留清晰可见的通道。右侧背景沿墙建立一组连续的暖色胡桃木橱柜，包含两台上下排列的钢制烤箱和同材质石材背板。落地窗墙横跨左侧与后方背景，深色窗框等距排列。清晰渲染石材纹理、定向木纹、拉丝金属、透明玻璃和细微表面反射。使用明亮自然侧光、柔和阴影边缘、平衡的暖棕与冷白色调以及安静通透的氛围。画面保持两座岛台和固定橱柜这一单一连贯布局，维持准确错位、开放地面、简洁台面和连续建筑边界。";

        serde_json::json!({
            "global_scene": {
                "art_style": "高真实感商业室内摄影",
                "atmosphere": "宁静、开放、精致",
                "color_palette": ["暖棕色", "冷白色", "钢灰色"],
                "lighting": "明亮自然侧光，柔边阴影和受控反射"
            },
            "composition": {
                "camera_angle": "平视",
                "focal_length": "宽广角镜头感",
                "framing": "横向宽幅，两座岛台前后错位",
                "depth_of_field": "深景深"
            },
            "reconstruction_blueprint": {
                "frame": "横向16:9宽幅，完整保留左右窗墙与右侧橱柜边界",
                "camera": "平视机位，宽广角取景，垂直线端正，深景深",
                "fixed_layout": ["前景中央偏右为烹饪岛", "中景偏左为平行水槽岛", "背景右侧为连续胡桃木橱柜和双烤箱"],
                "spatial_relationships": ["两座岛台相互平行并保留开放通道", "窗墙位于岛台后方并横跨左侧背景"],
                "surface_and_light": "白色纹理石材、胡桃木、拉丝钢和玻璃受到明亮自然侧光照射",
                "scene_invariants": ["主要岛台数量固定为两座", "台面和开放通道保持简洁"]
            },
            "entities": [{
                "label": "两座厨房岛台",
                "appearance": "一座带钢制燃气灶的白色纹理石材烹饪岛和一座平行水槽岛",
                "pose": {
                    "action_description": "静态建筑陈列",
                    "body_language": "",
                    "spatial_position": "前景中央偏右与中景偏左前后错位"
                },
                "sub_elements": ["燃气灶", "水槽", "石材端面", "开放通道"]
            }],
            "environment_details": {
                "foreground": "前景为石材烹饪岛和清晰可见的地面",
                "midground": "中景为平行水槽岛及两岛之间的开放通道",
                "background": "背景为落地窗墙、连续胡桃木橱柜和双层钢制烤箱"
            },
            "model_prompts": {
                "gpt_image_2": { "prompt_en": gpt_en, "prompt_zh": gpt_zh },
                "nano_banana": { "prompt_en": nano_en, "prompt_zh": nano_zh }
            }
        })
    }

    #[test]
    fn inference_instruction_requires_detailed_natural_model_prompts() {
        let instruction = build_inference_instruction();
        assert!(instruction.contains("model_prompts"));
        assert!(instruction.contains("gpt_image_2"));
        assert!(instruction.contains("nano_banana"));
        assert!(instruction.contains("视觉库存"));
        assert!(instruction.contains("100 至 320 words"));
        assert!(instruction.contains("120 至 380 words"));
        assert!(instruction.contains("不显示字段标题"));
        assert!(instruction.contains("reconstruction_blueprint"));
    }

    #[test]
    fn model_prompt_parser_keeps_model_outputs_distinct() {
        let result = serde_json::json!({
            "prompt_en": "legacy gpt prompt",
            "model_prompts": {
                "gpt_image_2": { "prompt_en": "natural gpt production brief" },
                "nano_banana": { "prompt_en": "Create a standalone nano reconstruction" }
            }
        });

        assert_eq!(
            model_prompt_text(&result, &["gpt_image_2"], "prompt_en").as_deref(),
            Some("natural gpt production brief")
        );
        assert_eq!(
            model_prompt_text(&result, &["nano_banana", "nano_banana_pro"], "prompt_en").as_deref(),
            Some("Create a standalone nano reconstruction")
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
    fn model_prompt_contract_rejects_visible_template_headings() {
        let mut result = complete_model_prompt_result();
        result["model_prompts"]["gpt_image_2"]["prompt_en"] = serde_json::json!(format!(
            "OUTPUT FRAME: 16:9. {}",
            result["model_prompts"]["gpt_image_2"]["prompt_en"].as_str().unwrap()
        ));

        let error = validate_model_prompt_contract(&result).unwrap_err();
        assert!(error.contains("visible template headings"));
        assert!(error.contains("OUTPUT FRAME"));
    }

    #[test]
    fn model_prompt_contract_rejects_short_chinese_summaries() {
        let mut result = complete_model_prompt_result();
        result["model_prompts"]["gpt_image_2"]["prompt_zh"] = serde_json::json!("一个明亮的厨房场景。");
        result["model_prompts"]["nano_banana"]["prompt_zh"] = serde_json::json!("一个明亮的厨房场景。");

        let error = validate_model_prompt_contract(&result).unwrap_err();
        assert!(error.contains("GPT 中文提示词仍是摘要"));
        assert!(error.contains("Nano Banana 中文提示词仍是摘要"));
    }

    #[test]
    fn model_prompt_contract_rejects_shallow_visual_blueprint() {
        let mut result = complete_model_prompt_result();
        result["reconstruction_blueprint"]["fixed_layout"] = serde_json::json!(["只有一个笼统区域"]);

        let error = validate_model_prompt_contract(&result).unwrap_err();
        assert!(error.contains("fixed_layout needs at least 3 entries"));
    }

    #[test]
    fn model_prompt_contract_accepts_detailed_natural_prompts() {
        let result = complete_model_prompt_result();
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
