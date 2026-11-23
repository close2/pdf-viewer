# 3521 — A repair inside content is told by its site, and section 5.1 is answered by what the repair was

Session 2205. Status: **accepted**; amends ADR 3506's converter consequence. Context: ADR 3505 (the
channel), ADR 3506 (section 5.1 over the log), ADR 0816 (the fence), ADR 3473 (ASCII data cut
before its marker); ISO 19005-2 and ISO 19005-4 section 5.1.

## Context

ADR 3506 put `conformance/adheres-to-the-base-standard` in the converter's `WRITER_EMITS`: the
output is the repaired reading written in the serializer's syntax, and a repair the rewrite cannot
undo was left to the output's own check. Eighteen census documents then ended as an anonymous
"output-fails" with no sentence of their own. Two kinds of repair survive a rewrite: one inside a
content stream, which crosses the verb byte for byte, and stream data a filter decoded only in part,
which the writer copies as it found it. Neither could be told from the log: the content survey
recorded its lexical classes at `Site::Object(page or form)`, the same site as a repair in that
object's dictionary.

## Decision

- **A site of its own, not a marker.** `pdf_syntax::Site::Content(Option<ObjectId>)` is where the
  content survey records what lexing a page's or a form's content stream repaired; `None` is a
  stream its reader holds without an object. A marker beside the class (a second class name, a flag
  on `Repair`) would have made every reader of the log learn two spellings of one class; the site is
  already the place a log says *where*, and "inside content" is a where. `pdf_archive` places it at
  the object as before, so section 5.1's findings read the same.
- **Section 5.1 leaves `WRITER_EMITS` for `REMEDIES`**, as a per-document row
  (`pdf_transform::archive::base_standard`). Read after `pdf_archive::check`, whose whole-file read
  completes the log, its answer is:
  - every repair outside content and outside stream data: the whole-file rewrite, losing nothing;
  - a repair at `Site::Content`: answered where the content does not cross byte for byte — its
    page is photographed, or the run rewrite (ADRs 2638, 2859) writes every content stream of the
    page or form again and the class is one it writes out (`number-salvaged`,
    `delimiter-unmatched`); everywhere else **refused by name** under the fence, one sentence per
    lexer class naming the class, its ISO 32000-2 clause and section 5.1 (a unit test holds each to
    `pdf_syntax`'s name and clause). This is settled after the content rewrites are prepared;
  - `syntax/stream-data-damaged`: the streams found and restated at a loss (ADR 3522);
  - `syntax/stream-data-undecodable`: left to the output's own check. There are no decoded bytes
    to write, and whether the output still carries the stream is another rewrite's to say — the
    census's `issue19484_1` and `_2` are encrypted files whose undecodable stream the conversion does
    not carry, and a refusal before writing refused them though their outputs conform. Where the
    output still fails section 5.1, its refusal now names every class the output's own log holds
    (`declined_output`), so the failure is never only the row's name;
  - `syntax/decode-parms-mismatched`: answered where a restatement re-encodes the stream anyway
    (its damage, or the ASCII row's, ADR 3473's `/DecodeParms []`), since the re-encoding writes one
    entry per stage it keeps; elsewhere refused by name, not built, because restating the stream for
    that alone would write this reader's pairing of parameters to filters as the producer's.

## Consequences

- No document fails section 5.1 in its output without the source's refusal having named why first.
- govdocs1 `475121.pdf`'s cause: its source log was read as `object-unparsable` ×3 and
  `integer-past-i64` ×1 at object 5 — a page, so the integer was in its content stream, logged at
  the page's id. The output re-read the same bytes and logged the same class at the same page; the
  `object-unparsable` objects were gone. Under this decision it is refused by name, the integer
  sentence.
- A repair inside a form `XObject` only photographed pages draw is still refused by name: the form's
  own site is not the page's, and whether the output still carries it is not asked.
