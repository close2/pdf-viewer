# 1321 — A word typed without its marks finds one printed with them

The HOST-UI round of batch fifty. ADRs 1477 and 1478. No ledger row moved; no question.

**Marks (ADR 1477).** Before: all three windows compared lowered characters one for one, so
"cafe" missed "café", a precomposed "é" missed `e`+U+0301, "كتب" missed "كَتَبَ". §9.10.1 sends
searching to Unicode's "information content", so both sides are now canonically decomposed
(`pdf_font::shaping::decompose`, `UnicodeData.txt` field 5 to its fixed point, Hangul by
arithmetic). A mark (`shaping::mark_class`: `Mn` with a combining class other than zero) the
needle leaves off is skipped; one it states must be printed, the two runs compared in canonical
order. One rule for every script; a class-zero vowel sign (Devanagari, Thai) is a letter.
Tests: `select.rs` four, `fold.rs` two, `headless.rs` one; driven in all three windows.

**Mirror.** `select::axes` takes *right* from the box's own base side, the determinant's sign, so
a mirrored glyph votes as its producer stored it; `headless.rs` drives 1315's three-ways page under
`-1 0 0 1 200 0 cm` (3 found; 0 under the old turn). Found beside it: under a mirroring `Tm`,
`pdf-model`'s `separate_text` puts a space between `TJ`-placed glyphs (gap read along user x).

**Confined refusal.** The Type 3 cycle crosses as pixels (its list outweighs its raster, ADR 0607).
`drive-coverage.pdf`, a thousand stars, crosses as marks and lavapipe's coverage sheet refuses it;
`28-confined-refusal` reads `drawn on the processor … — confined` in the title.

**The ten (ADR 1478).** Popup: pixels of `/C` open against closed. `quorra`'s restrictions card:
driven, its trace states `copy:On`. Print: GTK's `Print` dialogue, the others' "over 3 page(s)".
Reopened form: AT-SPI reads `1 2 3 True Blue` off GTK's and Qt's widgets; `quorra`'s fields
publish no value, so a golden (looked at once, 0 pixels off after; an erased digit is 78).

**Driven** (release, Xvfb, lavapipe):

| windows | works | wrong | not offered | to look at |
|---|---|---|---|---|
| `quorra`, `quorra-confined` | 30 | 0 | 0 | 0 |
| `quorra-gtk`, `quorra-qt` | 57 | 0 | 3 | 0 |

**Gates.** fmt (5 files); clippy `--no-deps` `pdf-font` `viewer-core` (with deps it stops in a
sibling's `pdf-model/src/image/cut.rs`); nextest `pdf-font` `viewer-core` 541; `conformance`;
`launch_path` (`xfa_filled_imm1344e` 1817.7 k, ceiling 1820); both censuses: exit 0.

**Left.** `quorra`'s form nodes carry no value (a wire change, `doc/todo/31`); `separate_text`
under a mirroring `Tm`; a fatha's highlight sliver sits at its kerned origin, not over its bar.
