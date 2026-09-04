# 0902 — The guard is the best of fifty, and every figure is a first pass

Session 931. Status: **accepted**. The first of this round's two records; the second,
[ADR 0903](0903-what-a-declined-figure-now-says-and-what-a-quiet-machine-still-owes.md), is the
change that follows from it.

## Context

[ADR 0885](0885-what-the-launch-path-costs-and-which-of-principle-2s-claims-hold.md) brought
`CLAUDE.md` principle 2's four numbers under a gate, and session 926 ran that gate on `main` four
times. Twenty-seven of its twenty-eight figures held; one did not.
`doc/PDF20_AN001-BPC.pdf`'s cold open, banded `0.49 .. 0.80` ms, read **0.87, 1.30, 0.72 and
0.86**, and two of those four runs had both probes in band, judged the figure and failed.

Session 926 **did not move the band**, which was right, and wrote three hypotheses into
`doc/todo/42` with the experiments that would separate them:

1. the gate's second probe measures *throughput* where this figure is *latency*;
2. that round had swept the build directory, and the gate's cold arm reads a copy it makes beside
   it — so every copy since the sweep lands on freshly allocated extents;
3. a real regression in the open.

This round ran them. **All three are refuted, two of them by construction**, and what is left is
a fourth thing none of them named.

## What each experiment said

### Hypothesis 2 is refuted by `filefrag`, in one command

`std::fs::copy` on Linux is `copy_file_range`, and on btrfs `copy_file_range` is a **reflink**.
`filefrag -v` on the gate's four copies and on the four repository files they were made from
prints the *same physical extents*, flagged `shared`:

```text
launch-path/PDF20_AN001-BPC.pdf   0: 0..31  184487366..184487397  encoded,shared
                                  1: 32..42 184483740..184483750  last,encoded,shared,eof
doc/PDF20_AN001-BPC.pdf           0: 0..31  184487366..184487397  encoded,shared
                                  1: 32..42 184483740..184483750  last,encoded,shared,eof
```

So **no copy this gate has ever made has had an extent of its own**, and a sweep of the build
directory cannot move a byte of one. The A/B confirms it: a copy written with `--reflink=never`,
which does get fresh extents, reads cold in `0.109 / 0.125 / 0.543` ms (min, median, max of forty)
against the reflink's `0.109 / 0.127 / 0.613`.

The general form is worth more than the instance, and it is where the hypothesis came from:
**a copy is not a rewrite, and on a copy-on-write filesystem it is not even a write.** A round
reasoning about where a file landed has to ask the filesystem rather than the source of the copy.

### Hypothesis 1 is refuted in the form it was written, and true in a finer one

The disk is not what moves this figure, and the evidence is in the *warm* arm. Over twelve
consecutive runs of the gate on merged `main`, every run in which the five-page cold open left its
band had its **warm** open leave its band by a *larger* proportion:

| run | cold open | ×median | warm open | ×median |
|---|---|---|---|---|
| 2 | 0.81 | 1.13 | 0.60 | 1.28 |
| 7 | 0.82 | 1.14 | 0.65 | 1.38 |
| 9 | 1.35 | 1.88 | 1.01 | 2.15 |

A warm open reads a file whose pages are already in memory. **There is no disk in it**, so a
latency probe beside the throughput one would have seen nothing that mattered — and the probe that
exists reported `2.0 .. 4.5` ms in all twelve runs, including those three.

What is true is the shape underneath the hypothesis, and it is more general than a disk:
**a probe has to be made of the same stuff as the figure it guards.** The calibration probe is the
*quickest of fifty passes inside one process, taken after that process has already run its phase*.
Every figure it guards is **one first pass in a fresh process**. By pass fifty the allocator, the
caches, the branch predictors and the core's own clock have been warmed by the forty-nine before
it; a first pass has none of that. Over those twelve runs the probe moved **1.3 %** — 0.703 to
0.749 ms — while the figures it stands guard over moved by factors of two.

