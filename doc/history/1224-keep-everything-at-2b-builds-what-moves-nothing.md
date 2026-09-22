# 1224 — keep-everything at 2b builds what moves nothing

The corpus-named slot, on `doc/todo/66`: the unbuilt answers `Configuration::unbuilt` printed for
`keep-everything` at PDF/A-2b, taken in the order printed.

`quorra-transform archive --remedy-sites --to 2b --config doc/profiles/keep-everything.toml`:
**before**, "30 of 77 sites answered with a remedy not carried out yet"; **after**, "16 of 77".
The same command's site total went from "47 of 77 sites not built yet" to "44 of 77".

Built, each with a fixture that fails the site, converts, and validates clean under `pdf-archive`:

- **Showing a hidden annotation** (ISO 19005-2 section 6.3.2): `preserve` keeps it and writes
  `flags_permitting`'s `/F`; the report names each shown (ADR 1285).
- **A reference XObject's proxy** (section 6.2.9.2): the `Ref` goes, the form stays, §8.10.4.1's
  own reason; the report names file and page (ADR 1285).
- **A published CMap embedded** (section 6.2.11.3.3): Adobe's program byte for byte, refused by
  name where it builds on an unlisted CMap, uses an operator §9.7.5.4 forbids, or describes another
  collection than the font's (section 6.2.11.3.1) (ADR 1286).
- **The write-mode disagreement** (section 6.2.11.3.3): a `supply`, `program` or `stream` (ADR 1286).
- **The three packet rows** were built already; the profile's `prefer` key was one no reader read.
  They are `original = "page"`, and `attach` at 4f and 4e.

Withdrawn at the four targets that hold no file, by the profile's own *else stop* and ISO 19005-2
section 6.8: six rows whose only keeping is an attachment. The three halftone rows were **not**
withdrawn — the catalogue names a part 2 preserve for them (an `xmpMM:History` record), buildable
and unbuilt. `fonts/cmap-uses-only-predefined-cmaps` **cannot** take the shipped-CMap answer:
embedding what a chain names leaves the reference off the list. It is now a configuration error.

Left at 2b: encryption's `record = ["page"]` (three sites), a supplied destination profile (three),
ICC `via = "alternate"` (two), the halftone history record (three), a PDF/X profile reference
(one), JPEG 2000 to Flate (three — the tree's decoder returns eight-bit rasters, which is not a
lossless transcode) and the extension-schema field `supply`, which ships an empty table.

Two findings: `Outcome::Failed`'s `places` is bounded, so a remedy over findings checks `total`
first; and the chain site's validator reads only `/UseCMap`, never the program's `usecmap`, so the
converter reads the program — the stricter half of the clause's second paragraph.

Gates: tier 1 clippy/nextest on pdf-transform, pdf-archive, pdf-font and `cargo test -p conformance`;
tier 2 `archive_corpus` and `cross_check` behind the lock, both exit 0.
