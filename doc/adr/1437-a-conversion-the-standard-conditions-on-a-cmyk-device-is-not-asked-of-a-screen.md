# 1437 — A conversion the standard conditions on a CMYK device is not asked of a screen

Session 1301. Status: **accepted**.
Context: ISO 32000-2 §10.2, §10.3.2, §10.4.2.1, §10.4.2.3, §10.4.2.5, §8.7.4.4, §8.6.5.7, §11.6.6;
ADRs 0204 (a condition read), 0263, 1194, 1207 (which raised this and left it).
Rows: §10.3.2 (`implemented`, unchanged), §10.4.2.3 (`departed`, unchanged), §10.4.2.5 (`departed`,
unchanged) — each note now states the reading below.

## 1. The sentence, and its condition

§10.3.2 says, after asking a processor to establish CIE-based definitions for device spaces that
do not match the raster output device:

> If the native device colour space is CMYK, then converting colours in the DeviceGray colour
> space to that CMYK should follow the method described in 10.4.2.3

It is the only sentence in the standard that sends an ICC enabled processor *back* to one of the
classic methods that §10.4.2.1 otherwise ranks below §10.3 ("a less-capable PDF processor may
choose to use the algorithms specified in the following subclauses 10.4.2.2 through 10.4.2.5").
Its condition is a property of the output device, the same kind of condition that makes §10.6's
halftones inapplicable (ADR 0204).

## 2. No output of this program meets it

- The screen: §10.2's row, the device is RGB with no spot colourants.
- Print: RFC 0004's Route B, as built (ADRs 1179, 1180), hands the platform an RGB raster
  rendered by `render-cpu`; the program never drives a CMYK device.
- `pdf-transform archive`: writes CIE-based *definitions* — an output intent, a `/DefaultCMYK` —
  and never converts a grey value to device CMYK in a content stream.
- §8.6.5.7 NOTE 2 describes a processor that lets a user choose a calibration "considered to be
  that of the native colour space of the intended output device" for proofing; this tree has no
  such mode.

So the second sentence of §10.3.2 is **inapplicable on its own condition**. The row stays
`implemented`, since its first sentence (the remapping, `CMYK_CORNERS` as the established source
under the NOTE) is executed. The note now says which sentence is inapplicable and why.

## 3. §10.4.2.3: ADR 1194 held

The one grey-to-CMYK conversion this tree does perform is into a `DeviceCMYK` *blending* space
(§11.6.6, `Compositing::Subtractive`). A blending space is not "the native device colour space", so
§10.3.2 does not reach it and §10.4.2.1's ranking decides it, which is the ground ADR 1194 took.
Reading §10.3.2 does not overturn that departure; it tells a later round **where it would stop
being one**: a build that drove a CMYK device would owe §10.4.2.3's grey-to-CMYK exactly, and
`colour::rgb_to_cmyk` already computes it as the nominal slice. Status unchanged.

## 4. §10.4.2.5, read the same way

No sentence sends an ICC enabled processor to §10.4.2.5. The nearest is §8.7.4.4, for a shading in
a device space: colour values "shall be converted to the native colour space using the standard
conversion formulas described in 10.4". That points at §10.4 whole, §10.4.2.1's ranking included,
and a shading's `DeviceCMYK` therefore takes the route a flat fill takes (trap 6's one
conversion). The departure stands (ADR 0263). The note now names the route in the order
`ColourSpace::device_family` asks it: `/DefaultCMYK` (§8.6.5.6), then an output intent's
`/DestOutputProfile` (§14.11.5), then `CMYK_CORNERS`; a colour in an `ICCBased` space goes
through its own profile.

## 5. What would revisit it

A CMYK output: a print path that separates, or a converter that writes device CMYK values. Either
makes §10.3.2's second sentence applicable and §10.4.2.3's grey-to-CMYK the recommended answer on
that path, and those would then be two rows' worth of different dispositions in one note.
