# 839 — The drag that was two gestures

An attribution round on `doc/todo/47-the-resize-frames.md`, which had carried "9–19 ms per step of
a drag" since ADR 0704 recorded it in passing and had never been opened. The file's own rule was
the brief — *do not build before attributing* — and nothing was built.

## What was done

- §5 first: the release binaries were older than `HEAD` (`tools/round.sh` said so) and were
  rebuilt and installed before a single number was taken.
- The window was driven under `Xvfb` with `xdotool windowsize` — forty-step drags and twelve-step
  settled sequences, on `doc/ISO_32000-2_sponsored_EC3.pdf`, `tmp/Entwurf.pdf` and
  `doc/PDF20_AN001-BPC.pdf`, with `--trace=frames,events,window,panel`. The owner's measurement
  loop has not ticked since 2026-08-29, so nothing was queued to it.
- One A/B sitting, the round's cap: `--supersample 2` against `--supersample 1`, alternating, to
  remove ADR 0699's sharp pass and see whether the drag noticed.
- One headless run on the real 890M through `render-quorra/examples/zoom_frame`, for the one term
  that needed the real adapter.
- ADR 0766 is the reading; `doc/todo/47` is rewritten around it and `doc/todo/README.md`'s line
  with it.

## What it found

The core is 6–12 µs and interpretation is not on the path at all. The surface reconfigure and the
chrome rebuild — the file's own first two guesses — are both below the spread of the frame they sit
in. A resize has two arms, and which one a document is in decides everything: the page's raster
either follows the window or it does not, and only the first has a cost, which on the real adapter
is quorra's `encode` and nothing else.

Two things nobody had predicted: ADR 0699's sharp pass reruns on every step of a drag (and costs it
nothing here, and never starts on the arm where it would hurt, because ADR 0761's budget declines
it — that budget watched working on a gesture it was never measured on); and §14.7's tree is
republished on every step, at 0.8–1.3 ms, which is the largest term left on the event thread once
`present` belongs to the adapter. That last one is left open as a decision for the owner rather
than taken as a patch.

## Second track

`doc/conformance/ledger.toml` §12.5.3, `partial`, read against `Viewer::settle`. Its NoZoom
sentence was wrong twice: it named the *gesture* where the trigger is a change of magnification —
so a window resize under a fit mode takes the same path a wheel notch does — and it stated a share
over 974 documents, a denominator the same row's own census replaced with 1126 curated and 65 703
crawled. Both corrected; the status stays `partial`, which bit 8 still owes.
`doc/todo/46-a-wheel-tick-that-interprets.md` gains the second gesture. The `parts` and `owed`
sweeps were run beside it; `parts` produced only its documented noise shape and `owed`'s reading
list is unchanged.

## Gates

`cargo fmt --all --check`, `cargo fmt --manifest-path fuzz/Cargo.toml --check`,
`RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets`, the same for
`fuzz/Cargo.toml`, `cargo nextest run --workspace`, `cargo test --workspace --doc`,
`cargo test -p conformance`, and `--bin quotations` and `--bin pointers` for the moved documents.
The change is documents only.
