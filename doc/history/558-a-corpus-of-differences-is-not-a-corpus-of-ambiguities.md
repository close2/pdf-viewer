# 558 — A corpus of differences is not a corpus of ambiguities

**Finding.** `doc/todo/03` §13 named `pdf-differences`' 37 documents as the last unranked
population on this disk, and §4 had held them behind a decision since the four-hundred-and-twenty-second:
what verdict a page gets when conforming readers legitimately differ. **The premise was wrong about
the corpus.** Sixteen of its eighteen test cases quote a normative sentence of ISO 32000-2 and then
publish the correct picture — the repository's own README makes that a convention — so it exhibits
*implementations* differing, not the standard permitting them to. Exactly two of its differences are
the standard's own permission and both say so in the standard's words. So the references on this
population are the **subject under test**, the vocabulary needs no new verdict, and the corpus is a
reading list with per-case gates in it rather than an oracle population. On the way, the ranking's
head and one document's clause produced a defect this tree had pinned with a test: a `d1` Type 3
glyph description stroked in the *non-stroking* colour, which §9.6.4's own EXAMPLE, its NOTE 2's
plural "current colours" and its list of what a stroking description must set for itself all refute
— and which `poppler` and `ghostscript` get wrong too, so the reference majority was the wrong
reading.

**Date.** 2026-08-17.
**ADR.** [0393](../adr/0393-a-corpus-of-differences-is-not-a-corpus-of-ambiguities.md).
**Touched.** `crates/pdf-model/src/content/run.rs` (`d1` no longer collapses the two colour
parameters), `crates/pdf-model/tests/type3.rs` (the test that pinned the old reading, inverted, plus
the clause's own EXAMPLE as a second case), `crates/pdf-model/tests/page_geometry.rs` (an inherited
`clippy::doc_markdown` warning — see below), `doc/conformance/ledger.toml` (§9.6.4),
`doc/todo/03-more-corpora.md` (§4's bullet answered, §14 new),
`doc/todo/11-shapes-that-still-disappear.md` (§6 new),
`doc/todo/17-a-rebuild-that-misses-compressed-objects.md` (new),
`doc/todo/21-font-substitution.md` (§6 new), `doc/todo/README.md`,
`doc/oracle-and-corpus.md` (§2's row, §2d new), `doc/third-party-data.md` (the licence this row had
half right), `doc/adr/0393-*` (new), this file.

## The decision, in one paragraph

`pdfref::Outcome` gains nothing. Every verdict it has is a function of the rasters; "the standard
permits this difference" is a function of a clause, and a term the instrument cannot compute becomes
the bucket every page nobody wants to explain goes into. `ambiguous` already says the true thing —
the references disagree, so there is no consensus to hold us to — and a page where we differ from
three agreeing references by a permission **stays `contradicted`**, because that is a true statement
about the evidence and the one fact a later round needs. What turns it from an accusation into a
documented choice is a named group quoting the permission, which is what `oracle.rs` has always
done. ADR 0393 §2.

## The chunk

37 documents, 18 test cases, CC BY 4.0 — a licence this tree recorded as Apache-2.0 until this
round, reading the repository's `LICENSE` and not the sentence in its `README.md` that splits the
PDFs from the code.

The survey line, a baseline and never a ratchet: 37 documents, 0 unopenable, 0 locked, 0 encrypted
beyond us, 0 pageless, 0 slow. One report above `doc/oracle-and-corpus.md` §2's row, both moves
being new reports on purpose (ADRs 0356, 0359). **`PDF_SANDBOX_WORKER` has to point at a built
worker** or the line reads two higher, which is the confinement working rather than a property of
the files — the same trap ADR 0389's digest note describes one directory over.

Then page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the
page box, ranked by our ink minus the lightest live reference's, with session 554's size column
beside it. **The head is real for the first time since session 505**: `OverlappingGlyphClipping.pdf`
at −8.989 against a next-largest of −1.237, on a page where the three references agree with each
other to 0.32. And the size column found its second row in two chunks — `LineCap-Degenerate.pdf` is
4000 × 4000 here, in `mutool` and in `gs`, and 400 × 400 in `poppler`, which is Table 31's
`/UserUnit 10`.

## What each of the four findings is

- **§9.6.4's two colours — fixed.** `Type3Test.pdf`, and the argument is ADR 0393's.
- **§8.4.3.5's mitre — `doc/todo/11` §6.** We draw a bevel where the file's `333 M` admits a mitre.
  Reduced to four lines, and the tip's position is a closed form the standard supplies: at
  φ = 0.68752° the ratio is 166.676 and the tip sits 833.38 units above the join, where `mutool` and
  `gs` put it. The cause is `tiny-skia`'s `AngleType::Nearly180` shortcut — an angle test that is a
  ratio cutoff near 90 in disguise — so a fix is three strokers rather than one.
- **§9.3.6 over two substituted faces — `doc/todo/21` §6.** The ranking's head, and the diagnosis
  was built rather than guessed: Helvetica-over-Helvetica unions, Times-over-Times unions,
  Times-over-Helvetica cancels. Our compiled-in sans is an `sfnt` and our serif a bare CFF, and
  their contours wind opposite ways under the clause's non-zero rule.
- **`UnknownFilter-Linearized.pdf` — `doc/todo/17`, new file.** A documented "fully processable"
  document that loses its text here, because `xref::rebuild`'s scan finds `N G obj` headers and
  therefore no object inside an object stream. §7.5.7 states that recovery itself.

## The one thing the round did not inherit clean

`cargo clippy --workspace --all-targets` was **not** silent on the base: one
`clippy::doc_markdown` in `crates/pdf-model/tests/page_geometry.rs`, from session 554's work, which
is exactly the shape `doc/todo/02` §2's merge paragraph describes — green in a worktree, broken on
`main`. Fixed here in passing and recorded rather than absorbed silently.

## Every gate

- `cargo fmt --all --check`: silent. `cargo clippy --workspace --all-targets`: silent after the
  inherited warning above.
- `cargo nextest run --workspace`: 2057 → **2058**, all passing, 15 skipped. The one is the clause's
  own EXAMPLE as a test.
- `cargo test --workspace --doc`, `cargo test -p conformance`: pass.
- **corpus**: `974 documents in 7.9s: 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 65
  incomplete, 0 slow`, with the three silence lines at 5 codes over 2 documents, 57 over 9, and
  1226 over 41. Unchanged in every field: the document that moved is `Type3WordSpacing.pdf`, which
  is complete on both arms.
- **oracle**: `1794 pages in 127.4s (1690 we call complete, 104 incomplete)` — `agrees` 906/862,
  `contradicted` 67/66, `ambiguous` 786/753, our geometry 1/0, reference geometry 2/2, `not
  comparable` 13/7, `no render` 19/0. **No page moves a verdict**, which is a complete statement
  rather than a weak one: exactly one document's display list changed and a verdict change would
  have moved two of those rows. `Type3WordSpacing.pdf` agrees on both arms, and the reason is
  legible in the ink — after the fix we sit at 7.161 beside `mutool`'s 7.186 where `pdftoppm` and
  `gs` are at 8.920 and 9.158, so the three references have no tight consensus on that page and the
  derived bound is wide. **The oracle can say nothing there and the clause can**, which is ADR 0393
  §3 in one page.
- **text_extraction** (two gates): PDFBox `40 documents … overall 99.8% (14257/14281 words)` against
  both orders, 4 below 90%; pdf.js `974 documents … overall 99.2% (22836/23015 words)`, 22 below
  90%. Both unmoved.
- **dates**, **xmp**, **jpeg2000**: pass.
- **render-quorra corpus**, default lane: `956 pages compared in 36.4s: 931 agree, 23 differ, 2
  refused, 18 not comparable`.
- **`display_list_digest` over all 974 first pages**, both arms in one sitting with the same worker
  on disk: **one line differs**, `Type3WordSpacing.pdf`, same command count and same list length
  with a different paint. Pixels move, so the two runs below were owed rather than argued away.
- **quorra `gpu` lane at 4×**: `951 pages compared in 372.2s: 937 agree, 10 differ, 4 refused, 23
  not comparable`, ratchets off as that lane always reports.
- **`doc/todo/00` step 7's ink sweep**, over every one of the gate's own 786 `ambiguous` lines, on
  this file's own recipe: 784 measured and 2 skipped because no reference has any ink under it,
  both of them pages this tree already reports. **Nineteen at or past −1, sixteen of them documents
  this tree calls incomplete**, and the three complete ones are the same three names the
  five-hundred-and-fourteenth recorded — `issue16038.pdf` −6.087, `issue12295.pdf` −2.846,
  `issue7821.pdf` −1.120. The alarm holds. `issue14297.pdf` sits at −0.910 where 514 had −1.145,
  and **this round cannot have moved it** — its display list is byte-identical — so that is the
  drift this file's own warning describes: forty-four rounds and 6175 freshly produced reference
  renders (0% cache hit, `pdftoppm` 26.07.0, `mutool` 1.28.0, `gs` 10.07.1) since anybody looked.

## What this leaves

For the first time in nine chunks `doc/todo/03` §1 names no successor, because every population on
this disk is ranked. The two things it still offers are SafeDocs' 31 GB issue-tracker corpus and —
cheaper, and this round's own recommendation — the per-case gates `pdf-differences` makes possible,
one clause and one hand-built witness apiece, each expected value derived from the standard rather
than voted for by three programs that were wrong about six of the eighteen.
