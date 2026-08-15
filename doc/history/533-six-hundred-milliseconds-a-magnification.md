# 533 — Six hundred milliseconds a magnification, and the phase nobody had divided

2026-08-15. One round, a design round asked by the project owner: **could `tmp/Entwurf.pdf` be
rendered every frame?**, with permission to answer that the cost is too high and an instruction to
step back — maybe the boundary between quorra and this viewer should move, maybe the IR should
change, maybe something else. ADR 0368 is the decision; this is the record.

## What was built

Nothing. Three temporary probes were written, used and removed before the commit: quorra's
`Options::instrument_encode` wired to an environment variable in `QuorraPresenter`'s two surface
constructors with `Timings::phases` printed, `viewer-ui`'s `coverage_for` forced to
`Coverage::Gpu`, and a display-list census over `pdf_render::command_extents`. ADR 0368's last
section names all three so the next round need not rediscover them.

## Where the frame goes

`Xvfb :91` at 900×1100, llvmpipe, the release binary of this tree, three scripted sessions —
settle, `+`, `+`, `-`, `-` — load average 1.6 to 13 on a shared 24-core machine. The cull counts
are deterministic and identical across every run and against the owner's own trace (8763 and 17986
at the two zoom steps), which is what says the runs are the same five frames.

The frame each session ends on — the whole page at the fit view, 58 009 commands, nothing culled,
the magnification new — 639.8 / 660.0 / 661.9 ms, attributed on the first:

| | ms | share |
|---|---:|---:|
| `scene`, ours | 15.8 | 2.5% |
| `encode`, quorra, host processor | 475.9 | 74.4% |
| `transfer` | 65.4 | 10.2% |
| `execute`, the adapter's timestamps | 29.1 | 4.5% |
| `elsewhere`, a bound | ~52 | ~8.1% |

And `encode` divides — the first use of `Options::instrument_encode` in this tree:
**geometry 79.2%**, recording 16.2%, staging 4.6%. So 59% of the frame is one thread turning
3 011 879 path segments into 58 003 coverage tiles.

The control that identifies the phase rather than merely naming it: the same view drawn twice in one
session costs `encode` 483.8 ms and then 90.6, and the second draw's subdivision is geometry 1.7
against recording 91.8. So the 406 ms is coverage rasterisation and the 92 ms that survives every
cache is instance building. A repeat view is 140 ms and a new one 640 — though the fifth frame
returns to the *first* frame's view and pays full geometry, which ADR 0368 records as open rather
than explaining away.

## Where the launch goes

Three runs: document joined 64–103 ms, interpretation **+996 to +1038**, first scene built +317 to
+327 (which is the first frame's 58 029 resource uploads, not the scene walk), first present +643
to +740 — 2.04 to 2.21 s in all.

Interpretation by callgrind, `RAYON_NUM_THREADS=1`: **14 662 M instructions**. Lexing 37.6% by self
cost where ADR 0332's table read 63.6%, the flate 17.0% and now interleaved through ADR 0365's
window rather than a prologue, the allocator ~14.0%, the interpreter loop 16.8%, operand conversion
8.7%.

## The design space, priced

| candidate | worth on this document |
|---|---|
| nothing | a still window already replays at 21.5–34.7 ms; a zoom step is 640 |
| a page-space scene under a root affine | our `scene` phase only: 2.4% |
| batching by paint state | 590 of 58 009 commands, 1.0%, longest run 3 — and a loss once merged bounding boxes reach quorra's tiles |
| a device-side geometry buffer | measured and declined by quorra's own `take_gpu_lane`: forcing `Coverage::Gpu` changed nothing |
| tiling / damage | zero on a zoom, and quorra's `encode` never reads the damage list |
| level of detail | forbidden — §10.7.4's "no shape ever disappears", and 48.1% of this page's commands are under a device pixel at the fit view |
| `encode` on more than one thread | the only item with a factor in it |

## What went out

`doc/QUORRA_ENCODE_THREADS.md`, the ask, written beside `doc/QUORRA_FUNCTION_PAINT.md` in the same
voice — and it states its own ceiling: with geometry at *zero* the frame is still ~235 ms, so what
it buys is a zoom step of 250–300 ms rather than sixty frames a second.

## Gates

No code changed, so nothing could regress; run anyway. `cargo fmt --all --check` clean;
`cargo clippy --workspace --all-targets` silent of lints (the 20 `warning:` lines are `viewer-qt`'s
cold-build gcc noise, which `doc/todo/02` §2 documents); `cargo nextest run --workspace` **1952
passed, 15 skipped**; `cargo test --workspace --doc` ok; `cargo test -p conformance` ok.

One setup finding worth leaving: `pdf-font`'s corpus tests walk `doc/*.pdf`, so a worktree that has
symlinked only the standard itself fails five of them with *"no name-keyed bare CFF font in the
corpus"*. Link all fourteen.
