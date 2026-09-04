# The launch path: 110 to 119 ms to the first frame, and what is left on it

Status: **open**, measured in the two-hundred-and-seventy-fourth session; four of its five items
are closed and the fifth is **partly answered**: §9 of `doc/QUORRA_FEEDBACK.md` asked for the
first frame's fixed cost to be warmed, and quorra's ADR 0031 found a fifth of it was an instrument
of its own and moved it to the constructor — 2.2 to 2.5 ms off, confirmed here. What is left was an
**API question**, and the four-hundred-and-seventy-eighth session **answered it and closed it**:
the API arrived (`Device::warm_for`, quorra's ADR 0035), the first frame was measured across both
revisions here, and this host still cannot call it — with a second reason that is upstream's own.
Item 5 has the table. **What is left of item 5 is a number nobody has taken**, and it is the
owner's: a launch on the real adapter through a real window.
Priority: 42 — performance, measured and priced, not yet taken
Corpus: every document; the two costs that scale do so with the *document*, not with page one
Code: `crates/viewer-ui/src/bin/pdf-viewer.rs` (`Launch`), `crates/pdf-model/examples/open_cost.rs`,
`crates/render-quorra/examples/bring_up.rs`, `first_frame.rs`, ADRs 0179, 0180, 0181, 0182

## What changed in the nine-hundred-and-twenty-second session

**There is a gate now, and this file should stop being where a round reads these numbers.**
`crates/viewer-ui/tests/launch_path.rs` measures principle 2's four figures and the fifth it makes
a gate of its own, over four documents, with a band on each in `doc/checks/launch-path.toml`;
`tools/state.sh launch` prints them and `doc/todo/02` §2 runs it. ADR 0884 is the construction —
including why a wall-clock band is believable on this machine at all — and ADR 0885 is the
reading. Three consequences for the items below:

- **Item 1's question is now answered by a command rather than by this file.** The open is tens of
  times more expensive on 1023 pages than on 5 and the *launch* is not, because `document joined`
  and `device up` come back as the same figure in every run: the document thread finishes before
  the graphics device does, so the whole of the open is hidden. The margin is now legible as well
  as the ratio.
- **Item 5's "number nobody has taken" is half-taken.** A launch on the *real adapter* is measured
  every time the gate runs — headless, on this machine's Radeon 890M through RADV, which
  `doc/environment.md` has said since session 552 is reachable without the owner's session. What
  is still the owner's is the other half: that adapter through a real window, with a swapchain and
  a present.
- **The "Nothing eager" rule quoted below has one clause that is false as written**, and it is not
  the one this file was about: a page naming a font it does not embed sends `pdf_font::substitute`
  through the machine's font directories on the launch path, which roughly doubles time to first
  page. It is *needed to show page one*, so it is not eager by the rule's own definition — but the
  bullet says "[n]o system font enumeration" without a condition. The gate carries such a document
  as a row so the cost has a band rather than a sentence; ADR 0885 has the evidence, and the
  wording is the owner's (round 921's question).

## The one band that would not sit, settled in the nine-hundred-and-thirty-first

Session 926 ran the gate on `main` four times: twenty-seven of the twenty-eight figures inside
their bands, and `doc/PDF20_AN001-BPC.pdf`'s cold open — held to `0.49 .. 0.80` — reading
**0.87, 1.30, 0.72 and 0.86**. It did not move the band, wrote three hypotheses here, and named the
experiments that would separate them. Session 931 ran them. **All three are refuted, two of them by
construction**, and this section is now what they found rather than what they asked. ADR 0902 is
the reading and ADR 0903 the change; `doc/questions/Q29` is what is left for the owner.

- **The copies are reflinks, so the sweep hypothesis is refuted by one command.** `std::fs::copy`
  is `copy_file_range`, and on btrfs `copy_file_range` is a reflink: `filefrag -v` prints the same
  physical extents for the gate's copy and for the repository file it was made from, flagged
  `shared`. No copy this gate has ever made has had an extent of its own. A copy written with
  `--reflink=never` — which does get fresh extents — reads cold in `0.109 / 0.125 / 0.543` ms
  against the reflink's `0.109 / 0.127 / 0.613`. **A copy is not a rewrite, and on a
  copy-on-write filesystem it is not even a write.**
