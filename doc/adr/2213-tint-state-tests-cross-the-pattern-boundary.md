# 2213 — Tint state tests cross the pattern boundary

Status: accepted; final combined integration checks remain due. Session 1689.
Context: ADR 2212; the recurring source-loss failures examined in round 1687.

The first fixture pins a clause-derived colour, rather than comparing two paths
that could share the same mistake. It fails on fixed baseline b9d05ee8. Further
stateful tests select once, change the pair, paint, and restore through q/Q. Nested
isolated RGB, CMYK and Gray groups are compared with separately constructed solid
references, with the definition-state distinction required by §11.6.7 kept explicit.
An achromatic Gray assertion independently checks that the group really changed
target. Non-isolated groups were unsuitable for that discriminator: their target
inherits from the parent, so naming another space alone proves nothing.

Stencils on both sides of the cell, text within it, alpha after conversion, forbidden
colour operators and a Type 3 glyph without a tint binding cover distinct consumers
and admission boundaries. Existing mixed-paint tests from round 1687 are unchanged.
A calibrated copied-source mutation must expose lost nested admission or binding,
and the fixed baseline must fail the arithmetic correction.

The source lives with immutable selection; the mark binding lives with graphics
state. This avoids another interpreter-global field that every speculative run
would have to save and restore manually. The new admission argument carries the
specific evidence needed by the group boundary, rather than lifting the general
uncoloured restriction. File splitting alone would not repair either lost input.

The cost comparison measures complete interpretations of repeated pattern marks,
alternating tints and changing parameters, with one-cell and many-cell lattices.
Document opening is outside the timer; content parsing and lazy work remain inside.
It uses isolated fixed-baseline and candidate exports, pinned balanced repetitions and
an exclusive quiet clock hold. No performance-neutral claim follows from correctness.
Final corpus and launch checks remain separate integration obligations.

A separate non-RGB solid defect remains under §11.7.5.3. The generated three-table
ICC witness selects component .5, then sets Perceptual before painting: it produces
[116,111,133], whereas setting the intent before selection produces [253,0,0], the
profile's red perceptual transform. This isolated equality assertion fails on the
candidate; it is not a tint regression or a desired-output test. Non-RGB solids still
retain only converted colour. Preserve the clause's partial status for this ordering
obligation, separately from the deliberate output-intent profile bypass. The exact
patch, receipt and next-round contract are preserved in /home/AI/r1689-deferred-icc/.
The following round must cover current rendering intent/BPC, fill/stroke, q/Q,
Form/group inheritance and stencils without weakening the tint-state controls.

An additional mixed pattern/ordinary-solid test uses identical RGB values with
different pairs, restores through q/Q, then paints both fill and stroke and changes
alpha. It protects the existing shared slots from cross-consumer contamination.
A generated three-table ICC tint test changes rendering intent after selection;
DeviceRGB alone cannot make that parameter observable. These test-only additions do
not change either frozen timing population from the first comparison.

The revised existing-cache candidate passes ten dedicated tint controls, the added
ICC-intent control and the actual-paint profile diagnostic. Warnings-denied all-target
Clippy and formatting pass in the same 48-second, 1.75-GiB bounded hold. Independent
review of the implemented routing and shared-slot witness found no concrete defect.
The earlier uncached full suite passed 2283 tests with 21 skips. The revised suite
passes 2285 with 21 skips (58.871 seconds tests; 156-second hold, 3.91 GiB). The fresh
balanced comparison accepts the bounded cost documented in ADR 2212: all twelve
pairs and 48 guards pass, with every sample retained. The first candidate's measured
regression remains rejected and preserved separately. All four revised PNGs match
the correct uncached candidate byte for byte; the tint picture was also inspected.
