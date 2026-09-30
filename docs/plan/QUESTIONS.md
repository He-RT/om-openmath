# Open questions

No blocking question for workspace initialization.

Known engineering contract checks are tracked in [DEVIATIONS.md](DEVIATIONS.md). Resolve them with evidence before the task that consumes the affected interface; do not silently change mathematical test expectations.

Resolved M3.2 test-fixture check: a long left-associative division chain is a wide raw Times tree (each denominator contributes Power[..., -1]), not deeply nested syntax. The depth-limit fixture now uses right-associative rules; a separate assertion retains the accepted division chain and its 1,000 factors. No plan-provided mathematical vector or expected expression was changed.

Resolved M3.3 test-fixture check: the explicit-machine long-mantissa fixture omitted its trailing backtick. The official [Numbers documentation](https://reference.wolfram.com/language/tutorial/Numbers.html.en) distinguishes a long bare mantissa (arbitrary precision) from the same mantissa with a bare backtick (machine precision). Added the missing marker, retained the expected numeric value, and added separate unmarked 17/18-digit category checks. No plan-provided vector was changed.
