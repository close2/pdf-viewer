# 0283 — A release that changed a lane nobody was measuring

**Status.** Accepted.
**Context.** quorra released three commits; the project owner asked what they change here. The
last time that question was asked (ADR 0282) the answer was structural — the gates said to have
moved do not link quorra at all. This time the answer is a measurement, and the measurement did
not exist when the question was asked.

## What the release is

`c1f6e2f4` → `74c4994d`, three commits, and every line of all three is inside quorra's **GPU
coverage lane**:

- `9624ac0` cuts the winding target into bands, and re-derives the lane crossover.
- `6ef954e` makes that target a *pane* over the lane's own tiles rather than a band of the shared
  scratch sheet — and fixes a shader agreement under which every band but the first discarded
  every fragment.
- `74c4994` walks the scene before encoding it and counts how often each outline is placed, so the
  lane turns on what the atlas will *do* with a tile rather than on whether it would accept one.

## The instrument that did not exist

`render-quorra`'s corpus gate — the only place in this tree where a backend is put beside the CPU
oracle at the corpus's scale — has always run quorra's **default** lane, which is the CPU one.
`viewer-ui` switches to the other lane past `GPU_COVERAGE_MAGNIFICATION`, ten times magnification,
so **the lane a person sees when they zoom in had been judged by fixtures alone.** That is trap
12b's exact shape one lane over: fourteen small scenes said the Vello backend was fine and the
first real page at a window's size came back blank.

So the first thing this round built is a knob: `PDFVIEWER_QUORRA_COVERAGE=cpu|gpu` on the gate, and
`FIRST_FRAME_COVERAGE` on `examples/first_frame`. Three properties of it are decisions rather than
plumbing:

- **A value that is neither is a panic, not a fallback.** The variable's whole purpose is to say
  which lane produced the numbers under it; a typo that quietly measured the default would be a
  survey headed `gpu` that is not one.
- **The other lane skips the ratchets**, like the scale knob and for one more reason: the two lanes
  are *stated* not to draw identical pixels (quorra's ADR 0016 bounds the sampled lane against the
  exact one), so a differing-page list is a property of the lane that produced it.
- **The default is unchanged**, so the gate's own verdict still comes from the lane this tree draws
  every page with.

## What the release changed here

Every row is this machine — AMD Radeon 890M, RADV, Vulkan — `--profile gates`, the whole corpus,
one run per cell unless stated. **The CPU-oracle column is the control**: it is the same work at
both pins, so where it moves, the machine moved.

### The lane we draw with: nothing, in both directions

| Coverage::Cpu, scale 1 | agree | differ | refused | oracle | quorra |
|---|---:|---:|---:|---:|---:|
| `c1f6e2f4` | 917 | 35 | 5 | 2.16 s | 6.23 s |
| `74c4994d` | 917 | 35 | 5 | 2.20 s | 6.19 s |

Not one page's line changed. That is the release's own claim about this gate, arrived at
independently, and it is the third time a quorra release has been null on it.

### The other lane at the page's own scale: four documents that used to draw nothing

| Coverage::Gpu, scale 1 | agree | differ | refused | oracle | quorra |
|---|---:|---:|---:|---:|---:|
| `c1f6e2f4` | 904 | 44 | **9** | 2.01 s | 4.68 s |
| `74c4994d` | 908 | 44 | **5** | 2.27 s | 6.13 s |

The four are `bug1703683_page2_reduced.pdf`, `issue12810.pdf`, `issue1905.pdf` and
`issue9418.pdf`, each refused at the old pin with a winding target over the frame budget — 605,
311, 1324 and 481 megabytes against 256 MiB — and **each now agrees with the CPU oracle**. The
differing list is identical at both pins, so nothing moved out of agreement to pay for them.

quorra named two of the four. The other two are this gate's, which is what a corpus is for.

