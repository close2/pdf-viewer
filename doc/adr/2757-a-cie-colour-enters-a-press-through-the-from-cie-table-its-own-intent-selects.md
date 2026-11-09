# 2757 — A CIE-based colour enters a press through the "from CIE" table its own rendering intent selects

Status: accepted. Session 1973. Context: ISO 32000-2 §11.7.5.3, §8.6.5.8, §10.3.1; ADR 0796 (the press's `B2A` is the
conversion in), ADR 1032 (the intent selects the `A2B`); deficit 4 of `doc/gwg-output-suite.md`.

## The reading

§11.7.5.3: "The rendering intent influences the conversion from a CIE-based colour space to a target colour space,
taking into account the target space's colour gamut", and "the rendering intent to use for converting an object's
colour (whether that of an elementary object or of a transparency group) is determined by the rendering intent
parameter associated with the object". An ICC conversion's gamut mapping is its destination's `B2A` table, one per
intent (`B2A0` perceptual, `B2A1` colorimetric, `B2A2` saturation), and §8.6.5.8's intents "have been chosen to
correspond to those defined by the International Color Consortium". So the object's intent selects the press's table.

This tree selected the source's `A2B` by the object's intent (ADR 1032) but took the press's `B2A1` whatever it was,
and undid the black point under the *group's* parameters (`Press::rendering`). The ledger row said so: "Which
intent's table beyond that ranking is not selected".

## Two readings, planted

Measured on 13.0's own profiles with a throw-away probe (the X's colour into ISO Coated v2 300 %, against the square's
`0.85 0.03 1 0.15`, 217/8/255/38 in levels): HEAD's route (`A2B0` in, `B2A1` out) gave the RGB `X` 208/0/255/1 and
the CMYK `X` 205/2/255/8 — black 30 to 37 levels short, the green `X` the README names. The source's `A2B0` was
already honoured; `B2A0` gave 217/8/255/39 and 214/8/253/38. The CMYK pair's `X` had the same cause as the RGB pair's,
so the brief's second reading (the CMYK source profile's conversion) is the first one, not another.

## The construction

`Profile::inverse_for(transform)` answers `B2A0`, `B2A1` or `B2A2`, an intent the profile does not tabulate falling back
to the colorimetric table as `Profile::route` does for `A2B`; the two alternate tables are kept as bytes and parsed on
first use (`IntentInverse`), and `owned_bytes` charges both. `Profile::to_device` reads the table under its
`rendering`, and `xyz_to_ink` takes the object's rendering from `ColourSpace::to_press`. A device colour is not
CIE-based, so `rgb_to_ink` still enters under `Press::rendering`; the press's own rendering governs the conversion out.

## What moved

13.0's press view (all four squares), 16.1 ICCBased RGB in both views (`/Saturation`), 22.1 in both views, the
16-bit 18.0 and 18.2 press views and the ICC test pages: every one states `/Perceptual` or `/Saturation`. No patch
stating no intent moved.
