# ADR 0423 — The fifth of a launch nobody owned

Status: accepted, 2026-08-18. Session 588. Divides `render_quorra::FrameCost::scene`, finishes the
launch table with the first frame's own phases, and amends `doc/todo/44`, `doc/todo/45` §5 and
`doc/performance.md`'s launch section. Amends §10.8.3's ledger row on the spec-driven side.
Adds `doc/QUORRA_FEEDBACK.md` §33.

## What this round was sent to do, and what it found already done

`doc/todo/README.md`'s line for item 44 read: *"A draft the owner supplied that takes ten seconds to
appear and a third of a second per frame — evaluation owed, starting with the launch-table hole
`--trace` must learn to name."*

Both halves of that sentence had expired, and the file it indexes says so: session 497 closed the
launch table's hole (ADR 0332) and sessions 506 and 535 took the two levers under it (ADRs 0341,
0370). **Measured on this machine with the binaries this round built, the owner's document appears
in 1.37 to 1.48 seconds**, and the owner's own two later traces — `tmp/trace2.entwurf.txt` and
`tmp/trace3.entwurf.txt`, which nobody had read back against the index line — say 1.61 and 1.71 on
their hardware. The ten seconds is gone and the index line was the last place still claiming it.

That is not a licence to stop. `CLAUDE.md`'s startup rules make launch latency a first-class number,
and a 1.4-second launch is still a person waiting. So the round asked the question one round later
than the item was written for: **what is the 1.4 seconds now, and which of it is nameable?**

## The naming hole today, and it is not the one the index line named

The table on this machine, `Xvfb` and `llvmpipe`, one of four runs:

```text
document joined          65.011 ms  (+13.351)
interpreted, 58009 cmd  784.090 ms  (+719.079)
first scene built      1100.865 ms  (+316.775)
first present          1445.382 ms  (+344.517)
```

Every row is named and no gap is unattributed — ADR 0332's work holds. **What is unnameable is
what is inside the last two rows**, and between them they are half of this launch:

- `first scene built` is `FrameCost::scene`, one number since ADR 0227, covering both this crate's
  walk of 58 009 display commands *and* every `upload_*` call it makes across quorra's boundary on
  the way through. `doc/todo/45` §5's second bullet has said since the five-hundred-and-twenty-ninth
  session that "`scene` is measured and nothing inside it is", with the honest note that on a zoom
  frame it is 2.5% of the frame and nobody had needed to divide it. On a *launch* it is a fifth.
- `first present` is the frame's device work, whose three phases `FrameCost` has carried since ADR
  0227 and which the trace prints **only in the summary at exit, as medians over every frame of the
  run**. A distribution cannot answer a question about the one frame a person waited for, and a
  launch measured by killing the window prints no summary at all.

Both are the same failure one round on from ADR 0332's: a step whose name does not say what happened
in it. So both are closed here.

## What was built

**`FrameCost::handover`** — the part of `scene` spent inside quorra's own `upload_*` calls — and
**`FrameCost::outline_segments`**, its denominator. Every upload in `render-quorra` goes through
`Encoder::handing_over`, and the three outline sites through `Encoder::upload_outline` on top of it,
which is where the segments are counted. The argument a resource is built from is deliberately
*outside* the timed span: expanding a stroke, averaging an image and converting a path's segments
are this crate's work and charging them to quorra's would make the division a lie.

Three decisions inside that, each of which could have gone the other way:

- **The accumulator lives on `ResourceCaches`, beside `stored`.** That type's own comment said a
  count "costs an increment where a timer at each site would cost a clock read", which is an
  argument this round had to answer rather than ignore — it is answered below, with an A/B. Beside
  `stored` because the two are one fact: `stored` counts the misses and `handed` times them.
  Transient outlines and the fallback raster reach no cache, so both are added explicitly rather
  than hung off the miss path, or a frame drawn entirely from transients would report nothing
  handed over.
- **A denominator ships with the duration.** ADR 0387's finding was a duration read against a count
  that was not its denominator; a new duration with no denominator at all would be the same trap
  wearing the other shoe. An outline's upload costs by the *segment*, and a page of glyphs and a
  draughtsman's line work differ by two orders of magnitude in segments per resource.
- **The launch table prints the first frame's phases under itself**, off the same `FrameCost` the
  frame line reads, rather than inventing a second instrument. `Launch::arrived` takes the frame
  that closed the timeline; a window with no graphics device says so in one line instead, because
  a launch that did not put page one on the device is the fact worth printing there.

## What the instrument costs, which is the A/B a new clock owes

`examples/zoom_frame` on `tmp/Entwurf.pdf`, the frame that hands over 58 029 resources — the worst
case this instrument has — arms alternating in one sitting, `git apply -R` and `git apply` of this
round's own patch, minima of three rounds each, `scene` in ms:

| | run 1 | run 2 |
|---|---:|---:|
| instrument on | 216.4 | **201.9** |
| instrument off | 216.2 | 210.0 |

