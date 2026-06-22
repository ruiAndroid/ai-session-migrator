use crate::codex::backup::create_backup_dir;
use crate::codex::scan::config_provider;
use crate::codex::{CommandError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSwitchResult {
    pub configured_provider: String,
    pub previous_provider: Option<String>,
    pub config_backup_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRestartRequest {
    pub codex_home: String,
    pub target_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRestartResult {
    pub configured_provider: String,
    pub previous_provider: Option<String>,
    pub config_backup_dir: Option<String>,
    pub restart_attempted: bool,
    pub restarted: bool,
    pub restart_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RestartAttempt {
    attempted: bool,
    restarted: bool,
    message: String,
}

impl RestartAttempt {
    fn success(message: impl Into<String>) -> Self {
        Self {
            attempted: true,
            restarted: true,
            message: message.into(),
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self {
            attempted: true,
            restarted: false,
            message: message.into(),
        }
    }

    fn skipped(message: impl Into<String>) -> Self {
        Self {
            attempted: false,
            restarted: false,
            message: message.into(),
        }
    }
}

pub fn switch_provider_and_restart(
    request: ProviderRestartRequest,
) -> Result<ProviderRestartResult> {
    switch_provider_and_restart_with(request, restart_codex_desktop)
}

fn switch_provider_and_restart_with(
    request: ProviderRestartRequest,
    restart: impl FnOnce() -> RestartAttempt,
) -> Result<ProviderRestartResult> {
    let codex_home = Path::new(&request.codex_home);
    if !codex_home.exists() {
        return Err(CommandError::new(
            "codex_home_missing",
            format!("Codex home does not exist: {}", codex_home.display()),
        ));
    }
    let switch_result = switch_config_provider(codex_home, &request.target_provider)?;
    let restart = restart();
    Ok(ProviderRestartResult {
        configured_provider: switch_result.configured_provider,
        previous_provider: switch_result.previous_provider,
        config_backup_dir: switch_result.config_backup_dir,
        restart_attempted: restart.attempted,
        restarted: restart.restarted,
        restart_message: restart.message,
    })
}

pub fn switch_config_provider(
    codex_home: &Path,
    target_provider: &str,
) -> Result<ProviderSwitchResult> {
    let target_provider = target_provider.trim();
    if target_provider.is_empty() {
        return Err(CommandError::new(
            "target_provider_required",
            "Target provider is required.",
        ));
    }

    let config_path = codex_home.join("config.toml");
    let previous_provider = config_provider(codex_home)?;
    let existing_config = if config_path.exists() {
        Some(
            fs::read_to_string(&config_path)
                .map_err(|error| CommandError::io("read config", config_path.display(), error))?,
        )
    } else {
        None
    };
    let config_backup_dir = if config_path.exists() {
        Some(
            create_backup_dir(codex_home, std::slice::from_ref(&config_path))?
                .display()
                .to_string(),
        )
    } else {
        None
    };

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| CommandError::io("create config parent", parent.display(), error))?;
    }
    let next_config = replace_or_append_provider(existing_config.as_deref(), target_provider);
    fs::write(&config_path, next_config)
        .map_err(|error| CommandError::io("write config", config_path.display(), error))?;

    Ok(ProviderSwitchResult {
        configured_provider: target_provider.to_string(),
        previous_provider,
        config_backup_dir,
    })
}

fn replace_or_append_provider(existing_config: Option<&str>, target_provider: &str) -> String {
    let provider_line = format!(
        "model_provider = \"{}\"",
        escape_toml_string(target_provider)
    );
    let Some(text) = existing_config else {
        return format!("{provider_line}\n");
    };
    let mut replaced = false;
    let mut output = String::new();
    for line in text.split_inclusive('\n') {
        let (content, newline) = line
            .strip_suffix('\n')
            .map_or((line, ""), |content| (content, "\n"));
        if !replaced && is_model_provider_line(content) {
            output.push_str(&replace_provider_line(content, &provider_line));
            output.push_str(newline);
            replaced = true;
        } else {
            output.push_str(content);
            output.push_str(newline);
        }
    }
    if !replaced {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&provider_line);
        output.push('\n');
    }
    output
}

fn replace_provider_line(line: &str, provider_line: &str) -> String {
    let Some(comment_index) = line.find('#') else {
        return provider_line.to_string();
    };
    format!("{provider_line} {}", line[comment_index..].trim_start())
}

fn is_model_provider_line(line: &str) -> bool {
    let active = line.split('#').next().unwrap_or_default().trim();
    active
        .split_once('=')
        .is_some_and(|(key, _)| key.trim() == "model_provider")
}

fn escape_toml_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            character => vec![character],
        })
        .collect()
}

