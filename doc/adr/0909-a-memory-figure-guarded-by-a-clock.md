# 0909 — A memory figure guarded by a clock, and the band this round could not derive

Session 934. Status: **accepted**. The round's other record is
[ADR 0908](0908-two-questions-called-q27.md).

## Context

This round was sent to derive one band **only if the machine was genuinely quiet**: the first-pass
calibration probe session 931 added (ADR 0903), which is printed and not judged because banding it
needs an idle machine and session 931 had loads between 3.5 and 35.7 throughout.

**The machine was not quiet, and it was less quiet than 931's.** A sampler took the one-minute
load average every thirty seconds for the seventy-five minutes from 17:58 to 19:13 — **151 samples,
minimum 3.30, median 12.86, mean 16.69, maximum 61.55, and 62 % of them above 10**. Session 931
reported 3.5 to 35.7. Three neighbouring rounds were running rather than two: 932 and 933 were named
in this round's instruction, and `/home/AI/cargo-target/pdfv-r935/` appeared during it, running
`launch_path` probes of its own. At 18:24 both 932's oracle and 933's `optimize_corpus` were walking
the corpus side by side at 642 % and 629 % of a core.

**So no band was derived**, which is the instruction's own rule and the third round in a row to
decline: a band derived on a busy machine is a claim about a busy machine. What this ADR records is
what was found instead, which is worth more than the band would have been.

## What the sequence found

`doc/todo/02` §2 ran whole on the merged result. Thirty of its thirty-one lines exited 0. The
launch line exited **101**, and not on a clock:

```
launch-path: calibration 0.706 ms, band 0.620 .. 0.780
…
the launch path moved:
  doc/PDF20_AN001-BPC.pdf: the memory high-water is 99.137, outside peak_mib 127.000 .. 209.000
  doc/Well-Tagged-PDF-WTPDF-1.0.pdf: the memory high-water is 103.113, outside peak_mib 131.000 .. 214.000
  doc/ISO_32000-2_sponsored_EC3.pdf: the memory high-water is 104.195, outside peak_mib 132.000 .. 215.000
  doc/pdf.js/test/pdfs/bug1815476.pdf: the memory high-water is 116.480, outside peak_mib 143.000 .. 231.000
```

Every clock figure held. All four **memory** figures were a quarter below their minimum, together,
and the guard that let the run judge them at all is the calibration probe — which read 0.706 ms
against `0.620 .. 0.780` and was, by its own units, correct.

## It is not a regression, and that is checkable rather than argued

**The program is byte-identical to the `main` this round merged into.** The merge's whole diff over
`crates/` is `pdf-model/tests/oracle.rs`, `pdf-model/tests/text_extraction.rs` and
`viewer-core/tests/selection_census.rs`, plus `tools/pdfref/`; not one line of the launch path, and
the release binaries Cargo was asked to rebuild before the measurement came back `Finished` in
0.18 s because nothing they are made of had changed.

**Nine re-runs alone say the same thing in figures.** Run alone between 19:04 and 19:20 the peak
resident read **161 to 182 MiB on every document on every run** — inside every band, none of them
near an edge — and two of those runs judged with **0 outside**, one of them judging all 28 figures
with nothing declined at all. The failing run and the green ones differ in one measurable thing:

| | the failing run, 18:49 | the nine runs alone, 19:04 – 19:20 |
|---|---|---|
| free memory | ~9 GiB, with 19 GiB of swap in use | 29 GiB, 45 GiB available |
| what had just run | two neighbours' corpus walks | nothing |
| calibration probe | 0.706 ms, **in band** | 0.708 to 0.745 ms where judged |
| `peak_mib`, four documents | 99.1, 103.1, 104.2, 116.5 | 161–164, 168–169, 168–169, 180–182 |
| `open_peak_mib`, the memory figure **with no device in it** | 7, 8, 18 MiB — in band | 7, 8, 18 MiB — in band |

The last row is the one that names the mechanism. `open_peak_mib` is the high-water of an open with
no graphics device brought up, and it did not move by a megabyte. What halved is the resident set of
a process that *has* brought the device up — the driver's own allocation, which
`launch_path.rs`'s own doc comment already says "nothing in this process can see why" about, having
watched it fall 12 % between two runs an hour apart while the bands were being derived. Under memory
pressure it falls by 40 %.

## The finding: trap 34 has a third dimension, and it is the units

Trap 34 says *a guard has to be made of the same stuff as the figure it guards*, and session 931
wrote it about **state**: the probe is the quickest of fifty warm passes and every figure it guards
is one first pass in a fresh process. ADR 0884's own sentence — a guard has to sense every subsystem
the figure is made of — is the version about **which subsystems**.

This is the version about the **units**, and it is the sharpest of the three because nothing in the
gate's construction hints at it. `peak_mib` is classified `steady: false`, which means *judged only
where the calibration probe says this is the machine the bands were taken on* — and the calibration
probe is a **clock**. A clock cannot sense a memory allocator. So a figure whose known cause of
movement is the driver's allocation is gated behind a probe that would read in band on a machine
with a gigabyte free and a machine with sixty, and it did: 0.706 ms, judged, on a machine whose
available memory was a fifth of what the bands were taken under.

**A clean number about a question nobody asked**, which is the shape trap 33 and trap 34 already
share. The version to keep is: *a guard has to be made of the same stuff as the figure it guards —
the same work, in the same state, and in the same units.*

## What was changed, and what deliberately was not

- **No band was moved and no figure was reclassified.** Three rounds have now declined to widen a
  band on this gate, on one argument: a band widened to admit a machine has put that machine into
  the claim. Reclassifying `peak_mib` would turn a red gate green on a round's own reading, which is
  worse.
- **`doc/todo/42`** carries the finding and what it owes, beside session 931's, because that is the
  file a round working on this gate opens.
- **Trap 34** gains the third dimension, in the group whose rounds spring it.
- **`doc/questions/Q29`** gains the evidence, marked as this session's. It asks the owner about the
  *clock* half of this gate and recommends deriving a busy-probe band; this run says that
  recommendation is **necessary and not sufficient** — a busy-clock probe would have declined
  nothing here, because the clock was fine.

## What is owed next, priced

A memory probe beside the two that exist: what the machine had available when the sample was taken,
banded the way the disk probe is, so that `peak_mib` declines on a pressed machine instead of
failing. It is the same five-part construction ADR 0884 costs, and its band needs the same thing
this round did not have — an idle machine, about ten minutes. It joins `Q29`'s first option rather
than replacing it: one probe for the clock's state, one for the memory's.

**And there is a cheaper thing that is not a probe.** `peak_mib`'s minimum is what failed, and a
minimum on a memory high-water is a strange thing to hold: it is there to catch "we stopped doing the
work", which `first page`'s command counts, `open_peak_mib` and the timings already witness four
different ways. Whether principle 2's *memory high-water* gate should be a ceiling rather than a band
is a real question with a real answer, and it is not this round's to take on its own — it is a
change to a gate another round built, and the same sentence that stopped this round widening one.
