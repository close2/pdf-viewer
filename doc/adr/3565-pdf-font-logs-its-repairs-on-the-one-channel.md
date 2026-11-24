# 3565 — `pdf-font` logs its repairs on the one channel: the program, the maps, the widths

Session 2216, the silent-repair campaign's third fix round. Status: **accepted**. Context: ADR 3477
(the decision), ADR 3505 (the channel as built), ADR 3541 (`pdf-model`'s classes); review 2194's
`pdf-font` inventory (F001–F086), B05, T03 and T04; ISO 32000-2 §9.6.2.1, §9.6.5.1, §9.7.4.3,
§9.7.5.3, §9.9, §9.10.3.

## Context

`pdf-font` reads a malformed font as far as it can: a `loca` table rewritten, a collection cut to
one face, a `/ToUnicode` that stops in the middle of a section read as the part before, a `/W` read
to its first bad entry. Only the substitutions reached the validator (`is_substituted`, the
program's refusal); review 2194-B05 showed a TrueType program whose `loca` format was wrong, or
which was cut in half, passing every font row.

## Decision

- **`pdf_font::repair::CLASSES`**, 16 classes declared as `pdf_syntax` and `pdf_model` declare
  theirs (`font/…`, a clause, what was read instead): the program's seven (`sfnt-loca-extent`,
  `sfnt-loca-format`, `sfnt-loca-order`, `sfnt-table-past-the-program`, `collection-cut-to-one-face`,
  `font-file-subtype-disagrees`, `cff-font-dicts-rewritten`) and `program-unreadable`; the maps' and
  dictionary's seven (`to-unicode-not-a-stream`, `to-unicode-damaged`, `cmap-damaged`,
  `widths-malformed`, `cid-widths-malformed`, `differences-malformed`, `encoding-name-unknown`); and
  one bound, `to-unicode-past-a-bound`.
- **The loader records, on the document it already holds.** `pdf-font` depends on `pdf-syntax` and
  every loading path is handed the `Document` (that is how it resolves a reference), so no caller
  is asked to record for it: the viewer's cache, the validator and the converter all load fonts,
  and a class only some of them recorded would reach the log by accident. The byte-level parsers
  stay pure functions of their bytes and say what they repaired in their return value
  (`repaired_font_program_noting`, `ToUnicode::damaged`, `CMap::damaged`), as the lexer does; the
  loader that knows the stream records it. The push is `#[cold]` behind the test that found the
  departure, so a well-formed font costs nothing new.
- **The site** is the stream or entry that holds the departure where the file writes it as an
  indirect object, else the font's descriptor, which Table 109 and Table 117 require to be indirect
  (`repair::site`); a program's repairs are always at its stream.
- **One repair became a correction too.** A `head.indexToLocFormat` of 0 or 1 that `loca`'s own
  length and last entry both contradict, while the other width satisfies both, is corrected as the
  byte-swapped value already was, and the format is asked before the extent repair, which reads the
  stated width and would otherwise lengthen a short `loca` into the next table (B05's witness).
- **Not classes**, each with its clause beside the code: the Type 1 trailer restored where
  `/Length3` is 0 (§9.9.1 makes it the processor's job), an absent `/ToUnicode`, `/Widths` or
  `/Differences` (optional entries), and the CFF writer's re-readings (`nominalWidthX` 0 and no
  `Subrs` are Technical Note #5176's defaults). A stream whose data stops short stays the decoder's
  `syntax/stream-data-damaged`; a program of no bytes stays the font row's own reading.
- **The validator loads every font** when it reads the whole file (`Examination::read_whole`,
  `table::fonts::load_every_font`), a `CIDFont` through its Type 0 parent, and a refused program
  through the substitute the viewer draws, so the log is the whole file's.

## Consequences

- B05's two tests pass. T03's sites all log or say why not; T04's CFF Font DICTs and collection log.
- Left, by name: the drawing-time tolerances with no document (F061–F063: an outline that does not
  evaluate, a hinting instance that cannot be built, a one-point contour); the numeric fallbacks of
  the descriptor (F044–F049, F064–F068: `/DW`, `/DW2`, `/MissingWidth`, `/Flags`, `/Ascent`,
  `/FirstChar`); the TrueType `cmap` routes (F054–F056) and the program-side table readers
  (F074–F086), each a later round's.
