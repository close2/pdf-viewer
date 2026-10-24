# 2304 — The operator loop is one method per family of Table 50

Session 1735, the standing code-quality slot of batch 112. Status: implemented; no arm's behaviour changed, no bug fixed.

`Interpreter::run_from` in `crates/pdf-model/src/content/run.rs` was the largest function in the tree
(`tools/state.sh gods`: 1 003 lines) and carried `#[expect(clippy::too_many_lines)]` on the reason that a
dispatch table reads better whole. The reason was true of the table and not of what the arms did: the arms
are Table 50's operator families, each with its own state, and a reader looking for what `Q` restores had
to cross a thousand lines of colour, text and marked content to find it.

**What it is now.** `run_from` is 138 lines with its comments: it reads a token, settles what is not an
operator, counts the operator, applies §8.6.8's and §7.8.2's rules that precede every dispatch, and calls
`run_operator`, whose one `match` names every operator of Table 50 and sends it to the method for its family:

- `run_graphics_state_op` — `q`, `Q`, `cm`, `gs`, `w`, `J`, `j`, `M`, `d`, `ri`, `i`;
- `run_path_construction_op`, `run_path_painting_op` — §8.5.2, and §8.5.3 with §8.5.4's `W`, `W*`;
- `run_colour_op`; `run_text_op` (object, state, positioning) and `run_text_showing_op`;
- `run_xobject_op`, `run_shading_op`, `run_inline_image_op`;
- `run_marked_content_op` with `end_marked_content` for `EMC`, `run_compatibility_op`, `run_type3_op`.

What the loop held in thirteen local variables is one `Machine` (graphics state and stack, path, current point,
pending clip, text object, open marked-content sections, compatibility depth, the `/ActualText` end). Each
method destructures the fields it owns, so a method's signature says which state an operator family can touch:
`run_path_painting_op` cannot reach the text object. The token read moved to the free function `next_step` and
the keyword handling to `settle_step`, so that the loop is under the lint's bound without its `expect`.

**Choices.**

- *Each family's inner `match` ends in a wildcard arm.* `run_operator` is the exhaustive match, so a new
  operator added to `Operator` is refused there; the methods' wildcard is the price of one operator list per
  family rather than a second `enum` of families, which would be a second place to keep in step. Each wildcard
  says who sends operators to it. `d`'s sibling `i` (flatness) and `MP`, `DP` fall to their family's wildcard:
  they affect no geometry this renderer produces, which the arms already said in a comment.
- *`Machine` is private to the file and not a `Composed`.* `Composed` is what a run begins from and `Leftover`
  what it ends with; `Machine` is what it holds between them, and the two conversions are `begin` and
  `into_leftover`.
- *Two arms were collapsed by clippy into guards* (`ImageData | EndImage if compatibility == 0`,
  `GlyphWidthAndBox if glyph depth > 0`); the behaviour is the same, since the arm they fall to is empty.

**What did not move.** No file, no ledger row (none names a line range), no behaviour. The multiset of the
file's non-comment lines differs from HEAD's by: the declaration of the thirteen locals, which are `Machine`'s
fields; `&mut x` becoming `x` and `x = …` becoming `*x = …` where `x` is now a reference into the machine; the
method signatures and `let Machine { … } = machine;` destructurings; the dispatch's operator list; and the
bodies of `next_step` and `settle_step` with `array_depth` as a reference. No statement was added, removed or
reordered inside an arm.

**Proof.** `cargo test -p pdf-model` passes, clippy `--all-targets` with pedantic is clean, `raster_golden`
passes on an export of this round's file, and the six corpus arms on the same export are compared by page
against HEAD's six arms (the record has the result).
