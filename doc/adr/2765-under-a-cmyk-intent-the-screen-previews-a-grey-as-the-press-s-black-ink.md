# 2765 — Under a CMYK intent the screen previews a grey as the press's black ink

Status: accepted. Session 1977. Context: ISO 32000-2 §10.3.2, §10.4.2.3, §14.11.5, §8.6.5.6, §8.6.6.4; ADR 2712 (the
construction, for PDF/A), ADR 2734 (a grey in a `DeviceCMYK` mask group), ADR 2758 (the simulated press);
`doc/questions/A810`; deficit 7 of `doc/gwg-output-suite.md`, patch 23.0.

## The question and the answer

On §10.8.3's simulated press the device is CMYK, and §10.3.2's "If the native device colour space is CMYK, then
converting colours in the DeviceGray colour space to that CMYK should follow the method described in 10.4.2.3" makes a
grey black `1 − gray` alone (ADR 2758). On screen the native colour space is the screen's and the sentence does not
reach. §14.11.5 says the intent's `AToB` "may optionally be used to remap source colour values to some other
destination colour space, such as for screen preview", and this reader already previews `DeviceCMYK` that way; whether
a grey is previewed through the press too is the processor's choice. Q810 put three options; the owner chose the first
(A810): whenever the page's selected intent is a CMYK profile, whatever its subtype, `DeviceGray` with no
`/DefaultGray` is the intent's black ink in the screen view as well.

## The construction

`output_intent::grey_reading` now asks one thing, the profile's component count: four is `GreyReading::IntentBlack`,
anything else `Device`. ADR 2712's three conditions (a `GTS_PDFA1` subtype, a CMYK profile, `pdfaid:part` in the XMP)
reduce to the second, so the XMP is no longer parsed to draw a page, and the `simulated` argument goes with them. Two
pieces became redundant and were removed: `soft_mask::content_grey`, which made a grey in a `/DeviceCMYK` luminosity
group under a CMYK intent the press's black ink (ADR 2734) — the page's reading is now that already — and the mask
request's `grey` field that carried its answer. `/DefaultGray` still outranks the rule (§8.6.5.6), and an 'RGB' intent
leaves the grey this reader's.

## What it costs

Every grey in such a file moves on screen: `0 g` text draws as the press's black, about (35, 31, 32) under the assumed
inks, and not (0, 0, 0) — the colour `0 0 0 1 k` text in the same file already draws. The census below measures it.

**The census** (`tools/bounded.sh --lock --round 1977 --tree 6`; the session worked in
`~/scratch/agent/r1977/`). Of 67,445 files — the tracked corpora, `doc/pdf.js`'s test files and the
safedocs cache — 773 have a catalog whose first profiled intent is CMYK: 718 `GTS_PDFX`, 55 PDF/A. Page one of each was
drawn on the CPU at 72 dpi by HEAD's `render_at` and by this round's. **43 pages move, every one PDF/X**; no PDF/A
page moves, because a file that identifies as PDF/A already drew this way (ADR 2712). Over the 43, a mean 5.5 % of the
pixels change and the mean peak is 32 levels of 255; the largest peak is 61 (a caption set in `0 g`, which the press's black
ink alone draws as a dark grey, as the file's own `0 0 0 1 k` text already was), and the largest area 32 % (a scanned `DeviceGray` page). Of
the two CMYK-intent files in the tracked corpora, neither moves. Page-level intents were not counted; the
interpreter reads them, and they are rare (`output_intent::selected`).

## Tests

`pdfa_grey.rs`: `under_any_cmyk_intent_a_grey_draws_on_screen_as_the_intents_black_ink` (`0.5 g` against
`0 0 0 0.5 k`, in a PDF/X file and an unidentified PDF/A one, by every route), `under_an_rgb_intent_a_grey_is_unchanged`,
and `a_default_grey_outranks_the_rule` over PDF/X too. Three plants: the rule switched off fails the first (and five
PDF/A tests); the CMYK condition dropped fails the second; the rule moved above `/DefaultGray` fails the third. The
GWG harness checks six halves of 23.0 on screen, all passing.
