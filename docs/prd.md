# LLM Filter Product Requirements Document

## 1. Product Definition

`llmf` (LLM Filter) is a single-binary, locally run filter for one-shot
language-model prompts.

Each Markdown prompt can be invoked as a command:

```bash
git diff | llmf review
```

`llmf review` loads the project-level `.llmf/prompts/review.md`, falling back to the user-level `~/.llmf/prompts/review.md` if it does not exist. Model responses are written verbatim to stdout, so multiple prompts can be composed through Unix pipelines:

```bash
cat task.md \
  | llmf analyze \
  | llmf summarize \
  > result.md
```

Core principle:

> The prompt is the command. Markdown is the data. Model output is also data. Unix is the workflow.

The runtime has no agent loop, does not parse model actions, and does not execute model-generated commands.

---

## 2. Product Goals

The MVP must:

1. Provide `llmf` as a single native binary implemented in Rust.
2. Execute a named prompt through `llmf NAME`.
3. Read input from stdin.
4. Make exactly one model request per invocation.
5. Write the model response verbatim to stdout.
6. Never execute model output as a shell command, program, or tool call.
7. Use an OpenAI-compatible Chat Completions API.
8. Support both project-level `.llmf/` and user-level `~/.llmf/` workspaces.
9. Use a strict `config.json` configuration and fail fast on errors.

---

## 3. Platforms and Dependencies

Supported platforms:

* macOS arm64
* macOS amd64
* Linux amd64
* Linux arm64
* Windows WSL

Users do not need to install Python, Node.js, Bash, `curl`, `jq`, an MCP server, or another agent CLI. HTTP, TLS, and JSON are all handled internally by the Rust binary.

The runtime does not invoke `/bin/sh` and has no other external runtime dependencies.

---

## 4. CLI

### 4.1 Execute a Prompt

```bash
llmf <name>
```

### 4.2 Management Commands

```bash
llmf --init
llmf --list
llmf --show <name>
llmf --help
llmf --version
```

An argument without a `--` prefix is a prompt name; an argument with a `--` prefix is a runtime operation. Therefore, `llmf init` executes `init.md`, while initialization must use `llmf --init`.

### 4.3 Prompt Names

Names may contain only ASCII letters, digits, `.`, `_`, and `-`, and must begin with a letter or digit.

---

## 5. Workspaces

Project-level workspace:

```text
<project>/.llmf/
├── config.json
├── system_prompt.md
└── prompts/
```

User-level workspace:

```text
~/.llmf/
├── config.json
├── system_prompt.md
└── prompts/
```

Starting from the current directory, search parent directories for the nearest `.llmf/`. For the prompt, system prompt, and configuration, prefer the respective project file and fall back to the user file if it does not exist. Select the configuration as a complete file; do not merge configurations.

---

## 6. Initialization

`llmf --init` creates the following in the current directory:

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

The default files are embedded in the binary. If `.llmf/` already exists, fail immediately without overwriting it.

When `llmf NAME` is run and neither a project nor a user workspace exists,
automatically initialize `~/.llmf/`, tell the user to edit the configuration,
and exit `1` without making an API request. The nonzero result requires a
configured retry rather than appearing as an empty successful filter result.

## 7. Configuration

`.llmf/config.json` template:

```json
{
  "api_url": "http://127.0.0.1:1234/v1/chat/completions",
  "api_key": "",
  "model": "qwen/qwen3.5-35b-a3b",
  "request_timeout_seconds": 120
}
```

Field rules:

* `api_url`: a required `http` or `https` URL with a host, no username,
  password, query, or fragment.
* `api_key`: a required string that may be empty; when empty, do not send an `Authorization` header.
* `model`: a required, non-empty string.
* `request_timeout_seconds`: a required integer in the range 1–3600.

Parse the configuration strictly. Immediately return `78` for invalid JSON, a missing field, an unknown field, a duplicate field, an incorrect type, an out-of-range value, an invalid URL, non-UTF-8 content, or an unreadable file. Environment variables must not override the configuration.

There are no command-execution configuration options. Reject the legacy fields `max_steps`, `command_timeout_seconds`, `max_command_output_bytes`, and `allow-command` as unknown fields.

---

## 8. Prompts

Shared model instructions reside in `system_prompt.md`, and named prompts reside in `prompts/<name>.md`. Both must be UTF-8; return `66` if either is missing, unreadable, or not UTF-8.

The runtime combines the system prompt and named prompt into one system message, and uses stdin as one user message. The runtime does not support YAML front matter, prompt template languages, prompt arguments, prompt DSLs, or embedded prompt configuration.

The default system prompt must explicitly tell the model:

