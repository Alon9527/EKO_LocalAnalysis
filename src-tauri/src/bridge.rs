use crate::{analyzer, storage, AnalysisTask};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const BRIDGE_ADDR: &str = "127.0.0.1:17621";
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct BridgeAnalyzeRequest {
    #[serde(default)]
    id: String,
    #[serde(rename = "fileName", default)]
    file_name: String,
    #[serde(rename = "imageUrl", default)]
    image_url: String,
    #[serde(rename = "base64Data", default)]
    base64_data: String,
    #[serde(rename = "mimeType", default)]
    mime_type: String,
}

pub fn spawn() {
    tauri::async_runtime::spawn(async {
        if let Err(err) = run().await {
            eprintln!("EKO local bridge stopped: {err}");
        }
    });
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(BRIDGE_ADDR).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(err) = handle_socket(stream).await {
                eprintln!("EKO bridge request failed: {err}");
            }
        });
    }
}

async fn handle_socket(mut stream: TcpStream) -> io::Result<()> {
    let request = match read_http_request(&mut stream).await {
        Ok(request) => request,
        Err(message) => {
            let response = response_bytes(400, None, json_error(&message));
            stream.write_all(&response).await?;
            return Ok(());
        }
    };

    let origin = request.headers.get("origin").cloned();
    if !is_allowed_origin(origin.as_deref()) {
        let response = response_bytes(403, None, json_error("Origin is not allowed"));
        stream.write_all(&response).await?;
        return Ok(());
    }

    let response = match route_request(request, origin.as_deref()).await {
        Ok(value) => response_bytes(200, origin.as_deref(), value),
        Err((status, message)) => response_bytes(status, origin.as_deref(), json_error(&message)),
    };
    stream.write_all(&response).await
}

async fn route_request(request: HttpRequest, origin: Option<&str>) -> Result<Value, (u16, String)> {
    if request.method == "OPTIONS" {
        return Ok(serde_json::json!({ "ok": true }));
    }

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => Ok(serde_json::json!({
            "ok": true,
            "app": "EKO LocalAnalysis",
            "version": env!("CARGO_PKG_VERSION"),
            "bridge": BRIDGE_ADDR,
            "origin": origin.unwrap_or("")
        })),
        ("POST", "/analyze") => analyze_from_bridge(request.body).await,
        _ => Err((404, "Not found".to_string())),
    }
}

async fn analyze_from_bridge(body: Vec<u8>) -> Result<Value, (u16, String)> {
    let payload: BridgeAnalyzeRequest = serde_json::from_slice(&body)
        .map_err(|err| (400, format!("Invalid JSON: {err}")))?;

    let settings = storage::get_settings()
        .map_err(|err| (500, format!("Failed to load settings: {err}")))?;

    let id = if payload.id.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        payload.id
    };

    let task = if !payload.base64_data.trim().is_empty() {
        let (base64_data, mime_type) = normalize_base64_image(&payload.base64_data, &payload.mime_type)
            .map_err(|message| (400, message))?;
        AnalysisTask {
            id,
            source_type: "clipboard".to_string(),
            file_path: None,
            file_name: Some(non_empty(payload.file_name, "browser-image.png")),
            image_url: None,
            base64_data: Some(base64_data),
            mime_type: Some(mime_type),
        }
    } else if !payload.image_url.trim().is_empty() {
        validate_image_url(&payload.image_url).await.map_err(|message| (400, message))?;
        AnalysisTask {
            id,
            source_type: "url".to_string(),
            file_path: None,
            file_name: Some(non_empty(payload.file_name, "browser-image.jpg")),
            image_url: Some(payload.image_url),
            base64_data: None,
            mime_type: None,
        }
    } else {
        return Err((400, "Missing imageUrl or base64Data".to_string()));
    };

    let item = analyzer::run_analysis(task, settings)
        .await
        .map_err(|err| (500, err.to_string()))?;

    Ok(serde_json::json!({
        "ok": true,
        "item": item
    }))
}

