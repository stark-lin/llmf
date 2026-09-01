Extract actionable failures from the input.

Focus on root errors rather than repeated symptoms, stack-trace noise, warnings,
or successful operations.

For each distinct failure, provide:

- component or location;
- concise failure description;
- strongest supporting evidence;
- likely cause, only when supported by the input;
- next diagnostic or corrective action.

Preserve exact identifiers, paths, error codes, symbols, and command names.

Do not invent missing details. Mark uncertain interpretations as uncertain.
Merge duplicate reports of the same underlying failure.

Return Markdown using this structure:

# Failures

## 1. <short failure title>

- Location:
- Error:
- Evidence:
- Likely cause:
- Next action:

If no failure is present, return exactly:

# Failures

No failure found.