* to work only from the task prompt and stdin;
* that it has no tools and cannot inspect or modify the local environment;
* not to claim that it executed commands, modified files, or ran tests; and
* that the final response will be output verbatim and must not include an action header.

---

## 9. Input and Output

stdin is the model input and must be valid UTF-8. Empty stdin is allowed and is
sent as an empty user-message string. The runtime does not interpret its
application-level format. Plain text, Markdown, HTML, Git diffs, logs, source
code, and JSON are all allowed. There is no request-body size cap; only the
response is bounded.

stdout may contain only `choices[0].message.content`, written verbatim byte for byte. Logs, HTTP information, status messages, debugging information, and progress must not be written to stdout.

An empty string at `choices[0].message.content` is a successful response.
Every failure leaves stdout empty. The delimiter used internally to combine the
system prompt and named prompt is not a public prompt-file contract.

stderr contains only brief, single-line fatal errors.

Exit codes:

```text
0    Success
1    Runtime operation could not be completed
64   CLI argument error
66   Prompt or system prompt is missing or unreadable
70   Runtime, workspace, or I/O error
74   Network or API error
78   Configuration error
130  Interrupted by Ctrl-C
```

Exit codes for maximum-step limits, the model action protocol, or shell-platform errors are no longer used.

---

## 10. Model API

Support only the minimal OpenAI-compatible Chat Completions subset:

```json
{
  "model": "configured-model",
  "messages": [
    {"role": "system", "content": "<system prompt + task prompt>"},
    {"role": "user", "content": "<stdin>"}
  ],
  "stream": false
}
```

Read only `choices[0].message.content` from the response. Return `74` for a non-2xx status, invalid JSON, missing string content, a timeout, or a response body larger than 10 MiB.

The Responses API, MCP, function calling, native tools, structured outputs, streaming, multimodal content, server-side sessions, and provider-specific tool events are not supported.

---

## 11. Single-Run Flow

```text
1. Parse the CLI
2. Find the workspaces
3. Strictly parse config.json
4. Load the system prompt
5. Load the named prompt
6. Read stdin
7. Construct two messages
8. Make one model request
9. Write the response content verbatim to stdout
10. Return 0
```

There are no multi-turn model interactions, action parsing, observation feedback, command execution, automatic repair retries, or maximum-step limits.

For shell pipelines that must fail when any filter fails, users should enable
`set -o pipefail`. Shell quoting is owned by the invoking shell, not by `llmf`.

---

## 12. Command-Execution Boundary

The runtime never executes model output, including but not limited to:

* shell commands or scripts;
* shell-like action headers;
* code blocks, tool calls, or function calls; and
* requests for dynamic commands, redirection, pipelines, or subprocesses.

All such content is only a string and must be output verbatim. The runtime source code must not contain an action parser, shell executor, command allowlist, command timeout, command-output capture, or agent loop for model output.

Users can still explicitly use Unix pipelines outside the `llmf` process. External commands are part of a user-initiated workflow, not part of the runtime's model command-execution capabilities.

---

## 14. Fail-Fast Behavior and Security

Do not automatically repair, guess at, or degrade on configuration, prompt, API, stdin, stdout, or workspace errors. Stop execution when an error occurs, emit no partial result, write a single-line error to stderr, and return a specific nonzero exit code.

Because the runtime does not execute model output, a model cannot use an agent loop to directly read, modify, or delete local files or to start processes. The model service still receives the system message and stdin; users must not put secrets in the input if those secrets should not be sent to the configured service.

Model output may contain unsafe advice or command text. Callers should treat it as untrusted data. The runtime does not execute any of this content automatically.

---

## 15. Acceptance Criteria

### 15.1 Initialization and Lookup

`llmf --init` generates all default files without overwriting an existing workspace. Project prompts, the project system prompt, and the project configuration take precedence; when any is missing, fall back to the user workspace according to the rules above.

### 15.2 Single Request

One `llmf NAME` invocation produces exactly one API request containing only one system message and one user message. Any string returned by the model is written verbatim to stdout.

### 15.3 No Command Execution

When the model returns:

```text
SHELL
printf started > marker
```

The runtime must:

1. not create `marker`;
2. not make a second model request;
3. write the content above verbatim to stdout; and
4. return `0`.

### 15.4 Pipelines

The complete stdout of the first `llmf` becomes stdin for the second `llmf`. Runtime information must not contaminate the data stream.

### 15.5 Errors

Configuration errors must not trigger an API call, and API errors must not produce partial stdout.

---

## 16. Product Positioning

Full description:

> `llmf` is a tiny Rust LLM filter that applies named Markdown prompts to stdin and composes through Unix pipelines.

Concise description:

> A tiny Rust LLM filter for composable one-shot prompts.

Homepage tagline:

> Prompt is the command. Markdown is the data. Unix is the workflow.
