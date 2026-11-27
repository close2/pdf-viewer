# 2270 — A damaged `/ToUnicode` is written as read, and the twenty come back

Lane `gwg`, PDF/A: ADR 3570 point 5 built for `to-unicode-damaged`. ADRs 3781 and 3782.

- **The route** (ADR 3781): `font/to-unicode-damaged` joins `fonts/font-maps-read-as-written`,
  and `archive::font_maps::to_unicode` writes the map again from `pdf_font`'s reading — one
  `bfchar` per single, one `bfrange` per span, each code at the width the producer wrote it, the
  producer's codespace ranges that read carried — at the producer's stream object. Proved: read
  back, the map is whole and answers every code either map states, by value and at each width, as
  the producer's did. A map cut by a bound is refused by name. The loss is `font-maps-as-read`
  (ADR 3570's argument covers a destination no reader reads by §9.10.3's UTF-16BE); both shipped
  profiles already authorise it, so no profile changed.
- **`pdf_font`**: `tounicode::Mapping` carries each statement's code width, and `ToUnicode` names
  each entry it read past (`unread`, `unread_count`); the report's restatement line lists them.
- **`cmap-damaged` not built** (ADR 3782): an encoding `CMap`'s codespace decides how strings are
  cut into codes, so a lookup proof does not prove the drawing; it needs a draw proof over the
  shown strings, which the converter does not have. It stays section 5.1's, refused by name.
- **The brief's note number**: §9.10.3 cites Adobe Technical Note #5014; #5411 is ISO 32000-1's
  tutorial in its bibliography. ADR 3781 cites #5014.

- **Census, with the pack, against 2269's**: as if printed 2b pdf.js 929 → 938 (`basicapi`,
  `bug1186827`, `bug1473809`, `issue12963`, `issue18099_reduced`, `issue2074`, `issue2956`,
  `issue5677`, `mixedfonts`), corpora 455 → 456 (`499039`); part 4 951 → 952
  (`issue18099_reduced`); as shown 2b 903 → 911 (the same eight, `issue18099_reduced` refused there
  as before for another reason) and 414 → 415 (`499039`). Per-class document lists: only the
  output-fails class of `conformance/adheres-to-the-base-standard` lost those documents; no class
  gained one. Without the pack, the same lists lose the same documents but `mixedfonts` at 2b as if
  printed, which 2269's run without the pack had converted (929 → 937). veraPDF 1.30.3 over 5523
  outputs (2269's 5503 and the twenty): 40 not compliant, 12 unfinished, every line as 2269's.
- `tools/state.sh --round 2270 launch`: 6 documents, 36 figures banded, 0 outside.
- Ledger: §9.10.3 stays `implemented`, its `code` and `test` gaining the writer and two tests.
- Left, in order: a draw proof for `cmap-damaged` (ADR 3782); a font written directly inside
  another object, which `font_maps` does not reach; the report rendering `Restated::entries`.

**Gates.** Over `-p pdf-syntax -p pdf-font -p pdf-model -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6335 of 6337, the two failures the manifest's §10.5
entries; rustdoc of the five library crates 0; fuzz `cargo check --bins` 0 (`Mapping` changed);
`cargo test -p conformance` 0.
