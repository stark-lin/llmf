use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use llmf::{self, EXIT_INTERNAL, LlmfError};

fn main() -> ExitCode {
    llmf::install_interrupt_handler();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let error = llmf::check_interrupted().err().unwrap_or(error);
            let _ = writeln!(std::io::stderr().lock(), "llmf: {}", error.message);
            ExitCode::from(error.code)
        }
    }
}

fn run() -> Result<(), LlmfError> {
    let cwd = std::env::current_dir().map_err(|error| {
        LlmfError::new(
            EXIT_INTERNAL,
            format!("cannot access current directory: {error}"),
        )
    })?;
    let home = llmf::home_from_environment();
    let args = std::env::args_os().skip(1).collect::<Vec<OsString>>();
    let mut stdin = std::io::stdin().lock();
    let output = llmf::execute(args, &cwd, home.as_deref(), &mut stdin)?;
    llmf::check_interrupted()?;
    if !output.stdout.is_empty() {
        std::io::stdout()
            .lock()
            .write_all(&output.stdout)
            .map_err(|error| {
                LlmfError::new(EXIT_INTERNAL, format!("cannot write stdout: {error}"))
            })?;
    }
    if !output.stderr.is_empty() {
        std::io::stderr()
            .lock()
            .write_all(&output.stderr)
            .map_err(|error| {
                LlmfError::new(EXIT_INTERNAL, format!("cannot write stderr: {error}"))
            })?;
    }
    Ok(())
}
