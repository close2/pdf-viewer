# 559 — A caret, the node that may carry it, and what a page turn was paying for

**Finding.** Two of `doc/todo/31`'s three remaining entries, and they were one round's work because
the first is paid for out of the third. A screen reader could **hear** a paragraph and could not
**move through** it: `org.a11y.atspi.Text` is what a caret uses, and AccessKit builds it out of
`Role::TextRun` children carrying each character's length, position and width — all of which this
program has had per character code since ADR 0118 and had never shaped. It now answers with them,
and the whole query on the largest document this project holds costs **a third less** than it did.

**Which node may carry the interface is AccessKit's decision and not the standard's**, which is the
finding worth keeping: `accesskit_consumer::supports_text_ranges` admits only a text input or
`Label`, `Document`, `Terminal`, so **none of §14.8.4's forty-one structure types can carry it**.
Mapping `P` to `Label` to get it would trade the standard's vocabulary for the transport, which is
the trade ADR 0338 refused for a signature field, so the interface goes on the **page** node — this
program's own, formerly an unnamed group — and every element keeps its role. Read back off a real
bus, the page answers `CharacterCount`, `GetText`, `GetStringAtOffset` by word and by line, and
`GetCharacterExtents` in screen pixels.

**And a predicate written for a highlight was wrong about a line.** `select::joins` merges glyphs
for a selection and ends a run at the first overlap over a hundredth of a unit; ISO 32000-2's cover
is set in a tracked face, so it answered `In`, `terna`, `tiona`, `l `, `Sta`, `nda`, `rd ` where the
page says *International Standard*. `select::continues` is the same question with tolerances scaled
to the glyph's height, and it took the corpus from 1 004 514 lines to 114 010 for the same
characters. `joins` is unchanged: a highlight is a different question.

**Date.** 2026-08-17.
**ADR.** [0394](../adr/0394-a-caret-and-the-node-that-may-carry-it.md).
**Touched.** `crates/pdf-model/src/structure.rs` (`Tree::child`, one resolution),
`crates/viewer-core/src/accessibility.rs` (`TextLine`, `Character`, `AccessibilityNode::lines`),
`crates/viewer-core/src/select.rs` (`lines_for`, `continues`),
`crates/viewer-core/src/viewer.rs` (`device_lines`, `device_box`),
`crates/viewer-core/src/lib.rs`, `crates/viewer-core/examples/accessibility_cost.rs`,
`crates/viewer-core/tests/accessibility_census.rs` (the new counts and the invariant),
`crates/viewer-accessibility/src/tree.rs` (`runs`, `chunked`, `direction`, `along`, `word_starts`,
`join`, the page's role), `crates/viewer-accessibility/tests/tree.rs` (six tests),
`crates/viewer-confined/src/protocol/panels.rs` and `protocol.rs` (the wire field and its
invariant), `doc/conformance/ledger.toml` (§14.7, §14.7.5.1.1, §14.8.2.5.1, §14.9.3),
`doc/todo/31-accessibility-host.md`, `doc/adr/0394-*` (new), this file.

## The cost, which is the number this round was asked for

`viewer-core --example accessibility_cost` on `doc/ISO_32000-2_sponsored_EC3.pdf`, 1023 tagged
pages, page 700 — the page a screen reader turns to. The A/B is `valgrind --tool=callgrind
--collect-atstart=no "--toggle-collect=*Viewer*::query*"`, which counts the query and nothing else,
because ADR 0312 already paid for the lesson that a stopwatch measures the machine. A **warm** page
turn is the difference between three repeats and one, over two.

| instructions | before | `Tree::child` alone | with the caret as well |
|---|---|---|---|
| cold, the first query on the page | 264 451 233 | 239 386 443 | 241 213 800 |
| warm, what a page turn costs | **65 861 010** | 41 942 513 | **43 796 275** |

**It was not fine**, which is what the entry suspected and nobody had measured: 70.8% of the query
was `Tree::identified_children`, and inside it `Tree::child`, which read one `/K` entry with three
deep copies of it — two resolutions and a clone. One now. The caret costs 1.85 M instructions warm,
4.4%, which is what made it affordable. Wall clock over eleven repeats moved 46.3 ms → 11.7 ms, a
larger ratio than the instruction count and exactly why the instruction count is what is quoted.

## The census, before and after

`tools/state.sh accessibility`, 988 documents. Everything the previous rounds count is **unchanged**
— which is the point, since none of this was meant to move it — and one line is new:

```
elements reached: 102849
  §14.9.3's /Alt or §14.9.5's /E carried: 664
  placed by Table 379's /BBox or §12.5.2's /Rect: 7538
  cells with §14.8.4.8.3's headers resolved: 16617 (27273 associations)
  §12.7.5's controls behind §14.7.5.3's object references: 272
  elements a caret can move through: 57115 (114010 lines, 2974184 characters)
  a line whose characters and text disagree, which no consumer could index: 0
```

The last line is a class rather than a count and it is asserted, not printed only: `TextLine`'s
invariant is that the sum of its characters' bytes is its text's length, every platform indexes one
by the other, and a breach would show up as a screen reader reading the wrong word rather than as a
crash. The confined protocol checks the same sum on the receiving side, where the producer is not
to be trusted.

## The bus, which is where the claim is made

`doc/verify.md`'s recipe unchanged, with `GetInterfaces` asked at every node and the `Text` methods
asked wherever it appears. `doc/PDF20_AN001-BPC.pdf` page one:

```text
[DocumentFrame] "page Cover (1 of 5)"  +Text
   CharacterCount=165
   GetText(0,count)="A ppl ication NotePDF 2 .0 A pplication Note 0 01: B lack  Point …"
   word at offset 0=("A ppl ication ", 0, 14)
   line at offset 0=("A ppl ication Note", 0, 18)
   extents of character 0=(282, 236, 14, 32)
  [Image] "PDF Association logo"
  [Paragraph] "A ppl ication Note"
```

The page carries the interface and the `Paragraph` under it does not, which is the platform
constraint visible from a client rather than read out of a source file. `structure_simple.pdf` is
the small witness: 52 characters, lines `Heading 1`, `This paragraph 1.`, `Heading 2`,
`This paragraph 2.` — four elements in §14.8.2.5's order.

**One thing the bus showed that reading the code had not.** `Role::Label` *is* in AccessKit's
admitted set, so the `Span` and `Lbl` elements already mapped to it carry the interface too, free
and unasked. A windfall, not a design; the next round should not read it as one.

## What the next round should know

- **`select::joins` is right and `select::continues` is right, and they answer different
  questions.** The temptation is to notice they are nearly the same and merge them. A highlight's
  merge is a statement about what a person dragged across; a line is a statement about where a
  caret may stop. The evidence that they differ is in ADR 0394 and cost this round two builds to
  find, because no unit test with tidy glyph boxes can show it.
- **The instrument prints the longest line for a reason.** 1 004 514 lines looks like a large
  corpus; `"In"` looks like a defect. A count could not tell the two apart and the quotation could.
- **Actions are now the sharpest entry on `doc/todo/31`** and are the only one of its three that is
  not a reading of a clause: two halves of one change, in `viewer-accessibility` and in
  `pdf-viewer`'s `App`.
- **No raster can change.** Nothing on the drawing path was touched and no rasteriser reads
  `Query::AccessibilityTree`; `Tree::child` is read by the structure tree's consumers alone, none of
  which is the interpreter. The corpus, oracle and quorra gates were not owed and were not run.
