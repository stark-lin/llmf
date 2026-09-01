use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::cli::validate_prompt_name;
use crate::{
    EXIT_CONFIG, EXIT_INTERNAL, EXIT_OPERATION_FAILED, EXIT_PROMPT_NOT_FOUND, LlmfError, Result,
};

const CONFIG_TEMPLATE: &str = r#"{
  "api_url": "http://127.0.0.1:1234/v1/chat/completions",
  "api_key": "",
  "model": "qwen/qwen3.5-35b-a3b",
  "request_timeout_seconds": 120
}
"#;

const SYSTEM_PROMPT: &str = include_str!("defaults/system_prompt.md");

const PROMPTS: &[(&str, &str)] = &[
    ("analyze.md", include_str!("defaults/analyze.md")),
    ("clean-web.md", include_str!("defaults/clean-web.md")),
    (
        "commit-message.md",
        include_str!("defaults/commit-message.md"),
    ),
    (
        "compare-options.md",
        include_str!("defaults/compare-options.md"),
    ),
    (
        "customer-brief.md",
        include_str!("defaults/customer-brief.md"),
    ),
    (
        "decision-record.md",
        include_str!("defaults/decision-record.md"),
    ),
    (
        "extract-failures.md",
        include_str!("defaults/extract-failures.md"),
    ),
    ("handoff.md", include_str!("defaults/handoff.md")),
    ("implement.md", include_str!("defaults/implement.md")),
    (
        "implementation-plan.md",
        include_str!("defaults/implementation-plan.md"),
    ),
    ("review.md", include_str!("defaults/review.md")),
    ("review-diff.md", include_str!("defaults/review-diff.md")),
    (
        "safe-to-merge.md",
        include_str!("defaults/safe-to-merge.md"),
    ),
    ("summarize.md", include_str!("defaults/summarize.md")),
    (
        "translatetoen.md",
        include_str!("defaults/translatetoen.md"),
    ),
    (
        "translatetojp.md",
        include_str!("defaults/translatetojp.md"),
    ),
    (
        "translatetozh.md",
        include_str!("defaults/translatetozh.md"),
    ),
];

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct Workspaces {
    pub project: Option<PathBuf>,
    pub user: PathBuf,
    pub user_exists: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedFile {
    pub contents: String,
}

pub fn discover(cwd: &Path, home: &Path) -> Result<Workspaces> {
    let project = find_project_workspace(cwd)?;
    let user = home.join(".llmf");
    let user_exists = existing_directory(&user, "user workspace")?;
    Ok(Workspaces {
        project,
        user,
        user_exists,
    })
}

pub fn find_project_workspace(cwd: &Path) -> Result<Option<PathBuf>> {
    for dir in cwd.ancestors() {
        let candidate = dir.join(".llmf");
        match fs::metadata(&candidate) {
            Ok(meta) if meta.is_dir() => return Ok(Some(candidate)),
            Ok(_) => {
                return Err(LlmfError::new(
                    EXIT_INTERNAL,
                    format!("workspace is not a directory: {}", candidate.display()),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(LlmfError::new(
                    EXIT_INTERNAL,
                    format!("cannot access workspace {}: {error}", candidate.display()),
                ));
            }
        }
    }
    Ok(None)
}

fn existing_directory(path: &Path, label: &str) -> Result<bool> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(true),
        Ok(_) => Err(LlmfError::new(
            EXIT_INTERNAL,
            format!("{label} is not a directory: {}", path.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(LlmfError::new(
            EXIT_INTERNAL,
            format!("cannot access {label} {}: {error}", path.display()),
        )),
    }
}

pub fn init_project(cwd: &Path) -> Result<PathBuf> {
    let workspace = cwd.join(".llmf");
    if path_exists(&workspace)? {
        return Err(LlmfError::new(
            EXIT_OPERATION_FAILED,
            ".llmf already exists",
        ));
    }

    create_workspace(&workspace)?;
    Ok(workspace)
}

pub fn init_user(home: &Path) -> Result<PathBuf> {
    let workspace = home.join(".llmf");
    if path_exists(&workspace)? {
        return Err(LlmfError::new(
            EXIT_OPERATION_FAILED,
            format!("{} already exists", workspace.display()),
        ));
    }
    create_workspace(&workspace)?;
    Ok(workspace)
}

fn create_workspace(workspace: &Path) -> Result<()> {
    let prompts = workspace.join("prompts");
    fs::create_dir_all(&prompts).map_err(|error| {
        LlmfError::internal(format!(
            "cannot create workspace {}: {error}",
            workspace.display()
        ))
    })?;
    write_new(&workspace.join("config.json"), CONFIG_TEMPLATE.as_bytes())?;
    write_new(
        &workspace.join("system_prompt.md"),
        SYSTEM_PROMPT.as_bytes(),
    )?;
    for (name, contents) in PROMPTS {
        write_new(&prompts.join(name), contents.as_bytes())?;
    }
    Ok(())
}

fn write_new(path: &Path, contents: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            LlmfError::internal(format!("cannot create {}: {error}", path.display()))
        })?;
    file.write_all(contents)
        .map_err(|error| LlmfError::internal(format!("cannot write {}: {error}", path.display())))
}

fn path_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(LlmfError::internal(format!(
            "cannot access {}: {error}",
            path.display()
        ))),
    }
}

