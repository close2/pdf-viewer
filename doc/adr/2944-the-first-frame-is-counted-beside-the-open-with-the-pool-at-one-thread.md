# 2944 — The first frame is counted beside the open, with rayon's pool at one thread

Session 2065. Status: **accepted** and **built** in `crates/viewer-ui/tests/launch_path.rs`
(`phase_count_first_frame`, `counted_kinstructions`) and `doc/checks/launch-path.toml`
(`frame_kinstructions`, one band a row). It moves no existing band and no ledger row. Context: ADR
2926 (the counted open keeps no viewport, because the program's open has none), ADR 2916 (the
counted open no longer raises `/PV`, and its two floors are restated), ADR 2666 (an open's
instruction count is this program's own object, because the loader and the C library are the
system's), ADR 1321 (a page's photographs are decoded beside each other), ADR 1531 (page one is
interpreted while the device comes up).

## 1. What no figure counted

Since `79d0c6c4` the program's open does nothing a viewport decides: `quorra.rs`'s `open_document`
builds `Viewer::new(0, 0, 1.0)`, its document thread then calls `anticipate` (through
`viewer_host::Ahead`, ADR 1855), and the window, at the join, obeys Table 29's `/PageMode` in
`receive` and dispatches its first `Command::Resize` (`window.rs`, `take_the_opening`). That `Resize`
settles: it arranges, raises Table 197's `/PV` through `visibility_of` (§12.6.3), and asks for page
one's render. `open_kinstructions` counts the open alone (ADR 2926), and no other band file holds an
instruction figure, so page one's interpretation, the arrangement and `/PV` were seen only by the
first-page clocks, whose bands on this machine are tens of per cent wide (ADR 2916 §3).

## 2. The sequence counted, and where it stops

A child, phase `count-first-frame`, does what the program does on its two threads, in order: the
harness's `open_document` (the program's, step for step, ADR 2926 §2), `Viewer::anticipate`, the
page mode's panel (`Sidebar::show` of `Tab::of_page_mode`, as `obey_page_mode` does), and
`Command::Resize` at 800 × 1000 less that panel's inset, scale 1.0. **800 × 1000** is the window's
default inner size (`with_inner_size(LogicalSize::new(800.0, 1000.0))`); **scale 1.0** is the scale
every figure of this gate is taken at, under which logical pixels are device pixels. Three rows
open with a panel (`PDF20_AN001-BPC`, `Well-Tagged-PDF-WTPDF-1.0`, ISO 32000-2), so their page is
500 pixels wide. **The count stops at the `NeedsRender` request**: the display list is the core's
last word, and what follows — the host's hand-over, the device's encoding, the draw — is the
graphics device's and needs a device the child does not bring up. The window's own handling of the
open's events (title, panel lists, terminal lines) is the toolkit host's and is not counted; the
one answer of it that changes what the core is asked, the panel, is. The figure is ADR 2666's: the
self cost of the test binary's own object on every thread, the whole process printed beside it.

`Viewer::anticipate`, and not the program's `Ahead::anticipate`: `Ahead` draws page one beside its
read only once the read has run `GLIMPSE_AFTER`, a clock that callgrind slows fifty-fold, and a page
read inside that wait is read and taken in as `Viewer::anticipate` takes it (`viewer-host`'s
`a_page_read_inside_the_wait_is_taken_in_as_anticipate_takes_it_and_drawn_after`).

## 3. The pool at one thread

Counted as the open is, the first frame was not a property of the program. Four, then eight more,
children of one release build of HEAD, before its sandbox worker was built (own object, thousands):

| row | pool as the machine gives it | pool at one thread |
|---|---|---|
| `bug1815476.pdf` | 66678.4 to 91805.5 | 63548.8 to 63549.0 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 41304.8 to 44065.2 | 41197.6 to 41199.4 |
| `ISO_32000-2_sponsored_EC3.pdf` | 160660.9 to 162245.1 | 157676.7 to 157676.9 |

The other three rows did not move either way. By function, the font-walk row's high children ran
`decode_with_parms_reported` and `image::unpack` twice: the decode ahead of the `Do`s (ADR 1321) and
the interpreter decoded the same images. The 57-page row's spread is the idle workers' search for
work (`Stealer::steal`, `crossbeam_epoch` pinning, `WorkerThread::find_work`). Both are the
scheduler's, and callgrind serialises threads, so the share each run got was a lottery.

**The child is counted with `RAYON_NUM_THREADS=1`.** `plan_decodes_ahead` asks for two threads and
declines with one, so each image is decoded once, by the interpreter, and four children agree to
0.005 %. The open does not touch the pool and is counted as before.

## 4. The bands, and the sandbox worker

This file's rule over four one-thread children of the gate's own build (`517bf240` plus this
round's harness): the observed range widened by one per cent, rounded outwards. No
`open_kinstructions` band moved.

The first bands were taken from a build whose `pdf-sandbox-worker` had not been built, and the
first judged run put the font-walk row outside them: 64752.1 against 62913 .. 64185, with 1522
commands where the children had drawn 1490. One of that page's images is decoded in the sandbox
worker; with none beside the binary the decode was refused, the image not drawn, and
`ImageSource::is_opaque` spent 2.5 million on the refusal where the drawn image's
`fill_packed_rows` spends 3.9. That is trap 10, and the gate builds the worker inside its hold for
exactly this reason, so the bands are the build with the worker. The worker's own decode runs in a
process callgrind does not follow (`--trace-children` is off), so it is not in this figure either;
the clocks see it.

## 5. What it costs

- **The decode ahead is not in the figure.** Its plan, its walk and any decode it duplicates are
  seen by the first-page clocks only. The duplicate decode measured here under callgrind (up to
  25 million instructions on the font-walk row) is a question about the program, not about this
  figure: whether it happens at native speed is unmeasured.
- **A regression in the host's handling of the open's events, or in the device's encoding, is not
  in the figure.** The clocks see both.
- **The window's size and scale are stated, not derived.** A change to `quorra`'s default window
  changes what the program does and leaves this figure where it was until the constant follows it.
