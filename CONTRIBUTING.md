# Contributing to llmf

Thanks for helping improve `llmf`. The project deliberately keeps a narrow
scope: one Markdown prompt, one model request, and one stdout result. Before a
large feature proposal, please read the [product requirements](docs/prd.md) and
open an issue so the design can be discussed before implementation work begins.

## Development setup

Install a stable Rust toolchain that supports Rust 2024 edition, clone the
repository, and run:

```sh
cargo test --all-targets
```

The test suite starts local mock HTTP servers. It does not need an API key or
access to a real model service.

## Before submitting a change

Run all required checks from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Keep changes focused and include tests for observable behavior. In the pull
request description, explain the problem, the chosen approach, user-visible
changes, and the verification you performed.

Documentation is part of the interface. Update the relevant documentation when
changing:

- CLI syntax or output: `src/cli.rs`, integration tests, and `README.md`;
- configuration: `src/defaults.rs`, configuration tests, and `README.md`;
- workspace resolution: workspace tests, `README.md`, and
  `docs/architecture.md`;
- runtime or security boundaries: integration tests, `SECURITY.md`, and the
  product requirements.

## Design invariants

Changes must preserve these invariants unless the project explicitly changes
scope:

- each named prompt invocation makes at most one model request;
- model output is returned as data and is never parsed as an action or executed;
- successful named-prompt stdout contains only model response content;
- errors are concise, written to stderr, and return a documented exit code;
- API keys, authorization headers, stdin, and model responses are not logged;
- project and user configuration files are selected whole, never merged or
  overridden by environment variables.

See [docs/architecture.md](docs/architecture.md) for the code map and request
flow.

## Reporting bugs and security issues

For a normal bug, include the platform, Rust version, `llmf --version`, the
smallest reproduction, expected behavior, actual behavior, and exit status.
Remove credentials and private model input from logs or examples.

Do not open a public issue for a suspected vulnerability. Follow the private
reporting instructions in [SECURITY.md](SECURITY.md).
