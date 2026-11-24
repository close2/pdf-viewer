# Round 2217 — a font this reader read past is written as read

The archive lane, in the `gwg` worktree, over round 2216's commit (fast-forwarded, not yet on main).
Its subject: the fifteen pdf.js documents round 2216's font classes stopped from converting.

## What changed

- `pdf_font::as_read::program_as_read` (ADR 3569) hands over the bytes the loader drew from — the
  one face, the `loca` repaired, a CFF's Font DICTs made readable, an sfnt's directory laid out
  again without the records past its end — proved to read back with no repair and, for an sfnt,
  to draw every glyph's outline bit for bit at the same advance through the same `cmap`.
- The converter writes it at the program's own object (`archive::fonts::as_read`), refiled by
  Table 124, as the decision `font-program-as-read`, losing nothing; the metrics route restates
  advances in the same bytes. The six program classes breach section 6.2.11.4.1 again, read at the
  rendered fonts' programs; `pdf_archive::program_read_repaired` is the row's reading.
- A row of section 5.1, `fonts/font-maps-read-as-written` (ADR 3570), for `/ToUnicode` not a
  stream, an unknown `/Encoding` or `/BaseEncoding` name and a malformed `/Differences`, answered
  by `archive::font_maps` at the new loss `font-maps-as-read`, which both shipped profiles
  authorise. Each font is proved before it is written; the proof refuses a nonsymbolic TrueType
  font's `/Encoding /NULL`, which this reader does not read as an absent entry.

**The fifteen.** All convert at 2b under `as-if-printed`: by the program route issue11131_reduced,
issue17671, issue2537r, issue3405r, issue7074_reduced (with the metrics route), issue8480 and
issue9278; by the maps route IdentityToUnicodeMap_charCodeOf, bug859204, bug920426, issue3323,
issue4402_reduced, issue7200 and javauninstall-7r; bug1650302_reduced by both.

**Census.** The fifteen outputs under veraPDF 1.30.3: 15 compliant, 0 not. The corpus census with
and without the pack, and veraPDF over its outputs, were queued behind the heavy-walk lock at the
hand-back; their logs are `r2217/census-pack.log`, `census-nopack.log` and `vera.log`.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 over `pdf-font`, `pdf-archive`,
`pdf-transform` and `viewer-qt`; `pdf-sandbox --bins` 0; nextest of the four 0, 2253 of 2253, no
failure against an empty manifest; rustdoc 0 on the three library crates; fuzz `cargo check
--bins` 0; conformance 0.

Left: `to-unicode-damaged` and `cmap-damaged` (a new `CMap` of the mappings that read); the report's
list of each font and entry the maps route restated, and the program route's "0 done" count; a
font written directly inside a resource dictionary, which the maps route does not reach.
