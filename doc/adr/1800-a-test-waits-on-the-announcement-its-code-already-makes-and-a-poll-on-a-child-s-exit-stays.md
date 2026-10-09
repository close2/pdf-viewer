# 1800 — A test waits on the announcement its code already makes, and a poll on a child's exit stays

Session 1482. Status: **accepted** and **built**. The shape is ADR 1780's and ADR 1794's, read
against the thirteen `thread::sleep` lines left in `crates/*/tests/*.rs`. Code: the nine test files
named below; no crate under test changed.

## What was there

Thirteen sleeps in nine files, in four kinds:

- **A poll for an answer the code under test announces.** `viewer-host`'s `fetch_import.rs`,
  `submit.rs` and `signature_policy.rs` (twice) looked at `Submitter::collect` every 10 ms. Each
  `Submitter` method already takes a `wake`, which its thread calls once the answer is on the
  channel. That is how `winit`'s proxy hears of it. The two `signature_policy` loops were a race as
  well: 500 looks of 10 ms each are a five-second bound on a loopback answer, on a shared machine.
- **A guess at a child's start.** `render-raster`'s `turn_path.rs:908` slept 2 s for the planted busy
  device's bring-up before it read the device's busy counter. A slow bring-up (a cold pipeline cache,
  a contended driver) would have read an idle device and failed the calibration.
- **A settle that decided nothing.** `pdf-sandbox`'s `reaping.rs:91` slept 300 ms before counting
  children. `Connection`'s `Drop` kills and waits inside the refusal. So a reaped worker has left
  `/proc` before `confinement` returns, and an unreaped one is counted whether it is running or a
  zombie.
- **A bound with no event to wait on.** These are the four polls on a child's exit
  (`save_round_trip.rs`, `text_extraction.rs`, `foreign_corpus.rs`, `headless.rs`), the abandoned-slot
  count in `in_process_deadline.rs`, and `turn_path.rs`'s counter cadence (`DEVICE_READING_GAP`) and
  load back-off (10 s).

## Decision

**A test that waits for an answer waits on the announcement the code under test already makes.** The
four `viewer-host` tests pass a `wake` that sends on a channel the test owns. They wait on it with
`recv_timeout(ANSWERED_WITHIN)`, then take exactly one answer from `collect`. The bound now only
catches a failure: 1 min, or 30 s in `submit.rs`, which keeps its former figure. This is the window's
own path, so the tests now cover `wake` as well. Calibrated (trap 13): with `wake()` not called, all
seven tests that go through these helpers fail at the wait, at once, because the dropped closure
disconnects the channel. With the answer not sent, all seven fail at "there is one".

**A test that needs a child's work in progress holds it at a line the child prints.** The plant
prints `BUSY_FROM_HERE` once its first dispatch has been answered. The parent reads the plant's
stdout up to that line, then drains the pipe so that the plant's last line has somewhere to go.

**The hold exposed a second wait that the 2 s had been covering: the driver's counter lags.** Read
once, straight after the line, the first run said 23.9% planted and 78.2% after the plant had ended
(threshold 20%). In the ten back-to-back runs below, every run after the first read about 40%
"before", which was the previous run's plant still decaying. The gate's neighbour has been busy for a while, so the calibration reads the counter
repeatedly until a reading is above the threshold, for as long as the plant is still busy with a
second to spare (`PLANT_SECONDS`, 6). That makes it a bounded wait on the counter, not on a guess.
Ten runs in a row: each was above the threshold at its first reading, 25.9% in the first run and
46.2 to 47.1% in the other nine. Calibrated (trap 13): at a planted threshold of 1000%, the loop
read on until the plant ended (7.1 s, at most 99.0%) and failed.

**A sleep that decides nothing is removed.** Calibrated two ways. With `Drop` killing but not
waiting, the stranger test fails ("left alive"). With no `Drop` at all, it fails on `left: "1"`. The
`/bin/true` test passes under both plants, because `died` waits on that worker. That matches the
module's own account.

**A poll on a child's exit stays, with its reason beside the sleep.** The standard library's
`Child::wait` has no deadline. The two other ways to bound it are to kill by pid from another thread,
or to call `pidfd_open` and `poll` through `rustix`, which is a Linux-only dev-dependency in three
crates. Either would save at most one poll interval (10 to 25 ms) per reference run. The verdict
never rests on that interval: it is the child's exit status or output, and the budget is a kill for a
hung reference. `pdfref` has the same loop twice (`reference.rs`, `extract.rs`). So a shared
`wait_within` belongs there, and is work for a round that owns `tools/pdfref`. `in_process_deadline`
keeps its 50 ms poll on `in_process::overran_still_running`. A notification beside that count would
be code in the decode path that only this test uses. `turn_path`'s cadence is the measurement itself:
a mean over an interval. Its back-off waits on a neighbour's load, which no test can be told the end
of.

**A pipe found under a poll is replaced by a file.** `save_round_trip`'s `ask` piped the witness's
stdout and read it only after the poll ended, which is the deadlock its neighbours' comments warn
about. Both streams now go to files. `a_witness_that_says_more_than_a_pipe_holds_is_read_whole`
prints 256 KiB: through a pipe it is killed at the 30 s budget ("timed out after 30s"), and through
a file it takes 0.03 s. No corpus document reaches this today. Over the 974 files of
`doc/pdf.js/test/pdfs`, the mupdf witness prints at most 10 863 B (`prefilled_f1040.pdf`, whose
poppler witness prints 4 954 B). That is a sixth of a 64 KiB pipe buffer. So this removes a latent
defect, and no excluded document moves.

## Consequences

- `grep -rn thread::sleep crates/*/tests/*.rs` prints seven lines. Each has its reason in a comment
  beside it, which cites this ADR.
- Every changed test passed ten runs in a row. The `turn_path` gate, run once under `--clock`
  before the counter loop was added, kept its 33 figures inside their bands.
