# 2234 — Indexed ICC paint retains the decoded entry

Status: **accepted** at the merge that took it, `b17ea075` (line amended by ADR 2260). The round
wrote “proposed for integration; independent controls and paired cost complete”. Session 1700.

## Decision

For an ordinary Indexed colour with an immediate ICCBased base, retain the resolved base profile and decoded lookup components through painting. Section 8.6.6.3 defines the index as a selection of base-space components; section 11.7.5.3 requires the current parameters when elementary paint converts that CIE source. Keeping only the colour converted at selection loses later rendering intent and black-point compensation.

The existing selected-source constructor recognizes this one-hop case, calls `ColourSpace::entry_of`, and delegates to its direct-ICC constructor. The centralized lookup rounds, clamps and returns already-scaled components. The constructor does not decode PDF bytes again, divide those components by 255 again, or retain an index in place of components. Declared colour space and actual operand-recovery `read_in` remain separate from this normalized painting source.

No new source variant, cache or invalidation flag is introduced. Existing ICC profile identity, exact component bits and complete conversion context determine the two bounded pre-alpha memo entries. Ownership survives saves and Form resource shadowing. Initial index zero reaches the same constructor. Pattern binding precedence, actual-paint diagnostics and current alpha retain their existing boundaries.

Indexed process-tint/overprint handling remains a separate obligation. The former comment claiming `entry_of` was private was stale; its public availability does not implement that rule. Other Indexed base families, calibrated/Lab and Separation/DeviceN retention are not established by this fix. Section 11.7.5.3 remains partial, and the deliberate output-intent profile bypass is unchanged.

## Evidence

Root committed the independent tests before production changes. Fixed baseline 9cd2c665 passes the original thirteen tests and one of ten new Indexed controls; nine fail intended colour assertions. Test expectations derive from literal encoded XYZ knots, rational byte/255 interpolation, a pure matrix/transfer stage and explicit intermediate premultiplied-byte arithmetic. Cold comparisons supplement independent expectations. No expected value was copied from candidate output and no tolerance changed.

The first candidate passes all 23 controls: exit 0, 0.23 s tests, 16 s bounded hold, 17 s elapsed, peak 0.28 GiB. Warnings-denied model/colour all-target clippy and formatting pass. The serialized full model/colour suite passes 2311 tests with 21 skipped: exit 0, 66.391 s tests, 169 s hold/elapsed, peak 3.66 GiB. All five isolated omissions fail intended colour assertions; ADR 2235 records the artifact-collision limitation and correction. ADR 2235 records the full paired costs: repeated paint changes little, selection and restoration add measurable bounded work. Combined integration remains pending; this is not a performance-neutrality claim.
