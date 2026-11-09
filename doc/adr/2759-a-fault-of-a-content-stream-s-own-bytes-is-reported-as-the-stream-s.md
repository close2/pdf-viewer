# 2759 — A fault of a content stream's own bytes is reported as the stream's

Session 1974, a single archive round.
Status: **accepted and built** in `pdf-model` (`content/report.rs`: `Unsupported::ColourOperands`,
`Unsupported::MalformedInlineImage`, three more `StreamSyntaxFault`s; `content/colour.rs`,
`content/pattern/shading.rs`, `content/run/steps.rs`, `content/run/operands.rs`,
`content/run/xobject_ops.rs`, `inline_image.rs`), with `viewer-core/src/report.rs`'s sentences.
Context: ADRs 2516 (decision 5), 2618; `doc/questions/A694` item 3; ADR 2760 is the archive's half.

## 1. What the premise was, and what the documents showed

The brief read R03 and R08's "a page this reader does not draw whole" as this reader's gaps against
§7.4, §8.7 and §8.9: a filter, an image or a shading shape. The census names seven documents, each
refused at both parts under `as-if-printed` and `state-every-loss`: six govdocs `error_set_2` files
(`150277`, `219789`, `427669`, `492484`, `504076`, `507676`) and pdf.js's `poppler-90-0-fuzzed`.
**None of the seven holds a gap of that kind.** Every page the gate stopped has a `/Contents` whose
Flate data fails its own RFC 1950 check value (Python's `zlib` rejects the same streams), and the
reports that stopped it are what the window met in the bytes that decode: an `sc` with 2, 6, 8 or 27
operands under `DeviceRGB`, `sh` naming `/` that no `/Shading` defines, `BI` followed by a number
where a key belongs, an inline image whose pairs state no `/W`, a `Do` with no name, `EI` with no
`BI`, a stream ending after `W`, a bare `true`. ADR 2618 decision 1 already bakes such a stream in;
what held the pages was that the interpreter reported several of these consequences as
`Shading` or `Image` — "a shading or image this reader could not build" — or, for `true`, `EI`, `W`
and `Do`, as operators the fence's survey did not list.

## 2. The decision

Each is reported as the fault of the stream that it is, and the drawing does not change:

1. **A colour operator given a count no space takes** (§8.6.8, Table 73: "The number of operands
   required and their interpretation depends on the current stroking colour space") is
   `ColourOperands { stroking, given, takes }`, the class of `OperandShortfall`. A space this reader
   could not build is reported where `cs` selected it, so this report never stands for one.
2. **`sh` naming no shading** is `MissingResource { category: "Shading" }`, the report `Do`, `scn` and
   `gs` already make for §7.8.3's undefined name.
3. **An inline image not written as §8.9.7 writes one** — a token where a key belongs, a value that
   does not parse, a stream ending inside it, or no `/W` or `/H`, which Table 91 marks required — is
   `MalformedInlineImage`. A value past a parser bound is split out as `InlineImageError::PastABound`
   and stays `Image`, with no terminator found and data past the lookahead, and so does an image
   `XObject` with no `/Width`: that one is an object the file holds, not the stream's syntax.
4. **Three sentences join `StreamSyntaxFault`** (ADR 2618 decision 5): `Do` with no name (Table 86),
   `W` or `W*` the stream ended before painting (§8.5.4), and `ID` or `EI` with no `BI` open (§8.9.7:
   "an ID operator shall only appear between a BI and its balancing EI"). Each carries white space,
   so none is an operator's spelling.
5. **`true`, `false` and `null` are operands** (§7.3.2, §7.3.9) wherever they stand in a content
   stream, as `pdf-archive`'s survey already reads them; the interpreter called them operators nobody
   lists and emptied the operands before them. Operators read their operands from the back (ADR
   0302), so a valid stream draws the same.
6. **A number past f32's range is no number.** `number_at` refused an f64 infinity and narrowed a
   finite f64 past f32's range to an infinity; it now refuses both, so the operator declines that
   operand as it declines one of the wrong type. Annex C's Table C.1 bounds a conforming real inside
   that range. `150277`'s page 57 put an infinity in a path, and the CPU backend refused the page.

## 3. What it costs

Two variants on a public enum, and an `#[expect(clippy::too_many_lines)]` on `viewer-core`'s
`describe`, whose arms are one sentence per kind. The corpus partition (`tests/corpus.rs`) classes
both new variants `TheFile`, where `Shading` was `ThisReader`: that is the correction, not a side
effect. Declining an out-of-range operand is as silent as declining a string where a number belongs;
a report for an unreadable operand of either kind would be one change for both, and is not made here.

## 4. Measured

The pdf-model test `stream_faults_by_name.rs` (six tests) fails whole against the reader before
this change. Drawn at 72 dpi, the sources of the two documents ADR 2760 frees (`504076`, `492484`)
are identical before and after on all 33 and 32 pages.
