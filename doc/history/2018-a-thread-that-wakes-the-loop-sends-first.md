# 2018 — A thread that wakes the loop sends first, and its test asks at the wake

Session 2018, a single round, the ledger lane, on a race `viewer-ui`'s own test found in round 2011's merge gates.
ADR 2854. No clause, no ledger row; no Q849.

**The slice.** `a_finished_frame_wakes_the_loop_after_it_is_sent` failed once under load ("the wake came before the
frame it is for was sent"). Asked: can the composing thread wake the loop before its frame is in the channel?

**The premise.** It did not hold as a defect of the thread. `compose_until_told_to_stop` sends `Finished::Frame`
and then wakes; a look sends `Finished::Partial` and wakes only on a send that succeeded. The test took the first
wake to be the frame's, but a draw past `GLIMPSE_AFTER` (500 ms) is looked at, and the look's wake comes first; an
empty 60 × 84 page does that on a starved machine.

**Reproduced.** The test alone in a loop, at `nice 19` under 64 busy processes at `nice 10`: the old test failed 2
of 4 400 (0 of 2 020 unloaded). A diagnostic copy printing what each wake found saw a look before the frame in 9 of
4 000 runs and never an empty wake: the mechanism.

**Built.** `every_wake_comes_after_the_news_it_is_for_is_sent` replaces it: the wake closure drains the channel on
the composing thread before it returns, every wake must find news, and the test reads wakes until one finds the
frame. No sleep stands for an order. The comment over the frame's wake now says the send comes first and why.
**After**: 0 of 6 400 (4 400 under the same load). **Plant**: the frame's send and wake swapped fail it 200 of 200;
restored, it passes.

**Elsewhere** (a Sonnet subagent, then the one caveat read here): `viewer-device`'s render thread (frame, sharp,
scene, read, partial) and `read_back`'s `send`; `viewer-host`'s page-turn read, submission, URL and policy fetches,
archiving; the machine-faces search; the accessibility bridge. All publish before they wake. The page-turn read's
`Ends::drop` wakes before its closure returns, so `land`'s `join` may block a moment; it is documented there and
never misses the read.

**Left.** Nothing of this race. The diff is a test and a comment: no type changes, so no fuzz check is owed, and no
non-test code a walk builds differs, so no tier-2 walk can move.

**Gates.** `rustfmt --check` and `cargo fmt --check -p viewer-ui`: exit 0; clippy `-D warnings --all-targets`
`-p viewer-ui`: exit 0; nextest `-p viewer-ui`: 141 of 141 (1 skipped); rustdoc `-D warnings --document-private-items`
`-p viewer-ui --bins`: exit 0; conformance after `git add -N`: exit 0 once this paragraph was written (the first run
failed only this record, its gates unstated); `state.sh docs`: exit 0. Tier 2 (`corpus`, `raster_golden`, the viewer
censuses, `launch_path`, `script_corpus`) not run: nothing outside `#[cfg(test)]` changed but a comment.