- **The disk is not what moves this figure**, so a latency probe beside the throughput one would
  not have separated anything. In every one of the twelve-run set's excursions the *warm* open —
  which has no disk in it at all — moved by a **larger** proportion than the cold one (×1.28,
  ×1.38 and ×2.15 against ×1.13, ×1.14 and ×1.88), while `io_ms` stayed inside `2.0 .. 4.5`
  throughout.
- **There is no regression.** `git diff db4a76f1 HEAD` over `pdf-syntax`, `pdf-model`,
  `viewer-core` and `pdf-font` is session 925's outline change — the same page-tree walk, held —
  plus `type3.rs`, a test and an example, and not a line of `Document::open` or of `Open::around`'s
  cost. The calibration probe *is* `Document::open` plus `Pages::new` plus `interpret` on fixed
  bytes, and it reads 0.703 to 0.749 ms against the `0.62 .. 0.78` session 922 derived.

**What is true instead**, and it is a fourth thing none of the three named: the calibration probe
is the **quickest of fifty passes inside one warmed process**, and every figure it guards is
**one first pass in a fresh one**. Over twelve consecutive runs the probe moved 1.3 % while the
figures moved by factors of two. The rule that leaves a round is general — *a probe has to be made
of the same stuff as the figure it guards* — and it is a finer version of ADR 0884's own sentence
about sensing every subsystem: that one is about *which* subsystems, this one about the *state*
they are in.

**And the band that will not sit is not the one 926 named.** Of six failing runs in twelve, five
failed on `doc/pdf.js/test/pdfs/bug1815476.pdf`'s cold open and one on `PDF20_AN001-BPC.pdf`'s.
Reversing the check file's own derivation rule, every clock figure's median today sits within a few
percent of its derived maximum — high for the two smallest documents, *low* for the two largest,
which is a per-operation cost and not a regression:

| figure | derived range | 12 runs on merged `main` | median against derived max |
|---|---|---|---|
| `bug1815476` cold open | 0.376 .. 0.417 | 0.42 .. 0.95, median 0.50 | **+20 %** |
| `bug1815476` warm open | 0.271 .. 0.292 | 0.28 .. 0.60, median 0.34 | **+16 %** |
| `PDF20` cold open | 0.576 .. 0.667 | 0.67 .. 1.35, median 0.72 | +8 % |
| `PDF20` warm open | 0.412 .. 0.475 | 0.45 .. 1.01, median 0.47 | −1 % |
| `WTPDF` cold open | 2.294 .. 2.55 | 2.44 .. 5.16, median 2.50 | −2 % |
| `ISO 32000-2` cold open | 22.59 .. 25.33 | 22.35 .. 50.73, median 23.16 | −9 % |

**And the gate declines nearly everything when it is run the way `doc/todo/02` §2 runs it.** In
session 931's own full sequence the launch line read its calibration at 1.577 ms against
`0.62 .. 0.78` at a one-minute load average of about 20 — two other rounds building and walking —
printed `NOT JUDGED`, printed all twenty-eight figures with the reason beside each, and exited 0.
The guard is working; the consequence is that principle 2's four numbers are gated in principle and
unwatched in practice, which is what `Q29` is about.

**No band was moved, for the second round running.** A band is a claim about a machine, and
widening one to admit a loaded machine puts the loaded machine into the claim. What changed is the
instrument: every child now reports its **first pass** beside its best of fifty (printed, not
judged — there was no quiet machine to derive a band on), and a figure that is declined says which
probe declined it and what all three probes read. What is owed is the band for that first-pass
probe, about ten minutes of an idle machine, and it is the first of `Q29`'s three options.

