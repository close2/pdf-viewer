# 3887 — The New Zealand Map Grid, the near-conformal Lambert and the Tunisia grid are read

Session 2296. Status: **accepted**. Context: Guidance Note 7-2 sections 3.4.1, 3.1.1.6 and 3.4.2
(September 2019); LINZ Office of the Surveyor-General, *Conversion between Latitude and Longitude
and New Zealand Map Grid*, OSG Technical Report 4.2, 21 November 2003
(https://www.linz.govt.nz/sites/default/files/cust/tr04-conversion-between-latitude-longitude-nzmg-2003.pdf);
`doc/todo/79` rows 24 to 26.

## Decision

- **The New Zealand Map Grid's formulas are the Surveyor-General's.** Section 3.4.1 describes the
  double projection and names Technical Circular 1973/32 (I. F. Stirling). It prints no formulas,
  and the Dataset's record says "See information source". No public copy of the circular was found.
  The government office responsible for the grid publishes Technical Report 4.2. It states the
  formulas both ways, the coefficients A1–A10, B1–B6, C1–C6 and D1–D9, and three test points, and it
  credits the projection to W. I. Reilly (1973). Those are evaluated, and the test points are the
  test, through NZGD49 / New Zealand Map Grid (EPSG 27200), to the millimetre and 1e-8°.
  The coefficients belong to the grid, not to any parameter. A system's origin and false
  coordinates take the report's `φ0`, `λ0`, `E0` and `N0` roles. The ellipsoid's semi-major axis takes
  its `a`, which is International 1924's, NZGD49's own. The report says two Newton steps "will give
  millimetre accuracy". The reverse takes steps until `θ` stops changing, so that it undoes the forward.
- **The near-conformal Lambert's `θ′` takes the cone's sign.** Section 3.1.1.6 signs `r′` by ϕO
  ("taking the sign of ϕO") but prints `θ′ = atan2{(E – FE), [rO – (N – FN)]}` unsigned. By its own
  forward formulas, `E – FE = r sin θ` and `rO – (N – FN) = r cos θ`. South of the equator `r` is
  negative and the printed `atan2` answers `θ ± π`, so both arguments are multiplied by `rO`'s sign,
  as section 3.1.1.2 does for the regular 1SP. The example is northern and decides nothing. A
  constructed southern origin round-trips on both sides of its meridian. An origin on the equator is
  refused, because `rO = kO νO / tan ϕO` divides by zero there.
- **Its reverse is the section's iteration.** Of the two reverses printed, the exact one
  ("[i]f an exact solution is required") is evaluated, so the reverse undoes the forward. The one-step
  solution's printed `m′` and `ϕ′` are checked in a unit test as values. Every printed constant
  (`A`, `A′` to `E′`, `sO`, `rO`, `s`, `m`) matches the example to its digits. The arc series is the
  expansion of the rectifying radius `a(1 + n²/4 + n⁴/64)/(1 + n)` and agrees with section 3.2.2's
  meridian distance to a millimetre at every latitude tested.
- **The Tunisia grid's sentence after the longitude formula names λ.** It says "where ϕ and ϕO are
  in grads relative to the Paris meridian". The formula above it is `λ = λO + [(E – EF) * 0.012185]`,
  and the sentence is read as naming λ and λO. The section also prints the grid-to-geographic
  direction first and calls the other "the reverse". Here, as for every method, the forward direction
  is from latitude and longitude. The rate changes at the false origin, `A = 0.010015` strictly north
  of it, and the grid is one piece across it. The test checks both sides. The section's six end values
  (94 and 490 km, 40 and 860 km, Djebel Kebar's 582 km) are the rates' to the printed digit.
