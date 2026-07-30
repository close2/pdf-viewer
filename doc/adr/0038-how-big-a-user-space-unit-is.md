# ADR 0038 — How big a user space unit is

Status: accepted, 2026-07-30.

## Context

The oracle had a bucket nothing else in the gate resembles. `GEOMETRY` is not "we drew this
page differently"; it is "we and the references disagree about how large the page *is*", and
the comparison cannot even proceed. It held three documents, and its comment held two
statements — one a hypothesis, one an admission:

> `bug1947248_forms.pdf` and `bug1947248_text.pdf` carry `/UserUnit 3` … `mutool` and `gs`
> scale the page by it and produce 1836x2376, we and `poppler` do not and produce 612x792. We
> neither apply it nor report it. … `issue19176.pdf` is the reverse case — we and `poppler`
> take a 9x11 page where `mutool` and `gs` fall back to 612x792 — and has not been looked
> into.

§7.7.3.3 was also one of the five clauses in `REVIEW_OWED`: cited by the code that reads
`/Rotate`, never read as a whole.

## What reading it found

**All three documents are the same entry, and the third is not the reverse case.**
`issue19176.pdf` writes `/MediaBox [0 0 8.5 11]` with `/UserUnit 72`. That is a page stated
in **inches**, and it is US Letter the moment Table 31's entry is applied. The comment saying
it had not been looked into had been sitting beside the clause that explains it for as long as
the two obvious documents had.

## Decision

### `/UserUnit` scales the page and everything on it

> A positive number that shall give the size of default user space units, in multiples of
> 1 ⁄ 72 inch. The range of supported values shall be implementation-dependent. Default value:
> 1.0 (user space unit is 1 ⁄ 72 inch).

A device is asked to draw at a *resolution*, which is a number of pixels per inch. If a user
space unit is three seventy-seconds of an inch, the same page is three times as large in
device pixels. So the scale folds into the two places a page's geometry already lives —
`rotated_size` for the display list's extent and `base_transform` for its contents — and
everything downstream, including annotations and pattern space, follows without knowing.

Order matters in `base_transform`: the `/Rotate` matrices carry translations stated in the
page's own units, so the scale goes **after** them. Scaling first would move the page off its
own origin by a factor of the unit.

**`mutool` and `ghostscript` scale; `poppler` does not.** That is a two-against-two split and
this project's principle 5 says it is a question with an answer rather than a vote. The clause
says what a unit *is*, and a renderer asked for a resolution has no reading under which a
larger unit produces the same number of pixels. The three documents all agree with the
consensus now.

### Two decisions the clause leaves to the implementation, written down

- **"A positive number."** Zero collapses the page and a negative turns it inside out; both
  are malformed, the clause states no recovery, and the default of 1.0 is this reader's
  answer.
- **"The range of supported values shall be implementation-dependent."** The upper bound is
  1000. A page is at most 14 400 units on a side, so a unit of 1000 is a two-hundred-metre
  page; past that the rasteriser's own pixel budget would refuse it anyway, and refusing here
  keeps a nonsense value from turning into a nonsense allocation request.

### It is not inheritable, and one word decides that

§7.7.3.3 closes the question for every entry Table 31 does not mark: "attributes that are not
explicitly identified in the table as inheritable shall not be inherited". Four entries are
marked — `/Resources`, `/MediaBox`, `/CropBox`, `/Rotate` — and `/UserUnit` sits among a dozen
that are not. Reading it through the same `Inherited` overlay the media box uses is one line's
difference and would scale pages that state nothing, so the test that pins it puts `/UserUnit
3` on a `Pages` node beside an inheritable `/MediaBox` and `/Rotate` and demands that the
first two apply and the third does not.

## Consequences

| | before | after |
|---|---|---|
| pages agreeing with the reference consensus | 815 | **818** |
| **pages where we and the references disagree about the page's size** | **3** | **0** |
| ledger subclauses unreviewed | 428 | **420** |
| clauses in `REVIEW_OWED` | 5 | **4** |

`GEOMETRY` is empty. The list stays in `oracle.rs` because the class is real and worse than a
pixel difference, but nothing is on it.

Reviewing §7.7 as the family around the entry recorded eight rows and found nothing else
wrong, which is worth saying plainly: the catalog reads the five entries a *renderer* needs
and ignores twenty-five a *viewer* would want, and the page tree deliberately does not trust
`/Count` because the clause's own NOTE says the `Kids` arrays "definitively determine" the
number of pages. Both were already right; the review's value here was the one entry that was
not, and the fact that its third witness had been mislabelled.

**A `no`-shaped list is worth reading for what it says it does not know.** The sentence "has
not been looked into" had been in `oracle.rs` for many sessions, next to the answer.