**And the figure that fails now is the one with no clock in it, which is new in the
nine-hundred-and-thirty-third.** Everything above is about clocks, and a clock the gate can decline
to judge; `peak_mib` cannot be declined by any probe, because contention does not lower a memory
high-water. In session 933's full sequence — under `release`, calibrations 0.712 .. 0.720 ms, all
inside the band, so the run *was* judging — all four rows failed together **below** their floors:

| document | `peak_mib` band | this run | again, alone |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 127 .. 209 | 109.035 | 109.918 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 131 .. 214 | 114.082 | 113.902 |
| `ISO_32000-2_sponsored_EC3.pdf` | 132 .. 215 | 114.625 | 114.535 |
| `bug1815476.pdf` | 143 .. 231 | 126.504 | 126.227 |

Reproduced within a kilobyte on the second run, so it is neither noise nor a neighbour: it is the
same *thing* `doc/checks/launch-path.toml`'s own header records — "an hour later — same tree, same
binary, idle machine — all four rows had fallen together by about 12%. What moved is the driver's
allocation" — happening again and landing about 13% under the floors that were widened to span it.
**No band was moved, for the third round running**, and by a round that did not touch the launch
path at all: the figure that would have to be re-derived is a property of the graphics driver, and
a coverage round widening a band it did not measure is how a guard becomes a formality. What is
owed is one derivation on an idle machine of what this driver now allocates — the same ten minutes
`Q29`'s first option asks for, on the one figure a loaded machine could not have caused.

## The figure that failed next is not a clock at all — the nine-hundred-and-thirty-fourth

**No band was moved for the fourth round running either**, and the machine was quieter for none of
it: session 934 sampled the one-minute load average every thirty seconds for seventy-five minutes —
151 samples, **minimum 3.30, median 12.86, maximum 61.55, and 62 % of them above 10** — with three
neighbouring rounds rather than two: 932, 933 and a 935 that appeared during the round and ran
`launch_path` probes of its own. So the first-pass band above is still owed.

What that round found instead is a **second** guard failure of a different kind, and ADR 0909 is the
reading. Run inside `doc/todo/02` §2 on a machine with ~9 GiB free after two neighbours' corpus
walks, the line exited **101 on all four `peak_mib` figures at once** — 99.1, 103.1, 104.2 and 116.5
MiB against floors of 127, 131, 132 and 143 — with the calibration probe at **0.706 ms, inside its
band**, which is what let the run judge them. Nine re-runs alone, on a binary the merge had not
touched a line of, with 29 GiB free: **161 to 182 MiB on every document on every run**, inside every
band, two of the runs judging with nothing outside at all.

| | the failing run | the nine runs alone |
|---|---|---|
| free memory | ~9 GiB, 19 GiB of swap in use | 29 GiB, 45 available |
| calibration probe | 0.706 ms, in band | 0.708 .. 0.745 ms |
| `peak_mib`, four documents | 99.1, 103.1, 104.2, 116.5 | 161–164, 168–169, 168–169, 180–182 |
| `open_peak_mib`, no device in it | 7, 8, 18 MiB, in band | 7, 8, 18 MiB, in band |

The last row names the mechanism: what moves is the resident set of a process that has brought the
**graphics device** up, which this file's harness already records falling 12 % between two runs an
hour apart while the bands were being derived. Under memory pressure it falls by 40 %.

**So `peak_mib` is a memory figure whose only guard is a clock**, and a clock reads in band on a
machine with a gigabyte free and on one with sixty. That is trap 34's third dimension — the same
work, in the same *state*, and in the same **units** — and it is the one a `steady: false`
classification looks like it has already handled.

**What is owed here is a third probe**, beside the processor's and the disk's: what the machine had
available when the sample was taken, banded as the disk probe is, so a pressed machine *declines*
`peak_mib` rather than failing it. Its band needs the same idle ten minutes the first-pass band
does, so the two are one errand. **And there is a cheaper question that is not a probe**: a minimum
on a memory high-water exists to catch "we stopped doing the work", which the command counts,
`open_peak_mib` and the timings already witness four other ways — whether principle 2's memory
high-water should be a ceiling rather than a band is a real question, and it is a change to a gate
another round built, which is the same sentence that stopped three rounds widening one.

