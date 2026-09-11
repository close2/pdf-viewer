# 0985 — The sweep that had no denominator, and the sixty-three pages it never named

Status: accepted. Session 974.
Context: `tools/pdfref/src/undrawn.rs`, `tools/pdfref/src/bin/undrawn.rs`,
`tools/pdfref/src/lib.rs`, `crates/pdf-model/tests/oracle.rs` (read, not changed),
`doc/todo/00` step 7, `doc/todo/01`'s standing rule about prose sweeps,
`doc/traps/oracle-and-references.md` trap 3, `doc/traps/instruments-and-reports.md`
traps 11, 13, 25 and 39, ADRs 0433, 0541, 0940 and 0945.

## What the round was sent to do

`doc/todo/00`'s named heads — the pages where our reading and the references' disagree — with the
instruction to read one all the way down, because the robustness denominator had had no attention
for five rounds. The round did that first and it came back **null**, and the null is worth stating
before the work that came out of it, because a clean instrument is a result rather than an absence
of one.

## The null, and what makes it readable

The oracle was re-run whole in this tree — 35 seconds against a warm reference cache, every
verdict the gate prints. Its three rankings were read against this tree's own record:

- **The undiagnosed ranking is empty**, as it has been since the six-hundred-and-ninety-fourth.
- **The *we are alone* list**, the ratio of our nearest to the closest voting pair, prints 26 rows
  in three measures with 13 of them marked `[widened: outside]` — and `doc/todo/00` step 1's rule
  is to work down the marked rows and stop at the first unmarked one. Every one of the thirteen is
  read and held: `bug766086.pdf` and `bug1552113.pdf` (ADRs 0674, 0675), `bug1743245.pdf` (ADR
  0688), five `freeculture.pdf` pages and `issue16224.pdf` (trap 9's `libfreetype` pair, ADR 0643),
  `issue4260_reduced.pdf`, `endchar.pdf`, `freeculture.pdf` page 1 and `copy_paste_ligatures.pdf`
  (ADR 0684's own reading).
- **The contradicted pool** is 62 pages and the gate prints, under the ranking, that every one of
  them is held by a group by name — which is ADR 0805's finding holding for a hundred rounds.

Then step 7's sweep, whose population is every page the gate calls `ambiguous` and whose number is
our ink minus the lightest live reference's. **Every row of the negative side reproduces to the
thousandth**, and the two figures that moved since the run recorded in `doc/todo/00` moved by
exactly the amount their ADRs said they would:

- The alarm count is **18 at or past −1, 15 of them documents this tree reports on**, where the
  nine-hundred-and-forty-third's run said 19 and 16. The name that left is `bug866395.pdf`, whose
  empty `/FontFile3` ADR 0940 made readable — it draws `l'impayé` now and the page `agrees`.
- `issue12295.pdf` is **−3.198** where that run said −2.362, which is ADR 0945's stated direction:
  this tree moved its sub-pixel ECG strokes *onto* the geometry and away from four references that
  each floor such a stroke at a device pixel's width. The head above it, `issue16038.pdf` −5.642,
  is unchanged.
- On documents the gate calls complete there are **three** names past the alarm — `issue16038.pdf`,
  `issue12295.pdf`, `issue14297.pdf` — and all three are diagnosed, which is the alarm's hold for
  the seventh consecutive recorded run.

**The positive head is held too, and it had never been read as a list.** Its top is
`bug1552113.pdf` at **+50.365** — a number no round has recorded, and not a regression: ADR 0674
made a border whose width reaches its rectangle's dimension a *filled* region, which is what
§12.5.4's "drawn completely inside the annotation rectangle" makes of `/Border [0 0 112]`, and the
two references nearest us draw no link border at all. Below it `bug1743245.pdf`, `bug920426.pdf`,
`issue4260_reduced.pdf`, `issue5475.pdf`, `issue11740_reduced.pdf`, two `calrgb.pdf` pages,
`issue6621.pdf`, `issue5244.pdf`, `issue14953.pdf` and `issue13343.pdf` — every one held by an
`AMBIGUOUS_*` group whose argument is why it is there, and four of them by
`AMBIGUOUS_REFERENCE_DREW_NOTHING`, which is what the positive side is for.

## And then the instrument turned out to be the finding

`doc/todo/00` step 7 has been a **recipe** since the two-hundred-and-sixty-fifth session. Its
paragraph gives four lines of Python and the artefact path `<target>/tmp/oracle/<stem>/p<n>/`, and
every round that has run it — at least fifteen by the file's own record — has written that loop
again. `doc/todo/01` states the rule this breaks: *a sweep described in prose is a sweep each round
rebuilds from the paragraph*, and a description is what let one sweep go unrun for twenty-four
rounds and then be rebuilt from its own words. Two of step 7's rebuilds are recorded as having cost
the round that made them — one took the ink with a greyscale of its own and moved the head by a
quarter of a level, which is the size of the movement the sweep is watched for.

This round's rebuild found the third, and it is the one that makes a program worth more than a
corrected paragraph.

**The gate prints a page from a labelled corpus as `pdfbox/cweb.pdf page 10` and writes its
artefacts to `pdfbox/cweb/p10/cweb-p10-ours.png`.** The label is a *directory* in the path and not
part of the file's own name — `Work::name` carries it into the name and `Work::artefact_directory`
carries it into the path, and the two are not the same string. A loop written from the recipe joins
the whole printed name into the file name, looks for `pdfbox/cweb/p10/pdfbox/cweb-p10-ours.png`,
finds nothing, and **skips the page in silence**. On this round's first run that loop measured
**775 of the 838 pages it had listed** — and it had listed one fewer than the gate printed, which
is the same defect in the population rather than in the path. The 63 it lost are the whole
`doc/corpora/pdfbox` population the six-hundred-and-ninety-second session added
to the bucket (ADR 0541), and the head, the alarm count and every row of the report looked exactly
as they always had. Nothing was wrong with the numbers that were printed. What was wrong is that
nobody could tell which pages they were over.

That is **trap 25 with the sign reversed**. Trap 25 is a hand-written population naming a thing
that never existed, where finding nothing there reads as a pass; this is a hand-written population
failing to name things that do exist, and it reads as a pass the same way. The answer is the same
one that trap applies: the population is derived rather than written, and **the denominator is
printed**.

## The program

`cargo run --release -p pdfref --bin undrawn -- <the oracle's log>` — `doc/todo/00` step 7, with
its population and its exclusions both read off the gate's own report and its recipe compiled in.
It renders nothing: every panel is already on disk beside the report, so the whole bucket is two
minutes of `magick`.

It lives in `tools/pdfref` because that crate is the oracle's harness. It was first written into
`tools/conformance` beside the twenty-four ledger sweeps and moved out before it was committed,
because a parallel round owns that crate this block — which is the shared-namespace lesson
`doc/environment.md` already states about the stash and the scratchpad, arriving in a third place.

**Four corrections, and the first three are `doc/todo/00`'s own**, each paid for once by a round
that ran the loop without it: a reference that drew nothing is dropped before the minimum is taken;
the population is every `ambiguous` line rather than the *undiagnosed* list, which would shrink as
the bucket is explained; and the gate's `(incomplete)` label is carried through, because a page
this tree reports on is expected to be light.

**The fourth is this round's, and it is trap 3 arriving in a reader.** `issue21436.pdf`'s
`mupdf.png` is **zero bytes**: `mutool draw` creates its `-o` file before it decides it cannot draw
the page, and the gate already handles that — it judges the page without `mupdf` and says so on the
page's own line, `[judged without: mupdf did not render: … invalid page number: -1]`. To a sweep
reading the *directory* the page looks complete, and the blank is then either a lower bound of zero
— which is correction 1's defect one step earlier, another program's failure becoming our surplus —
or a page the sweep cannot measure at all. So **a panel file is not a render**: the exclusions are
read off the report beside the population, and an empty file is not a panel either.

That correction is visible in the program's own exit status. Before it, `undrawn` printed
`839 listed, 838 measured` and exited 1 naming `issue21436.pdf` and the reason; after it,
`839 of 839` and exit 0. The round's own hand-written Python had said `838 of 838` — it lost the
page to a swallowed exception, and it lost the *line* too, because its regexp missed one of the
gate's 839 `ambiguous` verdicts. **Two hand-written instruments over one report, two different
populations, neither of them able to say so.**

### What it refuses, and what it only reports

Trap 39 is the reason the two are separate. It exits non-zero **only** where it could not measure a
page the report listed, or could not run at all — including where `magick` is not on the machine,
because a sweep that quietly measured nothing reads exactly like a clean one. A row past the alarm
is printed and not failed: the `AMBIGUOUS_*` groups that hold a page live in the gate, and whether a
gap is a defect of ours or a reference's excess is a person's reading of a clause. A sweep that
ratcheted the alarm would turn every one of those readings into a thing to be made to go away.

### Calibrated against the defect, both ways (trap 13)

- **The defect it looks for.** Pointed at `issue16038.pdf`'s evidence alone it reports −5.642 and
  fires the alarm; over the whole bucket 18 of 839 rows fire and 821 do not.
- **The instrument's own failure.** With our raster moved aside, the same run prints
  `1 listed, 0 measured`, names the page and the path, and exits 1.
- **The labelled corpus.** `pdfbox/cweb.pdf page 10` resolves to `pdfbox/cweb/p10/cweb-p10-ours.png`
  and measures at +0.236; the unit test asserts that path and the unlabelled one beside it, so the
  defect that prompted the program fails the build if it returns.
- **Correction 4.** The witness line is a unit test in both directions — the page the gate judged
  without `mupdf` excludes exactly `mupdf`, and a page judged whole excludes nothing. The second
  half matters because a renderer's error text quotes file paths, so the sentence is looked for
  inside the gate's bracket and nowhere else.

## What the sixty-three pages say, now that they are measured

All of them, between **+0.048 and +0.749**, every one against `hayro` or a C reference as the
lightest — no member of the alarm, and nothing on the negative side at all. So the five rounds that
read this sweep's null were not reading a wrong answer; they were reading a right answer over an
unstated population. The finding is the denominator, not the pages.

## What this leaves for a later round

The prose in `doc/todo/00` step 7 is now a description of a program rather than a recipe to be
retyped, and it should say so — including the two lines a round actually needs, which are the gate
line and the `undrawn` line above. `doc/verify.md`'s list of instruments that are not §2 gates
wants the same entry. Neither file was edited here: both belong to other hands this block.