pub fn load_config(workspaces: &Workspaces) -> Result<Config> {
    let candidates = workspaces
        .project
        .iter()
        .map(|path| path.join("config.json"))
        .chain(std::iter::once(workspaces.user.join("config.json")));
    for path in candidates {
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(invalid_config(format!(
                    "cannot read {}: {error}",
                    path.display()
                )));
            }
        };
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| invalid_config(format!("{} is not UTF-8", path.display())))?;
        let config = serde_json::from_str(text)
            .map_err(|error| invalid_config(clean_json_error(error.to_string())))?;
        validate_config(&config)?;
        return Ok(config);
    }
    Err(invalid_config("config.json not found"))
}

fn validate_config(config: &Config) -> Result<()> {
    if config.api_url.trim().is_empty() {
        return Err(invalid_config("api_url must not be empty"));
    }
    let url =
        Url::parse(&config.api_url).map_err(|_| invalid_config("api_url must be a valid URL"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(invalid_config("api_url must be an http or https URL"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(invalid_config("api_url must not include credentials"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid_config(
            "api_url must not include a query or fragment",
        ));
    }
    if config.model.trim().is_empty() {
        return Err(invalid_config("model must not be empty"));
    }
    if !(1..=3600).contains(&config.request_timeout_seconds) {
        return Err(invalid_config(
            "request_timeout_seconds must be between 1 and 3600",
        ));
    }
    Ok(())
}

fn invalid_config(message: impl Into<String>) -> LlmfError {
    LlmfError::new(EXIT_CONFIG, format!("invalid config: {}", message.into()))
}

fn clean_json_error(message: String) -> String {
    if message.starts_with("duplicate field") {
        "duplicate field".into()
    } else if message.starts_with("unknown field") {
        "unknown field".into()
    } else if message.starts_with("missing field") {
        "missing required field".into()
    } else if message.starts_with("invalid type") {
        "invalid field type".into()
    } else {
        "invalid JSON".into()
    }
}

pub fn resolve_system_prompt(workspaces: &Workspaces) -> Result<ResolvedFile> {
    let candidates = workspaces
        .project
        .iter()
        .map(|path| path.join("system_prompt.md"))
        .chain(std::iter::once(workspaces.user.join("system_prompt.md")));
    for path in candidates {
        match fs::read(&path) {
            Ok(bytes) => {
                let contents = String::from_utf8(bytes).map_err(|_| {
                    LlmfError::new(
                        EXIT_PROMPT_NOT_FOUND,
                        format!("system prompt is not UTF-8: {}", path.display()),
                    )
                })?;
                return Ok(ResolvedFile { contents });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(LlmfError::new(
                    EXIT_PROMPT_NOT_FOUND,
                    format!("cannot read system prompt {}: {error}", path.display()),
                ));
            }
        }
    }
    Err(LlmfError::new(
        EXIT_PROMPT_NOT_FOUND,
        "system prompt not found",
    ))
}

pub fn resolve_prompt(workspaces: &Workspaces, name: &str) -> Result<ResolvedFile> {
    let file_name = format!("{name}.md");
    let candidates = workspaces
        .project
        .iter()
        .map(|path| path.join("prompts").join(&file_name))
        .chain(std::iter::once(
            workspaces.user.join("prompts").join(&file_name),
        ));
    for path in candidates {
        match fs::read(&path) {
            Ok(bytes) => {
                let contents = String::from_utf8(bytes).map_err(|_| {
                    LlmfError::new(
                        EXIT_PROMPT_NOT_FOUND,
                        format!("prompt is not UTF-8: {}", path.display()),
                    )
                })?;
                return Ok(ResolvedFile { contents });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(LlmfError::new(
                    EXIT_PROMPT_NOT_FOUND,
                    format!("cannot read prompt {}: {error}", path.display()),
                ));
            }
        }
    }
    Err(LlmfError::new(
        EXIT_PROMPT_NOT_FOUND,
        format!("prompt not found: {name}"),
    ))
}

pub fn list_prompts(workspaces: &Workspaces) -> Result<Vec<String>> {
    let mut names = BTreeSet::new();
    let dirs = workspaces
        .project
        .iter()
        .map(|path| path.join("prompts"))
        .chain(std::iter::once(workspaces.user.join("prompts")));
    for dir in dirs {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(LlmfError::internal(format!(
                    "cannot list prompts in {}: {error}",
                    dir.display()
                )));
            }
        };
        for entry in entries {
            let entry = entry.map_err(|error| {
                LlmfError::internal(format!("cannot list prompts in {}: {error}", dir.display()))
            })?;
            let file_type = entry.file_type().map_err(|error| {
                LlmfError::internal(format!(
                    "cannot inspect {}: {error}",
                    entry.path().display()
                ))
            })?;
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|part| part.to_str()) != Some("md") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|part| part.to_str()) else {
                continue;
            };
            if validate_prompt_name(name).is_ok() {
                names.insert(name.to_owned());
            }
        }
    }
    Ok(names.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_defaults() {
        let temp = TempDir::new().unwrap();
        let workspace = init_project(temp.path()).unwrap();
        assert!(workspace.join("config.json").is_file());
        assert_eq!(
            fs::read_to_string(workspace.join("system_prompt.md")).unwrap(),
            SYSTEM_PROMPT
        );
        for (name, _) in PROMPTS {
            assert!(workspace.join("prompts").join(name).is_file());
        }
    }

    #[test]
    fn nearest_project_workspace_wins() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("a/b");
        fs::create_dir_all(temp.path().join(".llmf")).unwrap();
        fs::create_dir_all(nested.join(".llmf")).unwrap();
        fs::create_dir_all(nested.join("c")).unwrap();
        assert_eq!(
            find_project_workspace(&nested.join("c")).unwrap(),
            Some(nested.join(".llmf"))
        );
    }

    fn valid_json(extra: &str) -> String {
        format!(
            r#"{{
                "api_url":"http://localhost/v1/chat/completions",
                "api_key":"",
                "model":"test",
                "request_timeout_seconds":120{extra}
            }}"#
        )
    }

    #[test]
    fn config_rejects_unknown_duplicate_missing_and_wrong_type_fields() {
        assert!(
            serde_json::from_str::<Config>(&valid_json(",\"provider\":\"x\""))
                .unwrap_err()
                .to_string()
                .contains("unknown field")
        );
        assert!(
            serde_json::from_str::<Config>(&valid_json(",\"model\":\"again\""))
                .unwrap_err()
                .to_string()
                .contains("duplicate field")
        );
        assert!(
            serde_json::from_str::<Config>(
                r#"{"api_url":"http://localhost","api_key":"","request_timeout_seconds":120}"#,
            )
            .unwrap_err()
            .to_string()
            .contains("missing field")
        );
        assert!(
            serde_json::from_str::<Config>(&valid_json("").replace(
                "\"request_timeout_seconds\":120",
                "\"request_timeout_seconds\":2.5",
            ))
            .is_err()
        );
    }

    #[test]
    fn config_validates_content() {
        let mut config: Config = serde_json::from_str(&valid_json("")).unwrap();
        config.request_timeout_seconds = 3601;
        assert!(
            validate_config(&config)
                .unwrap_err()
                .message
                .contains("between 1 and 3600")
        );

        config.request_timeout_seconds = 120;
        config.model.clear();
        assert!(
            validate_config(&config)
                .unwrap_err()
                .message
                .contains("model must not be empty")
        );

        config.model = "test".into();
        config.api_url = "file:///tmp/socket".into();
        assert!(
            validate_config(&config)
                .unwrap_err()
                .message
                .contains("http or https")
        );

        let config: Config = serde_json::from_str(CONFIG_TEMPLATE).unwrap();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn config_rejects_whitespace_only_fields_and_sensitive_url_components() {
        let mut config: Config = serde_json::from_str(&valid_json("")).unwrap();
        config.model = " \t".into();
        assert!(
            validate_config(&config)
                .unwrap_err()
                .message
                .contains("model must not be empty")
        );

        config.model = "test".into();
        for url in [
            "https://user:password@example.test/v1/chat/completions",
            "https://example.test/v1/chat/completions?token=secret",
            "https://example.test/v1/chat/completions#secret",
        ] {
            config.api_url = url.into();
            assert!(validate_config(&config).is_err(), "{url}");
        }
    }

    #[test]
    fn json_parse_errors_have_stable_non_echoing_categories() {
        assert_eq!(
            clean_json_error("duplicate field `api_key` at line 1".into()),
            "duplicate field"
        );
        assert_eq!(
            clean_json_error("unknown field `token` at line 1".into()),
            "unknown field"
        );
        assert_eq!(
            clean_json_error("missing field `model` at line 1".into()),
            "missing required field"
        );
        assert_eq!(
            clean_json_error("invalid type: integer at line 1".into()),
            "invalid field type"
        );
        assert_eq!(
            clean_json_error("expected value at line 1".into()),
            "invalid JSON"
        );
    }
}