## What the merge did with the four floors, and what session 935 owns — the nine-hundred-and-thirty-seventh

**Session 932 did move them, and the merge did not take the move.** That round lowered the four
`peak_mib` floors in `doc/checks/launch-path.toml` to 95, 100, 100 and 112 on readings of 98.5 to
116.4, and wrote `Q32` asking whether the figure should be a ceiling only. It branched before 934
and could not have read the table above it. The two rounds' measurements of the same figure on the
same binary disagree — 932 read 99 to 116 with the machine pressed, 934 read **161 to 182 on nine
runs with 29 GiB free** — so lowering a floor to 95 admits the pressed machine into the claim,
which is precisely what 931, 933 and 934 each declined to do. The merge therefore restored session
931's floors and rewrote that file's paragraph to record both observations and the disagreement;
932's paragraph, its numbers and `Q32` all stay, because the observation is real and only the
conclusion drawn from it was one round's alone. **No band was moved in either direction by the
merge**, and no new one was derived: the floors below are 931's, unchanged since it derived them.

**Session 935 owns the resolution, and it is one errand rather than two.** It is deriving this
figure properly in its own worktree; whatever it finds supersedes this section and 932's paragraph
together. The three things that are open, in the order they answer each other:

1. **the availability probe** — what the machine had free when the sample was taken, banded as the
   disk probe is, so a pressed machine prints `NOT JUDGED` for `peak_mib` instead of failing it.
   That is 934's ask, and it is what makes any floor believable again;
2. **`Q32`'s question**, which the probe does not close: whether a memory high-water should carry a
   minimum at all, given that "we stopped doing the work" is already witnessed by `open_peak_mib`,
   by `read_kib` and by every clock in the row;
3. **the first-pass band** `Q29` asks for, which needs the same idle ten minutes and should be
   taken in the same sitting.

A round that answers 1 and 2 should delete this section and 932's paragraph rather than adding a
fourth account of one figure.


## Why this is a todo and not a caveat

`CLAUDE.md`'s startup section states two rules this path breaks:

> **Nothing eager.** No system font enumeration, no full page-tree walk, no configuration or
> recent-file scanning, no thumbnail generation on the launch path.

> **Incremental parsing.** Opening a document reads the trailer and the objects page one needs —
> not the whole file. **A 500-page document must open no slower than a 5-page one.**

Measured when this file was written: **27.8 ms against 0.84 ms**, on 1023 pages against 5. The
rule had been written down and never instrumented, which is the same shape as every claim this
project has found stale — with the difference that this one is about a number, so the instrument
settles it.

**And the instrument now says the rule holds, by a route nobody had considered.** (That is the
*parsing* half of the sentence; the *bytes* half — "not the whole file" — was false until the
eight-hundred-and-eighty-first session read the file on disk where its offsets point, ADR 0809,
and `examples/open_cost` prints both routes.) 41% of the open
went in ADR 0180; what was left went *beside* the window in ADR 0182 and is joined after a device
bring-up that costs 13 to 19 ms. Measured in the two-hundred-and-eighty-ninth session, three runs
each:

```text
                         ISO 32000-2, 1023 pages    PDF20_AN001-BPC, 5 pages
document joined            +5.2 to +5.6 ms            +2.6 to +5.4 ms
process start → frame      110 to 119 ms              105 to 124 ms
```

**The two documents cost the launch the same**, and what the join measures is the handshake rather
than the file. That is the rule satisfied where a person can see it. What is *not* satisfied is the
rule read as a statement about `Document::open` itself — 10 to 13 ms against 0.2 — and item 1 below
keeps that question open, now correctly labelled as a question about the function rather than about
the launch.

## The five items, in the order the timeline ranked them

### 1. `Document::open` — **taken in the two-hundred-and-seventy-sixth session, 41% off**

