# Specification audit of ISO 32000-2 §9.2 to §9.4 (fonts' use, text state, text objects) — round 2161

The project's seventh specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128 to
2155 ([`2155-specification-audit-8-4-8-5.md`](2155-specification-audit-8-4-8-5.md)). Baseline: main
at `7e990758`. Evidence: [`2161-specification-audit-9-2-9-4/`](2161-specification-audit-9-2-9-4/)
— `findings.json` (2038's schema) and `expected-failures.json` (one failing test per behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §9.2.1 to §9.2.4, §9.3.1 to §9.3.8 (Tables 102 to 104) and §9.4.1
   to §9.4.4 (Tables 105 to 107) was read in `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 7160 to
   7627. §9.4.4's formulas are not decoded there; the combined displacement and `Trm` were read
   from the PDF. Errata Collection 3's addition to §9.4.2 — `q` and `Q` inside a text object push
   and pop `Tm` and `Tlm` (ADR 0421) — is honoured and was not re-derived.
2. 2038's findings on these clauses were read first and are not repeated: D09, D62, T22, T58,
   T79, T100, T115, T146, all closed.
3. The interpreter was traced through `pdf-model`'s `content/run/text_ops.rs` (Table 103's and
   Tables 105 to 107's operators), `content/text/show.rs`, `placement.rs` (`Trm` and §9.4.4's
   step), `glyph_painting.rs` and `paint.rs` (Table 104's three operations), `end_object.rs`
   (`ET`'s clip and §9.3.8's group), `type3.rs`, `readback.rs`, `run/steps.rs` and
   `run/operands.rs` (how an operand reaches an operator), and `pdf-font`'s
   `Code::takes_word_spacing`. The backends were read where text could differ from a path: a
   glyph reaches `render-cpu`, `render-raster` and `render-gpu` as an ordinary fill or stroke of
   an outline under its `Trm`, so §9.2 to §9.4 decide nothing a backend answers on its own.
4. The lens was the earlier audits' habits: a rule keyed on a name applies wherever the name is
   read (a text object's end, wherever a text object ends); a parameter has a type as well as a
   place (Table 103's operands, `TJ`'s array); a value has a reader on the way out as on the way
   in (the readback that reads the positions the glyphs were drawn at).
5. Each candidate was probed end to end — content stream, interpreter, display list, readback —
   and each B finding has a failing `*_spec_audit` test with a control that passes on main, in
   `pdf-model/tests/text_objects_spec_audit.rs`, a module of `text_suite` (ADR 3086). No
   portable-suite test was added: every finding is about a stream the clause forbids (a nested
   `BT`, a show outside a text object, an operand of the wrong type) or about extracted text, and
   `portable-conformance/` states what a valid file must show.

What holds, probed and passing on main, is not listed one by one. Among it: §9.4.4's step in both
writing modes — `Th` scaling `Tc`, `Tw` and the `TJ` number only in horizontal writing, a `TJ`
number subtracted from `ty` in vertical; a negative `Tz` and a negative `Tfs` mirroring glyph and
advance together; `Tw` on the single-byte code 32 only; `'` and `"` setting `Tw` and `Tc` before
the move and keeping them; `TD`'s leading and `T*`'s; `BT` resetting both matrices; modes 3 and
7 advancing `Tm`; clip outlines accumulated across a `q … Q` inside the text object and applied at
`ET` beneath the clip in force there; a clipping text object that showed only a space clipping
nothing; a Type 3 glyph's own text object leaving the outer font, spacing, scaling and mode
untouched; `TK` ignored inside `BT`/`ET`. Two readings were
checked and left. Table 103's "Zero sized text shall not mark or clip any pixels" is read as *no
clip from it* — a mode 7 text object at size 0 clips nothing — where a zero `Tz` or a singular `Tm`
shows glyphs with degenerate outlines and clips everything after `ET`; §9.3.6 says outlines shown
are a clip, and only the size has its own sentence. And §9.4.3's "Showing multiple glyphs with one
invocation ... shall produce the same results" is read as being about the marks: `100 Tc (AA) Tj`
reads `AA` and `(A) Tj (A) Tj` reads `A A`, which NOTE 2 ("the text that is extracted can vary
depending on the grouping of glyphs into strings") allows.

## Findings

Five behaviour gaps. In order of how much a person would see:

- **2161-B01** — a second `BT` before an `ET` discards what the interrupted text object owes at its
  end: the outlines a clipping mode accumulated never become a clip, and a translucent mode 2 or 6
  glyph loses §11.7.4.4's knockout group and is drawn as a fill and a stroke composited apart. An
  unclosed `BT` is reported for the same loss; a nested one says nothing.
- **2161-B02** — `TJ` reads every pending operand as an element: a leftover number before the `[`
  moves the text, a string outside any array is shown, and a string inside an array nested in the
  operand is shown. `set_dash` already splits the flattened operands on the brackets; `TJ` does
  not.
- **2161-B03** — a text state operand of the wrong type is dropped in silence. `/F1 (x) Tf` as the
  first `Tf` draws nothing and reports nothing — ADR 0303's failure, reached by a type rather than
  by a missing digit — and after an earlier size it keeps that size for the new font. `1.5 Tr` and
  `/X Tr` keep the old mode in silence where `-1 Tr` is reported; `Tc`, `Tw`, `Tz`, `TL` and `Ts`
  take a string the same way.
- **2161-B04** — a show after `ET` is placed by the text matrix `ET` discarded (`BT 50 50 Td ET
  (A) Tj` draws at 50, 50), and a text-showing or text-positioning operator outside a text object
  is not reported.
- **2161-B05** — a Type 3 glyph whose description holds a text object puts a word break into the
  page's text: `read_back` is gated on `glyph_depth`, the readback cursor is not, and two glyphs
  exactly followed by an `A` read `aa A`.

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `9.4.1` (B01: its note
says a second `BT` "restarts the text object"; B04; B05), `9.3.6` (B01, B03), `11.7.4.4` (B01),
`9.4.3` (B02, B04), `9.3.1` (B03: its note says a malformed size "is a reported loss").

## Suggested fix groups

1. **`pdf-model/src/content/run/text_ops.rs` with `content/text/end_object.rs`** — B01 and B04:
   one decision on a text object's bounds. A `BT` inside an open object ends it as `ET` would (or
   keeps it open) and is reported; `ET` discards `Tm` and `Tlm`; a showing or positioning operator
   outside a text object is reported, and if it is drawn that is a written choice, not a stale
   matrix. The `BT` comment and the `9.4.1` row corrected with it.
2. **`text_ops.rs`'s `TJ` with `run/steps.rs` and `content.rs`'s `set_dash`** — B02: one reading of
   a flattened array, the last bracketed run, shared by `d` and `TJ`; what stands outside it is a
   leftover operand, and an element that is neither a string nor a number is reported.
3. **`text_ops.rs`'s `run_text_state_op` with `run/operands.rs`** — B03: an operand of the wrong
   type for Table 103 reported beside `OperandShortfall`, and a `Tf` whose size is not a number
   leaving the size unset rather than the previous one under a new font. 2155-B04's typed reading
   of a dash array is the same habit one family over.
4. **`content/text/readback.rs` and `content/text/type3.rs`** — B05: the readback cursor and the
   word-gap state saved and restored around a glyph description's run, or gated on `glyph_depth`
   as `read_back` is.
