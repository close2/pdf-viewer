# 2288 — The text interpreter splits by concern

Session 1727, the standing code-quality slot of batch 111. Status: implemented; no statement, signature or arm moved.

`Interpreter`'s text impl in `crates/pdf-model/src/content/text.rs` was the second-largest impl block
in `tools/state.sh gods` (1 426 lines, 21 methods) inside a 2 306-line file. `text.rs` is now the
module root (the types a sibling outside `text/` reads — `TextObject`, `Coverage`, the re-exported
`WordGaps` — and the module's tests) over eight children in `content/text/`:

- `show` — §9.4.3's showing operators: `show_text`, the refused-font advance and the coverage tally;
- `readback` — what each code reads back as, the glyph box (`glyph_quad`), and the word-gap decisions
  (`read_back`, `separate_text`, `settle_word_gaps`, the `/ActualText` cut and push);
- `word_gaps` — the per-font steps along a line and the thresholds read off them (ADRs 1502, 1515);
- `glyph_painting` — what one glyph is to have done to it, decided once per show string (Table 104
  and §11.7.4's two questions);
- `placement` — §9.4.4's arithmetic: text space, the glyph's matrix, the advance;
- `paint` — fill, stroke and clip of an outline glyph, and its implicit group;
- `end_object` — `ET`: the accumulated clip and §9.3.8's knockout judged whole;
- `type3` — §9.6.5's glyph descriptions run as content streams.

**What did not move.** No method body, signature or order inside its group. The changed lines are
`use` lines, module headers, `pub(super)` and `pub(in crate::content)` prefixes, rustfmt's rewrap of
two signatures the longer prefix pushed past the width, one `super::ledger` path now written
`crate::content::ledger`, and the intra-doc links to `Interpreter::…`, which resolved in one module
and now name it by path.

**Visibility.** What `content.rs`, `run.rs` and `divide.rs` called stays `pub(in crate::content)`,
which is what `pub(super)` meant in `text.rs`. What only a sibling in `text/` reads is `pub(super)`,
chosen by the compiler's refusals rather than by guess.

**Proof.** The multiset of non-comment, non-`use` lines of `text.rs` at HEAD against the union of the
new files, with the visibility prefixes stripped, differs only by the rewrap and the path above and
five repeated `impl Interpreter<'_> {` / `}` pairs. `cargo test -p pdf-model --lib` passes, clippy
with warnings denied, `cargo fmt --check` and `cargo doc --document-private-items` with warnings
denied are clean. The six corpus arms, run on an export of this round's files, are compared by page
against HEAD's six arms (see the record).

**Ledger and teaching.** The eight rows that named `text.rs` name the child holding the clause's code
(8.7.3, 9.3.8, 9.6.4, 9.10.2, 11.7.4, 11.7.4.3, 11.7.4.4, 14.9.4); `how-text-is-selected-and-copied.md`
names `show.rs` and `readback.rs` in its diagrams and `how-a-page-lands-on-the-screen.md` names the
directory; `doc/crate-map.md` lists the modules.
