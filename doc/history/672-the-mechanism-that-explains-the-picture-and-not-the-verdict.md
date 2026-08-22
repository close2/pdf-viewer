# 672 — The mechanism that explains the picture and not the verdict

One contradicted group, taken apart. Parallel round, worktree `r672`, branch `round-672`.
**No pixel moves**: what changed is a group note that had never measured its own page, two ledger
rows and two paragraphs of trap 9. ADR 0497 has the argument and the ladder.

## Which group, and why — the sixth criterion

668's is spent, and it is the one worth escalating because it audited a premise rather than prose:
it established that the mechanism a note names is *real*. This round asked the question after it.

> **A contradicted entry is a standing exemption from a specific failing bound. Does the mechanism
> a note names — verified or not — account for the measurement the gate actually fails us on, or
> only for the difference a person sees?**

Verified is not sufficient. `oracle.rs` already contained the sentence this inverts:
`CONTRADICTED_MASK_QUANTISATION` closes with *a number stated correctly is not a mechanism
explained*, and the sixth criterion is its mirror — **a mechanism explained is not a number
accounted for**.

Over all fourteen non-empty lists: five price their mechanism against a bound the page fails, eight
price it in an aggregate the gate does not use (ink, ink ÷ length, cap rows, sampled channels, one
row's coverage), and `CONTRADICTED_VISIBILITY_EXPRESSION` **contains no measurement of its page at
all** — four source citations and a clause. That is the group.

## What the page is, and what the bounds are made of

`visibility_expressions.pdf` page 1 sets five strings twice on a 340-point square: pale at
`0 0 0 0.150 k`, then dark at `0 0 0 0.890 k` inside five `/OCMD`s carrying a `/VE` and no `/OCGs`
and no `/P`. Two of the five are false under `/OFF [10 0 R]`. We and `poppler` hide them;
`mupdf`, `ghostscript` **and `hayro`** draw all five.

The gate fails two of its four bounds — worst tile 50.01 of 40.00 and differing 6.38% of 5.00% —
with mean and structural similarity inside. The bounds are `TEXT_HEAVY` unwidened.

Two §7.5.6 incremental updates take one mechanism out of the file at a time. Ours against `mupdf`
at 72 dpi:

| | mean | worst tile | differing | ssim |
|---|---|---|---|---|
| the document | 3.8696 | 50.01 | 6.3456% | 0.95234 |
| `/VE` → `/OCGs 8 0 R /P /AnyOn` | 0.9012 | 4.59 | 6.3086% | 0.99553 |
| … and `k` restated as the same `rg` | 0.5133 | 2.75 | 1.9378% | 0.99589 |

**Removing the group's entire subject moves the failing differing fraction by 0.037 of the 1.35
points it is over by**, and the page stays contradicted without it. The 6.3456 points are `/VE`
0.037, the `DeviceCMYK` press 4.371 and glyph edges 1.938; the worst tile is the other way round,
`/VE` owning 45.42 of 50.01. The mechanism owns one failing bound and 0.6% of the other, and the
reason it lands where it does is `JUST_NOTICEABLE = 4`: the camps are 2–3 levels apart in the pale
tint and 7–11 in the dark one.

Three things came out of the controls. `mupdf`, `ghostscript` and `hayro` render the `/VE`-free
variant **byte for byte** as they render the document, which measures the gap on the binaries that
ran rather than in two source trees; an `/OCMD` naming the OFF group through `/OCGs` and `/P` is
hidden by all three, so they read §8.11 and not `/VE`; and `ghostscript`'s quoted warning no longer
exists — `strings` on `libgs.so.10` at 10.07.1 finds neither it nor `not supported (ignoring)`.
`hayro` is a fourth program with the gap and the note had never said so.

## The clause was half the one cited

§8.11.2.2's `shall` decides the two hidden lines and the worst tile, and this page is the corpus's
only witness for it. The bound the page misses by most belongs to §8.6.4.4 with §10.3.2's NOTE —
`CONTRADICTED_DEVICE_CMYK_CONVERSION`'s row. Fifth round running in which the deciding clause sat in
a different row than the group cites, and the first in which it sits in a different *group*.

## Changed

- `oracle.rs` — `CONTRADICTED_VISIBILITY_EXPRESSION`'s note rewritten around the ladder; the title
  names both mechanisms and which one the verdict is made of.
- `doc/conformance/ledger.toml` — §8.11.2.2 gains the corpus witness and what the `shall` is worth
  on it; §10.3.2 gains a sixth witness, the one that isolates the black channel.
- Trap 9 — two paragraphs: a page can carry two of the eight mechanisms and be named for the
  smaller; and a citation of another project's source has no gate on it.
- ADR 0497.

## Owed, and what the pool looks like at the end of the block

Fifty rounds close here and six of them have each invented and spent one criterion for choosing a
contradicted group — closed form, name, member coverage, clause count, mechanism, and now
sufficiency. **That pattern cannot continue much further, and the sixth already had to be built out
of a sentence sitting in the file rather than out of open ground.**

Where the pool stands, in the sixth criterion's own buckets. Fourteen non-empty lists hold the 65
pages. **Five** notes price their mechanism against a bound their page actually fails —
`IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`, `TIGHT_CONSENSUS`, `GLYPH_EDGES`, `ON_A_PAGE_WE_REPORT` and
`CALRGB_TO_SCREEN`. **None** is left with no measurement at all, which is what this round changed.
**Eight** are the middle bucket, and they are the next block's population: `SUBSTITUTED_FONT` (8
pages), `SHARED_JBIG2_DECODER` (7), `DEVICE_CMYK_CONVERSION` (5), `LINK_BORDER` (3),
`REFERENCES_DREW_NOTHING` (2), and `REFERENCE_GLYPH_WIDTHS`, `SUBPIXEL_IMAGE` and
`NEGATIVE_LINE_WIDTH` (1 apiece) — **28 pages**. Each prices a real mechanism in something the gate
does not measure: ink, ink ÷ length, cap rows, a perimeter, sampled channels, one row's coverage, a
reference's log. Two of the eight do name which bound fails without ever converting their cause into
it.

**So the criterion a next round should reach for is not a seventh — it is the sixth pointed at those
eight.** It is the first of the six to produce a *population* rather than a single target, and on
the one group it was aimed at it inverted the diagnosis. The instrument travels: edit the document
so that one named mechanism cannot act, re-measure with `examples/compare_rasters`, and check the
control — the renderers that should not move must not move. Twenty-eight pages of that stand in
front of anybody needing a seventh idea.

Also owed, unchanged from 668 and 489: nothing links a group's note to the gate figures it quotes,
and this round adds that nothing links one to another project's source either.