§7.5's trailer and cross-reference table, for 101 318 objects. Localised with
`crates/pdf-syntax/examples/callgrind_open.rs` and fixed the same round: **40% of it was one
searched `BTreeMap` insert per entry**, re-deciding §7.5.6's "most recent copy" rule 101 318 times
for a rule that is a property of the whole file. One stable sort and one bulk build instead —
130.7 M instructions per open to 76.6 M, and the launch timeline's `document open` step from
+27.8 ms to +21.0. ADR 0180, with two allocation fixes beside it.

**What is left of this item is the design question, and it is still open.** §7.5.8's
cross-reference streams are a compressed table, and a processor that wants object 12 must find its
entry — which does not require materialising the other 101 317. Whether this tree can defer that is
a question about `xref.rs`'s `Option<Location>` map (ADR 0100), and the answer changes what
`was_recovered` and the writer can promise.

**The floor this paragraph quoted was four hundred and eighty rounds old, and its *ranking* was
what had rotted** — the total having been re-taken in the meantime and the composition not. It read
"76.6 M instructions, of which inflating the two cross-reference streams is 18 M and nothing can
remove it, so the remaining ceiling on this route is roughly a further 40%", and two rounds have now
gone at the three quarters it dismissed in a subordinate clause. ADR 0667 took §7.4.4.4's predictor
(−11.15%) and ADR 0677 took §7.5.8.2's `/W`, re-derived once per stream instead of once per record
(−10.49%, and the whole of it inside `read_section`). **Both were larger than the inflation the
sentence called irreducible**, and neither was on any list.

So the number a round should re-derive rather than read is what `callgrind_open` prints today, and
the two lessons this paragraph earned are worth more than any figure:

- **A composition decays faster than a total**, because a total is what a later round re-takes.
  Whoever re-runs a floor re-runs the attribution under it, or the ranking that decides the next
  move is the ranking of a tree that no longer exists.
- **`#[inline(never)]` on one function is the attribution instrument here.** Under this profile's
  fat link, callgrind reports a hot callee's cost against its caller's name with the inlined
  library code filed under `uint_macros.rs` and `slice/index.rs`; one temporary attribute and one
  rebuild turn that into a row. ADR 0677 §"How this was found".

### 1a. …and what was left of it is now *beside* the window rather than in front of it

**Taken in the two-hundred-and-eighty-first session** (ADR 0182). Nothing the window needs — an
event loop, a window, a graphics device — depends on the document, and the document depends on
none of them, so `main` opens it on a thread of its own and `resumed` joins that thread *after*
the presenter exists. `document joined` now lands 3 to 6 ms after `graphics device`, where
`document open` used to be a step of 21 to 28. `viewer-core`'s rule 4 is kept: the core is made on
that thread and moved back, single-threaded throughout.

**So items 1, 3 and 5's second bullet are the only launch costs still on this list that are ours**,
and item 1's is the design question rather than a number.

### 2. `Outline::read` — **closed by ADR 0182's structure, in the two-hundred-and-eighty-ninth**

The item was *eagerness*: 3 to 7 ms of a launch spent reading a panel nobody had opened, on a
document tree whose §12.3.4 thumbnails are deferred with an argument and whose outline is not.
**It costs the launch nothing now** — it happens on the thread that opens the document, which
finishes while the main thread is still bringing the graphics device up, and the join costs 5 ms
whether the outline has 988 items or 5. Deferring it would now buy zero milliseconds and cost the
title bar its §12.3.3 section on the first frame.

What is recorded rather than removed: the reason it is *allowed* to be eager is that something
slower runs beside it. If the device ever became free, this would be back.

### 2a. What the item said before, and it was wrong twice

**This entry said "6.716 ms for 38 items" when it was written, and 38 is `items.len()`** — the top
level, which for a book is its chapters. 988 items at 3.4 to 6.7 µs apiece is one indirect object
fetch and a text-string decode each, which is proportionate; the example prints both counts now.

