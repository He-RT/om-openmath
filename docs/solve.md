# Equation solving

The terminal, desktop and browser use the same Solve/NSolve/FindRoot/Reduce/Eliminate implementation. Modern `solve(x^2=2,x)` and Wolfram `Solve[x^2==2,x]` return the same two exact assignments. The default domain is Complexes; Reals, Integers and Rationals can be requested explicitly.

| Problem | Wolfram example | Behavior |
|---|---|---|
| Polynomial roots | `Solve[x^2-5x+6==0,x]` | Exact roots 2 and3 |
| Original pole | `Solve[x/x==1,x]` | Identity with zero excluded |
| Real roots | `Solve[x^2+1==0,x,Reals]` | No real solutions |
| Linear system | `Solve[{x+y==10,x-y==2},{x,y}]` | Exact ordered assignments |
| Real inequality | `Reduce[x^2<4,x,Reals]` | Open interval −2<x<2 |
| Numerical roots | `NSolve[x^5-x+1==0,x]` | Verified numerical approximations |
| Local search | `FindRoot[x^2==2,{x,1}]` | A verified nearby root |
| Elimination | `Eliminate[{x+y==3,y+z==2},y]` | Polynomial relation on retained axes |

## Source restrictions and domains

Solving retains direct unevaluated arithmetic before simplification. `x/x==1` excludes zero, and delayed definitions/pure functions preserve such holes when expanded by the solver. Immediate assignments and explicit `Cancel`/`Expand` transformations carry their already evaluated meaning; restrictions erased before Solve cannot be recovered.

Square roots, logarithms and rational powers use principal branches. Squaring or clearing denominators produces candidates that must be checked against the original equations, branches and poles. A numerical midpoint near zero never proves an exact identity or integer membership.

Requested axes retain their order. Omitted axes are inferred by name; if there are fewer equality constraints than variables, Solve emits `Solve::svars` and solves the first axes while treating the rest as parameters. Explicit empty axes remain empty. Free axes, generated integer periods, parameter assumptions and source conditions are preserved. Generated names avoid collisions with source constants.

Real roots use exact isolation or directed branch certificates. Integer/rational filtering uses exact algebraic certification, never proximity to an integer. Affine integer systems use an exact unimodular lattice certificate and return a particular solution plus a complete integer nullspace family. Symbolic coefficient lattice solving remains unsupported.

`MaxExtraConditions` controls generic assumptions; original source restrictions remain mandatory. The result distinguishes finite solutions, all values, no solutions and real regions. In Wolfram-shaped output roots repeat according to multiplicity. Logical branches are combined only when every nonempty branch is complete; an unconditional identity absorbs the union.

## Evidence and steps

Solution cards report actual `Exact`, `ByConstruction`, numerical verification or unresolved evidence. `ByConstruction` records an exact polynomial/linear construction; it is distinct from direct substitution proving zero. Numeric checks use directed balls and preserve their claimed precision. The solver does not turn unsupported input into an empty answer.

Derivations record real normalization, exclusions, denominator clearing, factorization, zero-product splitting, substitutions, elimination, branch/domain filtering and verification. The UI and CLI render the same32 rule identifiers in English and Chinese. Disabling recording suppresses step creation during computation. Eliminate's existing expression-only API has no synthetic derivation output.

## Supported algorithms

Exact polynomial solving combines factorization, low-degree formulas or certified Root objects. Root indices put real roots first, then nonreal conjugate pairs in the documented real/imaginary order. Dense polynomial queries have degree at most4096; closed algebraic certification has degree at most64. RootReduce performs exact algebraic arithmetic; ToRadicals returns a supported principal radical representation or retains the exact Root.

Polynomial systems use certified Gröbner/FGLM, triangular or primitive-element recovery when complete. Mixed systems substitute verified univariate branches and preserve their original restrictions. Closed coordinate ordering uses directed enclosures and exact algebraic projection ties. Bounded caches reuse immutable proofs with fresh cancellation/deadline checks; displayed source and verification semantics remain unchanged.

Reduce-lite supports one-variable real rational inequalities. Exact numerator/denominator roots define a sign chart; rational gap samples and minimal-polynomial remainders decide open/closed endpoints. Canceled denominator holes remain excluded, and accepted intervals merge only when justified. Nonrational, symbolic/inexact-coefficient, multivariate or discrete-domain inequalities remain unevaluated with diagnostics. Logical normalization is bounded to64 branches.

NSolve starts from complete supported solutions and rounds certified algebraic coordinates to the requested precision. FindRoot uses damped Newton with an analytic or numerical Jacobian, or safeguarded Brent for a real bracket. Local convergence requires both residual and correction/bracket evidence; successful values are checked again against raw residuals and poles. Exact precision is invalid for numerical requests. WorkingPrecision accepts MachinePrecision or5..2466 decimal digits; FindRoot also accepts MaxIterations and compatible Automatic/Newton/Brent methods.

Eliminate computes a polynomial elimination ideal with eliminated axes first. Restrictions involving retained axes remain explicit. Unsupported restrictions involving eliminated axes are rejected rather than silently erased. General constructible/existential projection is outside this version's scope.

## Limits and reproducibility

This is a bounded P0/P1 subset. Arbitrary transcendental systems, full multivariate inequality solving and general parameter case splitting are not implemented. Factorization may report incomplete work when recombination bounds are exhausted; it never invents irreducibility. Numerical ProductLog enclosure currently covers the principal nonnegative real axis. Unsupported branches retain source and messages such as `Solve::nsmet` or `Reduce::nsmet`.

Injected step/time/cancellation limits apply throughout. CLI/desktop interruption signals the computation directly; the browser restarts its worker and restores source/definitions while marking other output stale. Use a suitable explicit timeout for expensive debug builds. Algorithm-specific term, precision and degree bounds are described in [PLAN §8](plan/PLAN.md#8-算法规格规范形式多项式solve-管线零判定).

All53 authority rows in `tests/corpus/solve.toml` are checked through public APIs, Evaluator and actual CLI processes. Exact forms preserve order and multiplicity; numerical rows use independent700-bit raw residual/coordinate checks, exceeding200 decimal digits, with residual thresholds below1e−100. Original mathematical expectations are unchanged. Release timings and native/production UI acceptance are recorded in [pre-alpha.md](pre-alpha.md).
