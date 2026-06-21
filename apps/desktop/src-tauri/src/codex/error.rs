use serde::Serialize;
use std::fmt;
use std::io;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            backup_dir: None,
            operation: None,
        }
    }

    pub fn io(action: &str, path: impl fmt::Display, source: io::Error) -> Self {
        Self::new("io_error", format!("{action}: {path} ({source})"))
    }

    pub fn post_backup(
        backup_dir: impl fmt::Display,
        operation: impl Into<String>,
        source: Self,
    ) -> Self {
        let operation = operation.into();
        let backup_dir = backup_dir.to_string();
        Self {
            code: "post_backup_write_failed".to_string(),
            message: format!(
                "{operation} failed after backup was created. Backup directory: {backup_dir}. Cause: {}",
                source.message
            ),
            backup_dir: Some(backup_dir),
            operation: Some(operation),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CommandError {}

pub type Result<T> = std::result::Result<T, CommandError>;
