# 74 — The god classes: what the census finds, in the order to split it

Status: **open** — entries 1 and 2 are split (ADRs 2004, 2018); the rest is the order. Ranking rule: ADR 2005.
Priority: the owner's, 2026-10-19: "Refactor god classes,..." and "Do not fix any bugs.", in a round
asked to improve code quality for "the 'project which we can show students' part".
Instrument: `python3 -I -B tools/god-census.py --top <n>`; `tools/state.sh gods` prints the top five
of each of its four tables. The census counts and this file orders; a number is read from the
command, never copied here.

**A split here is a move.** Behaviour held byte for byte, proved by the multiset of the moved lines
(habit 93); the ledger rows that named the file re-pointed to the module holding the row's item
(habit 96); every `pub` narrowed to what a sibling reads; a module's doc comment says what it is for
and nothing about how it came to be (ADR 1023). A bug met on the way is a line under "Found, not
fixed" below, never a change.

## 1. Built — `Viewer` (`crates/viewer-core/src/viewer.rs`), ADR 2004

One `impl` of the census's largest block, over a hundred methods, now one module per concern under
`viewer/`: `opening`, `layout`, `navigation`, `pointer`, `outcomes`, `editing`, `printing`,
`search`, `geometry`, `accessibility`, `answers`, `media`, `scripts`, `anticipation`; the root keeps
the state, the identities, the bounds and the one routing function.

## 2. Built — `ViewState` (`crates/pdf-model/src/view.rs`, with `view/scripts.rs`), ADR 2018

The second block, and the second type by methods once its files are summed. `view.rs` is now the
root — the state, the types a host names, the constructor, the re-exports and the tests — and ten
modules each hold an `impl ViewState` for one concern: `settings`, `optional_content`, `annotations`,
`markup`, `attachments`, `fields`, `field_tree`, `writing`, `pages`, `actions`. The `view/script_*.rs`
files and `scripts.rs` stay as they were. The entry named eight modules; the field-tree walk
(`field_tree`) and the reading of an annotation (`annotations`) are the two it did not, and the
fields did not have to be cut last: they call the others through `pub(super)` and no field was
widened.

## 3. `Walk` (`crates/pdf-transform/src/redact.rs`)

One walk over a content stream that rewrites it under a region: the stream and its operators
(`run`, `run_stream`, `operator`), text (`select_font`, `show_*`, `rewrite_show`, the code/box
tests), paths (the `*_subpath`, `paint_path`, `cut_hairline`, `stroke_outline`), images (the `plan_*`
family, `jpx_*`, `image_layout`, `inline_*`), forms and frames (`do_xobject`, `run_form`,
`enter_frame`), and graphics state (`text_state`, `ext_gstate`, soft masks). Split by those: the
images are the largest and the most separable; `redact/` would take `text.rs`, `paths.rs`,
`images.rs`, `forms.rs`. The same type has seven files across the crate's archive path; read them
together before cutting.

## 4. `Rewriter` (`crates/pdf-transform/src/archive/rewrite.rs`)

One block of the archive conversion's restatements: values (`restate_*`, `agree_separations`,
`write_colorants`, `write_default_cmyk`), appearance states (`collapse_appearance_states` to
`strip_automatic_states`), configurations (`name_*`), page structure (`attach_page_resources`,
`remove_page_boundaries`, `take_out_of_the_page`, `append_the_pages`), attachments (`file_*`,
`attachments`), annotations (`remove_the_*annotations`, `relocate_onto_page`), the catalogue and
form (`rewrite_catalog`, `edit_*`, `unsign`, permissions) and streams (`rewrite_stream`,
`reencode`, `hexadecimal_repair`). Split along those seven; `archive/` already has `attach.rs`,
`boundaries.rs`, `decision.rs`, `hexadecimal.rs`, `prepare.rs` beside it, so some of the names exist.

## 5. `Tree` (`crates/pdf-model/src/structure.rs`) — the largest source file

