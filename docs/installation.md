# Installation

`llmf` is a native command-line binary. At runtime it needs only access to an
OpenAI-compatible Chat Completions endpoint; it does not require Python,
Node.js, Bash, `curl`, `jq`, or another agent CLI.

## Choose an installation method

| Method | Best for | Rust toolchain required |
| --- | --- | --- |
| Build from source | Contributors and users who want to build the checked-out revision | Yes |
| Download a snapshot | macOS or Linux users who want a prebuilt binary | No |

The project currently publishes rolling development snapshots, rather than
stable versioned releases. For a reproducible build, install from a pinned
commit instead of `main-latest`.

## Build from source

Install a stable Rust toolchain with Rust 2024 edition support from
[rustup](https://rustup.rs/), then clone the repository and install the binary:

```sh
git clone https://github.com/stark-lin/rust-agent.git
cd rust-agent
cargo install --path . --locked
```

Cargo installs `llmf` into its binary directory (normally `~/.cargo/bin`). If
the shell cannot find `llmf`, add that directory to `PATH` or reopen the shell
after completing rustup's setup instructions.

To build without installing, run:

```sh
cargo build --release --locked
./target/release/llmf --version
```

## Install a prebuilt snapshot

The rolling [`main-latest` release](https://github.com/stark-lin/rust-agent/releases/tag/main-latest)
contains a `.tar.gz` archive and a matching `.sha256` checksum for each native
target below.

| System and CPU | Archive target |
| --- | --- |
| macOS on Apple silicon | `aarch64-apple-darwin` |
| macOS on Intel | `x86_64-apple-darwin` |
| Linux on ARM64 | `aarch64-unknown-linux-gnu` |
| Linux on x86-64 | `x86_64-unknown-linux-gnu` |

Download the archive and its checksum from the release page for your target.
For example, on Apple-silicon macOS:

```sh
target=aarch64-apple-darwin
base="https://github.com/stark-lin/rust-agent/releases/download/main-latest"
curl -fLO "$base/llmf-$target.tar.gz"
curl -fLO "$base/llmf-$target.tar.gz.sha256"
shasum -a 256 -c "llmf-$target.tar.gz.sha256"
tar -xzf "llmf-$target.tar.gz"
mkdir -p "$HOME/.local/bin"
install -m 755 llmf "$HOME/.local/bin/llmf"
```

On Linux, replace the checksum command with:

```sh
sha256sum -c "llmf-$target.tar.gz.sha256"
```

Ensure `~/.local/bin` is on `PATH` before running the binary. You may instead
copy `llmf` to any directory already on `PATH`. Windows is supported through
WSL; use the Linux archive that matches the WSL distribution's CPU.

## Verify and initialize

Confirm the installed program is the expected binary, then create a workspace:

```sh
llmf --version
llmf --init
```

`--init` creates `.llmf/config.json`, `.llmf/system_prompt.md`, and bundled
prompts in the current project. Edit `config.json` with the endpoint, model,
and (when required) API key before running a named prompt. See the
[configuration reference](../README.md#configuration-reference) for the
complete field requirements.
