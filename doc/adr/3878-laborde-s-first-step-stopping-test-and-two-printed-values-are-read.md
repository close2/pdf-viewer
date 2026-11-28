# 3878 — Laborde's first step, its stopping test and two printed values are read

Session 2294. Status: **accepted**. Context: Guidance Note 7-2 section 3.2.4.2; ADR 3876's habit of
evaluating a printed special case on both sides; ADR 3877.

## Decision

- **The first step.** The reverse prints `H1 = H0/(H0 + G.H0³)` and then the general step
  `Hk+1 = (H0 + 2.G.Hk³)/(3.G.Hk² + 1)`, Newton's for `H + G H³ = H0`. The printed `H1` is not that
  step at `k = 0`: it is near one (the example prints 0.999820949 − 0.000001503i), not near `H0`. The
  iteration starts from `H0`. A test evaluates the printed `H1` to its printed digits and shows the
  iteration reaches the same root from both starts.
- **The stopping test.** The section stops at `ABSOLUTE(REAL(H0 − Hk − G.Hk³)) < 1E-11`, which leaves
  the imaginary part, and so the easting, unchecked. The modulus is tested instead, to the module's
  `CONVERGED`.
- **The reverse example's L.** It prints `L = -0.3464508142`; the forward's `L` is −0.034645081 and the
  printed longitude follows only from the latter. A decimal-place slip; nothing is evaluated from it.
- **The comparison table's second row.** At 16°12′S 44°24′E the pair printed under "Using Laborde
  formula" (1097651.447, 182184.982) is the pair the Hotine variant B formulas give there, to the
  millimetre, and the pair under Hotine (1097651.426, 182184.985) is what the Laborde formulas give.
  The same formulas reproduce the worked example six kilometres away to the millimetre, where Hotine
  is 2 cm off. The columns are read as transposed, and the test takes the Laborde value.

The `d = 0` cases, the rotated sphere's poles, are tested against the general formula a hair off them.
