# Security policy

## Supported versions

Before the first tagged release, the latest `main` branch is the only supported
line. After releases begin, security fixes will target the latest release unless
a release announcement says otherwise.

## Reporting a vulnerability

Please do not disclose a suspected vulnerability in a public issue. Use the
repository's [private vulnerability reporting
page](https://github.com/stark-lin/llmf/security/advisories/new) and include:

- the affected revision or version and platform;
- a minimal reproduction or proof of concept;
- the impact and conditions required to trigger it;
- whether the issue may expose credentials, private input, or local files.

Remove real secrets from every report. If private vulnerability reporting is
not available yet, wait for the repository owner to publish a private security
contact rather than posting exploit details publicly.

The maintainer will acknowledge a complete report, reproduce and assess it,
coordinate a fix and disclosure with the reporter, and credit the reporter when
requested and appropriate.

## Security boundaries

`llmf` sends the complete resolved prompts and stdin to the configured model
service. That disclosure is expected behavior; users are responsible for
choosing a service permitted to process their data.

The runtime must not:

- execute or interpret model output as a shell command, action, tool call, or
  program;
- log API keys, authorization headers, stdin, or model responses;
- silently fall back to a different configuration or service;
- include runtime diagnostics in successful named-prompt stdout.

Model hallucinations, unsafe suggestions, prompt injection contained entirely
inside model output, and outages of a configured third-party service are not by
themselves vulnerabilities in this runtime. They may become vulnerabilities if
they cross one of the boundaries above—for example, if output is unexpectedly
executed or credentials are written to logs.
