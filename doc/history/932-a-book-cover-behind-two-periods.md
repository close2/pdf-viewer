# 932 — A book cover behind two periods, and the cache nothing asked

2026-09-04. Argued in
[ADR 0904](../adr/0904-a-dimension-written-as-a-real.md) (a dimension written as a real, and one
answer to §7.3.3 rather than two) and
[ADR 0905](../adr/0905-a-search-is-a-selection-and-the-census-now-asks-for-one.md).
[`Q31`](../questions/Q31-how-far-a-readers-tolerance-of-7-3-3-travels.md) and
[`Q32`](../questions/Q32-a-memory-band-whose-floor-nothing-controls.md) are the two halves that are
not this round's to decide.

Merged: `main` at `ef92a769` (rounds 929 and 931), cleanly, before the gates.

Touched: `crates/pdf-model/src/image.rs`, `crates/pdf-model/tests/inline_images.rs`,
`crates/viewer-core/tests/selection_census.rs`, `doc/checks/fixed-documents.toml`,
`doc/checks/launch-path.toml`, `doc/conformance/ledger.toml` (§7.3.3, §8.9.7, §O.2.2),
`doc/todo/03-more-corpora.md` (§49), two ADRs, two `Q` files, this file.
**This round moves pixels**, so `doc/todo/02` §2 ran whole.

## 1. `batch5/qpdf`, and the head by a factor of two hundred

The survey line, the ranking and what every row of it is are
[`doc/todo/03` §49](../todo/03-more-corpora.md). What belongs here is the shape.

**The head reported its own cause and the cause was two periods.** `qpdf-278-0.pdf` is a book cover
— a full-bleed photograph on a 1062 × 1425 sheet — drawn as a blank white page in **0 commands**,
against `pdftoppm`'s 177.973 and `mutool draw`'s 177.313. Its whole content stream is one inline
image whose dictionary says `/W 1062.00 /H 1425.00`, and Table 87 types those integers, so
`Object::as_integer` answered `None` and the image had no grid.

