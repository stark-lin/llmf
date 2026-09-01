use std::ffi::OsString;

use crate::{EXIT_USAGE, LlmfError, Result};

pub const HELP: &str = "\
NAME
    llmf - filter standard input through named LLM prompts

SYNOPSIS
    llmf NAME
    llmf --init
    llmf --list
    llmf --show NAME
    llmf --help
    llmf --version

DESCRIPTION
    llmf reads standard input, applies the Markdown prompt named NAME, and
    writes the model response verbatim to standard output. Named prompts
    compose naturally in Unix pipelines.

    Runtime operations start with two dashes. An argument without a leading
    double dash is always treated as a prompt name; for example, 'llmf init'
    runs the prompt init.md, while 'llmf --init' creates a workspace.

OPTIONS
    --init
        Create a .llmf/ workspace in the current directory. Existing
        workspaces are not overwritten.

    --list
        List the effective prompt names in lexical order, one per line.

    --show NAME
        Write the resolved Markdown prompt named NAME to standard output.

    --help
        Display this help page and exit.

    --version
        Display the llmf version and exit.

OPERANDS
    NAME
        A prompt name corresponding to prompts/NAME.md. It must begin with an
        ASCII letter or digit and may also contain '.', '_', and '-'.

FILES
    .llmf/
        The nearest project workspace in the current directory or one of its
        ancestors.

    ~/.llmf/
        The user workspace and fallback source for configuration and prompts.

ENVIRONMENT
    HOME
        Locates the user workspace at $HOME/.llmf/.

EXIT STATUS
    0    Success.
    1    Runtime operation could not be completed.
    64   Invalid command-line arguments.
    66   Prompt or system prompt was missing, unreadable, or invalid UTF-8.
    70   Internal, workspace, or input/output error.
    74   Network or model API error.
    78   Invalid configuration.
    130  Interrupted by Ctrl-C.

EXAMPLES
    Summarize a file:

        cat notes.md | llmf summarize

    Review the current Git changes:

        git diff | llmf review

    Inspect the effective review prompt:

        llmf --show review
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Run(String),
    Init,
    List,
    Show(String),
    Help,
    Version,
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command> {
    let args = args
        .into_iter()
        .map(|arg| {
            arg.into_string()
                .map_err(|_| usage("arguments must be valid UTF-8"))
        })
        .collect::<Result<Vec<_>>>()?;

    match args.as_slice() {
        [arg] if arg == "--init" => Ok(Command::Init),
        [arg] if arg == "--list" => Ok(Command::List),
        [arg] if arg == "--help" => Ok(Command::Help),
        [arg] if arg == "--version" => Ok(Command::Version),
        [arg, name] if arg == "--show" => {
            validate_prompt_name(name)?;
            Ok(Command::Show(name.clone()))
        }
        [name] if !name.starts_with("--") => {
            validate_prompt_name(name)?;
            Ok(Command::Run(name.clone()))
        }
        [] => Err(usage("missing prompt name or runtime operation")),
        [arg, ..] if arg.starts_with("--") => {
            Err(usage(format!("invalid runtime operation: {arg}")))
        }
        _ => Err(usage("too many arguments")),
    }
}

pub fn validate_prompt_name(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(usage("invalid prompt name"));
    };

    if !first.is_ascii_alphanumeric()
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(usage(format!("invalid prompt name: {name}")));
    }
    Ok(())
}

fn usage(message: impl Into<String>) -> LlmfError {
    LlmfError::new(EXIT_USAGE, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_documented_prompt_names() {
        for name in ["review", "code-review", "review_v2", "go.review", "A1"] {
            assert!(validate_prompt_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_unsafe_prompt_names() {
        for name in ["", "../review", "foo/bar", "-review", "review task", "é"] {
            assert!(validate_prompt_name(name).is_err(), "{name}");
        }
    }

    #[test]
    fn init_without_dashes_is_a_prompt() {
        assert_eq!(
            parse([OsString::from("init")]).unwrap(),
            Command::Run("init".into())
        );
    }

    #[test]
    fn help_uses_unix_manual_sections_in_order() {
        let mut previous = 0;
        for section in [
            "NAME\n",
            "SYNOPSIS\n",
            "DESCRIPTION\n",
            "OPTIONS\n",
            "OPERANDS\n",
            "FILES\n",
            "ENVIRONMENT\n",
            "EXIT STATUS\n",
            "EXAMPLES\n",
        ] {
            let position = HELP.find(section).unwrap();
            assert!(position >= previous, "{section:?} is out of order");
            previous = position;
        }

        assert!(HELP.ends_with('\n'));
        assert!(HELP.lines().all(|line| line.len() <= 80));
    }
}
