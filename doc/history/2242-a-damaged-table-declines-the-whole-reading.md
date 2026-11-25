# 2242 — A damaged table declines the whole reading, and a script's view logs what it reads past

Lane `gwg`, the silent-repair campaign's census burn-down, third pass (ADRs 3574, 3597, 3621). ADR 3669.

- **`HELD` 513 → 373.** Nine files to zero: `content/reader.rs` (20, the window's own bounds and
  Table 31's empty page), `xfdf/annotations.rs` (15), `pairs.rs` (15), `crypt.rs` (14),
  `type1_glyphs_given.rs` (14), `cff_cids_given.rs` (12), `view/script_model.rs` (11),
  `image/dct/cut.rs` (9), `rich_text/style.rs` (8). Left held: `substitute.rs` 3, `geospatial/system.rs`
  6, `image/soft_mask.rs` 8, `rich_text/markup.rs` 6, `jpeg2000.rs` 6 — each read as a repair.
- **Declines, not repairs.** `Pairs::read` reads every table layout later consults and declines a
  program where one does not read (the run is then said unkerned); a negative `/Length` is refused
  where it is read (the crypt test that pinned the old leniency now pins the refusal); an XFDF
  `length` that is not a count is a damaged `<data>`, said on `owed`.
- **Fixed against the text**: XFDF's `\ddd` keeps its low eight bits (§7.3.4.2's "high-order
  overflow shall be ignored", which the convention's escape is "just like"); a unit test holds it.
- **Logged**: `/V` joins `syntax/encryption-entry`; the script model logs `/MaxLen`, `/V`, `/F`,
  `/MK` captions, the border width and `/Rect` on the document it holds; new class
  `model/optional-content-group-name-unusable` (§8.11.2.1), section 5.1 alone, carried by the
  whole-file rewrite.
- **Helpers.** Three Sonnet helpers marked the image, rich-text, geospatial and font files; every
  marker was read back, and one (a JPEG 2000 default) was reworded to rest on the function's own
  documentation.
- Left, in order: a soft mask's absent `/BitsPerComponent` and undecodable data; the rich-text
  markup that does not parse; the JPEG 2000 short boxes; the geospatial nameless system and datum;
  `substitute.rs`'s derived weight and width.
- **Census**, with and without the pack, against round 2235's: no line moves. veraPDF 1.30.3 over
  the 5493 outputs written as conforming: 40 not compliant and 12 unfinished, as round 2235's.
- `tools/state.sh --round 2242 launch`: 6 documents, 36 figures banded, 0 outside.

**Gates.** Over `-p pdf-syntax -p pdf-model -p pdf-font -p pdf-archive -p pdf-transform -p
viewer-qt -p conformance`: fmt 0, clippy `-D warnings --all-targets` 0, `pdf-sandbox --bins` 0,
nextest `--lib --bins --tests` (`xvfb-run -a`) 6223 of 6227, the three measurement audit tests the
manifest lists and this record's own gate before this paragraph (`audit-diff.py`: "failed 4,
expected 3", the fourth `records`); rustdoc of the four reader crates 0; fuzz 0;
`cargo test -p conformance` 0 once this record stated its gates.
