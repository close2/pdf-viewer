# 1934 — A font drawn only in mode 3 is described by the face substituted for it

Session 1934, a single archive round. ADR 2688; no question; ADR 2689 not used.

**The premise.** Checked with `pdf-archive`'s `one` example and `qpdf --qdf`: `scan-bad` (part 4)
has a `Type1` `Helvetica` with none of the four entries, shown once under `3 Tr`; `0000490` (2b)
has `TrueType` `Arial`, `TimesNewRoman` and `CourierNew`, none of the four, under `3 Tr`. It held,
but for one part: the tree ships the fourteen's published *widths* only, not the AFM header
values, so a descriptor's other numbers cannot come from them.

**The construction (ADR 2688).** `archive/fonts/described/undrawn.rs`. The font is given
`/FirstChar`, `/LastChar`, `/Widths` (the advances this reader lays it out by) and an indirect
`/FontDescriptor` with no program. Table 120's entries are the shipped face's, measured as ADR 2146
measures it; `/Flags` is Table 121 read off that face; `/FontName` is the `/BaseFont`. §9.8.1 says a
descriptor's metrics are for selecting a substitute when the program is unavailable, and the
shipped face is the substitute this reader selects, the same on every machine. Proved before it is
written: all 256 codes advance bit for bit as before and read as the same text. ADR 2686's stated
interpretation, extended; `layout-from-this-viewer` is not charged, as nothing is drawn or embedded.

**Refusals kept, by name** (none reached in the census): no `/BaseFont` name, no shipped face, a
face not measurable, an advance or text that would change, no spare number, ADR 2686's own two.

**Text extraction.** For all 28 outputs of the seven (shipped profiles, 2b and 4): `pdftotext
-raw`, `pdftotext -layout`, `mutool draw -F txt` and `mutool draw -F stext` (1 170 552 characters,
with positions) are identical to round 1932's outputs, and `pdftotext` to the sources.

**Tests.** The refusal test is replaced by
`a_standard_font_drawn_only_in_mode_three_with_no_entries_is_given_a_descriptor_at_four` and
`a_truetype_font_drawn_only_in_mode_three_with_no_entries_is_given_a_descriptor_at_both_parts`.
Planting the refusal back failed both.

**Census.** Against round 1933's runs. Shipped: 16 new outputs, the seven named above (`0000490`
at 2b and 4, the six at 4, both profiles); none gone; the other 5028 identical once dates and
identifiers are masked. Stating: the same 16, and 4512 identical. R10 goes from 11/17 to 10/10.
veraPDF 1.30.3 passes all 32 new outputs.

**Gates.** fmt and clippy `-D warnings --all-targets` (`pdf-transform`): exit 0. nextest of
`pdf-font`, `pdf-archive`, `pdf-transform`: 1737 passed, exit 0. Archive tier (`corpus`,
`archive_corpus`, `witnesses`): exit 0 each; both censuses exit 0. `cargo test -p conformance`
after `git add -N`: exit 0. `state.sh docs`: exit 0.
