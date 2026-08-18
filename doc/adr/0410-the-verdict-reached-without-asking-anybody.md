# ADR 0410 — The verdict reached without asking anybody

Status: accepted, 2026-08-18. Session 575. Takes the robustness question rather than the ledger's,
from the oracle's own verdict buckets. Adds the `no render` ratchet to `crates/pdf-model/tests/oracle.rs`,
a third `MediaBoxSubstitution`, and `doc/oracle-and-corpus.md` §3d. Amends §7.7.3.4's, §7.9.5's and
§14.11.2.1's ledger rows, and one sentence of `doc/HANDOVER.md`'s trap 1.

## Where this round started, and what it found instead

`CLAUDE.md`'s *Two questions, two denominators* says work is chosen from both tracks. This round was
told to take the second — what share of the files that actually exist render correctly — and to start
from the oracle's `contradicted` and `ambiguous` buckets.

**Both of those are diagnosed to the last page**, and have been since the three-hundred-and-seventy-ninth
session emptied `ambiguous_undiagnosed.txt` and the four-hundred-and-thirty-first re-derived the last
five of `CONTRADICTED_SUBSTITUTED_FONT`. So sizing them is a minute's work and picking from them is
re-litigation. What the sizing *did* produce is the finding, and it came from reading the summary
rather than the lists: the gate prints **seven** verdicts and holds **two**.

```
  agrees / contradicted / ambiguous / our geometry / reference geometry / not comparable / no render
```

`contradicted` is a ratchet, `ambiguous` is a ratchet plus a diagnosis per page, `our geometry` is
`GEOMETRY` held at empty. The other three are printed and watched by nothing — and one of the three
is the bucket where a defect is *worst*, because a page in it is not a page drawn differently. It is
a page a person is shown nothing of.

## Why `no render` is different from every other verdict, and why nobody had noticed

`examine` returns as soon as `render_ours` fails:

```rust
match rendered {
    Ok(rendered) => rendered,
    Err(detail) => return Examined::unjudged(name, Verdict::NoRender(detail), false, spent),
}
```

`render_references` is on the next line and is never reached. So **`no render` is the one verdict
this gate reaches without invoking a single reference renderer** — every other one is a statement
about a comparison, and this one is a statement about us alone, arrived at with the other four
programs' opinions unasked.

That has two consequences and both were live:

- **A page three readers draw and we do not looks exactly like a page nobody can read.** The bucket
  held one of each.
- **Nothing held it in either direction.** A change that stopped a document opening would have added
  one line to a report of 888 non-agreeing pages and failed nothing at all. `doc/HANDOVER.md`'s trap
  1 has called the count "a to-do list of pages nobody has looked at" since the
  hundred-and-seventy-seventh session, and the sentence was exactly true for four hundred rounds.

## Asking them

The recipe is `doc/oracle-and-corpus.md` §3d and it costs minutes: `pdftoppm`, `mutool` and `gs` on
each page, with `tools/pdfref/src/reference.rs`'s own invocations copied verbatim so that all three
are explicit about the page box — trap 3 binds a measurement taken outside the harness exactly as it
binds one taken inside it — then `magick identify` and the ink of whatever came back.

Two rules the run earned, both of which change what the answers mean:

- **Read the stderr beside the raster.** Three of the answers here are a renderer printing why it
  refused, and reading only the exit status would have lost the reason.
- **A sheet of zero ink is not a page.** `poppler` answers `bug1782186.pdf` with an 842 × 596 raster
  after printing *Unsupported version/revision (4/4) of Standard security handler*, and
  `PDFBOX-4352-0.pdf` with a 200 × 50 one; both are blank. Counting either as "a reference drew it"
  would have manufactured two defects.

The whole bucket, asked, splits four ways:

| pages | what they are | what the references say |
|---|---|---|
| 8 | §7.6.4.1's password, which this gate supplies none of | all three refuse each one, in the same words |
| 2 | encryption ISO 32000-2 states no algorithm for (`/R` 5; an `/Encrypt` a fuzzed xref makes unreachable) | refused, or a blank sheet |
| 7 | a page tree that yields nothing, which `tests/corpus.rs` documents file by file | nothing, or `poppler`'s 1 × 1 — **except one** |
| 1 | past this gate's own `PIXEL_BUDGET` | all three draw it, and so do we |

The eight password refusals are worth a sentence because they are the *strong* result of the exercise
rather than the boring one: all three references refuse each of them in the same words, which is
principle 5's direction of inference saying that §7.6.4.1's empty user password is being correctly
**rejected** here rather than our key derivation failing. That is a claim about eight documents that
nothing in this tree could previously make.

## The one page that was ours

`boundingBox_invalid.pdf` page 1. `poppler` and `mutool` draw it at 612 × 792 with real ink;
`ghostscript` exits with *Unrecoverable error*; this tree produced `no target: target dimensions are
degenerate` and **nothing at all**.

