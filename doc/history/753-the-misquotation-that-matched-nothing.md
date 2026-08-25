# 753 — The misquotation that matched nothing

Fifteenth merge round of the block. Four branches, **no conflicts**, and a batch whose findings are
all about the same thing from four directions: **an instrument keyed on being right cannot see being
wrong.**

## The sequence, whole, on a quiet machine (load 1.15)

Both workers built first; §5's binaries installed from the directory `cargo metadata` names. `fmt` ·
`clippy --workspace --all-targets` under `-D warnings`, exit 0 · the fuzz check, exit 0 · `nextest`
**2663 passed, 18 skipped** · conformance 192 + 5 + 1 + 1 · `cargo deny` all four ok · corpus **974
documents, 67 incomplete** · oracle **1945 pages — 983 agrees, 61 contradicted, 836 ambiguous, 3 our
geometry, 2 reference geometry, 42 not comparable, 18 no render** · `render-quorra` **933 agree, 22
differ** · both censuses · `fixed_documents` 40/0 · text, dates, XMP, JPEG 2000. Ledger **445
implemented, 223 partial, 0 unreviewed**.

## 750 — the quotation that survived because it was wrong

A doc comment quoted §7.11.4.1 as saying *the tree shall map name strings to file specifications*.
The clause opens with *the associated name tree*, over a sentence Issue #481 struck out. **The two
accurate copies of that quotation were caught by `spec-errata check` within a round of the erratum
being read.** This third one survived **320 sessions** — and the reason is the round's title:

> **A misquotation matches nothing, so the instrument that catches drift cannot see it.**

`check` compares struck passages against what the tree quotes. A quotation that is *correct* and
*stale* matches the struck text and is found; a quotation that was never the clause's words matches
neither the old text nor the new.

Beside it, `pdf_syntax::tree` cited §7.9.6 for **"by unsigned character code" — a phrase ISO 32000-2
prints nowhere**, in any clause, annex or technical specification. Three instruments could have
caught it and none was placed to: `check` needs struck text, `--bin quotations` does not read
`crates/`, and the conformance gate verifies only blockquotes.

**And the rule's own reliability is worse than recorded: three of four uses have found something
wrong with the instrument**, for two distinct reasons. 746 wrote its issue numbers as **bolded bare
numbers**, a form *neither* step-2 grep can see — which is exactly why the head re-offered them; the
repair is a writing rule rather than a third grep, since a bare-number search collides with another
project's issue list. And **the ranking excludes `implemented` rows while the true head is inside
them**: admitted, two implemented rows carry 11 and 8 unread annotations against the live head's
seven, and one erratum this round read landed on two rows the rule structurally cannot see.

Its substantive find: **one erratum deprecates a whole feature surface**, visible only once the issue
was reassembled across two clause headings — where §7.7.4's row had two of its trees recorded as
*owed to a feature*.

## 749 — the policy that asks about want, not cost

745 showed the tree cannot *predict* a draw's cost. **749 shows cost is not the question.** Over 957
first pages the draw is median **2.2 ms**, p99 **73.9 ms**, slowest **252.1 ms** — against **27 600
ms** for a 1567-byte amplification fixture. A document's author picks where in that two-order gap to
sit, so no threshold separates the populations, and one low enough to catch the fixture **refuses one
legitimate first page in sixteen, every one of which could have finished** — which is what
`CLAUDE.md` §2's *correct frames wherever achievable* forbids.

So: **a draw is interrupted exactly where finishing it would produce a picture the program has
already decided it will never show**, and the tree already had that question in `stale::could_stand_in`.
A page turn, a resize, a re-interpretation and a zoom of a column are interrupted; **a scroll and a
zoom of a single page are not**, because their frame is kept as a *base* rather than thrown away.
That distinction is what makes it a policy rather than "interrupt whenever the view moved". All three
of the briefing's candidates were killed with reasons — the growing allowance because
`CpuRasterizer::rasterize` is not resumable, and the first-frame exception because **the first frame
is the one with no stand-in, which argues for a longer leash rather than a shorter one.**

**ADR 0650's predicted next step was wrong**, and reading the code says so: `Rendered::Failed` sets
`on_screen.shown` deliberately, so a host reporting a *failure* for a draw it merely chose not to
finish would **freeze the page**. An abandoned draw produces no outcome at all; a token never answered
is not re-asked either, so silence is not a leak. **Trap 20**, because it binds any host on this
boundary.

Measured under `Xvfb`: without the interrupt the window waits **2.18–3.62** frames of work for the
shape asked for, with it **1.34–1.68** — populations that do not overlap. And a habit worth its own
line: **in seconds the two arms crossed over**, because the load average went 3.7 → 72 while four
rounds shared the machine. *A duration on a shared machine measures the machine; a ratio inside one
run does not.*

