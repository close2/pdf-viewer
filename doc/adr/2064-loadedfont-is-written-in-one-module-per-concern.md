# 2064 — `LoadedFont` is written in one module per concern

Session 1614, slot 8 of batch ninety-six, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 6. Status: **accepted** and **built** in `crates/pdf-font/src/loading.rs` and `loading/`.
Context: the owner's words of 2026-10-19, "Refactor god classes,..." and "Do not fix any bugs.";
ADRs 2004, 2018, 2032 and 2046 (the same split of four other types). Clauses: none; no behaviour
moves. ADR 2065 is unused.

## 1. What the census found

`LoadedFont`, the font a page draws from, was one `impl` of 64 methods over about 2 070 lines of a
4 319-line `loading.rs`, the rest being the types it shares and four test modules.

## 2. The decision

`loading.rs` stays the root: the state (`LoadedFont` and its fields), the entry points `standard`,
`load` and `metrics_only`, the types and free functions several concerns share, and the tests. Eight
modules each hold an `impl LoadedFont` for one concern:

| module | concern |
|---|---|
| `opening` | the simple and the composite route, and whether the machine offers a stand-in (§9.6, §9.7) |
| `findings` | what loading found out about the font and what it repaired or truncated |
| `codes` | splitting a string into codes, §9.10.2's methods for a code's text, a character's code |
| `program` | what the embedded program states: its size, `cmap` subtables, glyph names, identifiers |
| `advances` | widths, vertical metrics (§9.7.4.3) and pair kerning |
| `glyphs` | from a code to a selector, a face and a glyph index |
| `drawing` | an outline: the cache, the extraction, the hinting instance, composite cycles |
| `substitution` | the stand-in face, §9.6.5.4's reader-chosen mappings, §9.8.3.3's classes |

The names the entry gave (reading, encoding, glyph lookup, metrics, substitution) held as concerns;
the encoding has no methods of its own, since the encoding work is in `opening` and `glyphs` and in
the crate's `encoding.rs`, so `codes` takes the text half.

**Visibility is what a sibling reads.** The compiler named twelve private methods a sibling or the
tests call (`load_simple`, `load_composite`, `cmap`, `selector`, `glyph_for`, `placed_glyph`,
`glyph_for_selector`, `program_glyph`, `substitutes_notdef`, `substituted_glyph`,
`addressable_codes`, `cached_outline`); each is `pub(super)`. Every other private method stayed
private to its module, and every `pub` method stays `pub`, so the crate's surface is unchanged.

## 3. The proof that nothing moved

The multiset of non-blank lines, whitespace stripped (habit 93), of the old file against the root and
the eight modules. Removed-only: the twelve signatures above without `pub(super)` and the old `use`
blocks. Added-only: the eight module docs, each module's `use` block, eight `impl LoadedFont {`
wrappers with their closing braces, eight `mod` lines, the twelve `pub(super)` signatures, and one
doc-comment line: `[`Engine::Interpreter`]` is now written `[`skrifa::outline::Engine::Interpreter`]`,
because the root no longer imports `Engine` and `tools/conformance/tests/names.rs` reads a bare
prefix it cannot resolve as a reference that does not work. `cargo test -p pdf-font` passes
unchanged (283 unit tests), clippy with `-D warnings` is clean over all targets, and the six corpus
arms agree with HEAD's page by page.

## 4. What this costs

`loading.rs` is still 2 390 lines, 1 200 of them tests. A reader looking for `LoadedFont::advance`
opens `loading/advances.rs`; one looking for `Face` or `Placed` opens the root. Fifteen ledger rows
now also name the module that cites their clause (habit 96).
