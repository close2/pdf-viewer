# 1956 — Variable text loads its `/DA` font once an interpretation

Session 1560, a script round of batch eighty-nine, `doc/todo/70` item 2. Status: **accepted, built**.
Context: ADR 1950 section 5 item 2, ADR 0710 (`FontCache`, a loaded font shared across
interpretations). Code: `crates/pdf-model/src/variable_text.rs` (`LayoutFonts`), the interpreter's
`layout_fonts` (`content.rs`, `content/annotations.rs`), and the parameter that carries it
(`annotation.rs`, `appearance.rs`, two sites in `view.rs`). Clause: §12.7.4.3.

## 1. What was measured

`set_in` called `pdf_font::LoadedFont::load` for every field it laid out, so a page of fields set in
one face loaded that face once a field. The witness is the framebuffer pattern ADR 1950 section 4
describes, written again from scratch (`scratchpad/r1560/make_framebuffer.py`, not committed): one
page, 200 text fields of one row under `/DA (/Cour 0 Tf 0 g)` and a Courier `/DR`, and a page `/O`
setting an interval that rewrites every row with 320 characters. Callgrind over `script_cost --ticks
3`: at HEAD 2 330 M instructions, 2 199 M of them interpreting the page, 1 034 M (44.4 %) in 603
loads, 600 from the layouts, and 199 M (8.6 %) in each fresh font filling its character-to-code memo.
Built: 1 085 M instructions, 954 M interpreting the page, 6 loads (3 the layouts', one an
interpretation; 3 the interpreter's own `Tf`), 1 M in the memo. In one clock hold, HEAD's export and
this one's interleaved three times on the fast cores, the page interpreted after a tick took a mean
of 43.5–43.9 ms at HEAD and 15.5–15.7 ms built, over 20 ticks a run.

## 2. Decisions

1. **The cache lives as long as one interpretation and is passed, not found.** The interpreter holds
   a `LayoutFonts` and hands it down `annotation::decide` → `construct` → `appearance::construct` →
   the widget, free text and regeneration paths → `Request::fonts`. Every other caller — a saved
   appearance, a host's caret question, a caption's two passes, `accepted_prefix`'s search — makes
   its own for the one question it asks. A parameter, rather than a field of `AnnotationView`, which
   is what the view state says about one annotation and compares equal by value; and rather than a
   process-wide memo, which would have to key by document and outlive it.
2. **The key is what the load reads, compared whole**: the font dictionary `resolve_font` answered,
   and the label. `LoadedFont::load` is a function of the document, that dictionary and that label,
   and `pdf_syntax::Document` is immutable, so equal keys load equal fonts and sharing one changes no
   code, width, outline or report. The cache holds the document it was made for and answers no
   other: a layout of a different document loads afresh.
3. **A failure is kept too**, as the sentence `Owed::FontUnusable` carries, because the same
   dictionary fails the same way for every field after the first.
4. **At most sixteen fonts are kept**; past them a font is loaded and not kept. A form's `/DR` names a
   handful, and the bound keeps what the cache holds alive small whatever a page names.
5. **Only the `/DA`'s font is asked of it.** The two re-loads `set_in` makes for a value the base
   encoding cannot reach (`named_glyphs_reach_more`, `machine_set`) build a dictionary for that
   value, so a cache would rarely answer them; and rich text's faces (`Face::load`) are left as they
   were.

## 3. Consequences

No pixel may move, and none did: the six arms, by page, against `/home/AI/arms-1558/` — 0 of 5 799
page digests moved. What a later round could take further: the interpreter's `FontCache` loads the
same `/DR` font for the appearance stream's `Tf` once more an interpretation, and a cache keyed across
interpretations (ADR 0710's) would take the layouts' one load a frame away as well.