Two things it found that belong to the hosts: **`viewer-gtk` and `viewer-qt` rasterise on the
toolkit's main thread**, so a 27.6-second page takes those windows with it and there is no second
thread to raise a flag from — recorded with the argument that a watchdog is *not* the answer, being
the automatic deadline §1 just refused.

## 751 — the floor, and four shapes that are not one

**Our own number must be outside a bound; the list goes 48 → 26.** What makes the threshold
defensible had to be checked rather than assumed: `pdfref::decide` returns the class `Tolerance`
**unwidened** where no consensus formed, so on an ambiguous page "outside 1" means outside the *fixed
class floor* — the same constant for every text page — rather than something the references decided.
**Below it the nearest reference would have accepted the page, and a page somebody accepts is not one
we are alone on.** The asymmetry it names *was* the defect: in four measures the same filter changes
nothing, because the pair is above 1 there by construction.

**"A sublist is not a diagnosis."** The four pages are four mechanisms and only one is 744's book:

- **`bug766086.pdf`** — removing the annotations takes our number 2.58 → 0.43 while the divisor pair
  is **byte-identical to the digit**; neither draws it. And the note's ink table hid a pixel: ours
  strokes columns 5/189 and rows 10/39, `poppler` 5/**190** and 10/**40** — **outside the rectangle on
  two sides**, where §12.5.4 says the border shall be drawn completely inside it.
- **`issue12337.pdf`** — a `/Highlight` with no `/AP`. Ours spans exactly the region `/Rect` and
  `/QuadPoints` both state; every reference spreads wider. Remove it and the page leaves the list.
  **We are alone because we are the only one inside the region the file specifies.**
- **`issue16224.pdf`** — trap 9's tenth: the shared-font-library pair is closest on **23 of 48**
  against **137 of 788**, where the pair with no shared rasteriser is 2 of 48 against 333 of 788.
- **`endchar.pdf`** — four ladders converge within **0.153 of 255** at 32×, ours between two
  references; three of the four figures the note was written on reproduce and **the fourth is ours.**

**The instrument is the transferable part**: take the mechanism out of the document and re-measure
*both* halves of the ratio. **A divisor that does not move was never about the page.**

## 752 — the A/B that needed no edit, and a quarter of `Document::open`

Chosen on the strongest argument available: **both arms were already in `Cargo.toml`.** A question
open since ADR 0222, restated as owed twice, that should have been run three hundred rounds ago.

| vs `release` (fat, 1) | `open` | `interpret` | `rasterise` |
|---|---|---|---|
| the gates profile (thin, 16) | **+12.30%** | +1.65% | +4.07% |
| thin, 1 | +4.06% | +1.84% | +0.67% |
| fat, 16 | +10.72% | −0.01% | −0.00% |

**Both settings are load-bearing on different paths** — cross-crate inlining carries the interpreter
and rasteriser, the single codegen unit carries §7.5's xref parse. No cheaper combination reproduces
the pair; profile unchanged, question closed. Three copies of the cost had decayed, including
`Cargo.toml`'s own comment: "78 s" against a measured **94.5 s**, which *is* §5's whole critical path.

**And a defect underneath the measurement.** The attribution named §7.4.4.4's predictor as the
function whose inlining moved — and **the predictor is about a quarter of `Document::open`**, which
nothing here had recorded. It asked, per byte, which filter the *row* declared, and fetched
neighbours that two of the five types never read. Hoisting the tag: **76.33 M → 67.82 M instructions,
−11.15%, byte-identical**, with the fall in one function exactly equal to the change in the total.

**Trap 13 earned its keep**: the differential check ran against three plants first and one caught
**2190 cases**, so a naive hoist really would have changed what malformed input decodes to. And **PNG
filter type 4 had no test at all** — its new one is built so a decoder implementing only one
neighbour would fail it.

## What the batch has in common

| the instrument | could not see |
|---|---|
| `spec-errata check` | a quotation that was never the clause's words |
| the errata ranking | rows written as bolded bare numbers, and every `implemented` row |
| a cost estimate | a draw whose cost is the author's choice, not the page's size |
| the *we are alone* ratio | that neither of its two numbers had a floor |

## Owed

- **A fourth step for the errata rule** — `implemented` rows admitted — and the writing rule that
  keeps an issue number greppable.
- **`viewer-gtk` and `viewer-qt` rasterise on the main thread**, so the interrupt cannot reach them
  until they have `viewer-ui`'s drawing thread; the policy moves into `viewer-host` after that.
- **The owner's abort**, deliberately not built: it needs an input, and rule 1 already keeps the
  window responsive while a hostile page draws.
- **Orca on all three binaries, by a person.**
- **The `#[non_exhaustive]` decision**, which quorra says is the project owner's to time.
- **The owner's `git stash drop`** — the one entry is verified dead and this account cannot drop it.
