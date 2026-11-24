# Round 2214 — section 5.1's content repairs photographed, written out, or refused by name

An archive round in the `gwg` worktree, on the 27 documents round 2210's repair classes stopped
(`r2210-moved-keep.txt`): 17 by a content class under the fence, 10 by a page entry in the output.

- ADR 3557: under `as-if-printed`, section 5.1 is a fenced row for its content repairs; the page
  holding one (or every page reaching the form it is in) is photographed, `page-as-image`, unless
  another rewrite writes the class out. A repair in an annotation's appearance keeps its refusal.
- ADR 3558: an operator `pdf_model` repaired (short, wrong type, lone `Q`, text outside `BT`,
  colour count) is written as white space with its operands under `content-as-drawn`, only where
  every page drawing the stream gives the same display list and text from the source and from the
  source plus the rewrite as §7.5.6's update. A page's `/Contents` array is one grammar walk.
- ADR 3559: a box the reader substituted or read as absent is written on a `/Type /Page` object as
  the box it drew the page with (ADR 3142 widened), counted as `page-boundary-removed`.
- ADR 3560: left refused by name: `inline-image-key-twice` under `as-shown`, `page-tree-cycle`,
  `contents-entry-not-a-stream`, `issue15590`'s action walked as a page.
- Tests: `archive/repaired_operators.rs` (three); `recovered`'s parent-cycle test now expects the
  second page written on the box it was drawn on, where ADR 3543 expected section 5.1's refusal.

**The 27.** `as-if-printed`, 2b and 4 alike: 24 convert (8 photographed, 7 written out, 2 both, 7 boxes);
refused: `Pages-tree-refs`, `T02-05-01_013`, `issue15590`. `as-shown`: 14 (7 written out, 7 boxes);
the other 10 refused by a class's name, `issue6342` by the operator row. veraPDF 1.30.3 passes all
76 outputs (`r2214/vera5.log`); a source and its `as-shown` output differ by at most 1 a channel.

**Trap.** A grammar walked per stream misreads a page whose `/Contents` array splits a text object or an
array across streams (`hesr13613`: thirteen streams flagged, the proof refusing them all); §7.8.2
makes the array one stream, and the walk carries its state across.

**Census.** With the pack, against round 2210's: `as-if-printed` pdf.js 2b 925 to 938, 4 938 to
952, corpora 2b 430 to 455, 4 460 to 477; `as-shown` pdf.js 897 to 905 and 910 to 919, corpora 403
to 409 and 423 to 428; none newly refused. veraPDF 1.30.3: 5497 outputs, 46 failing, the list
round 2210's; the 97 new pass. Without it, against round 2205's: `as-if-printed` pdf.js 937 and
951 alike, corpora 441 to 455 and 472 to 477; `as-shown` 910 to 905, 924 to 919, 414 to 409, 434 to 428.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0; `pdf-sandbox --bins` 0; nextest of the
three crates 100, 1781 of 1784 passing: the manifest's two (pdf-archive's TrueType audit tests) and
`recovered`'s parent-cycle test, then changed and passing (33 of 33 re-run); rustdoc 0; fuzz
`cargo check --bins` 0; `cargo test -p conformance` 0 once this paragraph was written.

Left: a page-tree cycle restated as walked; a non-stream `/Contents` entry; `as-shown`'s inline key.
