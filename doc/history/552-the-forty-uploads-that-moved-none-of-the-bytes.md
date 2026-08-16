# 552 — The forty uploads that moved none of the bytes, and the hull nobody read

2026-08-16. An attribution round on the project owner's `tmp/Entwurf.pdf` — one page, 58 009 display
commands, 3.0 M path segments — against their own `tmp/trace2.entwurf.txt`. ADR 0387 has the
argument and every number; `doc/QUORRA_FEEDBACK.md` §29 is what went upstream.

## The brief, and what it turned on

Every millisecond a correct frame costs is a millisecond ADR 0378's reprojection has to cover, so
the round was told to take the owner's zoom frame apart and reduce what this tree owns:

```
frame 272.0 median | scene 12.6 | device 258.4 (encode 128.9, transfer 64.3, execute 0.2, elsewhere 62.6) | 40 up
```

Two of the four items were premises rather than facts, and testing them was most of the round.

## The finding that changed how the round was run

**A headless quorra device comes up on the owner's real GPU for the agent user.** No window, no
session, no X authority cookie: `render_quorra::options()` names no adapter, so
`QuorraRasterizer::new_headless` reports *AMD Radeon 890M Graphics (RADV STRIX1)*. Every earlier
performance round in this project — including ADR 0368's, whose whole table is captioned "this
machine's software adapter" — took its numbers from llvmpipe under `Xvfb` on the belief that the
adapter needed the owner's session. It never did; only the *window* does.
`doc/environment.md` and `doc/todo/45` §4 are corrected, and every absolute in ADR 0387 is the
owner's hardware.

## The three answers

- **`transfer` is not the forty uploads and never was.** The frame line's `up` counts *resources*
  this tree's caches handed the device; `transfer` is quorra staging its own encoded scene. Read for
  the first time from `FrameCost::bytes_uploaded`, which has crossed this boundary unread since ADR
  0227: the frame that hands over **58 029** resources moves **6 898 596** bytes and the zoom frame
  that hands over **40** moves **8 475 012**. More bytes from fourteen hundred times fewer uploads.
  Nothing this tree stops handing over shortens that phase.
- **`elsewhere` is host time nobody times.** `Timings::phases` is carried across now, and the two
  entries quorra's own documentation directs a caller to read the remainder against — `target
  acquire` and `present` — are **0.035 ms** and **0.001 ms** against a remainder of over a hundred.
  What is in it, from upstream's own source: `submit_and_wait`, which quorra measures and then
  discards in favour of the adapter's timestamp, and `record_content`, which nothing times at all.
- **`scene` fell 20.5 %**, by deleting one line. `Encoder::fill` computed the device-pixel window of
  every fill up front; its only consumer is a §8.7.4.5.4 radial cone, and this page has none. The
  cost was not the arithmetic but `Path::bounds`'s first call, which walks the path to memoise a
  hull nothing else asks for — 3.0 M segments walked for an answer nobody reads.

And the row that decides the rest: **`execute` is 0.07 % of this frame.** The graphics device does
about a thousandth of the work; everything a person waits for is one host thread.

## What was built

| | |
|---|---|
| `crates/render-quorra/src/scene.rs` | the window asked inside the branch that reads it; `emit_fill` takes the shape |
| `crates/render-quorra/src/present.rs` | `FrameCost::readback`, and `QuorraPresenter::last_phases` |
| `crates/render-quorra/src/lib.rs` | `QuorraRasterizer::last_phases` |
| `crates/viewer-ui/src/bin/pdf-viewer/timing.rs` | the transferred-bytes line, `at_rank` generic, the `elsewhere` comment corrected |
| `crates/render-quorra/examples/zoom_frame.rs` | new: the zoom-step instrument, on the real adapter, without a window |

## The evidence

**callgrind**, `ZOOM_FRAME_ENCODE_THREADS=1`, one open plus two frames: **27 132 909 847 →
26 865 611 146 instructions, −267 298 701 (−0.99 %)**, of which `Path::walked` **227 021 102 → 0**,
`Path::bounds` **12 470 645 → 0**, `Encoder::commands` self **25 957 897 → 13 023 472**.

**Wall clock**, the owner's adapter, ten interleaved rounds an arm at load 2.8 → 3.2, the zoom
frame's `scene` in ms: before min **11.2** median **13.15**; after min **8.9** median **10.35** —
**−20.5 %** and **−21.3 %**, the two agreeing being what makes the claim worth stating on a shared
machine. The *first* frame is not claimed: 227 M of the saving is a once-per-path hull walk that
lands on `first scene built`, and a ten-round pass could not separate it from the machine's drift.

## Byte-identity

The changed value's only consumer receives exactly what it received before, from the same path under
the same transform, computed by the same function. Structurally, `git diff --stat` touches
`render-quorra` and `viewer-ui` and no crate on the interpretation path — `pdf-model`, `pdf-syntax`,
`pdf-render`, `pdf-font` are untouched, so `examples/display_list_digest` compiles from identical
sources in both arms and cannot move. The pixel proof is the gates below, and the quorra lanes in
particular: a page that paints a cone is exactly what would change, and the corpus carries several.

## Gates

Every one run after the last edit. `fmt` clean; `clippy --workspace --all-targets` silent of Rust
lints (the `viewer-qt` `cargo:warning=` lines are gcc's on a cold build, as always); the full
workspace test run, doctests, corpus, oracle, both text gates, both quorra lanes and conformance —
each recorded verbatim in the round's report. The one gate that caught something caught this round's
own prose: `every_citation_names_a_clause_that_exists` rejected `QUORRA_FEEDBACK.md §29` in a doc
comment, because a `§` in this tree is checked against ISO 32000-2's clauses and would have passed by
landing on one. The house form is "section N", and the checker is right to insist.

## What only the owner's machine can still say

The window: the swapchain, the present, the cadence, and whether the 20.5 % of `scene` is visible
beside the 95 % of the frame that is quorra's. A script was left at `tmp/run-on-gpu.sh` for the
graphical-session loop; the loop stopped answering at 20:20 and it was never claimed.
