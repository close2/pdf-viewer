# 0653 — The matrix a published table only counted

Status: accepted.
Context: the successor selection rule's third use, on §12.10.2.

## The rule, run

ADR 0627's rule, with ADR 0637's repair to its second step:

> Rank each live ledger row by the errata annotations that fall on it whose issue number this tree
> names nowhere. Reassemble the issue from every clause `emit` files it under, and read the issue
> whole.

**The head moved for the first time, and it moved by decay.** §12.8.1 and §12.5.2 — the two rows the
last two uses read — are off the ranking entirely, because every issue on them now carries a verdict
in `doc/errata-read.md`. What is left at the top is a *plateau*: three rows tie at seven annotations
apiece, §7.7.4, §12.10.2 and §14.8.5.3.

## Decision 1: the tie-break is what an annotation does, not how many there are

The rule ranks by annotation count and says nothing about a tie, and a tie is the normal case at the
head: an issue lands between one and seven annotations on a row, so the units are coarse enough that
the top of the list is flat. Both losing rows were read far enough to settle it.

- §7.7.4's seven are **#214**, the global *name string* → *string* rename, five separate strikes on
  five of Table 31's rows, and **#672**, which appends *; deprecated in PDF 2.0* to `/IDS` and
  `/URLS`.
- §14.8.5.3's are **#357**, *version* → *level* on three of Table 384's CSS attribute owners, and
  **#224**, which inserts *structure* into a NOTE.
- §12.10.2's change a requirement level and rewrite an entry's meaning.

So: **read the row whose errata strike a cell — a requirement level, a type, a description — ahead
of the row whose errata substitute a word in prose.** A count of carets weighs one repeated
editorial substitution as five, and that is the whole of the difference between these three rows.

This is a rule about ranking and not a judgement about the losers. #672's two deprecations are worth
a round on their own: `/IDS` and `/URLS` are Web Capture's, and this tree reads neither, so a
deprecation is the second reason it never will.

## Decision 2: `/PCSM` is applied, because an erratum says what its twelve numbers are

Table 269 as published says `/PCSM` is "[a] 12-element transformation matrix of real numbers,
defining the transformation from XObject position coordinates to projected coordinate system", and
that is a count rather than a layout — twelve numbers can be four rows of three, three columns of
four, or a 3×3 with a translation appended, and they are not the same map. **Issue #534 strikes
`projected coordinate system.` and writes the shape in its place**: *a 4x4 affine transformation
matrix in row order*, applied to a position written as *[ x y z 1 ]*, with `z` non-zero only under
the `Geospatial3D` requirement type.

**Twelve numbers for a 4×4 matrix is §8.3.4's own convention one dimension up**, which is why the
erratum is enough to implement from rather than merely enough to quote. There:

> Because a transformation matrix has only six elements that can be changed, in most cases in PDF
> it shall be specified as the six-element array [a b c d e f].

Three rows, their third column elided as 0, 0, 1, the point on the left as `[ x y 1]`. Four rows
with a last column of 0, 0, 0, 1 leave twelve, and the erratum's "row order" and "1x4 matrix" put
the point on the left in the same place. `Geospatial::projected_position` is that multiplication.

**What it buys is a correction to a blocker rather than a feature.** §12.10's row said "[t]urning a
page coordinate into a latitude means a geodesy library and that registry", and `Geospatial`'s own
doc comment said the same. That is two legs charged to the second one: Table 269 gives `/PCSM`
priority over `/GPTS` where a file states one, so on such a file the object-to-projected leg is
arithmetic the file supplies and the registry owns projected-to-geographic alone. Both sentences are
corrected.

**The erratum is evidence and it is not needed to be right about the guard.**
`Geospatial::matrix_has_priority` already answered *matrix present and `/GCS` projected*, written
from the published table's "should be ignored" when `/GCS` is geographic plus the reading that a
dictionary which is not a `PROJCS` is the geographic form. #534's *PCSM only applies when GCS is a
projected coordinate system* is that sentence stated positively. Like #121 in ADR 0637, this is an
erratum vindicating code rather than correcting it — the second in two uses of the rule, which is
worth noticing: a clause read carefully enough tends to have already reached where its errata go.

## Decision 3: the `Geospatial3D` triples form is declined, on the exclusion list

Table 269 requires `/GPTS` and `/LPTS` "to hold 3D point coordinates as triples rather than
pairwise" under the `Geospatial3D` requirement type, and `pairs` chunks by two unconditionally
under a row claiming *Table 269 entire*. That is an entry condition with no reader, and it is
**declined rather than owed**, on where the dictionary sits: Table 309's `/GEO` puts a geospatial
information section "within a 3D Annotation", **#284** puts the same entry in Table 333, which is a
RichMedia annotation's, and `measurement.rs` reaches a measure dictionary through §12.9's `/VP`
alone. Both annotation kinds are clause 13's, which `CLAUDE.md`'s exclusion list excludes for a
stated reason. The erratum that could have opened this widens the excluded population instead.

The denial is written into the row and onto the two fields, because a condition nobody reads and
nobody denies is indistinguishable from one nobody noticed.

## What was calibrated

Trap 13, three plants, all removed:

- the twelve numbers read transposed — `[22, 35, 48]` becomes `[16, 35, 54]`, which is why the
  fixture's 3×3 block is asymmetric; a diagonal matrix agrees with its own transpose and the first
  fixture written for this test was diagonal and passed the plant;
- the translation row dropped — `[12, 15, 18]`;
- the `/GCS` guard removed — the geographic viewport answers with a position instead of `None`.

No corpus document states a `/PCSM` at all: the one witness `tests/measurement.rs` walks is
`bug1146106.pdf`, a geographic system with four registration points and no matrix. So the fixture is
hand-built and says so (trap 8).

## Consequences

- §12.10, §12.10.1 and §12.10.2's rows say what the registry actually blocks. All three stay
  `partial`: projected-to-geographic is still ISO 19162's and the EPSG registry's.
- `pdf-model` gains one `pub fn` no crate outside it calls, which the `callers` sweep counts. That
  is the honest state of §12.9 and §12.10 in this tree — `viewer-ui` has no measuring tool and the
  module comment has said so since it was written — and it is a reader for a clause rather than a
  hook for a feature.
- `doc/todo/01` carries the tie-break as part of the rule.