fn normalize_base64_image(data: &str, fallback_mime: &str) -> Result<(String, String), String> {
    let trimmed = data.trim();
    if let Some(rest) = trimmed.strip_prefix("data:") {
        let Some((meta, base64_data)) = rest.split_once(',') else {
            return Err("Invalid data URL image".to_string());
        };
        let mime = meta
            .split(';')
            .next()
            .filter(|mime| mime.starts_with("image/"))
            .unwrap_or("image/png")
            .to_string();
        return Ok((base64_data.to_string(), mime));
    }

    let mime = if fallback_mime.starts_with("image/") {
        fallback_mime.to_string()
    } else {
        "image/png".to_string()
    };
    Ok((trimmed.to_string(), mime))
}

fn non_empty(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_image_url_shape(raw: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(raw).map_err(|_| "Invalid image URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only http and https image URLs are supported".to_string());
    }

    let host = url.host_str().ok_or_else(|| "Image URL has no host".to_string())?;
    let host_lower = host.to_ascii_lowercase();
    if host_lower == "localhost" || host_lower.ends_with(".localhost") {
        return Err("Localhost image URLs are not allowed".to_string());
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err("Private or local image URLs are not allowed".to_string());
        }
    }

    Ok(())
}

async fn validate_image_url(raw: &str) -> Result<(), String> {
    validate_image_url_shape(raw)?;
    let url = reqwest::Url::parse(raw).map_err(|_| "Invalid image URL".to_string())?;
    let host = url.host_str().ok_or_else(|| "Image URL has no host".to_string())?;
    if host.parse::<IpAddr>().is_ok() {
        return Ok(());
    }

    let port = url.port_or_known_default().unwrap_or(443);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| "Image URL host cannot be resolved".to_string())?;
    for addr in addrs {
        if is_blocked_ip(addr.ip()) {
            return Err("Image URL resolves to a private or local address".to_string());
        }
    }
    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_unspecified()
                || ip.octets()[0] == 0
                || ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1])
                || ip.octets()[0] == 169 && ip.octets()[1] == 254
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn is_allowed_origin(origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(origin) => {
            origin.starts_with("chrome-extension://")
                || origin.starts_with("tauri://")
                || origin.starts_with("http://127.0.0.1:")
                || origin.starts_with("http://localhost:")
        }
    }
}

async fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let header_end = loop {
        let mut chunk = [0_u8; 8192];
        let read = stream.read(&mut chunk).await.map_err(|err| err.to_string())?;
        if read == 0 {
            return Err("Connection closed".to_string());
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_BODY_BYTES {
            return Err("Request is too large".to_string());
        }
        if let Some(pos) = find_header_end(&buffer) {
            break pos;
        }
    };

    let header_text = std::str::from_utf8(&buffer[..header_end])
        .map_err(|_| "Invalid HTTP header encoding".to_string())?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| "Missing request line".to_string())?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid request line".to_string());
    }

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err("Request body is too large".to_string());
    }

    let body_start = header_end + 4;
    let mut body = buffer.get(body_start..).unwrap_or_default().to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0_u8; std::cmp::min(8192, content_length - body.len())];
        let read = stream.read(&mut chunk).await.map_err(|err| err.to_string())?;
        if read == 0 {
            return Err("Connection closed before body completed".to_string());
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method: parts[0].to_ascii_uppercase(),
        path: parts[1].split('?').next().unwrap_or(parts[1]).to_string(),
        headers,
        body,
    })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn response_bytes(status: u16, origin: Option<&str>, body: Value) -> Vec<u8> {
    let body_text = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    let origin = origin.unwrap_or("*");
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\n\
Content-Type: application/json; charset=utf-8\r\n\
Content-Length: {}\r\n\
Access-Control-Allow-Origin: {origin}\r\n\
Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
Access-Control-Allow-Headers: Content-Type\r\n\
Vary: Origin\r\n\
Connection: close\r\n\r\n{}",
        body_text.as_bytes().len(),
        body_text
    )
    .into_bytes()
}

fn json_error(message: &str) -> Value {
    serde_json::json!({
        "ok": false,
        "error": message
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localhost_and_private_image_urls_are_rejected() {
        assert!(validate_image_url_shape("http://127.0.0.1/image.jpg").is_err());
        assert!(validate_image_url_shape("http://10.0.0.8/image.jpg").is_err());
        assert!(validate_image_url_shape("http://localhost/image.jpg").is_err());
    }

    #[test]
    fn public_http_image_urls_are_allowed() {
        assert!(validate_image_url_shape("https://example.com/image.jpg").is_ok());
        assert!(validate_image_url_shape("http://203.0.113.10/image.jpg").is_ok());
    }
}
