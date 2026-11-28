# 3879 — The zoned Transverse Mercator's easting, its INT and its reverse zone are read

Session 2294. Status: **accepted**. Context: Guidance Note 7-2 sections 3.2.3.1 and 3.2.3.2; ADR 3877.

## Decision

- **The easting.** Section 3.2.3.2 says "The standard Transverse Mercator formulas above are modified as
  follows" and prints `E = Z*10⁶ + FE + kO.ν [A + (1 – T + C)A³/6 + …]` and `D = (E – [FE +
  Z*10⁶])/(ν1 kO)`. Those are the older USGS series, which section 3.2.3.1 no longer gives: its
  formulas are the JHS series. The modification is the prefix `Z × 10⁶` added to the easting, and
  subtracted before the reverse; the Transverse Mercator itself is section 3.2.3.1's, with
  `λO = λI + Z W − W/2`.
- **INT.** `Z = [INT((λ – λI)/W) mod (360/W)] + 1` defines `mod` by `floor` and leaves `INT` open.
  With `INT` as truncation, every longitude within one zone width west of λI gives 0 and lands in
  zone 1, which would be twice as wide. `INT` is read as `floor`, which makes the section's floor-based
  `mod` do its work: 3°W is zone 60 of a grid starting at Greenwich. Tested on both sides of a zone's
  edge and of the antimeridian.
- **The reverse zone.** The section does not say how a reverse finds Z. A grid's `FE + x` lies within
  one zone's million metres for every grid the method is used for (UTM's runs 166 to 834 km), so the
  zone is the easting's whole millions. This is a choice. An easting whose millions name no zone is
  refused.
- **The width** must divide 360°, since the section numbers the zones `1 ≤ Z ≤ 360/W`; another is
  refused by name.
