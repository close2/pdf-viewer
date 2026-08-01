# ADR 0101 — A correction that reached the code and not the ledger

Status: accepted, 2026-08-01.

## Context

ADR 0100 took `FILE_ONLY_EVIDENCE_CEILING` from 49 to 40 by auditing §7.5's nine rows. The
largest block left was clause 8's seventeen — four in §8.6's colour spaces, six in §8.7's
patterns and shadings, five in §8.9's images, plus `8.6.6.2` and `8.11.3` — and clause 8 is the
one that "decide[s] whether any page looks right at all".

## What the audit found

**The `DeviceCMYK` silence is still recorded in the ledger, three sessions after `CLAUDE.md`
recorded that it is false.** Principle 5's own text says so in as many words:

> **"The specification defines nothing here" is itself a claim about the specification, and it
> decays.** The standing example used to be `DeviceCMYK` → RGB, on the evidence of §8.6.4.4 …
> §10.4.2.5 defines that conversion outright, and §10.4.2.1 ranks it below §10.3's ICC route

`colour.rs`'s `CMYK_CORNERS` comment carries the correction in full. §8.6.4.4's ledger row still
said "**it defines no conversion to RGB at all**", §8.6.4's said "one conversion the standard
does not define", and two doc comments — `colour.rs`'s own test for the ink corners and
`content.rs`'s `device_space` — still said it too. **Four places, none of them the one that was
fixed.**

This is the stale-row habit at its most exact: a row is a hypothesis, nothing fails when it
overstates a *silence*, and the correction stops wherever the person making it stopped looking.
The check that would have caught it is one `grep` for the sentence that was retired.

**§8.6.5.2's row claimed a validation the code does not perform.** Table 62 says "[t]he numbers
X_W and Z_W shall be positive, and Y_W shall be equal to 1.0"; `white_point` enforces positivity
on all three and nothing about Y_W. That is the right behaviour and it needed saying rather than
implying: a Bradford adaptation maps whatever white the space declares onto D50's, so a scaled
white normalises itself and refusing the file would lose a page this device draws correctly.
Positivity is enforced because a zero or negative component divides or inverts the adaptation.
The row is now what the code is.

**Three rows had no test that could fail.**

- **§8.7.4.5.2, type 1 function-based shadings.** `shadings.rs` covered axial, radial and mesh
  and never a `/ShadingType 1` at all — the row was `implemented` on a file that does not
  exercise it. The new test makes the function the identity on its own domain, so every pixel
  names the domain point it came from, and then asserts that translating `/Matrix` by half the
  page moves the colours by half the page. It needs a two-input function, which Table 38 makes a
  *stream* — the exponential and stitching types take one input each — so it is the one fixture
  in that file with an extra object.
- **§8.7.3.2, coloured tiling patterns.** Table 75's rule is that the cell's colours are "the
  ones initially in effect in the pattern's parent content stream", not the ones at the fill.
  The distinction is invisible on the nonstroking colour, because `/Pattern cs` has already
  replaced that one — so the test asks it of the *stroking* colour, which a pattern fill never
  touches. A reader that runs the cell under the state at the fill draws a red line where the
  clause asks for black, in the right shape and the right place, reporting nothing.
- **§8.9.5.3, image interpolation.** Nothing pinned `Image::is_smoothed`. The new unit test is a
  table of both flag values against both regimes, because the defect it guards is one arm of it:
  reading `/Interpolate` as the whole answer point-samples every reduced image, and ignoring it
  blurs every magnified one.

## Decision

Correct the four stale statements, correct §8.6.5.2's overstatement, write the three tests, and
cite a named test on each of clause 8's seventeen rows. `FILE_ONLY_EVIDENCE_CEILING` falls
**40 → 23**.

Nothing here changes a pixel: all three new tests pass against the tree as it stands, and both
gates are unchanged at 840 agreeing / 65 contradicted and 90 incomplete. All three were confirmed
to fail when the rule they guard is broken — `/Matrix` forced to the identity, the cell's state
inherited from the fill, and the two arms of `is_smoothed` read separately.

## Consequences

Two rules worth carrying, both cheap:

**A retired claim is a string, and strings are greppable.** When a session disproves a sentence
this project has been repeating, the work is not done when the code is right; it is done when
the sentence is gone. Four copies of this one survived a correction that `CLAUDE.md` describes as
having taken thirty-two sessions to make.

**A row whose evidence is a file can be `implemented` for something the file never touches.**
§8.7.4.5.2 is the sharpest instance so far: `shadings.rs` has fourteen tests and not one of them
was a type 1 shading, so the row's evidence passed on every run while asserting nothing at all
about the row.