The file is a diagnostic, and its three pages are one construction apiece — the captions are the
producer's:

```
3 0 obj  /MediaBox [0 0 0 0]                                  (Empty /MediaBox)
4 0 obj  /MediaBox [0 0 800 600]  /CropBox [0 0 0 0]          (Empty /CropBox)
5 0 obj  /MediaBox [0 0 600 800]  /CropBox [600 800 1000 1000] (Empty /CropBox and /MediaBox intersection)
```

Pages 2 and 3 have been right for as long as the boxes have been read. Page 1 was not, and the reason
is a clause everybody had read and nobody had read *twice*.

### What the standard says, and which of the three outcomes this is

§7.9.5 defines the type, and its NOTE is the sentence that matters:

> NOTE Rectangles can have a width of zero or height of zero.

So `[0 0 0 0]` **is** a rectangle. It is not malformed, it is not out of range, and it is exactly the
right value somewhere else in this same standard — §12.5.6.4's text annotation is "attached to a
point", and `rc_annotation.pdf` states `/Rect [50 50 50 50]`. Every test a rectangle reader can apply
passes.

Table 31 then asks something §7.9.5 does not:

> ( Required; inheritable ) A rectangle (see 7.9.5, "Rectangles"), expressed in default user space
> units, that shall define the boundaries of the physical medium on which the page shall be displayed
> or printed (see 14.11.2, "Page boundaries").

A rectangle of zero extent bounds no medium. So the page's ancestry has supplied no usable value for
a required inheritable entry, and §7.7.3.4's second sentence — "[i]f the attribute is a required one,
a value shall be supplied in an ancestor node" — is unmet in exactly the way it is unmet when the
entry is absent. **The clause states no recovery**, which is ADR 0389's finding reached by a third
route, and the answer ADR 0389 decided applies unchanged: substitute `Page::DEFAULT_MEDIA_BOX`, keep
it A4 rather than moving it to the 612 × 792 two references chose, and **say so**.

Of principle 5's three outcomes this is the third — genuinely unspecified — and the evidence for
calling it that rather than "we misread" is that the references do not agree either: two answer
612 × 792 and the third refuses the file outright.

### Why the reader could not see it, and why that is the generalising half

Two reasons, and each is worth more than the page.

**The enum's vocabulary said there were two ways to fail.** `MediaBoxSubstitution` had `Absent` and
`NotARectangle`, written in the session that read §7.7.3.4 for the first time, and between them they
name the whole of what a *rectangle reader* can complain about. The value that fails Table 31 while
satisfying §7.9.5 falls through both and out the other side. A third variant is the fix, and naming it
`Empty` rather than widening `NotARectangle` is not tidiness: the array **is** a rectangle, and a
report saying otherwise would send a person looking for a syntax error that is not there.

**Four of §14.11.2's five boxes have had this rule since they were read, and the fifth had none.**
`build_page` already refuses a degenerate crop box, and a degenerate bleed, trim or art box, and falls
back — because §14.11.2.1 gives each of those four a *default*: "the value of MediaBox", "the value of
CropBox". Each has somewhere known-good behind it. The media box is what all four fall back **to**, so
there is nothing behind it, and an empty one takes the page's entire geometry with it rather than one
boundary. **A clause that defines a family by defaulting each member to the one before it leaves the
first member with no fallback, and that is where the rule will be missing.**

### The fix, and what it is checked against

`usable_media_box` asks §7.9.5's question and then Table 31's, in that order, and `Inherited` carries
the *fault* of the nearest node that stated the entry rather than a boolean saying one did. That
second half is a rule of its own and is pinned: an unusable value on a leaf **falls back to a usable
one on an ancestor** instead of erasing it, because §7.7.3.4 makes the entry inheritable and a file
that wrote a good rectangle one level up has still said how big the page is.

The page now draws 596 × 842 at ink **0.63587**, and the check that this is the same page rather than
a plausible one needs no reference treated as truth: `poppler` puts 0.655181 on 612 × 792 and `mutool`
0.657509, and ink is a mean over the sheet, so the marks' own area is 0.655181 × 484 704 = 317 570
and ours is 0.63587 × 501 832 = 319 099 — **0.5% apart on three different sheets**. The difference
between us and them is the sheet and nothing else, which is exactly what a documented substitution
should look like from outside.

The oracle's verdict for the page moves `no render` → `our geometry`, which is the honest place for
it: our 596 × 842 against their agreed 612 × 792 is a page-size disagreement, and it is one this tree
now *reports*, so `GEOMETRY`'s ratchet — which filters on `complete` — is untouched. `no render` 19 →
18, `our geometry` 1 → 2, every other bucket identical over 1794 pages.

## The page that was the instrument's refusal and not the program's

