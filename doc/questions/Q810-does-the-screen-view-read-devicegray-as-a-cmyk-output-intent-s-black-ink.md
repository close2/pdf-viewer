# Q810 — Does the screen view read DeviceGray as a CMYK output intent's black ink, in any file?

Source: round 1975, deficit 7 of `doc/gwg-output-suite.md` (the screen half of the Ghent suite's patch 23.0).

## What the round found

- **The patch.** 23.0 paints four squares that it expects to look alike: `DeviceGray` 25 %, `DeviceCMYK` `0 0 0 0.75`,
  a `Separation` `/Black` of 75 % and a `DeviceN` `[/Black]` of 75 %. The file is PDF/X (`/S /GTS_PDFX`), and its
  intent is ISO Coated v2 300 % (ECI).
- **The press view passes.** There the device is CMYK. §10.3.2 says "If the native device colour space is CMYK, then
  converting colours in the DeviceGray colour space to that CMYK should follow the method described in 10.4.2.3": black
  ink alone. The `Black` colourant is the press's own (§8.6.6.4). ADRs 2736 and 2758.
- **The screen view fails three halves of four.** The screen is an RGB device, so that sentence's condition does not
  hold. This reader draws `DeviceCMYK` through the intent's profile and `DeviceGray` as its own neutral grey. The
  `Separation` `/Black` and the image's `DeviceN` `[/Black]` both revert to a `DeviceGray` alternate, because a screen has no
  `Black` colourant (§8.6.6.4), so they draw as that grey too. Only the `DeviceN` `[/Black /None]` line art, whose
  alternate is `DeviceCMYK`, matches.
- **What ISO 32000-2 says for a screen.** It does not decide.
  - §10.3.2 says a processor "should establish CIE-based colour specifications for device colour spaces" that do not
    match the device's, and its NOTE leaves the means to the processor ("assumptions made by the PDF processor
    software", among others).
  - §14.11.5 says an intent profile's `AToB` "may optionally be used to remap source colour values to some other
    destination colour space, such as for screen preview". It also says the intent is "for informational purposes
    only".
  - Reading `DeviceGray` as the intent's black ink on screen is therefore allowed, and so is this reader's neutral
    grey.
- **What already binds.** A788 adopted ISO 19005's rule for PDF/A files: a `DeviceGray` with no `/DefaultGray`, under a
  PDF/A CMYK intent, is the intent's black ink in every view (ADR 2712, `output_intent::grey_reading`). A PDF/X file has
  no such rule.

## The options put

1. **Read `DeviceGray` as the intent's black ink in the screen view whenever the page's selected intent is a CMYK
   profile, whatever its subtype** (recommended). The screen then previews the stated press for greys, as it already
   does for `DeviceCMYK`, and PDF/A and PDF/X behave alike. 23.0 passes on screen: the two `[/Black]` spaces reach the
   same grey through their alternates. The build is one condition in `output_intent::grey_reading`. **The cost**:
   every grey in such a file moves on screen. `0 g` text draws as the press's black, about (35, 31, 32) under the
   assumed inks, rather than (0, 0, 0). That is the colour `0 0 0 1 k` text in the same file already draws. A corpus
   census of CMYK-intent files with grey content is the building round's evidence.
2. **Leave the screen neutral.** Nothing moves, and 23.0's screen view is judged n/a, like the overprint patches: it
   tests a CMYK output device, which the press view is.
3. **Option 1 as a host setting** (a "preview the output intent" switch, off by default). It costs a view-state flag
   beside `set_separation_simulation`, and it adds a third view that differs from the press view only in overprint and
   spots.

## What the tree does meanwhile

Option 2's behaviour. 23.0's screen verdict stays **fail** in `doc/gwg-output-suite.md`, because the owner may yet
choose option 1. The checks `grey_check(View::Screen, …)` in `crates/pdf-model/tests/gwg_output_suite.rs` pin the
three failing halves, so an answer is seen to move them.
