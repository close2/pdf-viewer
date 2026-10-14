# 1916 — The fourteen are kerned by the pairs Adobe published for them

Session 1540. Status: **accepted** and **built**. Builds `doc/questions/A308`'s "Owes" line; ends
the one property ADR 1682 section 4 left said for §9.6.2.2's fourteen, which ADR 1682 is not
edited for. Context: ISO 32000-2 §9.6.2.2, §9.4.3, §12.7.4.3, Table 109; XFA 3.3 chapter 27
(*Kerning*, pages 1203 and 1204), cited by section; Adobe Technical Note #5004, *Adobe Font Metrics
File Format Specification*, version 4.1, section 9.2 (*Pair-Wise Kerning*), fetched for this round
from Adobe's published copy and not held. Code: `data/standard-fonts/afm/`,
`crates/pdf-font/build.rs` (`published_pairs`), `crates/pdf-font/src/pairs/published.rs`,
`crates/pdf-font/src/pairs.rs` (`Source::Published`), `crates/pdf-font/src/loading.rs`
(`LoadedFont::pairs`, `pair_adjustments`), `/NOTICE` section 1c.

## 1. Whose numbers

§9.6.2.2 names fourteen typefaces a document may use without embedding them. Their metrics were
published with them, and Adobe's Core 14 AFM files state each face's pairs as `KPX` lines — 2 705
for Helvetica and its oblique, 2 481 for the bold pair, 2 073, 2 242, 2 321 and 2 038 for the four
Times faces, none for the Couriers, Symbol and ZapfDingbats. They are the typeface's facts, of the
same kind as the widths ADR 0007 already takes from Adobe's numbers, and the owner's answer reads
the notice that ships with them as attribution conditions a NOTICE file meets.

## 2. Where they are held, and what the binary carries

The fifteen files are held in `data/standard-fonts/afm/` as Adobe's own server publishes them, byte
for byte (`PROVENANCE.md` names the URL, the archive's digest, and why PDFBox's copies were not
taken: their line ends were normalised). `build.rs` reads them and writes a table into `OUT_DIR`:
the `KPX` lines and nothing else, glyph names numbered, pairs sorted, and a face whose pairs repeat
another's sharing its range. That table is a **modification** of the files and its head says so and
carries every file's notice line, which is `MustRead.html`'s third condition; `/NOTICE` carries the
paragraph unmodified, its fourth. The table holds no pointer, so it costs the loader no relocation
at launch (ADR 1708's argument); `viewer-ui/tests/notices.rs` checks the paragraph against the file
itself, each `.afm` named, and `afm/SHA256SUMS`.

## 3. Choices, each named

1. **The fourteen by the clause's own names only.** A non-embedded simple font whose `/BaseFont` is
   one of §9.6.2.2's strings (`standard::is_standard_name`, `CourierBoldOblique` read as Adobe's
   `Courier-BoldOblique`) is kerned by its file. A clone the substitution accepts for the same
   advances — `Arial`, `TimesNewRoman` — is another typeface whose pairs are its own, so it keeps
   `NoPairs::NotTheDocuments`. The widths take the wider reading because a page cannot be laid out
   without them; a run can be laid out without kerning, and the narrower reading puts no other
   typeface's numbers on it.
2. **Not an embedded program.** A document that embeds a face is kerned by what the program states
   (ADR 1682); a bare CFF or Type 1 program named `Helvetica` states none and is said as before.
3. **A face whose file states no pairs is kerned by none, and nothing is said**: that is its
   published metrics carried out, not a property left undone (trap 11).
4. **By the glyph name each code selects** (§9.6.5), which is what a `KPX` line names; a code whose
   name no pair states pairs with nothing.
5. **The amount is the left-hand glyph's**, in thousandths of an em: the kerning vector moves "the
   second character relative to the first" (5004, section 9.2), the files are writing direction 0,
   and a pair read right to left takes the pair of its two glyphs in display order, as the `kern`
   table's does in ADR 1682 section 2.

## 4. What it costs, and what moved

Nothing at load: the table is `static` data, read only when a run asks for `kerning-mode:pair`.
The binary grows by the table — 13 860 pairs of six bytes and the names. No corpus page asks for
the property (ADR 1696's census), so no page's pixels are predicted to move, and the six arms are
the test of that prediction. §12.7.4.3, §12.7.5.3 and §12.5.6.6 move from `partial` to
`implemented`, and their headings §12.7.4, §12.7.5, §12.5.6, §12.7 and §12.5 with them (ADR 1599).
