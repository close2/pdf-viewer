# ADR 0915: the ranking interval's floor can be a refusal, and then the head reads as agreement

**Status**: accepted, in the nine-hundred-and-thirty-seventh session.
**Instrument**: `doc/todo/03` §47's ink ranking, and `doc/oracle-and-corpus.md` §3d's rules for it.

## The measure, and the hole in it

Since §47 a chunk is ranked "by distance *outside the interval the two references bracket* rather
than from the lighter of them, because a row between the two references is a row neither disagrees
with us about." That is right, and it is a strict improvement on ranking from the lighter
reference. It has one failure mode and this round walked straight into it.

Ranking `batch5/pdfcpu` put `pdfcpu-131-0.zip-0.pdf` **inside** the interval, distance 0.000, at
rank 76 of 95. Its three numbers are

| ours | `pdftoppm` | `mutool draw` |
|---|---|---|
| 0.000 | **154.735** | 0.000 |

We draw a blank page where one reference draws 154.7 levels of ink, and the measure scored it as
agreement — because the *other* reference's zero formed the interval's floor and our zero sat on
it. The head of that ranking, on the same run, was 1.259 levels.

`doc/oracle-and-corpus.md` §3d already carries the rule that breaks this, in as many words:
**"Read the stderr as well as the raster … two are a renderer producing a sheet of *zero ink*,
which is not a page and must not be counted as one."** What had never been said is that the rule
binds the ranking's **endpoints** and not only the `no render` and `not comparable` buckets it was
written for. `mutool draw` exits 0 on that document after printing `format error: non-page object
in page tree`, `warning: Page tree load failed. Falling back to slow lookup` and forty repetitions
of `format error: corrupt object stream 1418`; `ghostscript` prints `Error: page not found`. One
reference drew. An interval needs two.

## The decision

**A ranking reads each reference's own log before it uses that reference's ink as an endpoint**,
and a reference whose ink is zero and whose log says it could not draw the page is dropped from
the interval exactly as `oracle.rs`'s consensus drops it (ADR 0769). Where fewer than two
references are left, the row is not ranked at all — it is *listed*, with what each program said,
because a page one reference draws and two refuse is a finding or an unreadable file and either
way it is not a distance.

Three consequences, and the second is the one that costs something:

- **The vocabulary is the same one `Reference::refusals` already owns**, so a ranking and the
  oracle read a refusal by the same words rather than by two lists that can drift.
- **`ghostscript` earns a place in a ranking it was not in.** §47's measure names two references;
  the moment an endpoint can be dropped, two is the *minimum* rather than the population, and a
  third reading is what keeps a row rankable when one program refuses. It costs one more
  invocation per document and it is cached like the others.
- **A row with fewer than two live references goes in the chunk's write-up by name.** In this
  directory there are four of a hundred, and the one above is why the section says so.

## What it changed here, and what it did not

On `pdfcpu` the corrected ranking moves nothing at the head — `pdfcpu-90-0.pdf` at −1.259 is still
the largest distance, and it is `doc/todo/21`'s standing font-substitution population — but it
takes `pdfcpu-131-0.zip-0.pdf` out of the body of the list and puts it where it can be read, which
is how this round came to open it at all. Opened, it is **poppler's** defect rather than ours:
that renderer cannot resolve the file's `/Cs6` colour space (`Syntax Warning: Bad color space
'Cs6'`, `Syntax Error: Incorrect number of arguments in 'scn' command`) and fills the page's
frame with its own default black, which is where 154.7 levels come from. A round that had ranked
it at −154.735 would have spent an hour before reaching that; a round that ranked it at 0.000
never reached it at all. **Both failures are the same failure**, and the reason to prefer the
first is that it is loud.

## Why this is an ADR and not a paragraph in `doc/todo/03`

Because it is a rule about the instrument rather than a fact about a chunk, and because the shape
generalises past the ranking: **an interval computed from a set that can contain a non-answer has
a floor that is not a measurement.** The oracle learned this once already, for the consensus
(ADR 0769) and for `pdftoppm`'s 1×1 raster outvoting the one renderer that drew
(`doc/oracle-and-corpus.md` §3e). This is the third place, and it is the one where the mistake is
silent: the consensus prints "not comparable" and the geometry verdict prints its sizes, while a
ranking prints a number that looks like every other number in the column.
