use crate::codex::metadata::BOM;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

pub fn write_jsonl(path: &Path, thread_id: &str, provider: &str, bom: bool, message: &str) {
    let data = format!(
        "{{\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"{thread_id}\",\"timestamp\":\"2026-06-15T01:00:00.000Z\",\"cwd\":\"D:\\\\work\",\"source\":\"vscode\",\"model_provider\":\"{provider}\",\"cli_version\":\"0.140.0\",\"thread_source\":\"user\"}}}}\n{{\"timestamp\":\"2026-06-15T01:01:00.000Z\",\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"{message}\"}}]}}}}\n"
    );
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut raw = Vec::new();
    if bom {
        raw.extend(BOM);
    }
    raw.extend(data.as_bytes());
    fs::write(path, raw).unwrap();
}

pub fn init_state_db(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "
            create table threads (
                id text primary key,
                rollout_path text not null,
                created_at integer not null,
                updated_at integer not null,
                source text not null,
                model_provider text not null,
                cwd text not null,
                title text not null,
                sandbox_policy text not null,
                approval_mode text not null,
                tokens_used integer not null default 0,
                has_user_event integer not null default 0,
                archived integer not null default 0,
                archived_at integer,
                git_sha text,
                git_branch text,
                git_origin_url text,
                cli_version text not null default '',
                first_user_message text not null default '',
                agent_nickname text,
                agent_role text,
                memory_mode text not null default 'enabled',
                model text,
                reasoning_effort text,
                agent_path text,
                created_at_ms integer,
                updated_at_ms integer,
                thread_source text,
                preview text not null default ''
            );
            ",
        )
        .unwrap();
}

pub fn insert_state_row(path: &Path, thread_id: &str, provider: &str, archived: i32) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute(
            "insert into threads (
                id, rollout_path, created_at, updated_at, source, model_provider, cwd, title,
                sandbox_policy, approval_mode, tokens_used, has_user_event, archived, archived_at,
                cli_version, first_user_message, memory_mode, created_at_ms, updated_at_ms, preview
             ) values (?1, '', 0, 0, 'vscode', ?2, '', 'old title', '{}', 'never', 0, 1, ?3, NULL, '', '', 'enabled', 0, 0, '')",
            (thread_id, provider, archived),
        )
        .unwrap();
}
