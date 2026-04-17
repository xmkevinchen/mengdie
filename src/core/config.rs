use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct MengdieConfig {
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub timeout_secs: u64,
    pub claude_cli: ClaudeCliConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ClaudeCliConfig {
    pub cli_path: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "claude-cli".to_string(),
            model: "claude-sonnet-4-6".to_string(),
            timeout_secs: 120,
            claude_cli: ClaudeCliConfig::default(),
        }
    }
}

impl Default for ClaudeCliConfig {
    fn default() -> Self {
        Self {
            cli_path: "claude".to_string(),
        }
    }
}

impl MengdieConfig {
    pub fn default_path() -> PathBuf {
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mengdie")
            .join("config.toml")
    }

    pub fn load() -> anyhow::Result<Self> {
        Self::load_from_path(&Self::default_path())
    }

    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        toml::from_str(&text)
            .with_context(|| format!("failed to parse TOML config at {}", path.display()))
    }

    pub fn load_with_env(mut base: Self, env: &HashMap<String, String>) -> anyhow::Result<Self> {
        if let Some(v) = env.get("MENGDIE_LLM_PROVIDER") {
            base.llm.provider = v.clone();
        }
        if let Some(v) = env.get("MENGDIE_LLM_MODEL") {
            base.llm.model = v.clone();
        }
        if let Some(v) = env.get("MENGDIE_LLM_TIMEOUT_SECS") {
            base.llm.timeout_secs = v.parse().with_context(|| {
                format!("invalid MENGDIE_LLM_TIMEOUT_SECS value: {v:?} (expected unsigned integer)")
            })?;
        }
        if let Some(v) = env.get("MENGDIE_LLM_CLAUDE_CLI_PATH") {
            base.llm.claude_cli.cli_path = v.clone();
        }
        Ok(base)
    }

    pub fn load_from_process_env() -> anyhow::Result<Self> {
        let base = Self::load()?;
        let env: HashMap<String, String> = std::env::vars().collect();
        Self::load_with_env(base, &env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp_config(dir: &std::path::Path, body: &str) -> PathBuf {
        let path = dir.join("config.toml");
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn defaults_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does-not-exist.toml");
        let cfg = MengdieConfig::load_from_path(&path).unwrap();
        assert_eq!(cfg, MengdieConfig::default());
        assert_eq!(cfg.llm.provider, "claude-cli");
        assert_eq!(cfg.llm.model, "claude-sonnet-4-6");
        assert_eq!(cfg.llm.timeout_secs, 120);
        assert_eq!(cfg.llm.claude_cli.cli_path, "claude");
    }

    #[test]
    fn partial_llm_section_keeps_other_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_tmp_config(
            tmp.path(),
            r#"
[llm]
model = "claude-haiku-4-5"
"#,
        );
        let cfg = MengdieConfig::load_from_path(&path).unwrap();
        assert_eq!(cfg.llm.model, "claude-haiku-4-5");
        assert_eq!(cfg.llm.provider, "claude-cli");
        assert_eq!(cfg.llm.timeout_secs, 120);
        assert_eq!(cfg.llm.claude_cli.cli_path, "claude");
    }

    #[test]
    fn nested_claude_cli_section_applies() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_tmp_config(
            tmp.path(),
            r#"
[llm.claude_cli]
cli_path = "/opt/claude/bin/claude"
"#,
        );
        let cfg = MengdieConfig::load_from_path(&path).unwrap();
        assert_eq!(cfg.llm.claude_cli.cli_path, "/opt/claude/bin/claude");
        assert_eq!(cfg.llm.provider, "claude-cli");
        assert_eq!(cfg.llm.model, "claude-sonnet-4-6");
    }

    #[test]
    fn env_map_overrides_generic_fields() {
        let mut env = HashMap::new();
        env.insert("MENGDIE_LLM_MODEL".to_string(), "foo-model".to_string());
        env.insert("MENGDIE_LLM_TIMEOUT_SECS".to_string(), "45".to_string());
        let cfg = MengdieConfig::load_with_env(MengdieConfig::default(), &env).unwrap();
        assert_eq!(cfg.llm.model, "foo-model");
        assert_eq!(cfg.llm.timeout_secs, 45);
        assert_eq!(cfg.llm.provider, "claude-cli");
        assert_eq!(cfg.llm.claude_cli.cli_path, "claude");
    }

    #[test]
    fn env_map_overrides_nested_cli_path() {
        let mut env = HashMap::new();
        env.insert(
            "MENGDIE_LLM_CLAUDE_CLI_PATH".to_string(),
            "/x/bin/claude".to_string(),
        );
        let cfg = MengdieConfig::load_with_env(MengdieConfig::default(), &env).unwrap();
        assert_eq!(cfg.llm.claude_cli.cli_path, "/x/bin/claude");
    }

    #[test]
    fn env_wins_over_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_tmp_config(
            tmp.path(),
            r#"
[llm]
model = "file-model"
"#,
        );
        let file_cfg = MengdieConfig::load_from_path(&path).unwrap();
        let mut env = HashMap::new();
        env.insert("MENGDIE_LLM_MODEL".to_string(), "env-model".to_string());
        let cfg = MengdieConfig::load_with_env(file_cfg, &env).unwrap();
        assert_eq!(cfg.llm.model, "env-model");
    }

    #[test]
    fn malformed_timeout_env_errors_with_value() {
        let mut env = HashMap::new();
        env.insert(
            "MENGDIE_LLM_TIMEOUT_SECS".to_string(),
            "not-a-number".to_string(),
        );
        let err = MengdieConfig::load_with_env(MengdieConfig::default(), &env).unwrap_err();
        let display = format!("{err:#}");
        assert!(
            display.contains("MENGDIE_LLM_TIMEOUT_SECS"),
            "err: {display}"
        );
        assert!(display.contains("not-a-number"), "err: {display}");
    }

    #[test]
    fn malformed_toml_error_mentions_path() {
        let tmp = tempfile::tempdir().unwrap();
        let path = write_tmp_config(tmp.path(), "not = valid = toml = [[");
        let err = MengdieConfig::load_from_path(&path).unwrap_err();
        let display = format!("{err:#}");
        assert!(
            display.contains(&path.display().to_string()),
            "expected path in error, got: {display}"
        );
    }
}