The two arms are inside each other's spread and the *on* arm holds the lowest sample of the four.
116 058 clock reads on the frame that pays most are below what this machine's run-to-run variation
does to the same code, which is the answer `ResourceCaches`' old comment was owed.

## What the division says, and it is a finding rather than a tidy-up

**`examples/zoom_frame`, the real adapter** (AMD Radeon 890M, RADV STRIX1), the page fitted to
800×228, minima of five rounds, ms:

| frame | scene | of it handed over | uploads | segments |
|---|---:|---:|---:|---:|
| first (cold) | 187.6 | **156.0 — 83%** | 58 029 | 3 011 919 |
| second (a zoom of the same display list) | 9.7 | 0.2 | 40 | 4 550 |

**The same run under callgrind**, `encode_threads` pinned to 1, inclusive:

| | Ir | share |
|---|---:|---:|
| `Encoder::commands` — the whole walk | 2 128.7 M | 8.21% |
| — `Device::upload_outline` | 1 743.2 M | 6.72% |
| — — `QuadOutline::from_segments` | **1 476.9 M** | 5.69% |

Two instruments, one answer: four fifths of the launch's scene translation is inside
`upload_outline`, and six sevenths of *that* is quorra converting this page's cubics into
quadratics — 490 instructions a segment.

**And the frame never reads them.** `encode::fill` is the only reader of a stored outline's
`quads`, behind `take_gpu_lane`, which answers `false` on sight under `Coverage::Cpu` — the
caller's default, and what `viewer-ui` draws with below ten times magnification (`GPU_COVERAGE_MAGNIFICATION`).
So every launch of this program pays 156 ms to build a representation that frame will not look at,
and the second frame does not look at it either.

## Why the fix is not taken here

It is `upload_outline`'s, and `upload_outline` is quorra's. The right shape is laziness — build the
quadratics on the first GPU-lane use of an outline — and it cannot be a flag on this side, because
an outline uploaded under `Coverage::Cpu` may be drawn under `Coverage::Gpu` after a zoom: the host
knows no more at upload time than the library does. `doc/QUORRA_FEEDBACK.md` §33 is the ask, with
both tables, the call site that reads `quads` and the two fallbacks we would take instead.

That is the round's honest result on the demand-driven half: the attribution is real, taken with two
instruments that agree, and **the remainder is priced at 156 ms of a 1.4-second launch** — 11% of it
on the real adapter, 15 to 17% of the `llvmpipe` launches, and it is upstream's.

## What the 1.4 seconds is, phase by phase

Four launches under `Xvfb`, and the shares are what carry rather than the absolutes:

| | ms | share |
|---|---:|---:|
| everything before the document is joined | 62–93 | 5% |
| interpretation of page one, 58 009 commands | 670–719 | **48%** |
| scene translation | 285–316 | 21% — of which 221–245 handed to the device |
| the device: encode 135–195, transfer 71–92, execute 38–45, elsewhere 52–58 | 305–381 | 24% |
| the present itself | 11–12 | 1% |

The interpretation is still the largest item and it is ours, and the callgrind run this round took
re-dates `doc/performance.md` §3c's warning rather than contradicting it: `Lexer::next_token` is
**40.2%** of `interpret` and §7.4's inflation through ADR 0365's window is **23.1%** — so a quarter
of the launch's largest step is decompression, which no table in this tree had said. ADR 0370's
three levers are gone from the profile, which is what a taken lever looks like.

## What this changes for a reader of the trace

`--trace=launch` on any document now ends its table with the first frame's own phases, and
`--trace=frames`' summary carries a `handover` row nested under `scene`. Nothing else moved: the
row is a subdivision rather than a phase beside one, and a reader who added it to the rows above it
would count the same milliseconds twice — which is why it is indented and says so in a comment.

## The spec-driven half: §10.8.3, and a claim this tree's own code disproves

§10.8.3's row was `reported` — "not implemented yet, but detected and reported at runtime" — with
**no `code` and no `test`**, and a note whose reason was that separation simulation is something "a
processor performs when asked to, and nothing in a PDF asks for it".

A PDF does ask for it. Table 275's `SeparationSimulation` requirement type is "Requires support for
simulation separations as described in 10.8", naming this subclause in the same sentence, and
`pdf_model::requirements` has carried that type by name for as long as it has existed — citing
§10.8.3 in its own doc comment, two hundred rows away from the ledger row denying the route existed.
So the report is real, its condition is a document stating the requirement, and the row now names
the code and the two tests that hold it.

The reading also sharpens what is owed. §10.8.3 imposes nothing: its verb is a permission — a
simulation "can be performed" — and its four steps are a `should` conditional on performing one,
over §10.8.1's "Whether separations are produced is up to the processing software". So the debt is
owed to Table 275 rather than to the clause, and what is missing is the *control*, which is the part
of the old note that survives intact.

This is the sixth refusal shape from `doc/habits.md`'s ledger section, in its sharpest form yet: not
a capability that arrived and announced nothing, but a capability that arrived, cited the very
clause whose row denies it, and was never read back.