So this is not a defect and the item is **eagerness** rather than cost: 3 to 7 ms of a launch is
spent reading a panel nobody has opened. §12.3.4's thumbnails are already deferred to the first
time their tab is shown, with the argument written down; the outline is not, and the reason it is
not is a real one — `Open::around` needs it for the title bar's section (`Outline::section_at`)
before the first frame. **The question to settle is whether the section is worth 3 to 7 ms at
launch**, or whether it should arrive on the frame after the page.

### 3. `signature::signatures` — **taken in the two-hundred-and-seventy-seventh session**

§12.8's field walk found nothing and charged 1.681 ms for it, on every launch of every document
with a form. The empty answer was reachable without it and the *standard says so*: §12.7.3's
Table 225 bit 1 exists so that a processor need not "scan the entire document for the presence of
signature fields", and Table 224 defaults the entry to 0. **1.681 ms → 0.017 ms**, counted over
the corpus before it was trusted, and it corrected a ledger row that had called the entry
"signature behaviour". ADR 0181.

### 4. The graphics device — **taken in the two-hundred-and-eighty-eighth session, 30 ms off**

ADR 0179's table said restricting the wgpu instance to Vulkan moves the cost from instance
creation into `request_adapter` and the total does not move, and that what was left to try was
**overlap** — an instance needs no window. That needed quorra's agreement, because
`Device::for_surface` created the instance itself.

**It agreed** (`doc/QUORRA_FEEDBACK.md` §8, answered at `7d5dafb`, their ADR 0014): five startup
fields where there were three, and `create_instance` + `for_surface_with_instance` so a host can
make the instance early. `main` now spawns *two* threads at its first line — one for the document,
one for the instance — and `resumed` joins the instance before building the presenter and the
document after it, because the device needs the first and not the second.

```text
graphics instance   +0.006 to +2.6 ms   (hidden behind EventLoop::new)
graphics device     +13.2 to +19.2 ms   (was 33.4 to 45.1)
start → first frame   110 to 119 ms     (was 145 to 152)
```

**What is left of this item is not ours**: `surface 0.03 ms, adapter 5.3, device 8.4` is what
bring-up now blocks for, and quorra's own headless numbers are 3.2–4.4 for adapter selection
against our 5.3–6.8 with a `compatible_surface` — the difference is the surface, and it is the
reason §8.1 asked for the field split.

### 5. The first frame pays ~12 ms of first-use allocation, and it is **not** the shaders

Measured in the two-hundred-and-eightieth session on the machine's real adapter, headless
(`crates/render-quorra/examples/first_frame.rs`): frame 1 costs 18.2 ms and frames 2 to 10 cost
3.7 to 5.1, and the difference is roughly fixed across scales — 13.3 ms at 1×, 14.3 at 2×, 18.1
at 4×.

**Sleeping between bring-up and the first render changes nothing** (16.05 / 15.26 / 16.65 ms after
0, 300 and 1000 ms), and the background thread reports its pipelines compiled in 5.3 to 5.7 ms —
so what the first frame pays for is device resource creation, not warmth. Two consequences:

- `CLAUDE.md`'s "nothing on the launch path waits for warmth" **costs nothing here**, and a
  `wait_until_warm` would buy zero milliseconds while hiding the twelve that matter.
- The ask is in `doc/QUORRA_FEEDBACK.md` §9: warm the *allocations* on the same background thread
  that already warms the shaders. Nothing about the API changes and ~12 ms comes off every cold
  launch of every host.

**Answered in part, and the remainder changed shape.** quorra timed the inside of `Device::render`
and found **2.43 ms of the first frame was making that frame's own timestamp query** — a `QuerySet`
and two buffers per frame, which the driver charges for the first time and pools afterwards. One
lives with the device now (its ADR 0031). Confirmed here on this side's own instrument, A/B/A with
eight samples an arm because the effect is smaller than the spread: the minimum first frame goes
**14.94 ms → 12.77 and 12.47**, and both `A` arms agree with each other.

