# 554 — The sheet a page was never measured on

**Finding.** `doc/todo/03` §12 names its own successor, so the chunk was `doc/corpora/pdfbox`'s
64 documents — another PDF library's regression corpus, never ranked against a reference. The ink
ranking separated nothing (whole negative tail −0.410 and shallower, glyph weight), and the finding
came out of the **audit column beside it**: one document's raster is 596 × 842 where all three
references say 612 × 792. Its page tree states no `/MediaBox` at all, which §7.7.3.3 makes required
and §7.7.3.4 requires of the page or an ancestor; this tree substituted A4 **in silence**, and had
since the page tree was written. A4 stays — three references say 612 × 792 and principle 5 forbids
moving a constant to match a vote on a question the standard answers nowhere, and on a
three-number `/MediaBox` the three answer 612 × 792, 792 × 612 and nothing — but the substitution
is now the eleventh place this program reports while drawing.

**Date.** 2026-08-16.
**ADR.** [0389](../adr/0389-the-sheet-a-page-was-never-measured-on.md).
**Touched.** `crates/pdf-model/src/page.rs` (`MediaBoxSubstitution`, `Inherited::media_box_stated`,
`Page::substituted_media_box`, `DEFAULT_MEDIA_BOX`'s reason), `crates/pdf-model/src/content.rs`
(the report, raised before the first mark), `crates/pdf-model/src/content/report.rs`
(`Unsupported::MediaBox`), `crates/pdf-model/src/lib.rs`, `crates/viewer-core/src/report.rs`
(the sentence a person reads), `crates/pdf-model/tests/page_geometry.rs` (a hand-built pair and a
third file), `crates/pdf-model/examples/media_box_census.rs` (new),
`doc/conformance/ledger.toml` (§7.7.3.3, §7.7.3.4), `doc/todo/03-more-corpora.md` §13,
`doc/adr/0389-*` (new), this file.

## The chunk

64 documents, `doc/corpora/pdfbox`, Apache-2.0, a submodule already pinned — §12's own named
successor and the one of its two that needs no decision first. `tools/safedocs survey --dir`
reproduces `doc/oracle-and-corpus.md` §2's row for it exactly; the single incomplete is the
`MAX_FORM_DEPTH` that table has carried since the corpus arrived.

Then page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the
page box, ranked by our ink minus the lightest live reference's — `doc/todo/00` step 7's number on
a population with no ambiguous bucket. Nothing separates: −0.410 `survey.pdf`, −0.328
`tiger-as-form-xobject.pdf`, −0.205 `PDFBOX-2984-rotations.pdf` and shallower, all three opened
side by side and all three one page four times. The largest positive, +1.332, is `gs` drawing
`PDFBOX-3127-…-VFont.pdf` a fifth lighter than the other three.

## What the ranking could not see

`merge/PDFBOX-6018-099267-p9-OrphanPopups.pdf`'s gap is `none`, because all four panels are blank —
its two `Text` annotations set Table 167's Hidden bit and every renderer correctly draws nothing.
What is not the same is the **size**: ours 596 × 842, theirs 612 × 792. The script prints each
panel's dimensions because trap 3's tell is a dimension rather than a difference, and that column
is the only thing in the run that could have found this.

The page object is `<< /Annots 22 0 R /Type /Page /Parent 161 0 R >>` and its node is
`<< /Count 1 /Kids [21 0 R] /Type /Pages >>`. No `/MediaBox`, and no other rectangle either.

## The population, and the corpus lesson inside it

`examples/media_box_census` walks the tree with `pdf_syntax` alone (trap 8), one process per
archive over the crawl. 4 of the 974, 1 of `pdfbox`'s 64, 4 of `format-corpus`' 167, 0 of
`pdf20examples` and `pdf-differences`, and **1 of the 65 703 crawled documents that open** —
`1407606.pdf`, 22 pages of arithmetic worksheet, every page with `/Contents`, drawn 50 points low
and 16 units narrow. Not one page of the population states another of §14.11.2's boxes, so the
guess discards nothing the file wrote.

Two of `format-corpus`' four are `T02-03_008_page-object-mediabox-missing.pdf` and
`T02-03_009_page-object-mediabox-not-rectangle.pdf`, one per branch of the substitution and built
to carry this defect and nothing else. §7 of `doc/todo/03` called that corpus **spent**; it was
spent for the *predicate* it was asked, which was a silent blank. These two never drew blank. They
drew the wrong page.

## Every gate, and what moved

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`: silent.
- `cargo nextest run --workspace`: 2040 → **2042**, all passing, 15 skipped. The two are the
  hand-built pair and the not-a-rectangle file.
- `cargo test --workspace --doc`, `cargo test -p conformance`: pass.
- **corpus**: 64 → **65 incomplete**, and the one is `issue15590.pdf`, the only document of the 974
  the report newly names. Trap 5's rise-on-purpose; every other field of the line is unchanged.
- **oracle**: 1794 pages, and the only two figures that move are the split — *1691 complete / 103
  incomplete* → *1690 / 104*, and `not comparable` *13 total, 8 on pages we call complete* →
  *13 total, 7*. `issue15590.pdf` was already `not comparable`: `pdftoppm`, `mutool` and `gs` all
  refuse it, so the report **cost zero judged pages**, which is trap 11's question answered.
  `agrees` 906, `contradicted` 67, `ambiguous` 786 — all identical.
- **text_extraction**: identical in both gates, to the word. `issue15590.pdf` was already among the
  25 `pdftotext` refuses.
- `dates`, `xmp`, `jpeg2000`, `render-quorra --test corpus`: pass, unmoved.
- **`display_list_digest` over all 974 first pages is byte-identical**, taken on both arms in one
  sitting with the same worker on disk. No pixel moves, so no quorra lane and no ink sweep were
  run — `doc/todo/00` step 7's own condition.

## What this leaves

`pdf-differences`' 37 is the last unranked population on this disk and still wants §4's decision
about the verdict vocabulary. And the general form of this round's lesson is worth carrying: **an
instrument is spent per question, not per corpus**, and a ranking's audit columns are where the
defects a ranking cannot express come out.
