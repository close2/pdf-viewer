# Q32 — Should the launch path's memory high-water be a ceiling rather than a band?

Raised by session 932, whose `doc/todo/02` §2 sequence failed on all four rows of the launch gate's
`peak_mib` — every one of them **below** its floor — on a tree whose only functional change is nine
lines in `pdf_model::image`.

`Q29` is round 931's and is about the *clock* half of this gate. This is the other half, and
`Q29`'s own text is what makes it a separate question: it says the eight figures with no clock in
them "are judged on any machine". Four of those eight are not.

## The question

`crates/viewer-ui/tests/launch_path.rs` bands each document's memory high-water on both sides.
The upper edge is a leak guard and is doing its job. **The lower edge is a claim nothing in this
program controls**, and it has now failed a gate twice for a reason no code change explains:

> `peak_mib` was in that group for most of a round and is not, and the demotion is a finding. It is
> the high-water mark of a process that has brought a graphics device up […] and an hour later —
> same tree, same binary, idle machine — all four rows had fallen together by about 12%. What moved
> is the *driver's* allocation, which nothing in this process can see the reason for

— `doc/checks/launch-path.toml`'s own comment, written when the band was widened the first time.

**Should the floor go, leaving `peak_mib` a one-sided ceiling?**

## Why it cannot be settled without you

Because a one-sided gate is a decision about what a perf gate is *for*, and `CLAUDE.md` principle 2
says only that "memory high-water" is a gate and that "a regression fails the build". A high-water
that falls is not a regression by any reading — but a *floor* is the only thing that would catch a
figure the harness stopped measuring at all, which is the failure mode ADR 0884 built the whole
guard against and which trap 16 and trap 33 are both about. So the choice is between a gate that
cannot cry wolf and a gate that can tell a real zero from a driver's good mood, and there is no
clause and no measurement that decides which this project wants.

## What was measured

Taken alone, on an idle machine, at one-minute load averages between 5.0 and 7.7:

| | this tree | `main`, the change taken back out | band floor before this round |
|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 98.5 – 100.1 | 111.9 | 127 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 103.4 – 104.8 | 116.2 | 131 |
| `ISO_32000-2_sponsored_EC3.pdf` | 103.8 – 104.5 | 128.9 | 132 |
| `bug1815476.pdf` | 116.4 – 116.5 | 140.6 | 143 |

Two things follow and both matter. **`main` fails these four figures too**, so the failure is not a
round's change; and **two runs of the identical binary ten minutes apart read 108.9 and 99.3** for
the first row, which is nine megabytes of movement with nothing moving in the program. Seven
consecutive runs then sat within a megabyte of one another — the figure is stable *within* a
plateau and wanders *between* them, which is the signature the file already names: the driver's.

## What the tree does meanwhile

**Nothing is blocked, and the gate is green.** This round set each floor a few per cent below the
lowest value observed today — 95, 100, 100 and 112 — and left every ceiling untouched, which is the
rule the check file already states for this figure ("its band spans what has been observed rather
than one afternoon's value"). The evidence and the arithmetic are in that file's comment beside the
rows.

## Recommendation

**Take the floor off `peak_mib` and put a floor on `open_peak_mib` instead**, which is the figure in
the same row that has never moved: it "came back identical in every one of the forty-four runs the
bands were derived from — no spread at all — because [it is a property] of the reader rather than of
the afternoon", and it has no graphics device in it. That keeps a two-sided claim about the *program's*
memory, which is what would catch a harness measuring nothing, and stops making a two-sided claim
about the *driver's*, which is what keeps failing. It costs one line of harness and one of the check
file, and this round did not take it because retiring half of another round's gate on its own reading
is not a thing a round does.