**What is left is not an optimisation and cannot be warmed on a thread**: about 6 ms inside
`run_frame` that scales with the target — page-sized textures and the driver's first touch of a
heap that size — and a warm-up thread cannot allocate those before the viewport exists. quorra
records it as the caller's contract rather than taking it, which makes it *this* side's question:
whether to ask for a size hint or a `Device::warm_for(extent)`, and what a host would pass it
before it has a window. `viewer-ui` knows its viewport only after `Resized`, so the honest answer
may be that the first frame keeps this cost and the number is stated rather than hidden.

**`Device::warm_for` now exists and the answer is still that one**, decided in the
four-hundred-and-seventy-eighth session with a measurement rather than by re-reading this
paragraph. `examples/first_frame.rs` on page 7 of the specification, eight runs an arm across the
two quorra revisions, read at the **minimum** because the spread is several times the effect and
five other sessions were compiling on the box:

```text
                          first frame      frames 3-5
  scale 1   2c9bdd0          26.15 ms        7.54 ms
            a7babab          26.24           5.58
  scale 4   2c9bdd0          51.35          30.44
            a7babab          56.91          31.78
```

Nothing in the release moves either column by more than the spread, which is the expected result
and worth having anyway: 0036 to 0039 size a frame's **layer** textures, and a frame with no
transparency group allocates none — upstream's own census puts layered frames at about 8 % of the
corpus at 4×. So the first frame's fixed cost on a launch-shaped page is what it was.

**And the hint would not fit even where it applies.** quorra's ADR 0039 says so about ADR 0035 in
its own *what it cost* section: `warm_for` warms a **target-sized** layer, and after the plans are
sized to what they mark that is the right size only for a root that fills its target — about a
quarter of layered frames. Its headline of 24.7 ms → 10.3 was measured on a page whose root did
fill the target and is not a general number. Both halves of the reason to decline are therefore
now on the record: this host cannot call it, and where a host could, the size would usually be
wrong. `doc/QUORRA_FEEDBACK.md` §9.2.

**And it re-scales the whole timeline.** ADR 0179's 145 ms is `lavapipe` under `Xvfb`, where the
first present is 54 to 68 ms because llvmpipe is drawing the page on the processor. On the real
adapter the same steps are bring-up 33 to 43, interpretation ~5 and a first frame of ~18, so a
launch on this machine's own GPU should be **75 to 90 ms**. Nobody has run it — `AI` has no X
authority cookie for the user's display, so the window half of that number is the user's to
measure (ADR 0126).

## 6. The *native* hosts have a launch path too, and it is a different number

This file's five items are `viewer-ui`'s, which drives its own event loop and presents itself.
`pdf-viewer-gtk` and `pdf-viewer-qt` place somebody else's widgets, so their launch has a term
`viewer-ui`'s does not: **page one's answer comes back through a main loop that is inside the
toolkit's own first frame.** Measured in the seven-hundred-and-fifty-ninth on a quiet machine, twenty
alternating pairs an arm (ADR 0678): `viewer-gtk`'s `opened` → first frame went from 9.5 ms to 53.4
when the drawing moved to a thread, and back to 9.9 once a window with nothing on the screen waited
for page one instead of polling for it. The instrument is `--trace=launch` in either binary and
`--trace=launch,frames` for the wait beside the draw; `doc/verify.md` has the A/B's shape.

**Two things stay open here and neither is ours to close alone:**

- **The 44 ms was llvmpipe's magnitude.** What is structural is that page one waits on a loop inside
  its first frame; what that frame costs is the driver's. On a real adapter it is smaller, and by how
  much is item 5's last paragraph's question in a second window.
- **A native launch has never been measured cold.** Every number above is a warm page cache and a
  warm loader, which is right for an A/B and wrong for `CLAUDE.md`'s cold-start gate. The gate does
  not exist for these two binaries at all.

## What is deliberately not here

**Nothing, since item 5 — and item 6, which is a *second* launch path rather than a sixth item on
this one.** This section used to say the first present was not yet an item because
nobody had split it; it is split now, and the half that is ours is item 5's second bullet.
