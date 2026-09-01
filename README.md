# llmf

[![CI/CD](https://github.com/stark-lin/rust-agent/actions/workflows/ci-cd.yml/badge.svg)](https://github.com/stark-lin/rust-agent/actions/workflows/ci-cd.yml)

> Prompt is the command. Markdown is the data. Unix is the workflow.

`llmf` (LLM Filter) is a small Rust command-line filter that applies named
Markdown prompts to stdin and composes through Unix pipelines. It is a single
native binary and does not require Python, Node.js, Bash, `curl`, `jq`, an MCP
server, or another agent CLI at runtime.

```sh
git diff | llmf review

cat task.md \
  | llmf analyze \
  | llmf implement \
  | llmf review \
  > result.md
```

Each invocation makes exactly one non-streaming model request and writes the
returned content to stdout verbatim. Model output is always data: `llmf` never
interprets or executes generated commands.

> [!NOTE]
> The project is at an early `0.1.x` stage. Its intentionally small command and
> configuration surface may still change before `1.0`.

## Installation

See the [installation guide](docs/installation.md) for source builds, prebuilt
macOS and Linux snapshots, checksum verification, `PATH` setup, and WSL notes.

## Quick start

After installing `llmf`, initialize a local workspace:

```sh
llmf --init
```

Then edit `.llmf/config.json` for your OpenAI-compatible Chat Completions
endpoint:

```json
{
  "api_url": "http://127.0.0.1:1234/v1/chat/completions",
  "api_key": "",
  "model": "qwen/qwen3.5-35b-a3b",
  "request_timeout_seconds": 120
}
```

Run one of the included prompts:

```sh
printf 'Explain why small interfaces compose well.\n' | llmf summarize
```

An empty `api_key` is valid for a local endpoint and omits the `Authorization`
header. For a hosted endpoint, store the key in the configuration file and
keep the workspace private and verify that credentials are not staged before
committing.

## Commands

Named prompts do not start with `--`; runtime operations always do:

```text
llmf <name>          Run prompts/<name>.md with stdin as model input
llmf --init          Create a project .llmf/ workspace
llmf --list          List effective prompt names
llmf --show <name>   Print the resolved prompt
llmf --help          Show CLI help
llmf --version       Show the version
```

For example, `llmf init` runs `init.md`; it does not initialize a workspace.
Prompt names may contain ASCII letters, digits, `.`, `_`, and `-`, and must
begin with a letter or digit.

To add a command, create a UTF-8 Markdown file such as
`.llmf/prompts/my-prompt.md`, then invoke it by file stem:

```sh
cat input.md | llmf my-prompt > result.md
```

`--list` prints the sorted effective prompt names. `--show` prints the resolved
prompt and does not require a valid model configuration.

## Workspaces and resolution

`llmf --init` creates:

```text
.llmf/
├── config.json
├── system_prompt.md
└── prompts/
    ├── analyze.md
    ├── clean-web.md
    ├── commit-message.md
    ├── compare-options.md
    ├── customer-brief.md
    ├── decision-record.md
    ├── extract-failures.md
    ├── handoff.md
    ├── implement.md
    ├── implementation-plan.md
    ├── review.md
    ├── review-diff.md
    ├── safe-to-merge.md
    ├── summarize.md
    ├── translatetoen.md
    ├── translatetojp.md
    └── translatetozh.md
```

Existing `.llmf/` directories are never overwritten. Add `.llmf/` to your
repository's ignore rules if the workspace contains credentials.

At runtime, the nearest project workspace is resolved against `~/.llmf/`:

| Resource | Resolution order |
| --- | --- |
| Configuration | nearest project `config.json`, then user `config.json` |
| System prompt | nearest project `system_prompt.md`, then user file |
| Named prompt | nearest project prompt, then user prompt of the same name |

Configuration is selected as a whole file and is not merged. Environment
variables do not override it. Missing, unknown, duplicate, mistyped, or
out-of-range fields are errors. When no workspace exists, the first named
prompt invocation initializes `~/.llmf/`, asks you to edit its configuration,
and exits with status `1` without calling the model API. This makes a pipeline
fail until it is configured and retried.

Project fallback is deliberately per resource: a project workspace can keep
its prompts while taking a missing `config.json` or `system_prompt.md` from the
user workspace. This is intentional, not a partial workspace override.

## Configuration reference

| Field | Requirement |
| --- | --- |
| `api_url` | non-empty `http` or `https` URL with a host; no credentials, query, or fragment |
| `api_key` | string; may be empty to omit bearer authorization |
| `model` | non-empty model identifier passed to the service |
| `request_timeout_seconds` | integer from 1 through 3600 |

The JSON parser rejects missing fields, extra fields, duplicate fields, invalid
types, invalid UTF-8, and values outside these constraints. Its errors use
stable field-level categories and never echo configuration contents.

## Request and output contract

The runtime supports the OpenAI-compatible Chat Completions subset. It sends
exactly two messages in a single non-streaming request:

1. one system message containing `system_prompt.md` and the named task prompt;
2. one user message containing stdin.

The configured service must return a string at
`choices[0].message.content`. Response bodies are capped at 10 MiB. The string
is written to stdout byte-for-byte; progress, logs, and errors never enter the
pipeline data stream. Empty stdin is sent as an empty string; stdin must be
valid UTF-8. An empty response string is a successful response. Request bodies
are not size-capped. The internal separator used while composing the two prompt
files is not a public prompt-file contract.

On every failure stdout is empty and stderr contains one brief line. In a shell
pipeline, use `set -o pipefail` when a failed filter must fail the pipeline;
shell quoting remains the shell's responsibility.

## Security and privacy

The configured model service receives the complete system prompt, named prompt,
and stdin. Do not provide data that the service is not permitted to process.
Treat model output as untrusted data even though this runtime never executes it.

The runtime never executes model output. See [SECURITY.md](SECURITY.md) for the
vulnerability reporting process and security boundaries.

## Deliberate limits

`llmf` is a one-request filter, not an agent framework. It intentionally does
not add streaming, retries, provider abstraction, tool calling, conversation
history, prompt arguments, templates, interactive mode, or a TUI.

## Exit status

| Code | Meaning |
| ---: | --- |
| 0 | Success |
| 1 | Runtime operation could not be completed |
| 64 | Invalid CLI arguments |
| 66 | Prompt or system prompt not found, unreadable, or invalid UTF-8 |
| 70 | Internal, workspace, or I/O error |
| 74 | Network or model API error |
| 78 | Invalid configuration |
| 130 | Interrupted by Ctrl-C |

## Development

Run the local verification suite with:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

The tests use local mock HTTP servers and do not require a model endpoint or API
key. See [CONTRIBUTING.md](CONTRIBUTING.md) before proposing a change.

Pull requests and pushes to `main` run the same formatting, Clippy, test, and
release-build checks in GitHub Actions. A successful push to `main` also moves
the `main-latest` tag and updates its rolling GitHub prerelease. Publishing uses
the repository-provided `GITHUB_TOKEN`; no additional secret is required.
Configure the `main` branch ruleset to require the stable `CI passed` check
after its first run. Because the snapshot is rolling, do not protect the
`main-latest` tag or enable immutable releases for that prerelease.

More project documentation:

- [Architecture and invariants](docs/architecture.md)
- [Product requirements](docs/prd.md)
- [Open-source release checklist](docs/open-source-checklist.md)

## License

This project is licensed under the
[GNU Affero General Public License v3.0 only](LICENSE), expressed by the SPDX
identifier `AGPL-3.0-only`. The AGPL includes source-sharing requirements for
modified versions used to provide functionality over a network; consult the
full license text before redistributing or operating a modified version.
