# Open questions

No blocking question for workspace initialization.

Known engineering contract checks are tracked in [DEVIATIONS.md](DEVIATIONS.md). Resolve them with evidence before the task that consumes the affected interface; do not silently change mathematical test expectations.

Resolved M3.2 test-fixture check: a long left-associative division chain is a wide raw Times tree (each denominator contributes Power[..., -1]), not deeply nested syntax. The depth-limit fixture now uses right-associative rules; a separate assertion retains the accepted division chain and its 1,000 factors. No plan-provided mathematical vector or expected expression was changed.

Resolved M3.3 test-fixture check: the explicit-machine long-mantissa fixture omitted its trailing backtick. The official [Numbers documentation](https://reference.wolfram.com/language/tutorial/Numbers.html.en) distinguishes a long bare mantissa (arbitrary precision) from the same mantissa with a bare backtick (machine precision). Added the missing marker, retained the expected numeric value, and added separate unmarked 17/18-digit category checks. No plan-provided vector was changed.

Resolved M3.5 display-fixture check: the initial readable-form fixture expected `1 + 2*x`, copied from the API's illustrative example, while FormatOptions explicitly defaults to descending degree. Its expected string is now `2*x + 1` (Modern `2x + 1`). The equation, parsed structure and all forty plan FullForm expectations are unchanged.

Resolved M4.4 scalar-fixture check: evaluator results contain canonical Rational atoms, while parser-only expected trees contain raw Times/Power fractions. Compare with canonicalize(expected) for symbolic calls with evaluated fractional arguments and lists containing fractions; numeric expectations remain unchanged.

Resolved M4.5 rounding check: the precision-doubling test for 1/3 exposed double rounding through the nearest rational-to-float conversion of dyadic endpoints. Construct and round exact binary endpoints directly; this also avoids expanding large binary exponents into huge rationals. The original expected convergence and all mathematical fixtures remain unchanged. Real branch-cut formulas retain analytically zero components so interval dependency does not block certification (e.g. ArcSec[1/2]).

Resolved M4.4 completion check: the 88-function M4 registry plus the existing Inequality and ProductLog extensions yields exactly 90 documented functions. The elementary family uses the 21 names already fixed in the builtin contract; this task does not change symbol IDs or add unlisted inverse-hyperbolic names. All listed functions have at least two behavior vectors across the M4 test suites.

Resolved M5.1 coefficient-context checks: static generic zero/unit factories initially compared unequal to runtime-context identities, and finite-field polynomial zero holes carried inconsistent modulus metadata. Explicit identity equality and canonical zero factories fix both regressions without changing the original ring-law or reconstruction expectations. The runtime-modulus contract question is settled in P47.

Resolved M5.2 budget-fixture check: integer exact division of a monic input by 2x+1 can legitimately reject 1/2 at the first coefficient before using its three-step budget. Use the monic divisor x+1 for the cancellation-only fixture so division enters its coefficient work loop. The Abort expectation, all authority division/prem/content vectors and the explicit nondivisibility checks remain unchanged.

Resolved M5.3 generated-property check: the initial authored scaling property incorrectly assumed gcd(6f,9g)=3*gcd(f,g) for arbitrary contents and zero inputs. The shrunk counterexample f=0,g=1 gives gcd(0,9)=9. Check the true identity: primitive gcd times igcd(6*content(f),9*content(g)), keeping the zero convention. Retain the regression seed; no authority GCD vector or implementation result was changed. Add an independent Q[x] Euclidean oracle to verify maximality rather than just common divisibility.
