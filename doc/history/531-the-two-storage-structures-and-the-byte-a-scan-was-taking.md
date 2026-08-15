# 531 — §7.5's two storage structures, and the byte a scan was taking off every stream

**Finding.** `doc/todo/03` §11 asked for three silent roles to be read: the census's `unclassified`
bucket, an object stream and a cross-reference stream. All three are answered — and the first
witness produced by the object-stream rule **disproved the reader rather than the file**. §7.3.8.2
lets `/Length` be an indirect reference, Table 5 *requires* the indirect form of a producer that
cannot know the length in advance, and a parser cannot follow a reference; the scan it falls back to
drops one end-of-line before `endstream`, and where a producer wrote none the byte it drops is the
data's. For a `FlateDecode` stream that byte is usually the last of RFC 1951's final block, so the
stream reads as **truncated while being whole**. The damaged-stream population sessions 508, 521 and
524 measured was mostly that: over the crawl, 2260 damaged streams in 726 documents becomes 1273 in
99, and 90 damaged page-one `/Contents` becomes 28 — with the *corrupt* count unchanged at 21, which
is the mechanism's own signature, since a slice one byte short cannot corrupt a deflate stream.

**Date.** 2026-08-15.
**ADR.** [0366](../adr/0366-the-two-storage-structures-and-the-byte-a-scan-was-taking.md).
**Touched.** `crates/pdf-syntax/src/document.rs` (`with_stated_length`, `expand_object_stream`'s
extent rule, `ObjectsLost` and `objects_lost_to_damage`, `cross_reference_entries_lost`),
`crates/pdf-syntax/src/parser.rs` (`stream_data_at`, `endstream_follows` reachable from the
document), `crates/pdf-syntax/src/xref.rs` (`Section::lost`, `XrefTable::entries_lost`),
`crates/pdf-syntax/src/lib.rs`, `crates/pdf-syntax/tests/cross_references.rs` and
`tests/stream_length_bound.rs` (three hand-built pairs),
`crates/viewer-core/src/notes.rs` (§7.5.8's shortfall at open, §7.5.7's losses when they become
known), `crates/viewer-core/src/open.rs` and `src/viewer.rs` (the said-once mark),
`crates/pdf-model/examples/damaged_stream_census.rs` (`who_names_what`, nine more roles, the two
§7.5 lines), `doc/conformance/ledger.toml` (§7.3.8.1, §7.3.8.2, §7.5.7, §7.5.8), `doc/todo/03` §11,
`doc/HANDOVER.md` (trap 5's two-routes instance), `doc/adr/0366-*` (new), this file.

## What the split said, which is why the round had time for the rest

`unclassified` was never a role: it is what the census could not classify, and the classifier only
ever read a stream's *own* dictionary except for a page's `/Contents`, which it read off the page
that names it. Every remaining kind is that same shape, so `who_names_what` walks the file once and
records the entry each stream is named under — Table 110's `/CharProcs`, §12.5.5's `/AP`, §8.6.6.3's
fourth array element, §9.10.3's `/ToUnicode`, Table 122's `/Encoding`, §12.3.4's `/Thumb`,
§12.6.4.16's `/JS`, Annex K's `/XFA` — plus two dictionary arms, `/Type /CMap` and `/ShadingType`.

The 296 split into a `/ToUnicode` majority, a Type 3 glyph description minority and 77 left over,
and the 46 form `XObject`s into 37 forms and 9 appearances. Both of the large parts had already been
decided — session 524 argued the first silent and made the second loud — so the bucket that looked
like the round's whole subject was mostly a classifier that could not see what it had.

## Three hand-built pairs, because the disk has no witness for any of them

Trap 8, three times over, and each pair differs in exactly the rule under test:

- an object stream whose single stored block is missing RFC 1951's BFINAL bit against one that has
  it — the same bytes, one of them saying it is finished;
- a cross-reference stream whose data carries four of the six rows its `/Index` states against one
  carrying all six;
- a stream whose `/Length` is `5 0 R` against the same stream with the length written directly, both
  with `endstream` hard against the data.

The third fails on the tree as it was, which is the only thing that establishes it guards anything.

## What it cost

Nothing anywhere. The display-list digest over all 974 corpus documents is byte-identical; the
corpus gate's incomplete count is the same; the oracle's verdicts are identical bucket for bucket,
measured by running both gates on the reverted tree in the same sitting rather than by argument; the
pdftotext gate and the word-box gate do not move. The nine streams the fix un-damages in the 974 are
all images, and the count of images short of the grid §7.3.8.2 infers is the same before and after —
so what was removed there was a false report rather than a wrong picture. `doc/corpora/pdfbox` was
checked out for the round, so PDFBox's frozen extraction ran rather than skipping.

## The lesson, which is trap 8's fourth shape and not a new one

A census whose predicate is this reader's own damage flag measures this reader. That was written
down before — "a measurement taken with the instrument under test is not independent of it" — and
three rounds ran the census without it applying, because the flag *looks* like a fact about the
file. What broke it open was opening one witness by hand: 83 bytes where the file's own `/Length`
object says 84.
