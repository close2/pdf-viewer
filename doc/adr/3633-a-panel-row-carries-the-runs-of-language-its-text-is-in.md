# 3633 — A panel row carries the runs of language its text is in

Session 2233. Status: accepted. Context: ADRs 2812, 3627; review 2229 (B04).

## Decision

§14.9.2.1 names "outline entries and names for optional content groups" among the text strings
the catalog's `/Lang` reaches, and §7.9.2.2.2 lets an escape sequence "appear anywhere in a
Unicode text string to indicate the language in which subsequent text shall be written". So a
row's text is not in one language but in **runs**: `(byte offset, BCP 47 tag)`, the first at 0
wherever a language is known. `pdf_model::structure::language_runs(prevailing, escapes)` is the
one place the two clauses meet: an escape at offset 0 replaces the prevailing language, a later
one begins a run of its own, and with neither the list is empty.

- The model keeps what the decoding removes: `outline::Item::title_languages`,
  `OptionalContent::name_languages` and `Presented::Collection::label_languages` hold each
  escape's tag and offset (`pdf_syntax::text_string_languages`).
- `viewer_core::Layer` carries `languages` on both variants, resolved against the catalog's
  `/Lang` when the answer is built — so the confined protocol carries it inside the answer
  rather than a second query, and `Answer::Layers` keeps its shape. A group with no `/Name` (the
  panel names it by object) and a collection with no label carry none.
- `PanelRow::languages` replaces `PanelRow::language`; `outline_rows` resolves `Outline::language`
  against each title's escapes, `layer_rows` copies the layer's runs.
- GTK sets one Pango language attribute per run, over that run's bytes.
- The C ABI reads runs one at a time, `quorra_outline_language_run` and
  `quorra_panel_language_run` (start byte, tag; out of range past the last), with the C# interop;
  `quorra_outline_language` stays, answering the run that opens the title.

## Why runs rather than one tag

One tag per row is exact only while no escape stands inside the text; a title such as
"Kapitel 1 \e en\e Introduction" is in two languages, and a single attribute would give the
second half the wrong one. Runs cost one list where a tag cost one string, and the toolkit that
states languages (Pango) states them per byte range already.

## Left

Qt's item views and quorra's panels set no per-row language (ADR 3627); the runs reach both and
are not read there.