### Hypothesis 3 is refuted by the diff and by the probe

Nothing on the open path has changed since the bands were taken. `git diff db4a76f1 HEAD` over
`pdf-syntax`, `pdf-model`, `viewer-core` and `pdf-font` is session 925's outline change — which
replaces one page-tree walk with the same walk held in a `OnceCell`, and which
`Outline::section_at` was already making unconditionally — plus `type3.rs`, `attachment.rs`'s
tests and an example. Not a line of `Document::open` or of `Open::around`'s cost.

And the probe says the same thing from the other side. The calibration work *is*
`pdf_syntax::Document::open` plus `Pages::new` plus `pdf_model::interpret`, on this very document,
and its band `0.62 .. 0.78` was derived in session 922. It reads **0.703 to 0.749** today. That
function has not got slower.

## What the twelve runs actually showed, and it is not what 926 was looking at

Two corrections to the picture in `doc/todo/42`.

**The band that will not sit is not the one 926 named.** Of six failing runs in twelve, five failed
on `doc/pdf.js/test/pdfs/bug1815476.pdf`'s cold open — banded `0.32 .. 0.50` — and one on
`PDF20_AN001-BPC.pdf`'s. That row's band came from fourteen runs where the other three rows' came
from thirty, and reversing the file's own derivation rule (`low = min × 0.85`,
`high = max × 1.20`) puts its observed range at `0.376 .. 0.417` against today's median of
**0.50**.

**Every clock figure's median today sits within a few percent of its derived maximum**, high for
the two small documents and *low* for the two large ones:

| figure | derived range | today, 12 runs | median against derived max |
|---|---|---|---|
| `bug1815476` cold open | 0.376 .. 0.417 | 0.42 .. 0.95, median 0.50 | **+20 %** |
| `bug1815476` warm open | 0.271 .. 0.292 | 0.28 .. 0.60, median 0.34 | **+16 %** |
| `PDF20` cold open | 0.576 .. 0.667 | 0.67 .. 1.35, median 0.72 | +8 % |
| `PDF20` warm open | 0.412 .. 0.475 | 0.45 .. 1.01, median 0.47 | −1 % |
| `WTPDF` cold open | 2.294 .. 2.55 | 2.44 .. 5.16, median 2.50 | −2 % |
| `ISO 32000-2` cold open | 22.59 .. 25.33 | 22.35 .. 50.73, median 23.16 | −9 % |

A regression cannot do that. A *machine* can: the smaller the figure, the larger a fixed
per-operation cost is as a fraction of it, and the first pass in a fresh process is where such a
cost lives. Session 922's forty-four runs were taken at one-minute load averages of 1.6 to 5.0
with nothing else on the box; these twelve were taken at 3.5 to 13.3 with two parallel rounds
compiling and walking a corpus.

## Decision

**The band was not moved, for the second round running, and the reason is now a finding rather
than an inability to choose.** A band is a claim about a machine; these figures were not taken on
that machine, and the instrument that was supposed to say so could not. Widening a band to admit a
loaded machine would put the loaded machine into the claim.

What changes instead is the guard, and it is [ADR 0903](0903-what-a-declined-figure-now-says-and-what-a-quiet-machine-still-owes.md).

## Consequences

- **`doc/todo/42`'s three hypotheses are closed** and replaced by what the experiments found. The
  file keeps the two that cost nothing to re-derive: the `filefrag` command, and the twelve-run
  table above.
- **The rule this leaves a round** is the one at the top of the second experiment, and it is
  general: *a probe has to be made of the same stuff as the figure it guards*. ADR 0884's own
  sentence — a guard has to sense every subsystem the figure it guards is made of — is about
  *which* subsystems; this is about the *state* they are in, and the launch gate satisfied the
  first and not the second.
- **A gate whose bands were derived on a quiet machine is a gate that needs a quiet machine**, and
  this project runs three rounds at once. That is not a defect this round can fix from inside the
  harness, and it is `doc/questions/Q29`.
