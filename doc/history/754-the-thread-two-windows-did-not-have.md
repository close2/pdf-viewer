# 754 — The thread two windows did not have

`doc/todo/30`'s open item, taken: `viewer-gtk` and `viewer-qt` no longer rasterise inside their
`Event::NeedsRender` arm. `viewer_host::drawing` is one arrangement for both, and ADR 0657's
interrupt policy reached them in the form a tier-1 host can state exactly. ADR 0668. Date
2026-08-25.

## What was built

- `crates/viewer-host/src/drawing.rs` — `Drawing`, `Finished`, and the thread. A queue rather than
  one slot, because a tier-1 host owes an answer for *every* request and a column asks for several.
- Both hosts: the `rasterize` call replaced by `drawing.ask(request)`, a `take_the_drawn` that
  answers the viewer, a `take_the_thread_back` that supplies the one fact `viewer-host` cannot ask
  for itself, and the toolkit's own one-shot — `glib::timeout_add_local_once` and a third `QTimer`
  in `window.cpp`, with `drawing_wait`/`drawing_pump` on the `cxx` bridge.
- `viewer-host` took a dependency on `render-cpu`, and its manifest comment stopped saying "nothing
  here draws".

## The finding: a tier-1 host states the rule more exactly than the host it was copied from

`viewer-ui`'s policy is a judgement about pixels — would a finished picture be presentable or
stand-in-able. A tier-1 host holds a **token**, and `Viewer::rendered` drops a `RenderReady` whose
token is not the one outstanding. So *is this answer still wanted* is decidable, and abandoning is
provably free of trap 20: the two conditions the code acts on are exactly the two that take a token
away.

That is a sentence `viewer-core` already had, written for the opposite direction — "a worker that is
slow costs a wasted render and never a wrong frame" — and read backwards it says when a host may
stop drawing.

## Numbers, and which of them is the program's

Under `Xvfb`, three other rounds on the machine, load average 13 to 40 across the session.

- **The handoff, in-run, with the loop otherwise idle: 2.16 ms in GTK and 2.13 ms in Qt.** The frame
  line prints the draw and the wait, so their difference is the arrangement's whole overhead,
  measured inside one process. Two toolkits agreeing to 0.03 ms is what "a thread and a channel"
  predicts.
- **The poll period is worth measuring and was measured**: median in-run gap on a page turn **6.9 ms
  at `POLL` = 1 ms** against **15.0 ms at 16 ms**, from two release builds differing in that
  constant alone, four page turns per run, three runs of each arm, alternating. The 16 ms arm
  centres half a period above the round trip, which is what a poll predicts.
- **The launch path could not be separated from the machine.** `opened` → `first frame on the
  screen`, ten alternating pairs against a build of `3f4ee908`: before 9–100 ms, after 62–448 ms.
  Both arms drifted an order of magnitude across the same sequence. The structural cost is one extra
  trip through the toolkit's main loop plus a `thread::spawn`; on an idle loop that trip is the
  2.1 ms above. **A quiet-machine launch A/B is owed and `doc/todo/30` says so.**

## What the screen said

`amplified3-754.pdf` — 1333 bytes, 1000 page-covering fills — in both windows, with three zoom-ins
700 ms apart driven through XTEST while the first draw was still running. GTK: the page drew for
4.842 s, the first frame landed at 5.540 s, and the next two draws were **abandoned after 737 and
699 ms**. Qt: 3.086 s, then abandoned after 702.6 and 702.7 ms. Before this round the key presses
would have queued behind the whole draw.

## Instruments left behind

In the scratchpad, named for the round rather than committed because they are about a machine:
`amplify-754.py` (the Rust fixture builder in Python; the Rust file says why the document itself is
not committed) and a `git worktree` of `3f4ee908` with its own `--target-dir` for the before arm.

## Gates

The core — `fmt`, `clippy --workspace --all-targets` under `RUSTFLAGS="-D warnings"`, `nextest`,
doctests, the `fuzz/` check — plus `cargo test -p conformance`, by §2's map: the change is confined
to `viewer-host`, `viewer-gtk` and `viewer-qt`, none of which any gate rasterises with. §5's
binaries were rebuilt and installed before the measurements.

## Ledger

Untouched. `CLAUDE.md` principle 3 against principle 2, citing no clause. The one clause the new
module names — §12.4.4.1, for the transition faces a finished page can begin — is `partial` and its
behaviour did not move.
