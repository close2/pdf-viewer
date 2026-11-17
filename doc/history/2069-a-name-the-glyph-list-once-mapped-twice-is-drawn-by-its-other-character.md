# 2069 — A name the glyph list once mapped twice is drawn by its other character

Session 2069, a fonts round in the gwg worktree. The owner's A888 on Q888; ADR 2952, which amends ADR 2935
section 2.

**The slice.** Give `tools/lgc-pack.py` a fallback table for the glyph names the Adobe Glyph List 1.x mapped to two
characters, so that ADR 2630's Greek document draws its `/Delta` at code 135 from the pack.

**The premise.** It held: the pack of round 2060 had no `Delta` in any Noto Serif face (`DOUBLE_MAPPED` absent,
ADR 2935), and the Greek document was photographed under `as-if-printed` at 2b and 4 for that code alone.

**The table.** From the source, not memory: AGL 1.2 (22 Oct 1998), whose header says "12 glyph names are mapped to
two UVs each" and marks each second value `Duplicate`; Adobe no longer publishes it, so the Wayback Machine's copy
of `partners.adobe.com/asn/developer/type/glyphlist.txt` at 20010215130957, its SHA-256 in ADR 2952 and the
builder. The current specification's README only names the 1.1 section on double mappings. The twelve: `Delta`,
`Omega`, `Scedilla`, `Tcommaaccent`, `fraction`, `hyphen`, `macron`, `mu`, `periodcentered`, `scedilla`, `space`,
`tcommaaccent`.

**Built.** `DOUBLE_MAPPED` and `borrowed_names`: a name takes its `Duplicate` character's glyph only where the
source maps no glyph for its 2.0 character and does map the `Duplicate` one; where it maps neither, the face has
no glyph of that name, as before. Against the pinned sources this adds `Delta` (Greek capital delta's outline) to
the four Noto Serif faces and nothing else; Noto Sans Mono maps U+2206. `PACK_VERSION` 2; the four serif digests
in `lgc_pack.rs`; a test of `Times-Roman` by name alone showing `/Delta` at 135, drawn at the metrics' 612.
`doc/lgc-pack.md`, `doc/todo/76` row 28, state-of-play.

**Conversions.** Two builds wrote the same bytes, the zip too. The Greek document under `as-if-printed` at 2b and 4
is set as text in `PKFGPY+NotoSerif-Regular` where HEAD photographed 23 pages; page 10's "ΔΔCt" was drawn and
looked at; veraPDF 1.30.3 passes both. **Plant**: the old pack under its old digests fails the new test.

**Gates.** Over `pdf-transform`: fmt exit 1 on the new test's line break, then exit 0; clippy `-D warnings
--all-targets` exit 0; nextest `--lib --bins --tests` under `xvfb-run -a` 1228 of 1228 passed, and the five ignored
pack tests and the digest test passed against the built pack; rustdoc `-D warnings --document-private-items` exit 0.
`cd fuzz && cargo check --bins` not owed: no public type or signature changed. Conformance after `git add -N`: exit
101 on this record (a path outside the tree, the gates paragraph unwritten), then exit 0. `raster_golden` exit 101,
950 held and 24 moved: the train's own list at `ce6bc1d8`, page for page (the known 23 and `issue18059`, which
moved at that commit), so this round moves no window page.

**Left.** An italic fixed-pitch request; a font outside the fourteen with no descriptor (ADR 2522); a code the
standard metrics give no width keeps the window's zero advance.
