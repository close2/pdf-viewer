# 3876 — The Polyconic reverse and Krovak's pseudo-longitude are read across their cases

Session 2293. Status: **accepted**. Context: ADR 3871's habit (evaluate a printed special case on
both sides of the line it splits); Guidance Note 7-2 sections 3.1.5 and 3.1.2.1; USGS Professional
Paper 1395 equations 7-12, 18-17 to 18-22.

## Decision

- **Polyconic's equator.** Section 3.1.5's reverse prints "If MO = (N-FN) then: ϕ = 0". But its
  forward puts the equator at `N − FN = −MO`, so the printed condition holds there only when MO is
  zero. Its own `A = [MO + (N−FN)] / a` is zero exactly at the equator. The case is read as `A = 0`,
  which is Professional Paper 1395's `y = −M0`. The forward's equator case `a(λ − λO)` is the
  general formula's limit, and both are tested at ϕ = 0 and ±1e-9 rad.
- **Polyconic's H.** The iteration prints `H = Mn` and `J = H / a`. The denominator's
  `C H − 2 / sin 2ϕ'` would then subtract a number from a length. The step is Newton's on the
  numerator, whose derivative needs `H = M′`, the derivative of `M / a` in latitude. The section's
  source writes it so (18-17, 18-21). `J` is `Mn / a`.
- **Krovak's D.** The asin form of section 3.1.2.1 holds within ±90° of pseudo-longitude. Past that
  it answers for the mirror point `π − D`, and the section gives the `atan2` of sin D1 and cos D1
  for it. The two agree on the near side, so the one `atan2` is evaluated everywhere. A test
  computes both forms on each side of the line. The reverse's `V'` keeps the printed `asin`; it
  folds only 90° of scaled longitude from the central meridian.
