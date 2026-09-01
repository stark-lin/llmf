# Architecture

`llmf` (LLM Filter) is a synchronous, one-request prompt runtime. The
deliberately linear design keeps the model boundary visible and makes stdout
safe to compose in Unix pipelines.

## Request flow

```text
CLI arguments
    │
    ▼
parse runtime operation or prompt name
    │
    ▼
discover nearest project workspace and user workspace
    │
    ├── resolve strict config.json
    ├── resolve system_prompt.md
    └── resolve prompts/<name>.md
    │
    ▼
read all stdin and build two chat messages
    │
    ▼
one non-streaming Chat Completions request
    │
    ▼
choices[0].message.content ──► stdout verbatim
```

Failures stop the flow and return one documented error on stderr. The runtime
does not produce partial model output.

## Module map

| Module | Responsibility |
| --- | --- |
| `main.rs` | process setup, stdio, and exit status |
| `lib.rs` | application dispatch, request orchestration, and process errors |
| `cli.rs` | CLI parsing, prompt-name validation, and help text |
| `workspace.rs` | workspace discovery, initialization, prompts, defaults, and configuration |
| `api.rs` | bounded HTTP request/response handling |

Unit tests live next to their modules. `tests/cli.rs` covers the public process
contract with local mock HTTP servers.

## Workspace rules

Starting at the current directory, `llmf` walks toward the filesystem root and
uses the nearest `.llmf/` directory as the project workspace. The user
workspace is always `$HOME/.llmf/`.

Configuration, the system prompt, and a named prompt resolve independently.
Each prefers the project file and falls back to the corresponding user file.
Configuration is never field-merged. This per-resource fallback is deliberate:
a project may share prompts while relying on user-local configuration. Do not
replace it with an all-or-nothing workspace override.

Workspace discovery walks lexical parent directories from the supplied working
directory. It does not canonicalize paths, so symlinked working directories
retain their shell-visible path semantics.

## Model boundary

The request body contains exactly:

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

Only a string at `choices[0].message.content` is accepted; an empty string is a
successful result. The runtime does not
support streaming events, tool calls, structured output, provider-specific
events, multi-turn state, or retries. HTTP redirects and environment proxy
configuration are disabled by the client.

## Invariants

The most important invariants are enforced by integration tests:

1. A named prompt results in no more than one model request.
2. Model output is copied to stdout and is never executed.
3. Runtime information cannot contaminate a successful pipeline data stream.
4. Invalid configuration and API responses fail before producing stdout.
5. Invalid UTF-8 stdin fails before a request is made.
6. Endpoint credentials and query/fragment data cannot appear in diagnostics.