**The clause has no answer and this tree already had one.** §7.3.3's "[a] real number shall not be
present when an integer is expected" is addressed to the writer; nothing addresses the reader. ADR
0371 met that same sentence in §7.10.5's calculator four hundred sessions ago and chose a rule —
truncate — on the ground that a file that breaks it "is a file this viewer still has to draw". ADR
0904 is that rule applied to `/Width` and `/Height`, and **most of its value is the consolidation**:
five call sites in `image.rs` had each written their own `as_integer().and_then(u32::try_from)`, so
changing only the one the head went through would have left a real `/Width` on a `/Mask` reading as
zero next door. The page now draws `mutool draw`'s raster **pixel for pixel** — `magick compare
-metric AE` is 0 — and is a row in `doc/checks/fixed-documents.toml`.

**The population was measured before the fix was believed, and it is two documents of 89 256.** The
whole survey re-ran over every corpus on this disk, before and after, and the passes diffed:
`qpdf-278-0.pdf` becomes `complete`, and `GHOSTSCRIPT-695872-0.pdf` — `batch2`, not this directory —
gains the line of type under its letterhead and a *different* report, §7.4.8's frame check, which is
ADR 0799's reading arriving one clause later. **Zero of the crawl's 65 944.** §47's finding for the
third round running.

**Two rows below the head were read and both are held**, and the instruments said so before any page
was opened: the second row's ladder converges outright, so it is the rasteriser's; the third, fourth
and fifth are `Damage::CheckValue` (ADR 0836) *and* four unembedded non-standard-14 faces at once,
with a difference map uniform over every glyph and empty everywhere else — `doc/todo/21`'s standing
population, the same shape session 926 found one directory over.

## 2. The cache nothing asked, and the third answer

Session 929 measured that `selection_census`'s readback cache is never consulted — forty caret
queries, `hits: 0, misses: 0` — and asked whether that was a weak census or a misplaced cache.

**Neither.** `Readbacks` is the *search* path's cache and it is live: `Viewer::readback` is its only
reader, `find_step` is that function's only caller, and `tests/headless.rs` holds its rules on a
five-page fixture. Selection never goes near it — `Selection::All` and a drag both read the
on-screen `Interpreted`. And `selection_census` is a *selection* instrument whose three properties
are all the pointer's. The two were never on one path, and the survey row that put them together
was the thing that was wrong.

**What was genuinely missing is a third thing: nothing reached the search path over a corpus at
all**, while an instrument that opens every corpus document at a fitted viewport sat one function
away. So the census asks for a search, and the reason it is *this* census rather than a fourth file
is §O.2.2's own verb: "selecting the first matching word in the document". A find ends in a
selection, which is where the drag ends too.

Property 4 reads **1002 of 1002 words selected over 451 documents, 1002 lookups answered out of the
cache, and 0 searches that interpreted a page the page turn had already read**. The last of those is
asserted and the fraction is printed — ADR 0323's rule, because the fraction has poppler in it and
the cost has nobody in it but us. It is a count rather than a clock, so no neighbour's load can move
it by one.

**One thing the first run taught, and it is the instrument's rather than the program's**: nine
documents "failed" on case. `select::find` folds case by design and documents it as "the only
judgement in it", so a byte-for-byte comparison against poppler's word was judging the rule instead
of judging under it.

## 3. The gates, and the one failure that is not this round's

`doc/todo/02` §2, whole, on the merged result. Two lines failed and both were run down.

**`cargo nextest run --workspace` at rc=100**, on the conformance gate: `§` is not one of the
escapes the `toml` crate's basic strings accept, and three of them had gone into ledger notes
because `tomllib` accepts it and had said the file was fine. Replaced with the character itself.
The lesson is small and exact: **a TOML file this tree can parse is not a TOML file this tree's
parser can parse**, and the ledger's own gate is the only one that answers.

**`cargo test --release -p viewer-ui --test launch_path` at rc=101**, on all four rows' memory
high-water, every one of them **below** its floor. Run down rather than widened away:

- Two runs of the **identical binary** ten minutes apart read 108.9 and 99.3 MiB for the first row.
  Nine megabytes with nothing moving in the program.
- `main` itself, with this round's nine lines checked back out and the binary rebuilt, is **also
  below all four floors** — 111.9, 116.2, 128.9 and 140.6 against 127, 131, 132 and 143. The
  failure is not this round's change.
- Seven consecutive runs then sat within a megabyte of one another. The figure is stable inside a
  plateau and wanders between them, which is exactly what `doc/checks/launch-path.toml` already
  says about it: it is the graphics driver's allocation, and the same file records the band being
  widened once before for the same reason.

So the four floors are set a few per cent below the lowest value observed today and **every ceiling
is untouched**, because a high-water that falls is not a leak. Whether that floor should exist at
all is `Q32`, which recommends putting one on `open_peak_mib` — the figure in the same row with no
device in it, identical across all forty-four runs the bands came from — and taking it off this one.
Retiring half of another round's gate on this round's reading is not a thing a round does.

Everything else exited 0 first time, `--test corpus`, the oracle, both text instruments, quorra,
`fixed_documents`, all six transform walks, both `pdf-vfs` walks and the conformance gate included.
The re-run of the four affected lines is green: 3321 workspace tests, the census at 100.00%, and
**28 launch figures banded, 0 not judged, 0 outside**.

## 4. What this round did not take

**The rule ADR 0904 states is scoped to `/Width` and `/Height`** and every other integer-typed entry
still refuses a real. That is deliberate — the measured population is those two entries, and a
tolerance no document exercises is untested code — but how far a documented departure travels before
it becomes this program's number grammar is a policy question, and it is `Q31`.

**No band was moved but the four that failed**, and none of the clock ones. `Q29` is still open.
