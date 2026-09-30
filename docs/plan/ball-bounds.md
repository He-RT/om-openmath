# Elementary ball certificates (M1.4)

`Ball` encloses a real interval. Every arithmetic operation and reconstruction of
the center/radius rounds interval endpoints outward; the stored radius is rounded
upward at at most 64 significant bits. Extra working bits improve width, while
outward rounding provides correctness. A cache stores the complete pi ball,
including its radius.

- **exp and ln:** monotonicity maps lower/upper endpoints to lower/upper results.
  Dashu 0.6.1's `Context::exp` and `Context::ln` certify the selected rounding mode
  with a Ziv loop around a Taylor-based enclosure. This implementation uses Down
  for the lower endpoint and Up for the upper, then includes recentering error.
  ln of an interval containing nonpositive values returns the whole real line.
- **atan:** monotonicity permits endpoint evaluation. Reciprocal reduction uses
  atan(x) = pi/2 - atan(1/x) for positive x>1. Two half-angle reductions put the
  argument below 1/4. The series is alternating with decreasing absolute terms;
  its first omitted term bounds the truncation error. Powers, denominators and
  sums are interval computations, so reduction and accumulated rounding are
  already enclosed. Odd symmetry handles negative arguments.
- **pi:** Machin's identity pi = 16 atan(1/5) - 4 atan(1/239), using the same
  decreasing-series tail certificates. The offline fixture uses a different
  identity (Chudnovsky) at 1300 decimal digits. The test proves that both pi-ball
  endpoints lie in the same decimal bin for all 1000 required digits.
- **sin and cos:** subtract an integer multiple of 2*pi. Pi's working precision
  includes the argument's magnitude in bits, because its error is multiplied by
  that integer. Reductions wider than the supported Taylor interval return the
  certified global range [-1,1]. On |x|<=4 the alternating terms decrease from
  degree 8 onward, so the first omitted term encloses the tail. The original
  input radius is added by the global derivative bound |f'|<=1. Intersecting with
  [-1,1] preserves the enclosure and covers interior extrema.
- **complex exp/sin/cos:** compose real ball formulas with interval arithmetic.
  Hyperbolic factors use exp(y) and exp(-y); their errors remain enclosed.
- **complex ln:** its real part is ln(x^2+y^2)/2. Away from the origin and the
  negative-axis cut, argument extrema over a rectangle occur at its corners
  (partial derivatives have fixed signs in each quadrant). Rectangles crossing
  the cut enclose [-pi,pi]. Origin-containing inputs return whole components.
- **complex sqrt:** its components are based on r=sqrt(x^2+y^2) and
  sqrt((r±x)/2), using the proven nonnegativity of these expressions to intersect
  widened intervals with [0,+inf) before applying sqrt. Stable quotient formulas
  avoid cancellation. The real component is nonnegative. The imaginary sign
  follows y, taking the positive sign at y=0 on the negative axis; intervals
  crossing that axis enclose both signs.

The Decimal fixtures and generator are committed under `crates/om-num/tests/`.
Tests are offline and also compare varied bit precisions with independent
directed Dashu trigonometric evaluations. No fixture or binary64 sample replaces
the mathematical tail and rounding bounds above.
