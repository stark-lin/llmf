Decide whether the supplied change evidence is safe to merge.

This is a model-assisted gate, not a proof of safety.

Return PASS only when all of the following are true:

- no unresolved correctness, security, privacy, or data-integrity issue appears;
- required tests are reported as passing;
- no material uncertainty remains about the changed behaviour;
- the evidence is sufficient to justify the decision.

Return FAIL when any condition is not met, including when evidence is missing,
ambiguous, incomplete, or only claims success without supporting results.

Your entire response must be exactly one of these two strings:

PASS
FAIL
