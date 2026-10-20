# 2067 — A property list's `/Lang` inside a content stream is respelled in place, and a removed `FileAttachment` keeps its file

Session 1615, the archive slot of batch ninety-seven. Status: **accepted**, **built**. Code:
`crates/pdf-transform/src/archive/languages.rs` (`malformed_inline`, `lang_of_property_list`),
`crates/pdf-transform/src/archive/hexadecimal.rs` (`streams`, `repaired`, now shared). Builds: ADR
2066 (what is written), ADR 1176 (the first content-stream rewrite), ADR 1120 and
`doc/questions/A65` (mark provenance is the fence's test), ADR 0816 (the fence). Clauses: ISO
32000-2 §14.6, §14.6.2, §14.9.2.3, §7.3.4.2; ISO 19005-2 section 6.7.4.

## 1. The question

A property list written into a `BDC` or `DP` operator's operands (§14.6.2's first form) is no object:
it is bytes in a producer's content stream, and every witness of the element-and-property-list row
that places its `/Lang` in a property list places it there (`6-7-4-t01-fail-c` and the PDF/UA
corpus's twelve `t29`/`8.4.4` files). So ADR 2066's answer reaches a content stream or it reaches
nothing.

## 2. Decision

**The value token is respelled and every other byte of the stream crosses as it was.** The lexer
decides what a token is, as it does for ADR 1176's repair, and `pdf_model::inline_image::scan` skips
an inline image's samples; a property list is the dictionary that is the second operand of `BDC` or
`DP`, and only its own `/Lang` is read — one nested a dictionary down is not the list's. The span of
the value is replaced by a literal string (§7.3.4.2's three delimiters escaped), the stream is
written back as one `FlateDecode`, and a stream ADR 1176 also repairs is respelled from the repaired
bytes, so the one stream written carries both.

**This is on the near side of ADR 0816's fence.** The fence's test is the provenance of the marks
(ADR 1120): §14.6 makes marked content information *about* the content it brackets, and the only
property-list entry a clause reads while drawing is `/OC`'s optional content membership (§8.11.3.2),
which this does not touch. No operator is added, removed or moved and no mark can change; ADR 1176
made a value-preserving respelling of a content stream admissible, and this one changes a value no
rendering reads. The 32 converted witnesses are byte-identical to their inputs at 72 and 720 dpi.

## 3. Limits, each loud

An operator's operand list past 65 536 tokens is not examined, and a property list split across two
elements of a `/Contents` array is not read whole; either leaves the `/Lang` in place, and the third
stage's validation of the output refuses the document by name. A stream that will not re-encode, or
whose `/DecodeParms` names another object, refuses the row with its own sentence.

## 4. `annotations/file-attachment-only-in-embedded-file-files` — the file stays in the name tree

ISO 19005-4 section 6.3.1 confines `FileAttachment` to PDF/A-4f. At PDF/A-4 and PDF/A-4e the
annotation goes in the subtype rows' removal (`Loss::ForbiddenAnnotation`, one population with `3D`
and `RichMedia`, `pdf_archive::file_attachments_outside_their_flavour`), and the file specification
its `/FS` names is filed in the catalog's `/EmbeddedFiles` tree — the one writer of that tree,
`super::attach`, gains a filing of an existing specification beside its keeping of new bytes —
under the specification's `/UF`, else `/F`, made unique by the annotation's object number only where
the key is taken. Nothing is written for the file but the key; nothing is added to the catalog's
`/AF`, because nothing new is associated. **ADR 1014's appended page is not needed**: every part 4
flavour admits an embedded file where it is (at plain 4 under section 6.9's own rule, already asked
of the input), so the page would hold nothing the tree does not. The icon's marks are the one thing
that would go, and `preserve` with `placement = "append"` keeps them through ADR 1123's relocation,
as at the subtype rows. `keep-attachment = false` is refused by name: a second loss nobody asks for.

Evidence: `6-3-1-t01-fail-f` (veraPDF not compliant before at 6.3.1) converts to 4 and 4e with
`--authorise forbidden-annotation`, veraPDF 1.30.3 compliant, its `File.pdf` listed from the tree.
Under `as-shown` the icon is kept: the same marks at the same place, the page differing from the
input's in 47 edge pixels at 72 dpi (at most 17 of 255) and 477 at 720 dpi — the relocation's
coverage, not this round's construction, which the 3D poster's witness does not show.
