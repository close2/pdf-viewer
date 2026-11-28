# 3871 — Hotine at an azimuth of 90° is evaluated by the section's general formula

Session 2292. Status: **accepted**, a documented choice where the text contradicts itself. Context:
ADR 3870; IOGP Guidance Note 7-2 (revised September 2019) section 3.2.4.1. Code:
`crates/pdf-model/src/geospatial/projection/oblique.rs`.

## Context

For variant B the section gives a special case where the azimuth αC is 90 degrees (Hungary's EOV,
the Swiss grid): `uC = A (λC − λO)`; in the forward, `u = 0` where λ = λC and otherwise
`u = A atan2{(S cos γO + V sin γO), cos[B (λ − λO)]} / B − [ABS(uC) * SIGN(ϕC) * SIGN(λC − λ)]`.
The general formula, "[i]n general", subtracts `ABS(uC) * SIGN(ϕC)` without the last factor.

Evaluated for CH1903 / LV03 (EPSG 21781), the `atan2` term is close to `uC` on both sides of the
centre's meridian, so the special formula agrees with the general one west of λC and **adds** `uC`
east of it: a point a metre east of the centre lands near u = 2uC, about 20 000 km away, while the
section defines (EC, NC) as the grid point of the centre and the projection is continuous there.
The reverse case states no special case, so the special forward would not return from its reverse
either. No erratum to the September 2019 revision is held.

## Decision

The general `u` is evaluated for every azimuth, the special case's `u = 0` is kept where λ = λC
(the general formula gives zero there too, to rounding), and the special `uC = A (λC − λO)` is
used at 90° (it equals the general formula's value). Held by
`epsg::tests::hotine_b_at_an_azimuth_of_ninety_degrees_puts_the_centre_at_its_stated_grid_point`:
LV03's centre at (600000, 200000) m and a metre either side within a metre and a half of it.

## Consequences

A later revision of the Guidance Note that corrects or explains the factor reopens this: the
special case is then read as it is written there.
