# 746 — The matrix a published table only counted

The successor selection rule's third use. Its head moved for the first time — by decay, which is
what the rule was chosen for — and the row it landed on had charged a whole journey to the second
leg of it.

Date: 2026-08-25.
ADR: [0653](../adr/0653-the-matrix-a-published-table-only-counted.md).

Touched: `crates/pdf-model/src/measurement.rs` (one method, three field doc comments, two helper
comments, the `Geospatial` type comment, one test), `doc/conformance/ledger.toml` (§12.10, §12.10.1
and §12.10.2), `doc/errata-read.md`, `doc/todo/01`, the ADR and this file. **No pixel moves**: §12.9
and §12.10 state no marks, which the module comment has said since it was written.

## What the rule gave

`spec-errata emit` over `doc/ISO_32000-2_sponsored_EC3.pdf`, the issue numbers this tree names by
`doc/todo/01`'s two greps unioned, the attribution to the nearest live ledger row — the recipe run
rather than read.

**The head moved, and it moved because it was read.** §12.8.1 and §12.5.2, the rows the two uses
before this one took, are off the ranking entirely: every issue on them now carries a verdict in
`doc/errata-read.md`. §12.7.5.5 and §9.8.1 — the other two rows ADR 0627's nine-base reconstruction
found in the top six at every base — sit at 8 and 7.

**What is at the top is a plateau rather than a peak.** Three rows tie at seven annotations apiece:
§7.7.4, §12.10.2 and §14.8.5.3. The rule ranks by count and says nothing about a tie, and a tie is
the normal case, because an issue lands between one and seven annotations on a row. All three were
read far enough to break it, and the tie-break is now part of the rule in `doc/todo/01`: **read the
row whose errata strike a cell — a requirement level, a type, a description — ahead of the row whose
errata substitute a word in prose.** §7.7.4's seven are a global *name string* → *string* rename
counted five times and two *deprecated in PDF 2.0* markers; §14.8.5.3's are *version* → *level* on
three CSS attribute owners and a *structure* inserted into a NOTE.

**The instrument was sound this time**, which is worth saying because the two uses before it each
found something wrong with it: 133 of the 302 issue numbers carrying a strike or a caret are named
nowhere, and the head's four survive both greps and `doc/HAYRO_ISSUES.md`'s other-tracker numbers.

## What the issues said

`doc/errata-read.md` has all four with the rectangle that places each, taken from the annotation's
own `/Rect` against `pdftotext -bbox`.

- **#534 is the one that moved the row.** It strikes `projected coordinate system.` from Table 269's
  `/PCSM` and writes a whole shape in its place — *a 4x4 affine transformation matrix in row order*,
  applied to the position as *[ x y z 1 ]*, `z` non-zero only under `Geospatial3D`. The published
  table says how many numbers `/PCSM` holds and nothing about what they mean, and twelve numbers can
  be four rows of three or three columns of four. Its last sentence, *PCSM only applies when GCS is
  a projected coordinate system*, is `Geospatial::matrix_has_priority` already — the second erratum
  in two uses of this rule that vindicates code rather than correcting it.
- **#533** strikes `Optional` from `/LPTS` and writes *Required*. Nothing here turns on the level;
  the entry's own description was already the reason, and it is recorded on the field.
- **#358** strikes `real` from `of real numbers`, in Table 269 and again in the OPI `Inks` entry on
  page 849 — editorial, since §7.3.3 calls the thing a number.
- **#284** widens the `Geospatial3D` triples condition on `/GPTS` and `/LPTS` from a 3D annotation to
  a RichMedia one, and puts a `/GEO` entry in Table 333 to match.

## What reading them made this round look at

**§12.10's blocker charged two legs to the second one.** The row said "[t]urning a page coordinate
into a latitude means a geodesy library and that registry", and `Geospatial`'s own doc comment said
the same. Table 269 gives `/PCSM` priority over `/GPTS` where a file states one, and #534 says what
its twelve numbers are — so on such a file the object-to-projected leg is a matrix multiplication
with nothing outside the standard in it. The registry owns projected-to-geographic alone.

**The erratum is enough to implement from because §8.3.4 already states the convention one dimension
up.** There a point is "expressed in vector form as [ x y 1]", the matrix is 3-by-3, and "[b]ecause
a transformation matrix has only six elements that can be changed, in most cases in PDF it shall be
specified as the six-element array [a b c d e f]" — three rows with their third column elided as 0,
0, 1, the point on the left. Four rows with a last column of 0, 0, 0, 1 leave twelve, and "row
order" with a "1x4 matrix, [ x y z 1 ]" puts the point in the same place.
`Geospatial::projected_position` is that multiplication.

