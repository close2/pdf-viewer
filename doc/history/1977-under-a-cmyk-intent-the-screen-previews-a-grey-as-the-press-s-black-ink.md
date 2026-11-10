# 1977 — Under a CMYK intent the screen previews a grey as the press's black ink

Session 1977, the sixth round of the GWG lane. ADR 2765 builds A810. ADR 2766 and Q812 were not needed.

**Deficit 7, screen half, built.** The owner chose Q810's first option. `output_intent::grey_reading` now answers
`IntentBlack` whenever the selected intent's profile has four components, in either view and whatever its subtype.
On the press §10.3.2 binds (ADR 2758); on screen §14.11.5's optional `AToB` preview is the owner's choice. ADR 2712's
other two conditions (the `GTS_PDFA1` subtype and `pdfaid:part`) fall away, so drawing a page no longer parses the XMP.
`soft_mask::content_grey` (ADR 2734) and the mask request's `grey` field were redundant with the new reading, and were
removed. 23.0's `DeviceGray`, `Separation` `/Black` and `DeviceN` image halves now match `DeviceCMYK` on screen.

**Tests and plants.** In `pdfa_grey.rs`, two new tests and one extended:
- `under_any_cmyk_intent_a_grey_draws_on_screen_as_the_intents_black_ink`;
- `under_an_rgb_intent_a_grey_is_unchanged`;
- `a_default_grey_outranks_the_rule`, now over PDF/X as well.

`colour_paths.rs`'s simulated-press test pinned the old neutral grey on screen, and now asserts the new reading. Three
plants, each applied and reversed by a script:
- the rule switched off fails the first test and five PDF/A ones;
- the CMYK condition dropped fails the second;
- the rule moved above `/DefaultGray` fails the third.

The harness's 23.0 screen checks flip to pass, and a `DeviceGray` image half is added.

**The census.** 67,445 files were walked (the tracked corpora, `doc/pdf.js` and safedocs). 773 are under a CMYK intent,
718 of them PDF/X. Page one was drawn before and after on the CPU: 43 pages move, all PDF/X. The mean peak is 32 levels,
and the mean changed area 5.5 %. The largest movers are `0 g` text and a scanned grey page. The intent census took 76
minutes on a loaded machine (load 130 to 230).

**Verdicts.** Screen: 35 pass, 16 n/a. Press: 51 of 51.

**Gates.** fmt, clippy, nextest (`pdf-model`, `pdf-colour`, `pdf-transform`, `pdf-archive`), the GWG harness,
conformance and rustdoc all exit 0. Tier 2 and `state.sh docs` were still queued on the heavy-walk lock at the close.
Both tracked CMYK-intent files are `raster_golden` pages, and neither moves in the census.

**Unfinished.** Tier 2's walks and `state.sh docs`, which the merge runs.
