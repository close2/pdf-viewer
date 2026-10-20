# 74 — The god classes: what the census finds, in the order to split it

Status: **open** — entries 1, 2, 3, 5 and 6 are split (ADRs 2004, 2018, 2032, 2046, 2064); the rest is the order. Ranking rule: ADR 2005.
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

## 3. Built — `Walk` (`crates/pdf-transform/src/redact.rs`, with `redact/`), ADR 2032

The third block. `redact.rs` is the root — the state, the entry points and the page's planning and
writing — and seven modules each hold an `impl Walk<'_>` for one concern: `text`, `painting`,
`frames`, `images`, `inline`, `graphics_state` and `regions`, beside the six constructions
(`forms`, `located`, `mesh`, `paths`, `shading`, `type3`) that were already there. The entry named
`text`, `paths`, `images` and `forms`; `paths` and `forms` are constructions the walk calls, so its
side of them is `painting` and `frames`. What is left in the root past the walk is the page's
writing (`write_document`, `build_page`, `Carried`) and the sample arithmetic, free functions and
types that are a different split.

## 4. `Rewriter` (`crates/pdf-transform/src/archive/rewrite.rs`)

One block of the archive conversion's restatements: values (`restate_*`, `agree_separations`,
`write_colorants`, `write_default_cmyk`), appearance states (`collapse_appearance_states` to
`strip_automatic_states`), configurations (`name_*`), page structure (`attach_page_resources`,
`remove_page_boundaries`, `take_out_of_the_page`, `append_the_pages`), attachments (`file_*`,
`attachments`), annotations (`remove_the_*annotations`, `relocate_onto_page`), the catalogue and
form (`rewrite_catalog`, `edit_*`, `unsign`, permissions) and streams (`rewrite_stream`,
`reencode`, `hexadecimal_repair`). Split along those seven; `archive/` already has `attach.rs`,
`boundaries.rs`, `decision.rs`, `hexadecimal.rs`, `prepare.rs` beside it, so some of the names exist.

## 5. Built — `Tree` (`crates/pdf-model/src/structure.rs`, with `structure/`), ADR 2046

The largest source file. `structure.rs` is the root — the state, the entry points that read `/K`,
`logical_range` and the types and free functions the concerns share — and five modules each hold an
`impl Tree` for one concern: `roles` (roles and namespaces, one walk), `attributes`, `tables`,
`owners` and `logical`; the tests are `structure/tests.rs`. `logical_range` stays in the root until
the clause slot of batch ninety-five that edits it has merged; moving it into `logical` is then one
cut. About 3 000 lines of types and helpers remain in the root, a different split.

## 6. Built — `LoadedFont` (`crates/pdf-font/src/loading.rs`, with `loading/`), ADR 2064

64 methods in one block, now a root and eight modules. `loading.rs` keeps the state (`LoadedFont`),
the entry points (`standard`, `load`, `metrics_only`), the types and free functions the concerns
share, and the tests; each module holds an `impl LoadedFont` for one concern: `opening` (the simple
and the composite route), `findings` (what loading found out and repaired), `codes` (a code, its
text and the inverse), `program` (what the embedded program states), `advances` (widths, vertical
metrics, pair kerning), `glyphs` (a code to a glyph index), `drawing` (outlines, the cache and the
hinting instance) and `substitution` (a stand-in face and its classes). About 1 200 lines of types and free functions remain in the root beside its four test modules (about 1 200 lines more); moving either is a different split.

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
`window.rs` and `viewer-device`'s `renderer/` before cutting.

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

- None in the code moved by entries 1 to 3; the moves changed no line of behaviour.
- `raster/crates/raster-gpu/src/device/render.rs:451` did not compile at one moment of this round
  (`missing field uploads in initializer of Counters`) while a sibling slot was editing the crate;
  not this round's, noted so the merge reads it as a mid-edit and not a regression.
- `tools/round.sh docs` prints "CI's last run on main is failure" (run 38691824231) and a shared
  stash that is not empty; both are facts about the repository, not about this round.
