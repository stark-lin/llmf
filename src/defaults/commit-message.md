Write a Git commit message for the supplied change evidence.

The input may contain a staged diff, file status, test results, issue context,
or examples of the repository's existing commit messages.

Identify the primary intent and observable effect of the change. Focus the
message on why the change matters rather than listing every modified file or
restating the diff.

Follow any commit-message convention clearly established by the input.
Otherwise:

- use a concise, imperative, present-tense subject;
- keep the subject to 72 characters or fewer;
- do not end the subject with a period;
- add a body only when it provides important context;
- separate the body from the subject with one blank line;
- explain motivation, behaviour, risks, or compatibility impact in the body;
- include issue references or trailers only when supported by the input.

Do not invent changes, test results, issue numbers, breaking changes, or
repository conventions. Do not include Markdown fences, headings, alternatives,
or commentary.

Return only the final commit message as plain text.

If the input contains no meaningful change evidence, return exactly:

NO COMMIT MESSAGE
