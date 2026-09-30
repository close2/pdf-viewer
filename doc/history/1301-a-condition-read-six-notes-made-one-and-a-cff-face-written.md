# 1301 — A CMYK device's condition read, six notes made one, stale crate names checked, a CFF face written

The ledger slot of batch forty-six. ADRs 1437, 1438.

**§10.3.2 against §10.4.2.3** (ADR 1437). "If the native device colour space is CMYK, then
converting colours in the DeviceGray colour space to that CMYK should follow the method described
in 10.4.2.3" is the only sentence that sends an ICC enabled processor back to a classic method, and
its condition is the device's. No output here is CMYK: the screen is RGB (§10.2), print hands over
an RGB raster (RFC 0004), and `archive` writes CIE definitions, not device values. So the sentence
is inapplicable on its own condition, like §10.6. §10.3.2 stays `implemented` and its note now says
so. §10.4.2.3 stays `departed`: the only grey-to-CMYK conversion here is into a blending space,
which is not a device's native space, so ADR 1194 stands. §10.4.2.5 stays `departed`: no sentence
sends this processor to it, and §8.7.4.4's "standard conversion formulas described in 10.4" takes
in the ranking. The note now names the route in order: `/DefaultCMYK`, then the output intent, then
`CMYK_CORNERS`.

**Six notes rewritten as one reading each** (§11.3.4, §12.3, §12.3.5, §12.3.5.1, §12.7.5.3,
§12.7.8.3.3). Statuses unchanged. Every quotation was re-checked against `doc/md/` and every name
checked to exist. `tools/superlatives.py` hits on these rows: 2 before, 0 after. Hits across the
whole ledger: 68 before, 66 after.

**Stale names.** The checker already verified `code =` and `test =` paths (`missing_site`). Nothing
checked the notes. `check_named_programs` in `tools/conformance/src/ledger.rs` now does: a
backticked crate, program or corpus name, or a Rust path's first segment, must exist in the
workspace. It found `render-quorra` in 10 rows, `quorra_scene::` in 6, `quorra-gpu`, and the old
binary names `pdf-viewer`, `pdf-viewer-gtk`, `pdf-viewer-qt` and `pdf-viewer-confined` in 6 rows.
All are renamed to the current names, and the unit test plants `render-quorra`. `mesh.rs`'s
comment now says the patch rows are `departed` (ADR 1217).

**A CFF machine face** (ADR 1438). Table 124's OpenType row: `/FontFile3 /Subtype /OpenType` under
a `CIDFontType0`, with `cmap` beside `CFF `. `pdf_font::embed::cff` writes it whole; the
`CFF ` table is not subset (Duployan face 631 KB of tables; saved file 252 KB after Flate).
`variable_text::cff_font_for_a_file` builds the dictionaries. A CID-keyed face whose charset
renumbers a shown glyph is refused by name. This machine has no Arabic CFF face, so
`machine_cff_face_written.rs` hands the layout Noto Sans Duployan through the provider port. The
saved file passes `qpdf --check`, the port is not asked again, and the page drawn from the file is
byte-identical to the screen; truncating the program fails the test.

Left: CFF subsetting; a CID-keyed CFF face that renumbers; the in-memory machine font still says
`CIDFontType2` for a CFF program.
