# 2758 — A grey is the press's black ink on the simulated press, and still `DeviceGray` to §8.6.7

Status: accepted. Session 1973. Context: ISO 32000-2 §10.3.2, §10.4.2.3, §10.8.3, §8.6.7, §8.6.5.7; ADR 2712 (the
same construction for PDF/A), ADR 2736; deficit 7 of `doc/gwg-output-suite.md`.

## The reading

§10.8.3 step a) processes the page "as if separations were to be created for a simulated device that supports
subtractive process colourants", whose process colours the `DestOutputProfile` determines: a device whose native
colour space is CMYK. §10.3.2: "If the native device colour space is CMYK, then converting colours in the DeviceGray
colour space to that CMYK should follow the method described in 10.4.2.3", which is black `1 − gray` and no other
ink. This tree converted a grey into the press through its `B2A` table as a neutral, four inks. On the screen the
native colour space is the screen's, the sentence's condition does not hold, and the grey stays this reader's own.

§8.6.7's nonzero overprint mode applies "only to painting operations that use the current colour in the graphics state
when the current colour space is DeviceCMYK (or is implicitly converted to DeviceCMYK", and the clause it points to,
§8.6.5.7, is the implicit conversion of CIE-based spaces. The grey's space is `DeviceGray`; the four inks it becomes
all take the mark under either mode.

## The construction

`output_intent::grey_reading` answers `GreyReading::IntentBlack` where the reader asked for the simulation and the
intent is a CMYK profile, so ADR 2712's construction — a one-component `Separation` named `DeviceGray` over the
intent's profile, tints `(0, 0, 0, 1 − gray)` — carries the reading on every route a colour takes. `cmyk_tints`
gives that space no process tints, so it never takes the nonzero mode. That holds for ADR 2712's PDF/A reading as
well: the construction stands for `DeviceGray`, which §8.6.7 names as it names any other non-`DeviceCMYK` space; the
PDF/A file's screen view draws through the same space and loses only an overprint mode its colour never had.

## What moved

Press view only: 23.0's DeviceGray halves; 3.0's b and h, and its k, whose `X` the grey's own tints record would
have kept under mode 1 had `cmyk_tints` given it one (planted, `a_grey_is_the_presss_black_ink_on_the_simulated_press`);
9.0 and 9.1's grey text, 18.3's grey photograph, 4.0.1 by 28 edge pixels, and the test pages. No screen
view moved.
