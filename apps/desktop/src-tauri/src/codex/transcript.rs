use crate::codex::metadata::{metadata_from_bytes, BOM};
use crate::codex::scan::session_files;
use crate::codex::{
    CommandError, Result, SessionTranscript, SessionTranscriptRequest, TranscriptTurn,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_session_transcript(request: SessionTranscriptRequest) -> Result<SessionTranscript> {
    let codex_home = Path::new(&request.codex_home);
    let file_path = find_session_file(codex_home, &request.thread_id)?;
    let raw = fs::read(&file_path)
        .map_err(|error| CommandError::io("read session transcript", file_path.display(), error))?;
    let metadata = metadata_from_bytes(&raw, &file_path)?;
    let text = session_text(&raw, &file_path)?;
    let mut turns = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()).skip(1) {
        let value: Value = serde_json::from_str(line).map_err(|error| {
            CommandError::new(
                "invalid_jsonl",
                format!("{} has invalid JSONL row ({error})", file_path.display()),
            )
        })?;
        let Some(payload) = value.get("payload") else {
            continue;
        };
        let timestamp = value
            .get("timestamp")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some((role, text)) = extract_turn(payload) {
            turns.push(TranscriptTurn {
                role,
                text,
                timestamp,
                index: turns.len(),
            });
        }
    }

    Ok(SessionTranscript {
        thread_id: metadata.thread_id,
        title: metadata.title,
        path: file_path.display().to_string(),
        turns,
    })
}

fn find_session_file(codex_home: &Path, thread_id: &str) -> Result<PathBuf> {
    for file in session_files(codex_home)? {
        let raw = fs::read(&file.path)
            .map_err(|error| CommandError::io("read session", file.path.display(), error))?;
        let metadata = metadata_from_bytes(&raw, &file.path)?;
        if metadata.thread_id == thread_id {
            return Ok(file.path);
        }
    }
    Err(CommandError::new(
        "selected_thread_missing",
        format!("Selected session is missing: {thread_id}"),
    ))
}

fn session_text<'a>(raw: &'a [u8], path: &Path) -> Result<&'a str> {
    let without_bom = if raw.starts_with(BOM) {
        &raw[BOM.len()..]
    } else {
        raw
    };
    std::str::from_utf8(without_bom).map_err(|error| {
        CommandError::new(
            "invalid_utf8",
            format!("{} is not valid UTF-8 ({error})", path.display()),
        )
    })
}

fn extract_turn(payload: &Value) -> Option<(String, String)> {
    if payload.get("type").and_then(Value::as_str) == Some("user_message") {
        let text = payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        return (!text.is_empty()).then_some(("user".to_string(), text));
    }

    if payload.get("type").and_then(Value::as_str) != Some("message") {
        return None;
    }
    let role = payload
        .get("role")
        .and_then(Value::as_str)
        .map(normalize_role)
        .unwrap_or_else(|| "other".to_string());
    let text = message_content_text(payload)?;
    Some((role, text))
}

fn normalize_role(role: &str) -> String {
    match role {
        "user" | "assistant" | "system" | "tool" => role.to_string(),
        _ => "other".to_string(),
    }
}

fn message_content_text(payload: &Value) -> Option<String> {
    if let Some(text) = payload.get("content").and_then(Value::as_str) {
        let text = text.trim().to_string();
        return (!text.is_empty()).then_some(text);
    }

    let mut parts = Vec::new();
    for item in payload.get("content").and_then(Value::as_array)? {
        let item_type = item.get("type").and_then(Value::as_str).unwrap_or_default();
        if matches!(item_type, "input_text" | "output_text" | "text") {
            if let Some(text) = item.get("text").and_then(Value::as_str) {
                let text = text.trim();
                if !text.is_empty() {
                    parts.push(text.to_string());
                }
            }
        }
    }
    (!parts.is_empty()).then(|| parts.join("\n"))
}
