# 2144 — The list-to-scene walk is written in one module per concern

Session 1654, slot 8 of batch one hundred and one, a docs refactoring round (Sonnet 5.5);
`doc/todo/74` entry 10. Status: **accepted** and **built** in `crates/render-raster/src/scene.rs`
and `scene/`. Context: ADRs 2004 to 2128. Clauses: none; no behaviour moves.

## 1. What the census found

`tools/state.sh gods` ranks `Encoder` (`crates/render-raster/src/scene.rs`) the second impl block by
lines once the entries before it were split: 1 511 lines, 40 methods, in a 2 071-line file that also
held the free functions stating a display-list value as raster's. Clips, groups, fills, shadings,
images and outlines were one block read top to bottom.

## 2. The decision

`scene.rs` stays the root and keeps what every concern reads: `Encoder` and its fields, the types the
concerns share (`ClipOutline`, `ResolvedClip`, `Admitted`, `ShadedPaint`, `GroupParts`,
`FunctionPaints`), the two constructors, `commands` (the routing walk), the view bookkeeping
(`consumed_view`, `consume_view`, the dots), `target` and `placed`. Six modules, each an
`impl Encoder`, and one of free functions, beside `own_space` which was already there:

| module | holds |
|---|---|
| `clips` | `crop_to_page`, `clip_chain`, `cuts_nothing`, `admitted_rect`, `walk_admitted`, `resolve_clip`, `clip_outline`, `rectangle` |
| `groups` | `group`, `shaped`, `stage`, `mask_id`, `inside_knockout`, `knockout_element`, `refuse_untranslatable_group` |
| `fills` | `fill`, `emit_fill` |
| `shadings` | `shading_paint`, `function_paint`, `function`, `sampled_fill`, `radial_cone`, `background_raster`, `mesh`, `ramp`, the two program-to-instruction maps |
| `images` | `image`, `device_pixels`, `cached_image`, `unit_square` |
| `outlines` | `handing_over`, `upload_outline`, `outline`, `expanded_stroke`, `transient_outline` |
| `vocabulary` | `affine`, `point`, `colour`, `byte_colour`, `fill_rule`, `blend_mode`, `compose`, `overprinted`, `segments`, `spec` |

`vocabulary`'s functions are re-exported from the root under the paths `present.rs` and `stroke.rs`
already use (`crate::scene::affine`, `colour`, `blend_mode`, ...), so no caller outside the module
changed. `GroupParts` stays in the root with the doc comment that sat above `refuse_untranslatable_group`
and described it. Visibility is what a sibling reads: ten methods and one function are `pub(super)`, each
widened from the compiler's errors and narrowed back one item at a time; the methods that were
`pub(crate)` are as they were.

## 3. The proof that nothing moved

The multiset of the old file's lines (compared with indentation removed, since `rustfmt` re-indents
nothing here but a long signature) against the root and the seven new modules: removed-only 15 lines
(eleven signatures that gained `pub(super)` or were re-broken by `rustfmt`, the old import list and
`self::compose(...)`, now `super::compose(...)`), added-only 102 non-blank lines (module docs,
`use` and `mod` lines, six `impl` wrappers, the widened signatures). `render-raster` is in the corpus
arms' graph; the six arms were run on the export and compared by page against `/home/AI/arms-1647/`
(see the record of session 1654).

## 4. What this costs

A reader looking for `Encoder::fill` opens `scene/fills.rs`. The ledger rows that named `scene.rs`
name the module holding the row's item; `how-a-page-lands-on-the-screen.md` and `doc/crate-map.md`
name the new files. `shadings.rs` is about 530 lines and is the next cut if one is wanted.
