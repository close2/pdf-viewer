# Round 2210 — `pdf-model`'s repairs on the one channel

The silent-repair campaign's second fix round (ADR 3477, review 2194), in the `gwg` worktree.

## What changed

- `pdf_model::repair::CLASSES`, 25 classes (ADR 3541): the page tree's (catalog, root `/Count`,
  scan, damaged page, `/Kids`, cycle, bound, `/Resources`, `/MediaBox`, boundaries, `/Rotate`,
  `/UserUnit`) at the node or page; the content reader's (`/Contents` entry, bound, inline objects,
  inline image key, colour space and extent) and the interpreter's five (B08) at the content site.
- `ContentReader::for_page` holds its document and logs, on drop and as its issues are taken, what
  reading the page needed, so the viewer's drawing logs as the survey does. A damaged or undecodable
  part stays the decoder's to log (`bomb_giant.pdf`: a bound the reader would have called damage).
- B08 in the validator without the interpreter (ADR 3542): `content::grammar` states Table 50's
  operand lists once, held to `Operator::operand_count`, and the survey's token walk feeds it.
- The viewer says each class's sentence once per document (ADR 3544, `notes::repairs`).
- `MAPPING`: every class to 5.1; the boundary classes to 6.1.13's boundary row too; the two bounds
  to no row by name; the colour class not to 6.2.4.3 (it judges the space the stream selected).
- The converter (ADR 3543): content classes join the fence's sentences; a page entry is left to the
  rewrites and the output's check; a page logged `resources-absent` gets Table 31's `<< >>`.
- Seven fixtures that broke a rule now read are corrected; two tests now expect 5.1's refusal.
- 2194-B07 and B08: seven tests pass; their manifest entries removed; findings `fixed`.

## Trap

A reader's `Undecodable` is two things: data the file damaged and a bound of this reader's. Logging
it as `syntax/stream-data-undecodable` failed `bomb_giant.pdf`'s output on a limit; the decoder,
which tells the two apart, is the one place that logs it.

**Census.** With the pack, against round 2205's: as-if-printed pdf.js 2b 940 to 925, 4 954 to 938,
corpora 2b 441 to 430, 4 473 to 460; 27 documents in all, none newly converted. 19 are refused under
the fence by a content class; 8 fail their output's 5.1, named (a substituted box, a cycle, a part
not a stream). veraPDF 1.30.3 passes 5342 of 5400 outputs; the 46 it fails are round 2205's.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0; `pdf-sandbox --bins` 0; nextest of the six
crates 100, 5856 of 5858 passing, the two failures exactly the manifest's (B05); rustdoc 0 on each;
fuzz `cargo check --bins` 0; `cargo test -p conformance` 0 once a test's `§` was spelled
`\u{a7}` and this paragraph was written.

Left: the outline, structure, XMP and JPX sites (M146–M187); M002, M114, M138 and T05 (round 4).