`issue19517.pdf` is 12 608 × 16 806 at one device pixel per point — 211 890 048 pixels against
`PIXEL_BUDGET`'s 67 108 864 — so `render_ours` never reaches the rasteriser and the verdict reads
exactly like a document this reader cannot handle.

It is not one. `examples/render_at` draws the same page at the same scale with the interpreter's own
bound: **12 608 × 16 806 at ink 172.597**, against `pdftoppm` 172.602, `mutool` 172.599 and `gs`
172.599. Agreement with all three to **0.005 of 255**, on a page this gate has never judged.

**The budget is not moved.** Three reference rasters of 212 megapixels are gigabytes to hold and to
cache for one page, which is what the constant is there to refuse. What was wrong is not the refusal
but that the bucket it lands in named the program: a verdict that accuses the program when the
*instrument* is what declined is a shape worth a name, and the group carries it.

(The same run produced a smaller instance of the same lesson, and it cost twenty minutes: the first
`render_at` of that page came back blank at ink 0, because the release profile's
`pdf-sandbox-worker` was not built and the page is one JPEG 2000 image. Trap 10, a fourth time, in a
measurement rather than in a gate.)

## What is ratcheted, and over which population

`NO_RENDER_NEEDS_A_PASSWORD`, `NO_RENDER_ENCRYPTION_THE_STANDARD_DOES_NOT_STATE`,
`NO_RENDER_NO_PAGE_IN_THE_TREE` and `NO_RENDER_LARGER_THAN_THIS_GATES_BUDGET`, held to equality in
both directions by `assert_ratchet`, and — unlike every other list in that file — over **all** pages
rather than the ones we call complete. A page this gate produced no raster of is never complete, so a
list filtered on `complete` would hold an empty list against an empty list and watch nothing. The
filter is right for the contradicted pages and wrong here, and the difference is worth stating rather
than inheriting.

`group_name` learns the `NO_RENDER_` prefix, so
`every_group_of_pages_carries_a_diagnosis_naming_one_of_them` covers the four new groups: a comment
welded onto the group above it fails the build here as it does for the other seventy.

## The population, measured before the code was written

`examples/media_box_census` gains the third kind — four finite numbers enclosing no area — and was run
over every corpus on this disk, one process per archive for the crawl:

| population | absent | not a rectangle | **empty** |
|---|---|---|---|
| pdf.js, 964 that open, 1760 pages | 4 | 0 | **1** |
| the 14 specification PDFs, 1382 pages | 0 | 0 | 0 |
| `pdf20examples` 7, `pdf-differences` 37 | 0 | 0 | 0 |
| `pdfbox` 64 | 1 | 0 | 0 |
| `format-corpus` 165 that open, 2710 pages | 3 | 1 | 0 |
| SafeDocs, 65 703 that open, **919 995 pages** | 22 (one document) | 0 | **0** |

Every figure in the first two columns reproduces session 554's to the digit, which is what says the
instrument did not move under this round's changes. The new column is **one page in 924 000**, and it
is a page pdf.js built to carry this defect and captioned *Empty /MediaBox*.

**That number is the honest size of the fix and it is not the argument for it.** `doc/todo/03` §1's
finding is the argument: a corpus built to be diagnostic outranks a corpus built to be large when what
a round wants is a defect rather than a rate, and this is the fourth defect that directory of
deliberate one-fault files has produced. The rate at which real producers write `[0 0 0 0]` is zero;
the cost when one does is the whole document.

## What this leaves

- **The other two unwatched verdicts.** `reference geometry` (2 pages) and `not comparable` (13) are
  still printed and held by nothing. Neither is an accusation against this tree by construction — one
  is the references disagreeing about the page's size, the other is fewer than two of them producing
  an image — but "by construction" is a claim, and the same claim was true of `no render`.
  `boundingBox_invalid.pdf` page 3 is in the first of them, which is this file's third page and the
  one construction of the three that nobody has taken.

  *(**Taken in the five-hundred-and-seventy-ninth session**, so the first sentence of this bullet is
  no longer true and is kept for the argument it makes. Both classes are held by name in `oracle.rs`
  and read page by page in `doc/oracle-and-corpus.md` §3e; the claim survived, and what did not is
  the label — `pdftoppm` writes a 1x1 raster and exits 0 when it fails to create a page, so on both
  `reference geometry` pages a refusal had been counted as an opinion about the page's extent. ADR
  0414, which also took this file's third page: §14.11.2.1's intersection `shall` is applied and no
  reference draws that page at all.)*
- **`PDFBOX-4352-0.pdf` could be reached by a rebuild.** Its `/Encrypt` is unreachable only because
  one cross-reference entry was fuzzed to nineteen digits; `poppler` and `mutool` rebuild the table
  and find it. What they then draw is blank, so the page is worth nothing — but the *route* is the one
  `tests/corpus.rs` already names as "recovered by at least one reference and not by us", and this is
  a fourth witness for it.
