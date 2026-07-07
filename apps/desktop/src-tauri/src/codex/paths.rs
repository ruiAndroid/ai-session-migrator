use std::path::Path;

pub fn visible_path_string(path: &Path) -> String {
    normalize_windows_extended_path(&path.display().to_string())
}

pub fn normalize_windows_extended_path(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
        return format!("\\\\{rest}");
    }
    if let Some(rest) = value.strip_prefix("\\\\?\\") {
        return rest.to_string();
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_windows_extended_drive_path() {
        assert_eq!(
            normalize_windows_extended_path(r"\\?\C:\Users\jianrui\.codex"),
            r"C:\Users\jianrui\.codex"
        );
    }

    #[test]
    fn normalizes_windows_extended_unc_path() {
        assert_eq!(
            normalize_windows_extended_path(r"\\?\UNC\server\share\.codex"),
            r"\\server\share\.codex"
        );
    }

    #[test]
    fn keeps_normal_path_unchanged() {
        assert_eq!(
            normalize_windows_extended_path(r"D:\dev\AI\AIPro\fun-claw"),
            r"D:\dev\AI\AIPro\fun-claw"
        );
    }
}
