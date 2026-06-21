use crate::codex::{CommandError, Result};
use chrono::Local;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn create_backup_dir(codex_home: &Path, files: &[PathBuf]) -> Result<PathBuf> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_dir = create_unique_backup_dir(codex_home, &stamp)?;

    let mut seen = BTreeSet::new();
    for path in files {
        if !path.exists() || !seen.insert(path.clone()) {
            continue;
        }
        let fallback_name = path.file_name().ok_or_else(|| {
            CommandError::new(
                "invalid_backup_file",
                format!("Cannot back up path without file name: {}", path.display()),
            )
        })?;
        let relative_path = path
            .strip_prefix(codex_home)
            .unwrap_or_else(|_| Path::new(fallback_name));
        let backup_path = backup_dir.join(relative_path);
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                CommandError::io("create backup parent", parent.display(), error)
            })?;
        }
        fs::copy(path, &backup_path)
            .map_err(|error| CommandError::io("copy backup file", path.display(), error))?;
    }
    Ok(backup_dir)
}

fn create_unique_backup_dir(codex_home: &Path, stamp: &str) -> Result<PathBuf> {
    fs::create_dir_all(codex_home)
        .map_err(|error| CommandError::io("create Codex home", codex_home.display(), error))?;
    for attempt in 0..100 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("-{attempt:02}")
        };
        let backup_dir = codex_home.join(format!("ai-session-migrator-backup-{stamp}{suffix}"));
        match fs::create_dir(&backup_dir) {
            Ok(()) => return Ok(backup_dir),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(CommandError::io(
                    "create backup directory",
                    backup_dir.display(),
                    error,
                ))
            }
        }
    }
    Err(CommandError::new(
        "backup_directory_collision",
        format!(
            "Could not create a unique backup directory under {}",
            codex_home.display()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_preserves_relative_paths_for_duplicate_file_names() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let root_db = codex.join("state_5.sqlite");
        let nested_db = codex.join("sqlite/state_5.sqlite");
        fs::create_dir_all(nested_db.parent().unwrap()).unwrap();
        fs::write(&root_db, "root db").unwrap();
        fs::write(&nested_db, "nested db").unwrap();

        let backup = create_backup_dir(&codex, &[root_db, nested_db]).unwrap();

        assert_eq!(
            fs::read_to_string(backup.join("state_5.sqlite")).unwrap(),
            "root db"
        );
        assert_eq!(
            fs::read_to_string(backup.join("sqlite/state_5.sqlite")).unwrap(),
            "nested db"
        );
    }

    #[test]
    fn backup_uses_a_new_directory_when_timestamp_collides() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        let rollout = codex.join("sessions/rollout-a.jsonl");
        fs::create_dir_all(rollout.parent().unwrap()).unwrap();
        fs::write(&rollout, "first").unwrap();

        let first = create_backup_dir(&codex, std::slice::from_ref(&rollout)).unwrap();
        let second = create_backup_dir(&codex, std::slice::from_ref(&rollout)).unwrap();

        assert_ne!(first, second);
        assert!(first.exists());
        assert!(second.exists());
        assert_eq!(
            fs::read_to_string(second.join("sessions/rollout-a.jsonl")).unwrap(),
            "first"
        );
    }
}
