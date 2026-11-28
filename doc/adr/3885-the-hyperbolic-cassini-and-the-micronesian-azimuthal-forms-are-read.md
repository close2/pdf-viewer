# 3885 — The hyperbolic Cassini and the Micronesian azimuthal forms are read

Session 2296. Status: **accepted**. Context: Guidance Note 7-2 sections 3.2.2.1, 3.3.3.1 and 3.3.3.2
(September 2019); `doc/todo/79` rows 19 to 21; ADRs 3823 and 3871.

## Decision

- **Hyperbolic Cassini-Soldner's divisor is a length in chains.** The reverse's first estimate is
  `ϕ1' = ϕO + (N – FN)/315320`. The quotient is an angle only if 315320 is a length, and the example
  works in chains (its `ρ` is 315176.48): the divisor is a meridian radius near Vanua Levu in the
  example's chain, 66 of the ellipsoid's international feet, 20.1168 m, whose hundredth is the grid's
  link (EPSG unit 9098). It is held in metres. The estimate only seeds `ρ1'` and `ν1'`, whose product
  moves the correction `q` by parts in a million per hundredth of a radian, so no tuning can hide in it.
- **The example's `ϕ1'` drops its sign.** It prints `ϕ1' = 0.293952249`. With `ϕO = −0.283616003` and
  `N – FN = −3259.2249` chains, the formula gives −0.293952249, and the same example prints the
  footpoint `ϕ1 = −0.293957437`. The magnitude is the formula's. The test is the example's grid point
  and its return within a ten-thousandth of a second, over the island too.
- **Modified Azimuthal Equidistant's distance is the sine rule's quotient with `sin α` divided out.**
  The section's `s = asin[sin(λ – λO) cos ψ / sin α]`, with its own case for `sin α = 0`, is
  `asin[(x² + y²)^½]` exactly, `α` being the `atan2` of `y = sin(λ – λO) cos ψ` and `x = cos ϕO sin ψ –
  sin ϕO cos ψ cos(λ – λO)`. On the meridian that is the section's case `asin(x) · SIGN(cos α)`. The
  printed quotient was evaluated first and failed both sides of the line: next to the meridian, `sin α`
  is a rounding error and the quotient is `0/0`. A point 116 km south of the origin and a nanodegree off
  the meridian moved by 478 m. A test checks the section's quotient off the meridian and its case on
  it, north and south of the origin, to 1e-15, and checks that the grid is one piece across the meridian.
- **Its reverse multiplies `sin Ψ′` through.** `ϕ = atan[(1 – e² K sin ϕO / sin Ψ′) tan Ψ′ / (1 – e²)]`
  is evaluated as `atan[(sin Ψ′ – e² K sin ϕO) / (cos Ψ′ (1 – e²))]`, the same quotient. It has no `0/0`
  where `Ψ′` is the equator.
- **Guam takes exactly three steps.** The section says "[t]he Guam projection uses three iterations".
  This is how the method is defined, not a loose precision setting, so the reverse takes three steps
  and stops. Over the island they return a point to a ten-thousandth of a second.