fn restart_codex_desktop() -> RestartAttempt {
    if !cfg!(target_os = "windows") {
        return RestartAttempt::skipped("当前系统暂不支持自动重启 Codex，请手动重启。");
    }

    let close_status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            r#"
$roots = Get-CimInstance Win32_Process |
  Where-Object {
    $_.Name -ieq 'Codex.exe' -and
    $_.ExecutablePath -like '*OpenAI.Codex_*\app\Codex.exe' -and
    -not ($_.CommandLine -match '--type=')
  }
foreach ($process in $roots) {
  $desktopProcess = Get-Process -Id $process.ProcessId -ErrorAction SilentlyContinue
  if ($desktopProcess) {
    [void] $desktopProcess.CloseMainWindow()
  }
}
$deadline = (Get-Date).AddSeconds(5)
do {
  Start-Sleep -Milliseconds 250
  $remaining = @($roots | Where-Object { Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue })
} while ($remaining.Count -gt 0 -and (Get-Date) -lt $deadline)
foreach ($process in $remaining) {
  Stop-Process -Id $process.ProcessId -ErrorAction SilentlyContinue
}
Start-Sleep -Milliseconds 700
"#,
        ])
        .status();
    if let Err(error) = close_status {
        return RestartAttempt::failure(format!("关闭 Codex 失败：{error}"));
    }

    let launch_status = Command::new("explorer")
        .arg("shell:AppsFolder\\OpenAI.Codex_2p2nqsd0c76g0!App")
        .status()
        .or_else(|_| {
            Command::new("cmd")
                .args(["/C", "start", "", "com.openai.codex:"])
                .status()
        });
    match launch_status {
        Ok(status) if status.success() => {
            RestartAttempt::success("Codex 已按新 provider 配置重新启动。")
        }
        Ok(status) => RestartAttempt::failure(format!(
            "Codex 配置已切换，但重启入口返回状态码：{status}。请手动重启 Codex。"
        )),
        Err(error) => RestartAttempt::failure(format!(
            "Codex 配置已切换，但自动重启失败：{error}。请手动重启 Codex。"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn switch_config_provider_replaces_existing_provider_and_backs_up_config() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();
        fs::write(
            codex.join("config.toml"),
            "model_provider = \"funai\" # active provider\nmodel = \"gpt-5\"\n",
        )
        .unwrap();

        let result = switch_config_provider(&codex, "yihubangg").unwrap();

        assert_eq!(result.previous_provider.as_deref(), Some("funai"));
        assert_eq!(result.configured_provider, "yihubangg");
        let text = fs::read_to_string(codex.join("config.toml")).unwrap();
        assert_eq!(
            text,
            "model_provider = \"yihubangg\" # active provider\nmodel = \"gpt-5\"\n"
        );
        let backup_dir = result.config_backup_dir.unwrap();
        assert_eq!(
            fs::read_to_string(Path::new(&backup_dir).join("config.toml")).unwrap(),
            "model_provider = \"funai\" # active provider\nmodel = \"gpt-5\"\n"
        );
    }

    #[test]
    fn switch_config_provider_appends_provider_when_config_has_no_provider() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();
        fs::write(codex.join("config.toml"), "model = \"gpt-5\"\n").unwrap();

        let result = switch_config_provider(&codex, "funai").unwrap();

        assert_eq!(result.previous_provider, None);
        assert_eq!(
            fs::read_to_string(codex.join("config.toml")).unwrap(),
            "model = \"gpt-5\"\nmodel_provider = \"funai\"\n"
        );
        assert!(result.config_backup_dir.is_some());
    }

    #[test]
    fn switch_config_provider_creates_config_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();

        let result = switch_config_provider(&codex, "custom-provider").unwrap();

        assert_eq!(result.previous_provider, None);
        assert_eq!(
            fs::read_to_string(codex.join("config.toml")).unwrap(),
            "model_provider = \"custom-provider\"\n"
        );
        assert_eq!(result.config_backup_dir, None);
    }

    #[test]
    fn switch_config_provider_escapes_toml_string_values() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();
        fs::write(codex.join("config.toml"), "").unwrap();

        switch_config_provider(&codex, "provider\\\"quoted").unwrap();

        assert_eq!(
            fs::read_to_string(codex.join("config.toml")).unwrap(),
            "model_provider = \"provider\\\\\\\"quoted\"\n"
        );
    }

    #[test]
    fn switch_config_provider_rejects_empty_provider() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();

        let error = switch_config_provider(&codex, "  ").unwrap_err();

        assert_eq!(error.code, "target_provider_required");
    }

    #[test]
    fn switch_provider_and_restart_reports_successful_restart() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();
        fs::write(codex.join("config.toml"), "model_provider = \"funai\"\n").unwrap();

        let result = switch_provider_and_restart_with(
            ProviderRestartRequest {
                codex_home: codex.display().to_string(),
                target_provider: "yihubangg".to_string(),
            },
            || RestartAttempt::success("Codex 已重启。"),
        )
        .unwrap();

        assert_eq!(result.previous_provider.as_deref(), Some("funai"));
        assert_eq!(result.configured_provider, "yihubangg");
        assert!(result.config_backup_dir.is_some());
        assert!(result.restart_attempted);
        assert!(result.restarted);
        assert_eq!(result.restart_message, "Codex 已重启。");
        assert_eq!(
            fs::read_to_string(codex.join("config.toml")).unwrap(),
            "model_provider = \"yihubangg\"\n"
        );
    }

    #[test]
    fn switch_provider_and_restart_keeps_config_when_restart_fails() {
        let temp = tempfile::tempdir().unwrap();
        let codex = temp.path().join(".codex");
        fs::create_dir_all(&codex).unwrap();
        fs::write(codex.join("config.toml"), "model_provider = \"funai\"\n").unwrap();

        let result = switch_provider_and_restart_with(
            ProviderRestartRequest {
                codex_home: codex.display().to_string(),
                target_provider: "gmn".to_string(),
            },
            || RestartAttempt::failure("未找到 Codex 启动入口。"),
        )
        .unwrap();

        assert_eq!(result.configured_provider, "gmn");
        assert!(result.restart_attempted);
        assert!(!result.restarted);
        assert_eq!(result.restart_message, "未找到 Codex 启动入口。");
        assert_eq!(
            fs::read_to_string(codex.join("config.toml")).unwrap(),
            "model_provider = \"gmn\"\n"
        );
    }
}
