# 1974 — A form's font is loaded once for its layout and the appearance's `Tf`

Session 1569, a script round of batch ninety, `doc/todo/70` item 2's residue. Status: **accepted,
built**. Context: ADR 1956 (variable text's `/DA` font loaded once an interpretation, `LayoutFonts`)
and ADR 0710 (`FontCache`, a loaded font kept across interpretations of one document). Code:
`crates/pdf-model/src/variable_text.rs` (`LayoutFonts::sharing`, `resolve_font_object`),
`crates/pdf-model/src/content/font.rs` (`FontCache::program`, `FontCache::keep_program`), the
interpreter's one constructor in `crates/pdf-model/src/content.rs`. Clauses: §12.7.4.3, §9.6.

## 1. What was measured

A field's layout reads its `/DA` font out of `/DR` and writes an appearance whose `/Resources` name
the same object; the interpreter then runs that appearance and its `Tf` loads the object again. So
after ADR 1956 an interpretation of a page of fields loaded each face twice: once in
`LayoutFonts`, once in the interpreter's own `fonts` (and `across`, where a caller keeps one). The
witness is ADR 1956's framebuffer page, rebuilt by `make_framebuffer.py` (one page, 200 one-row
text fields under `/DA (/Cour 0 Tf 0 g)` and a Courier `/DR`, an interval rewriting every row), and
the instrument `crates/pdf-script-worker/examples/script_cost.rs --ticks 3`, which interprets with
a fresh `FontCache` each time. Callgrind, HEAD's export against this round's, one binary each:

| | HEAD | built |
|---|---|---|
| `LoadedFont::load` calls | 6 | 3 |
| instructions, whole run | 1 085.3 M | 1 079.6 M |
| instructions in `interpret_with` | 953.7 M | 948.0 M |

The loads halve and the instructions fall by 0.53 %: a non-embedded Courier loads in about 1.9 M
instructions, so on this witness the load was no longer where the time is. What the change is for
is the open document's cache, which outlives the interpretation: there, the layouts of every later
interpretation and the `Tf` are both answered without a load (`tests/form_font_once.rs`).

## 2. Decisions

1. **The layout asks the cache the `Tf` asks.** The interpreter makes its `LayoutFonts` with
   `LayoutFonts::sharing(document, across)`. A `/DR` font named by reference is looked up by that
   object in `across` after the layouts' own memo misses, and a fresh load is kept there; the
   appearance's `Tf` then finds it through `cached_font`, which says a truncated `CMap` or
   `/ToUnicode` exactly as a fresh load does. A font named directly, a stand-in for a name `/DR`
   does not define, and every failure stay in the layouts' own memo, as ADR 1956 keeps them.
2. **The key is the object, and that is the key `FontCache` already argues** (ADR 0710):
   `LoadedFont::load` is a function of the document and the dictionary, the label words only its
   errors, and the document is immutable, so the font the layout loads is the one `Tf` would
   load. A metrics-only font is not kept, for `load_font`'s reason.
3. **The document is checked before the cache is asked.** An imported page (§8.10.4) draws its
   annotations through the containing interpreter's `LayoutFonts`, and `FontCache` empties itself
   when asked about another document, so `LayoutFonts::load`'s identity check comes first and an
   imported page's layouts load afresh, as they did.
4. **Every other caller keeps its own memo.** A saved appearance, a host's caret question and the
   other single questions ADR 1956 section 2 lists make `LayoutFonts::new`, which holds no cache.

## 3. Consequences

No pixel may move: the six arms by page against HEAD's six arms. Both tests of
`tests/form_font_once.rs` fail with the interpreter's `LayoutFonts::new` put back.
