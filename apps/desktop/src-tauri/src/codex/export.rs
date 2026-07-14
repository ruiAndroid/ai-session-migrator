use super::{CommandError, Result, SessionExportRequest, SessionExportResult};
use std::path::{Path, PathBuf};

pub fn export_session(request: SessionExportRequest) -> Result<SessionExportResult> {
    let codex_home = canonicalize_codex_home(&request.codex_home)?;
    let source = canonicalize_source(&request.source_path)?;
    validate_source_root(&codex_home, &source)?;
    validate_jsonl(&source, "session_export_source_format_invalid")?;

    let destination = PathBuf::from(&request.destination_path);
    if !destination.is_absolute() {
        return Err(CommandError::new(
            "session_export_destination_invalid",
            "export destination must be an absolute path",
        ));
    }
    validate_jsonl(&destination, "session_export_destination_format_invalid")?;
    reject_same_file(&source, &destination)?;

    let bytes_written = std::fs::copy(&source, &destination).map_err(|error| {
        CommandError::new(
            "session_export_write_failed",
            format!(
                "failed to export session to {}: {error}",
                destination.display()
            ),
        )
    })?;

    Ok(SessionExportResult {
        thread_id: request.thread_id,
        destination_path: destination.display().to_string(),
        bytes_written,
    })
}

fn canonicalize_codex_home(value: &str) -> Result<PathBuf> {
    Path::new(value).canonicalize().map_err(|error| {
        CommandError::new(
            "codex_home_missing",
            format!("Codex Home is unavailable at {value}: {error}"),
        )
    })
}

fn canonicalize_source(value: &str) -> Result<PathBuf> {
    let path = Path::new(value);
    let canonical = path.canonicalize().map_err(|error| {
        let code = if error.kind() == std::io::ErrorKind::NotFound {
            "session_export_source_missing"
        } else {
            "session_export_source_invalid"
        };
        CommandError::new(
            code,
            format!("session source is unavailable at {value}: {error}"),
        )
    })?;
    if !canonical.is_file() {
        return Err(CommandError::new(
            "session_export_source_invalid",
            format!("session source is not a file: {}", canonical.display()),
        ));
    }
    Ok(canonical)
}

fn validate_source_root(codex_home: &Path, source: &Path) -> Result<()> {
    let allowed = ["sessions", "archived_sessions"]
        .into_iter()
        .filter_map(|name| codex_home.join(name).canonicalize().ok())
        .any(|root| source.starts_with(root));
    if allowed {
        Ok(())
    } else {
        Err(CommandError::new(
            "session_export_source_outside_codex_home",
            "session source is outside the selected Codex Home",
        ))
    }
}

fn validate_jsonl(path: &Path, code: &str) -> Result<()> {
    let is_jsonl = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("jsonl"));
    is_jsonl.then_some(()).ok_or_else(|| {
        CommandError::new(
            code,
            format!("path must use the .jsonl extension: {}", path.display()),
        )
    })
}

fn reject_same_file(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        if let Ok(canonical) = destination.canonicalize() {
            if canonical == source {
                return Err(CommandError::new(
                    "session_export_source_destination_same",
                    "source and destination must be different files",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::codex::{export::export_session, SessionExportRequest};
    use std::path::Path;

    fn request(codex_home: &Path, source: &Path, destination: &Path) -> SessionExportRequest {
        SessionExportRequest {
            codex_home: codex_home.display().to_string(),
            thread_id: "thread-a".to_string(),
            source_path: source.display().to_string(),
            destination_path: destination.display().to_string(),
        }
    }

    fn write_source(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn exports_active_session_byte_for_byte() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        let source = codex_home.join("sessions/2026/07/14/rollout-a.jsonl");
        let destination = temp.path().join("exported.jsonl");
        let bytes = [b'{', 0xff, 0x00, b'}', b'\n'];
        write_source(&source, &bytes);

        let result = export_session(request(&codex_home, &source, &destination)).unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        assert_eq!(result.thread_id, "thread-a");
        assert_eq!(result.destination_path, destination.display().to_string());
        assert_eq!(result.bytes_written, bytes.len() as u64);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
    }

    #[test]
    fn exports_archived_session() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        let source = codex_home.join("archived_sessions/rollout-b.jsonl");
        let destination = temp.path().join("archived-export.jsonl");
        write_source(&source, b"archived session\n");

        export_session(request(&codex_home, &source, &destination)).unwrap();

        assert_eq!(std::fs::read(&destination).unwrap(), b"archived session\n");
    }

    #[test]
    fn rejects_source_outside_selected_codex_home() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        std::fs::create_dir_all(codex_home.join("sessions")).unwrap();
        let source = temp.path().join("outside.jsonl");
        let destination = temp.path().join("exported.jsonl");
        write_source(&source, b"outside\n");

        let error = export_session(request(&codex_home, &source, &destination)).unwrap_err();

        assert_eq!(error.code, "session_export_source_outside_codex_home");
        assert!(!destination.exists());
    }

    #[test]
    fn rejects_non_jsonl_source() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        let source = codex_home.join("sessions/rollout-a.txt");
        let destination = temp.path().join("exported.jsonl");
        write_source(&source, b"not jsonl\n");

        let error = export_session(request(&codex_home, &source, &destination)).unwrap_err();

        assert_eq!(error.code, "session_export_source_format_invalid");
        assert!(!destination.exists());
    }

    #[test]
    fn rejects_non_jsonl_destination() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        let source = codex_home.join("sessions/rollout-a.jsonl");
        let destination = temp.path().join("exported.txt");
        write_source(&source, b"session\n");

        let error = export_session(request(&codex_home, &source, &destination)).unwrap_err();

        assert_eq!(error.code, "session_export_destination_format_invalid");
        assert!(!destination.exists());
    }

    #[test]
    fn reports_missing_source_after_scan() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        std::fs::create_dir_all(codex_home.join("sessions")).unwrap();
        let source = codex_home.join("sessions/missing.jsonl");
        let destination = temp.path().join("exported.jsonl");

        let error = export_session(request(&codex_home, &source, &destination)).unwrap_err();

        assert_eq!(error.code, "session_export_source_missing");
        assert!(!destination.exists());
    }

    #[test]
    fn rejects_source_as_destination() {
        let temp = tempfile::tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        let source = codex_home.join("sessions/rollout-a.jsonl");
        write_source(&source, b"original\n");

        let error = export_session(request(&codex_home, &source, &source)).unwrap_err();

        assert_eq!(error.code, "session_export_source_destination_same");
        assert_eq!(std::fs::read(&source).unwrap(), b"original\n");
    }
}
