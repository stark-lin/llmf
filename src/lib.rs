use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::io::Read;
use std::path::{Path, PathBuf};

mod api;
pub mod cli;
mod interrupt;
mod workspace;

pub const EXIT_OPERATION_FAILED: u8 = 1;
pub const EXIT_USAGE: u8 = 64;
pub const EXIT_PROMPT_NOT_FOUND: u8 = 66;
pub const EXIT_INTERNAL: u8 = 70;
pub const EXIT_API: u8 = 74;
pub const EXIT_CONFIG: u8 = 78;
pub const EXIT_INTERRUPTED: u8 = 130;

#[derive(Debug, Clone)]
pub struct LlmfError {
    pub code: u8,
    pub message: String,
}

impl LlmfError {
    pub fn new(code: u8, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into().replace(['\r', '\n'], " "),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(EXIT_INTERNAL, message)
    }
}

impl Display for LlmfError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LlmfError {}

pub type Result<T> = std::result::Result<T, LlmfError>;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AppOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl AppOutput {
    fn stdout(value: impl Into<Vec<u8>>) -> Self {
        Self {
            stdout: value.into(),
            stderr: Vec::new(),
        }
    }
}

pub fn execute<R: Read>(
    args: impl IntoIterator<Item = OsString>,
    cwd: &Path,
    home: Option<&Path>,
    input: &mut R,
) -> Result<AppOutput> {
    interrupt::check()?;
    match cli::parse(args)? {
        cli::Command::Help => Ok(AppOutput::stdout(cli::HELP.as_bytes().to_vec())),
        cli::Command::Version => Ok(AppOutput::stdout(
            format!("llmf {}\n", env!("CARGO_PKG_VERSION")).into_bytes(),
        )),
        cli::Command::Init => {
            let workspace = workspace::init_project(cwd)?;
            Ok(AppOutput::stdout(
                format!(
                    "Initialized {}. Edit {}/config.json before running a filter.\n",
                    workspace.display(),
                    workspace.display()
                )
                .into_bytes(),
            ))
        }
        cli::Command::List => {
            let workspaces = workspace::discover(cwd, require_home(home)?)?;
            let names = workspace::list_prompts(&workspaces)?;
            let output = if names.is_empty() {
                Vec::new()
            } else {
                format!("{}\n", names.join("\n")).into_bytes()
            };
            Ok(AppOutput::stdout(output))
        }
        cli::Command::Show(name) => {
            let workspaces = workspace::discover(cwd, require_home(home)?)?;
            Ok(AppOutput::stdout(
                workspace::resolve_prompt(&workspaces, &name)?
                    .contents
                    .into_bytes(),
            ))
        }
        cli::Command::Run(name) => {
            let home = require_home(home)?;
            let workspaces = workspace::discover(cwd, home)?;
            if workspaces.project.is_none() && !workspaces.user_exists {
                let path = workspace::init_user(home)?;
                return Err(LlmfError::new(
                    EXIT_OPERATION_FAILED,
                    format!(
                        "initialized {}; edit {}/config.json before running a filter and retry",
                        path.display(),
                        path.display()
                    ),
                ));
            }
            Ok(AppOutput::stdout(run(&name, &workspaces, input)?))
        }
    }
}

fn run<R: Read>(
    prompt_name: &str,
    workspaces: &workspace::Workspaces,
    input: &mut R,
) -> Result<Vec<u8>> {
    let config = workspace::load_config(workspaces)?;
    let system_prompt = workspace::resolve_system_prompt(workspaces)?;
    let prompt = workspace::resolve_prompt(workspaces, prompt_name)?;
    let mut input_bytes = Vec::new();
    input
        .read_to_end(&mut input_bytes)
        .map_err(|error| LlmfError::new(EXIT_INTERNAL, format!("cannot read stdin: {error}")))?;
    interrupt::check()?;
    let system = format!(
        "{}\n\n# Task Prompt\n\n{}",
        system_prompt.contents, prompt.contents
    );
    let messages = vec![
        api::Message::system(system),
        api::Message::user(
            String::from_utf8(input_bytes)
                .map_err(|_| LlmfError::new(EXIT_INTERNAL, "stdin must be valid UTF-8"))?,
        ),
    ];
    Ok(api::ApiClient::new(&config)?
        .complete(&messages)?
        .content
        .into_bytes())
}

pub fn install_interrupt_handler() {
    interrupt::install();
}

pub fn check_interrupted() -> Result<()> {
    interrupt::check()
}

fn require_home(home: Option<&Path>) -> Result<&Path> {
    home.ok_or_else(|| LlmfError::new(EXIT_INTERNAL, "cannot determine user home directory"))
}

pub fn home_from_environment() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}
