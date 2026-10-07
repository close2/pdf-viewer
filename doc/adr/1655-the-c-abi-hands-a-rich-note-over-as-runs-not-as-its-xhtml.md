# 1655 — The C ABI hands a rich note over as runs, not as its XHTML

Status: accepted and **built**. Session 1409.
Builds on ADR 1642 (what crosses the boundary is the string *read*), ADR 1654 (the run's spacing and
scales), ADR 1090 (a reply is a comment under the window it answers, read by asking), ADR 0737 (a
struct added does not move `QUORRA_ABI_VERSION`).
Code: `crates/viewer-ffi/src/abi.rs` (`quorra_popup_rich`, `_paragraph`, `_run`, `_text`, `_family`,
`_unapplied`; `PdfvRichParagraph`, `PdfvRichRun`; `QUORRA_RICH_ALIGN_*`), `answers.rs`
(`Popups::rich`, `rich_paragraph`, `rich_run`), `include/quorra.h`. Tests: `a_c_program_drives_the_abi`
(a note on the form fixture read through the header's own structs), `header_and_library_agree`,
`unsafe_position`.

## The question

ADR 1642 left `quorra.h` with no rich note: a C caller drew `quorra_popup_text` and nothing said
there was more. Two shapes were open — the XHTML string and nothing more, or the runs, a struct per
run.

## The decision: runs

**The string is the document's, and the reading is this program's.** Handing a C caller the XHTML
asks it to parse chapter 27 again — the cascade, white space, `font` shorthands, list numbering —
with a reader of its own, and to escape the document's characters for whatever toolkit it hands
them to; the three windows of this program would then show one reading and a fourth host another,
and a caller who passed the string to a markup-reading label unescaped would hand the document a
way to write its markup. ADR 1642's rule is that what crosses is what the producer specified about
each character, and the runs are that. They are also what AppKit's attributed strings, DirectWrite's
text ranges and Pango's attribute lists take.

## The shape

`quorra_popup_rich(popups, window, note, &paragraphs, &unapplied)` says whether note `note` states a
rich note this program reads — `0` the window's own, `reply + 1` a reply's, so one loop walks the
thread — and `QUORRA_NO_ANSWER` is a note drawn from `quorra_popup_text` alone. A paragraph is
`quorra_rich_paragraph` (alignment, list level, whether piece 0 is its tag, how many pieces), a
piece is `quorra_rich_run`, and its characters, each name of its family search path and each
unapplied phrase are read with the two-call idiom. **Lengths cross as the caller resolves them** —
so many of its own text size plus so many points, §12.5.6.4 giving the window's size to the
processor — and a letter spacing is three numbers of which at most one is non-zero, the third a
share of a space in the face the caller picks. The structs are written into caller memory and never
passed by value, so `QUORRA_ABI_VERSION` stays where it is (ADR 0737's reasoning). Six entry points;
the header says, beside them, that characters and names are the document's and never markup.