**And the entry condition the row had no reader for is the triples form.** `pairs` chunks by two
unconditionally under a row claiming *Table 269 entire*. It is declined rather than owed, on where
the dictionary sits: Table 309's `/GEO` puts a geospatial information section within a 3D
annotation, #284 puts the same entry in a RichMedia annotation's Table 333, and `measurement.rs`
reaches a measure dictionary through §12.9's `/VP` alone. Both are clause 13's, which `CLAUDE.md`'s
exclusion list excludes — the erratum that could have opened this widens the excluded population.

Calibrated per trap 13, three plants, all removed: the twelve numbers read transposed, the
translation row dropped, and the `/GCS` guard taken out. **The first plant passed the first fixture
written for the test**, because that fixture's 3×3 block was diagonal and a diagonal matrix agrees
with its own transpose; the fixture is asymmetric now and the comment says why. No corpus document
states a `/PCSM` at all — the one witness is a geographic system with four registration points — so
the fixture is hand-built and says so (trap 8).

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`,
and the oracle took every one of its reference renders from it. `tools/round.sh` says this is not a
fifth round, but the change is in `pdf-model`, so §2's map asks for everything and everything was
run. Both workers were built before any gate that decodes an image (trap 10). §5's binaries were
rebuilt and installed — `round.sh` had flagged `target/` as holding none of them.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them re-run after the final
document edit. **The oracle says no pixel moved**, which is what a change to a module that states no
marks should look like.

Clippy failed three times on this round's own code before it passed, and each was worth the minute:
`arithmetic_side_effects` on the index arithmetic that a row-order stride is written with,
`many_single_char_names` on the twelve elements named after §8.3.4's six, and `doc_markdown` inside
an erratum's own wording. The method destructures the array into named rows now, which is what the
lint was asking for and is clearer than the strides were.

**The machine was loud and the sequence was split around it.** The one-minute load average was
between 43 and 45 on 24 cores while three sibling rounds ran, so the lines that spawn a reference
renderer — the oracle, the text extraction and quorra — were held until it fell below 12, and the
rest ran first. §2's own paragraph is the reason: a budget is wall clock, and a loaded machine is a
silent third program in a measurement of two.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them.
`quoted` and `unpriced` were not run: this round touches no page-list note and both take the
oracle's log as their right-hand side. The before run was taken with this round's one source file
restored to `HEAD`, so the delta is this round's writing and nothing else.

`entries`, `unread`, `capabilities` and `overstated` are byte-identical, and so is `spec-errata
check`; `blockers` and `spec-errata moved` differ in one line number apiece, which this round's
insertions shifted. **Not one defect bucket grew**, and one shrank:

- `owed` went 183 unnamed terms to **182** over the same 113 rows, and the term that left is
  `Geospatial::matrix_has_priority` — §12.10.2's note has named that function since it was written
  and no source carried the qualified path until this round's doc comment linked to it. That is the
  sweep working rather than prose written to dodge it.
- `callers` went 136 names no crate asks to **137**, and the one is `projected_position`. ADR 0653
  says why that is the honest state of this module rather than a hit.
- `counts` 7993 ← 7949 sentences with 415 ← 412 attributed counts, **149 the family agrees with, 58
  "no such way" and 4 places counting one family twice, all three unchanged**; `quotations` 6268 ←
  6250 document quotations over 960 ← 958 documents with **diverging unchanged at 38**, and 1942 ←
  1941 ledger quotations with **diverging unchanged at 2**; `tables` 6610 ← 6587 sentences and 2457
  ← 2446 key citations with **absent unchanged at 100, contradicted denials at 6 and keyless at
  58**; `pointers` 8275 ← 8264 with **absent unchanged at 131** and 137 symbol pointers with **13
  undefined**, both unchanged; `overtaken` 568 ← 567 decision records with **43 overtaken
  unchanged**; `inapplicable` unchanged at 55 / 233 / 224.
- `spec-errata applied` grew to 699 ← 677 places naming an erratum over a population of 56 125 ←
  56 067, with **the read-first list unchanged at 10, the corrections quoting retired wording at 90
  and the quotations of struck text at 172** — and the comparison count unchanged at 1710, which is
  the blindness this round met stated as a number: #534's strike is three words, so it is not among
  the 188 that `check` can compare against at all.

**`tables` staying at 100 absent took one deliberate sentence.** Issue #284 puts a `/GEO` entry in
Table 333 that the published table does not state, so a comment attributing the key to the table
would be a hit; written as a denial the table agrees with — *Table 333 states no `/GEO` entry until
the same erratum adds one* — it is both the accurate sentence and the one the sweep can read. That
is `doc/todo/01`'s own rule about writing a claim in the form its sweep reads, met from the errata
side for the first time.