**And the extra 1.45 s is those four pages, not a slowdown.** A refused frame is not timed, so
four pages joining the timed set adds their cost: measured directly at the new pin, twice, they
are **1.26–1.27 s** of quorra against 0.216–0.221 s of oracle. That leaves 4.86 s against 4.68 for
the other 953 — beside an oracle column that moved 2.01 → 2.05 on the same subtraction. A null on
speed, with the machine's own drift the same size.

### The other lane at a magnification, which is where the program uses it: 24

| Coverage::Gpu, scale 4 | agree | differ | refused | oracle | quorra | median ratio |
|---|---:|---:|---:|---:|---:|---:|
| `c1f6e2f4` | 900 | 16 | **36** | 10.20 s | 17.45 s | 2.87× |
| `74c4994d` | 920 | 20 | **12** | 12.15 s | 20.69 s | 2.55× |

**Twenty-four pages the second lane could not draw at four times the page's own scale now draw,
twenty of them agreeing with the CPU oracle.** Not one page went the other way: the four that
land in *differs* are four of the twenty-four, at means 0.12 to 1.10 and worst tiles 13.7 to
31.7 — `bug1743245`, `issue12295`, `issue19971`, `transparency_group`.

This is the number that matters, and it is the one nobody would have got from the release notes:
at 1× the release closes four holes, and at the magnification where this program actually selects
the lane it closes twenty-four.

### The first frame on a real page: nothing measurable

The release's headline is a first frame 2.5 to 3 times faster on tiles the atlas admits and the
page places once. Page 7 of ISO 32000-2 through `examples/first_frame`, GPU lane, three samples
each, milliseconds:

| | frame 1 | frame 10 |
|---|---|---|
| `c1f6e2f4`, 596×842 | 21.8 / 20.6 / 15.0 | 4.1 / 7.0 / 4.1 |
| `74c4994d`, 596×842 | 21.0 / 25.8 / 17.7 | 7.8 / 4.6 / 4.0 |
| `c1f6e2f4`, 2382×3368 | 44.5 / 55.0 / 44.4 | 36.2 / 25.6 / 29.2 |
| `74c4994d`, 2382×3368 | 51.8 / 52.4 / 51.2 | 26.6 / 25.7 / 30.5 |

Every band overlaps. A page of dense text is the case quorra's own commit says had to stay
untouched — its outlines are placed many times each, which is the atlas working — so this is the
release behaving as described rather than a contradiction of its table. **The corpus at 4× is
where the release's mechanism shows, and it shows as pages drawn rather than as milliseconds.**

## A claim in this tree that the release made false

`doc/quorra-gpu-coverage.md` ends with three things "the lane still does not do", and two of them
have stopped being true:

- *"Nothing chooses per command. A page mixing a huge headline with body text gets one lane for
  the whole frame."* The lane is chosen per command now, by what the atlas will do with the tile.
- *"No atlas stands in front of it."* One does, for every tile the atlas admits and the page
  places more than once — which on this corpus is the median page.

Neither was found by a sweep. Both were found by running the lane and reading the commit that
changed it, which is the same lesson as every stale claim this project has caught: a document
about somebody else's code decays on their schedule, not ours.

## Consequences

- The corpus gate can be pointed at either lane, and the second one now has a corpus-scale
  baseline for the first time.
- `doc/quorra-gpu-coverage.md` is corrected where the release overtook it.
- Two questions go back to quorra as `doc/QUORRA_FEEDBACK.md` §20: the twelve refusals that
  remain at 4× — eight of them frame-budget or scratch-image limits, four of them this side's
  §11.4.6 knockout hole — and `transparency_group.pdf`'s worst tile of 31.7 at 4×, which is
  larger than the sampled lane's stated spread and is a question rather than an accusation.
- **What is deliberately not done: the lane is not switched anywhere.** Nothing here says the GPU
  lane should draw a page at 1×; it says the lane a person reaches by zooming refuses twenty-four
  fewer of the corpus's pages than it did, and that this tree can now see such a thing happen.
