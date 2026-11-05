# 1949 — In a PDF/A file under a CMYK intent, a grey is the press's black ink

Session 1949, a single archive round. ADRs 2712 and 2713; the owner's A788.

**The rule.** ISO 19005-2 and -4 section 6.2.4.3, adopted by A788 as Q788's option 2: in a file
stating `pdfaid:part` whose intent is a `GTS_PDFA1` CMYK profile, `DeviceGray` with no
`/DefaultGray` converts by ISO 32000-2 §10.4.2.3 (black `1 − gray`) and renders through the intent.

**The construction.** `pdf-model` decides it with the intent (`output_intent::grey_reading`; the
XMP is read only for a page under a PDF/A CMYK intent) and carries `GreyReading` beside the intent
in `Reading`, `Conversion` and its keys, the soft mask's `/CS`, an imported page and the separation
pass. `ColourSpace::device_family` then resolves `DeviceGray`, after any default, to a
one-component `Separation` over the intent with `Tints::GreyAsBlack`, so fills, strokes, images,
shadings and mask backdrops all reach it. Its initial colour is 0.0, its ink is `1 − gray` (A23's
device branch on the clause's four components), and a `/DeviceGray` luminosity group under the rule
composites in the intent's press. A transparency group stating `/DeviceGray` is not taken (ADR 2712).

**Measured** (ADR 2713). No source corpus document is PDF/A with a CMYK intent, so the rule
reaches no corpus page; 57 of round 1945's 5 105 outputs meet it. Round 1947's six now draw
within 1 to 3 levels of their sources under `as-shown` (from 9 to 25), at both parts.

**Tests.** `pdf-model/tests/pdfa_grey.rs`, 5: a grey by `g`, `G`, `cs` and an inline image, and a
grey shading, draw as `0 0 0 (1 − g) k`; no `pdfaid:part`, or a PDF/X intent, keeps the grey; a
`/DefaultGray` outranks the rule; a grey in a `/DeviceCMYK` mask masks as its ink, and a
`/DeviceGray` mask of a black `/DeviceCMYK` group masks to the paper. Plants: the rule off fails 3;
the mask group out of the press fails the paper; the identification ignored fails the PDF/X and
`pdfaid` test; the rule before `/DefaultGray` fails that test. `drawn_through_the_intent`'s pin is
amended to the archive masking by 0.5, and the converter's sentence says the greys move.

**Census and veraPDF.** Both censuses, rerun: every output of round 1945's `outputs-shipped`
(10 210 files) and round 1943's `outputs-stating` (9 122) is the same bytes but its dates and
identifiers, so veraPDF had no changed output to read.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-colour`, `pdf-model`, `pdf-transform`):
exit 0. nextest of the three: 3622 passed, exit 0. Tier 2: `script_corpus` and `drawn_while_read`
exit 0; `corpus`, `raster_golden` and the engine's `script_corpus` exit 101 with the change and at HEAD,
the same report each; `drawn_ahead_in_strips` 0; archive tier 0. `cargo test -p conformance` after `git add -N`: exit 0. `state.sh docs`: exit 0.