7 872 lines of which about 2 670 are tests, one 2 125-line `impl Tree`. The brief for this round
guessed three concerns (the tree's reading, the logical order, the accessibility view); the method
list says five: roles and namespaces (`role`, `standard_role`, `namespace`, the MathML and foreign
counts), attributes (`attributes`, `inherited_attribute`, `unranked_owners`), tables (`cell_span`,
`header_scope`, `cell_headers`, `table_summary`, `cell_facts`, `allocation`, `block_spacing`,
`writing_mode`), owners (`search_owners`, `appearance_owners`, `stream_owners`, `xobject_owners`,
`unkeyed_widget_owners`, `object_owner`, the parent-tree entries) and the logical order
(`logical_order`, `logical_text`, `logical_range`, `descend`). The free items around it — the list,
table, language and artifact helpers — fall out with them. Tests go to `structure/tests.rs` or to
the module they test.

## 6. `LoadedFont` (`crates/pdf-font/src/loading.rs`)

64 methods in one block: loading (`standard`, `load_simple`, `load_composite`, `metrics_only`),
substitution (`substitute_program`, `substitutes_notdef`, `substituted_glyph`), text from codes
(`text`, `decode`, `to_unicode_*`), program access (`program_*`), vertical metrics, glyph lookup
(`glyph_for*`, `selector`, `class_face`) and outlines (`outline`, `build_outline`, `cached_outline`).

## 7. The transparency and pattern interpreters (`content/transparency.rs`, `content/pattern.rs`)

Two blocks of the one `Interpreter` type (207 methods over eighteen files; the type is the
largest by methods and is already split by file). Not a split to make by the type: the entries are
the two blocks, and the question for each is whether the file is one clause (§11) or several.

## 8. `Document` (`crates/pdf-syntax/src/document.rs`), `ColourSpace` (`crates/pdf-colour/src/colour.rs`)

`Document` (66 methods): opening and the cross-reference walk, object resolution, encryption,
the page tree and the catalogue's lookups. `ColourSpace` (45 methods in a 7 515-line file with no
test lines): one family per colour-space kind, which is the module boundary `colour.rs` lacks.

## 9. `App` (`crates/viewer-ui/src/bin/quorra/surface.rs`)

Already twelve files; `surface.rs` is the block left (1 652 lines, 30 methods). Read it against
`window.rs` and `renderer/` before cutting.

## 10. Files with no large block

`viewer-ffi/src/abi.rs` (7 147 lines: the exported surface, one group per object kind),
`viewer-confined/src/protocol.rs` (7 820, about 2 890 of them tests, and the two longest functions
after `run_from` live here), `pdf-colour/src/colour.rs`, `pdf-model/src/appearance.rs` (one `construct`
arm per annotation subtype), `pdf-signature/src/signature.rs`. Test files (`pdf-model/tests/oracle.rs`,
`pdf-transform/tests/archive.rs`, `viewer-core/tests/headless.rs`) are lists of scenes, ranked by
the census but not god classes: the remedy is a directory of test modules by clause, and it is a
different round.

## 11. Functions

`content/run.rs::run_from` (one match over the operator set; split by operator family into
functions the match calls, keeping its `#[expect]` reasoning true), `linearize.rs::place` and
`lay_out`, `archive/prepare.rs::of`, `notes.rs::about_one`, `text.rs::show_text`,
`function.rs::apply_operator`, and the two codec functions in `protocol.rs`
(`encode_command`, `decode_command_holding`), which are exhaustive matches on purpose and want
splitting only if the vocabulary does.

## Found, not fixed

- None in the code moved by entries 1 and 2; the moves changed no line of behaviour.
- `raster/crates/raster-gpu/src/device/render.rs:451` did not compile at one moment of this round
  (`missing field uploads in initializer of Counters`) while a sibling slot was editing the crate;
  not this round's, noted so the merge reads it as a mid-edit and not a regression.
- `tools/round.sh docs` prints "CI's last run on main is failure" (run 38691824231) and a shared
  stash that is not empty; both are facts about the repository, not about this round.
