# 2176 — The survey walk is written in one module per concern

Session 1670, slot 8 of batch one hundred and three, a docs refactoring round (Sonnet 5.5);
`doc/todo/74` entry 12. Status: **accepted** and **built** in `crates/pdf-archive/src/survey.rs` and
`survey/`. Context: ADRs 2004 to 2160. Clauses: none; no behaviour moves. (2177 unused.)

## 1. What the census found

`tools/state.sh gods` ranked `Walk` (`survey.rs`) the second impl block by lines once the entries
before it were split: 1 457 lines and 52 methods in a 4 300-line file. The walk of a document's
content (a page, its stream loop, fonts, nested forms and images, graphics state, colour spaces,
patterns, the literals inside a stream, marked content) was one block read top to bottom.
Slot 1 of the batch edits `pdf-archive`'s font rows under `table/`, not `survey.rs`, which the brief
named for this slot.

## 2. The decision

`survey.rs` stays the root and keeps what every concern reads: `Survey` and the observation types,
`Walk` with its fields, `State`, `Context`, `Operands`, `Operator` and the keyword tables, the free
functions that classify a space or a profile, and the tests. Eleven modules, each an `impl Walk`:

| module | holds |
|---|---|
| `pages` | `page`, `open_record`, `page_group`, `is_transparency_group`, `record_group_space`, `annotations`, `annotation_is_transparent`, `appearance` |
| `operators` | `run`, `apply` |
| `fonts` | `show`, `select`, `type3` |
| `xobjects` | `invoke`, `form`, `nested`, `image`, `image_space`, `inline_image` |
| `graphics_state` | `graphics_state`, `overprint`, `is_transparent_state` |
| `colour_spaces` | `set_device_colour`, `set_space`, `default_space` |
| `painting` | `paint`, `record_process_overprint`, `record_icc_paint` |
| `patterns` | `pattern`, `shading_resource`, `shading`, `record_shading_space` |
| `records` | `record_colour`, `record_space`, `push_icc`, `push_colour`, `look_up`, `first_time`, `peek`, `note_selection` |
| `literals` | `note_integer`, `note_real`, `note_length`, `note_nesting`, `note_actual_text`, `note_inline_*`, `note_unlisted_operator` |
| `marked_content` | `begin_marked`, `property_list`, `note_language` |

Visibility was widened to `pub(super)` for every method and narrowed back one item at a time by the
compiler (habit 137): 33 stay `pub(super)` because a sibling or the root calls them, the rest are
private to their module. Four doc links to root items were re-qualified (`super::`, habit 126).

## 3. The proof that nothing moved

The multiset of the old file's lines (whitespace removed, `pub(super) ` ignored) against the root and
the eleven modules: removed-only 6 lines (two import lists, and four signatures rustfmt re-wrapped),
added-only 143 non-blank (module docs, `use` and `mod` lines, eleven `impl Walk<'_> {` wrappers with
their braces, the re-wrapped signature lines). `pdf-archive` is outside the corpus arms' graph
(`cargo tree -p pdf-sandbox`, `-p render-raster`, `-p render-cpu` and `-p pdf-render` print no
`pdf-archive`), so the six arms are not owed; `cargo test -p pdf-archive` (226 library tests) holds
behaviour.

## 4. Ledger

Eight rows that named `survey.rs` (§7.8.3, §8.7.3.1, Annex Q and Q.1 to Q.5) also name the module
holding the row's item; the tests stay in the root, so their paths are as they were.
