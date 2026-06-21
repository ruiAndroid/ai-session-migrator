use crate::codex::{CommandError, Result};
use regex::bytes::Regex;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const BOM: &[u8] = b"\xef\xbb\xbf";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMetadata {
    pub thread_id: String,
    pub provider: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub cwd: String,
    pub source: String,
    pub cli_version: String,
    pub thread_source: Option<String>,
    pub title: String,
    pub first_user_message: String,
    pub preview: String,
    pub path: PathBuf,
}

pub fn metadata_from_bytes(raw: &[u8], path: &Path) -> Result<SessionMetadata> {
    let without_bom = if raw.starts_with(BOM) {
        &raw[BOM.len()..]
    } else {
        raw
    };
    let text = std::str::from_utf8(without_bom).map_err(|error| {
        CommandError::new(
            "invalid_utf8",
            format!("{} is not valid UTF-8 ({error})", path.display()),
        )
    })?;
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let first_line = lines.first().ok_or_else(|| {
        CommandError::new("empty_session", format!("{} is empty", path.display()))
    })?;
    let first: Value = serde_json::from_str(first_line).map_err(|error| {
        CommandError::new(
            "invalid_jsonl",
            format!(
                "{} has invalid session metadata JSON ({error})",
                path.display()
            ),
        )
    })?;
    if first.get("type").and_then(Value::as_str) != Some("session_meta") {
        return Err(CommandError::new(
            "missing_session_meta",
            format!("{} does not start with session metadata", path.display()),
        ));
    }
    let payload = first
        .get("payload")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CommandError::new(
                "missing_payload",
                format!("{} session metadata has no payload", path.display()),
            )
        })?;
    let thread_id = payload
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CommandError::new(
                "missing_thread_id",
                format!("{} session metadata has no id", path.display()),
            )
        })?
        .to_string();

    let mut first_user = String::new();
    let mut last_timestamp = first
        .get("timestamp")
        .and_then(Value::as_str)
        .or_else(|| payload.get("timestamp").and_then(Value::as_str))
        .map(str::to_string);

    for line in lines.iter().skip(1) {
        let value: Value = serde_json::from_str(line).map_err(|error| {
            CommandError::new(
                "invalid_jsonl",
                format!("{} has invalid JSONL row ({error})", path.display()),
            )
        })?;
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            last_timestamp = Some(timestamp.to_string());
        }
        if first_user.is_empty() {
            first_user = extract_user_text(value.get("payload").unwrap_or(&Value::Null));
            if first_user.starts_with("<environment_context>") {
                first_user.clear();
            }
        }
    }

    let preview = clean_preview(&first_user).unwrap_or_else(|| thread_id.clone());
    let title = title_from_preview(&preview);
    let created_at_ms = timestamp_ms(
        payload
            .get("timestamp")
            .and_then(Value::as_str)
            .or_else(|| first.get("timestamp").and_then(Value::as_str)),
    );
    let updated_at_ms = timestamp_ms(last_timestamp.as_deref());
    let provider = provider_from_bytes(without_bom).or_else(|| {
        payload
            .get("model_provider")
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    Ok(SessionMetadata {
        thread_id,
        provider,
        created_at_ms,
        updated_at_ms,
        cwd: payload
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        source: payload
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("vscode")
            .to_string(),
        cli_version: payload
            .get("cli_version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        thread_source: payload
            .get("thread_source")
            .and_then(Value::as_str)
            .map(str::to_string),
        title,
        first_user_message: preview.clone(),
        preview,
        path: path.to_path_buf(),
    })
}

pub fn compact_provider_marker(provider: &str) -> Result<Vec<u8>> {
    let encoded_provider = serde_json::to_string(provider)
        .map_err(|error| CommandError::new("invalid_provider", error.to_string()))?;
    Ok(format!("\"model_provider\":{encoded_provider}").into_bytes())
}

pub fn replace_provider_marker(raw: &[u8], provider: &str) -> Result<Vec<u8>> {
    let new_marker = compact_provider_marker(provider)?;
    let pattern = Regex::new(r#""model_provider"\s*:\s*"[^"]+""#)
        .map_err(|error| CommandError::new("regex_error", error.to_string()))?;
    let without_bom = if raw.starts_with(BOM) {
        &raw[BOM.len()..]
    } else {
        raw
    };
    let Some(match_) = pattern.find(without_bom) else {
        return Err(CommandError::new(
            "provider_marker_missing",
            "Session file does not contain model_provider marker.",
        ));
    };
    let mut fixed = Vec::with_capacity(without_bom.len() + new_marker.len());
    fixed.extend_from_slice(&without_bom[..match_.start()]);
    fixed.extend_from_slice(&new_marker);
    fixed.extend_from_slice(&without_bom[match_.end()..]);
    Ok(fixed)
}

fn provider_from_bytes(raw: &[u8]) -> Option<String> {
    let search_len = raw.len().min(20_000);
    let pattern = Regex::new(r#""model_provider"\s*:\s*"([^"]+)""#).ok()?;
    let captures = pattern.captures(&raw[..search_len])?;
    std::str::from_utf8(captures.get(1)?.as_bytes())
        .ok()
        .map(str::to_string)
}

fn extract_user_text(payload: &Value) -> String {
    if payload.get("type").and_then(Value::as_str) == Some("user_message") {
        return payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
    }
    if payload.get("type").and_then(Value::as_str) == Some("message")
        && payload.get("role").and_then(Value::as_str) == Some("user")
    {
        let mut parts = Vec::new();
        if let Some(content) = payload.get("content").and_then(Value::as_array) {
            for item in content {
                if item.get("type").and_then(Value::as_str) == Some("input_text") {
                    parts.push(item.get("text").and_then(Value::as_str).unwrap_or_default());
                }
            }
        }
        return parts.join("");
    }
    String::new()
}

fn clean_preview(text: &str) -> Option<String> {
    let mut value = text.trim().to_string();
    let marker = "## My request for Codex:";
    if let Some((_, rest)) = value.split_once(marker) {
        value = rest.trim().to_string();
    }
    let lines: Vec<&str> = value
        .lines()
        .filter(|line| !line.starts_with("<image "))
        .collect();
    let cleaned = lines
        .join("\n")
        .trim()
        .chars()
        .take(500)
        .collect::<String>();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn title_from_preview(preview: &str) -> String {
    preview
        .lines()
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Untitled session")
        .chars()
        .take(80)
        .collect()
}

fn timestamp_ms(value: Option<&str>) -> i64 {
    let Some(value) = value else {
        return chrono::Utc::now().timestamp_millis();
    };
    let normalized = value.replace('Z', "+00:00");
    chrono::DateTime::parse_from_rfc3339(&normalized)
        .map(|timestamp| timestamp.timestamp_millis())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_jsonl(thread_id: &str, provider: &str, message: &str) -> Vec<u8> {
        let text = format!(
            "{{\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread_id}\",\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"cwd\":\"D:\\\\work\",\"source\":\"vscode\",\"model_provider\":\"{provider}\",\"cli_version\":\"0.140.0\",\"thread_source\":\"user\"}}}}\n{{\"timestamp\":\"2026-06-15T01:01:00.000Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{message}\"}}]}}}}\n"
        );
        text.into_bytes()
    }

    #[test]
    fn parses_metadata_and_preserves_chinese_preview() {
        let raw = sample_jsonl(
            "019eca3b-941d-7340-9b14-328c635a6523",
            "funai",
            "你好，迁移 provider",
        );

        let metadata = metadata_from_bytes(&raw, Path::new("rollout.jsonl")).unwrap();

        assert_eq!(metadata.thread_id, "019eca3b-941d-7340-9b14-328c635a6523");
        assert_eq!(metadata.provider.as_deref(), Some("funai"));
        assert_eq!(metadata.title, "你好，迁移 provider");
        assert_eq!(metadata.preview, "你好，迁移 provider");
        assert_eq!(metadata.created_at_ms, 1_781_485_200_000);
        assert_eq!(metadata.updated_at_ms, 1_781_485_260_000);
    }

    #[test]
    fn accepts_utf8_bom_before_first_json_line() {
        let mut raw = BOM.to_vec();
        raw.extend(sample_jsonl(
            "019ec94d-720d-7a12-a379-28c8042bc6b4",
            "gmn",
            "带 BOM 的会话",
        ));

        let metadata = metadata_from_bytes(&raw, Path::new("rollout.jsonl")).unwrap();

        assert_eq!(metadata.thread_id, "019ec94d-720d-7a12-a379-28c8042bc6b4");
        assert_eq!(metadata.provider.as_deref(), Some("gmn"));
        assert_eq!(metadata.preview, "带 BOM 的会话");
    }

    #[test]
    fn provider_replacement_strips_bom_and_preserves_utf8_text() {
        let mut raw = BOM.to_vec();
        raw.extend(sample_jsonl(
            "019ec94d-720d-7a12-a379-28c8042bc6b4",
            "funai",
            "中文不会坏",
        ));

        let fixed = replace_provider_marker(&raw, "yihubangg").unwrap();

        assert!(!fixed.starts_with(BOM));
        let text = String::from_utf8(fixed).unwrap();
        assert!(text.contains("\"model_provider\":\"yihubangg\""));
        assert!(text.contains("中文不会坏"));
    }

    #[test]
    fn provider_replacement_accepts_unicode_target_provider() {
        let raw = sample_jsonl(
            "019ec94d-720d-7a12-a379-28c8042bc6b4",
            "funai",
            "中文不会坏",
        );

        let fixed = replace_provider_marker(&raw, "中文-provider").unwrap();

        let text = String::from_utf8(fixed).unwrap();
        assert!(text.contains("\"model_provider\":\"中文-provider\""));
        assert!(text.contains("中文不会坏"));
    }
}
