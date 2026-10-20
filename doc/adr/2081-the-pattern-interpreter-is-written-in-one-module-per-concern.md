# 2081 — The pattern interpreter is written in one module per concern

Session 1622, slot 8 of batch ninety-seven, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 7, second half. Status: **accepted** and **built** in `crates/pdf-model/src/content/pattern.rs`
and `pattern/`. Context: ADR 2080 (the transparency half, the same method), ADRs 2004, 2018, 2032,
2046 and 2064. Clauses: none; no behaviour moves.

## 1. What the census found

`pattern.rs` was 2 458 lines with no tests: the paints a `scn` selects, one `impl Interpreter` of 31
methods and two constants over 1 850 lines, and the lattice geometry as free functions. The entry asked whether it is
one clause or several. It is three: the colour of a pattern (§8.7.4.3, §11.6.7), a shading's painting
(§8.7.4) and a tiling's cell and its repeats (§8.7.3). `pattern/reach.rs` was already the fourth.

## 2. The decision

`pattern.rs` stays the root: the paint types (`PatternPaint`, `ShadingPattern`, `ShadingDefinition`,
`Tiled`) and the entry points the operators call (`pattern`, `tile`, `fill_paint`, `stroke_paint`,
`mark_transfer`). Eight modules beside `reach` hold the rest:

| module | concern |
|---|---|
| `initial` | §11.6.7: the graphics state a definition is evaluated under, and its `/ExtGState` |
| `colouring` | the half of a shading's colour that belongs to the mark, and the paint it becomes |
| `shading` | §8.7.4: the `sh` operator, the `/BBox` and domain clips, the mesh outline |
| `tiling` | §8.7.3: a tiling pattern read from its stream (`Tiling`) |
| `lattice` | §8.7.3.1: which sites a path's bounds reach, and the step between two sites |
| `cell` | one cell run once: its state, its box, the clip a repeat can drop |
| `repeat` | a cell's marks copied to every site, folded where a backend can repeat them, and budgeted |
| `wrapping` | a finished tiling put into a group, and its transfer function recorded |

Visibility follows ADR 2080 §2: `PatternInitial` is the one type the other files of `content` name
through the root; the methods they call (`paint_clip`, `paint_shading`, `unclip_redundant`,
`MAX_REACH_SCAN`) are `pub(in crate::content)`.

## 3. The proof that nothing moved

As ADR 2080 §3: the token multiset of the old file against the root and the eight modules, visibility
words removed, has an empty removed-only set. `cargo test -p pdf-model` passes unchanged and the six
corpus arms agree with HEAD's.

## 4. What this costs

A reader looking for `repeat_cell` opens `pattern/repeat.rs`, and for how a cell is run
`pattern/cell.rs`; `tile` itself, which sequences them, is in the root. Ten ledger rows name the
modules that cite their clause (habit 96).
