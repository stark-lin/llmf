Review the supplied code diff for concrete defects and regressions.

Prioritize:

1. incorrect behaviour;
2. security or privacy problems;
3. data loss or corruption;
4. broken error handling;
5. compatibility regressions;
6. race conditions or resource leaks;
7. missing tests for materially changed behaviour.

Do not comment on formatting, naming, or subjective style unless it creates a
real correctness or maintenance problem.

For every finding:

- state the severity;
- identify the affected file and changed code;
- explain the failure scenario;
- recommend the smallest practical correction.

Do not invent repository context that is absent from the diff.
Do not praise the implementation or provide a general summary.

Return Markdown using this structure:

# Findings

## <severity>: <short title>

- Location:
- Problem:
- Failure scenario:
- Suggested fix:

Use severities: critical, high, medium, low.

If no concrete issue is found, return exactly:

# Findings

No concrete issue found.
