# 1811 — A page still being read past the warning is a bar, and the window answers beside it

Asked by the project owner on 2026-10-10: "I don't like that we limit the number of operations, if we
have valid files, which are not shown. As long as it isn't a resource problem (only a time problem)
we should allow it, and possible display a loading bar." This is the bar. Status: **accepted** and
**built**. Context: ADRs 0182, 0241, 0657, 0668, 1531, 1808, and slot 1's ADR on the count (1809).
Code: `crates/pdf-model/src/progress.rs`, `crates/viewer-host/src/loading.rs`,
`crates/viewer-confined/src/notice.rs`, `confined_transport::frame::NOTICE`. Drive:
`tools/drive-windows.sh`'s `loading_bar` (step 75). Measurement: `viewer-host`'s `examples/loading_cost`.

## 1. Where the wait was — the brief's premise, and the part that did not hold

The brief placed the wait in the confined worker. **Three of the four windows hold their viewer in
process** (`viewer-confined`'s own module comment says so), and none of them waited in the drawing
thread: each opens the document on a thread of its own, has page one interpreted there (ADR 1531),
and then *joined* that thread when its window first had a size. On the owner's file that join held
the toolkit's thread for the whole interpretation — no repaint, no key, no close. `quorra-confined`
is the one window whose page is interpreted in the worker, and it joined the same way. So the
change is one shape in four windows, and the confined window's figure has to cross a pipe.

## 2. The figure

`pdf_model::progress::Progress` is three relaxed atomics: operators run, and the page's `/Contents`
input bytes taken out of held. **Bytes over bytes is the bar because it is the only fraction with a
known end**: no stream states its operator count, no filter states its output length (§7.3.8.2
Table 5's `/DL` is optional and "only a hint"), and the encoded length is the stream object's own.
`pdf_syntax::Pump::taken` and `ContentReader::progress` read it; a form, a glyph or an appearance is
not the page's stream and does not move the bar, so a page whose cost is one form shows a bar that
pauses, which is true. The operator count stands beside the share as a count, never as a fraction.

**The hook is one line**, inside the operator loop's existing periodic check (every `CHECK_EVERY`,
64 operators, the list budget's): `crate::progress::publish(self.operations, || reader.progress())`
in `content/run.rs`. It publishes to a handle a caller installs on its own thread for one call
(`Progress::watch`). **Priced against threading a parameter** the way the ledger is threaded: that
is a few dozen signatures between a page and its forms, patterns and glyphs, for a value none of
them computes with. The installed handle costs a thread with nothing installed one thread-local
look per check and nothing else, and the interpretation stays a pure function of the bytes —
nothing is read back. Measured with `examples/loading_cost` on the owner's page, three alternating
rounds each: **7.613 s bare, 7.548 s watched (−0.85 %, inside the noise)**; the figure the reading
thread saw ran 1 % → 100 % against 0.4 → 29.0 million operators in step, linear in both.

## 3. The windows — `viewer_host::Loading`

The opening thread is a `Loading`, shared by the four windows. **The wait is bounded by `WARN`**:
`Loading::wait` blocks for what is left of `drawing::WARN` (1 s) since the thread began, so any
document ready inside it — every launch-gate document — is joined the moment it is ready, as
before, a channel's wake standing where the join's was. A document not ready is left to its
thread; the window goes on answering its toolkit, looks again every `loading::TICK` (two 60 Hz refreshes), joins on the look
that finds the thread ended, and tells the core the window's size *then*. While it waits it shows
`loading::reading`'s sentence and the share: `GtkProgressBar` under the status label, `QProgressBar`
in the status bar (`loading_permille`), and a twenty-cell bar in the title for the two windows that
have no status band and say every wait there. **This is the `WARN` of ADR 0668 and not a deadline**:
nothing here ends an interpretation (ADR 0657). The person's way out is the window's close, which no
longer waits for the page, and in `quorra-confined` Escape, which ends the worker (ADR 0241). Driven
in `quorra-gtk` only: its window destroyed mid-read ended the process in 86 ms with the page unread.
Under `Xvfb` with no window manager nothing else delivers a close — `xdotool windowquit` closed no
window even with its page in, and `windowclose` ends GTK's and none of the other three.

## 4. Across the confinement — a notice, not a request

The worker performs every command under `Progress::relaying`, which hands a reading every
`RELAY_EVERY` (32 768) operators to `confined_transport::link::tell_the_host`: a `NOTICE` frame,
kind `0xF2`, 24 bytes of payload (`viewer-confined`'s `notice`), written on the port
`ask_the_host` armed and under its lock, so it adds no system call. `Host::read_frame` hands a notice
to a `Listener` and goes back to reading the answer it was waiting for; `Confined::report_progress_to`
stores what arrives in the window's figure. The greeting moved to `PDFVCF14`, because an older host
would read the new frame as a message. **Priced against the brief's two alternatives**:
`ask_the_host` is a round trip — the interpreter would stop every N operators until the host had
answered, which is a cost on the work and a re-entrancy hazard where the host is the one waiting;
and a shared counter needs memory both processes map, which this tree has no safe way to make and
the confinement no reason to admit. A notice costs one `write` of 33 bytes per relay, about 885 on the
owner's page. Counted in operators rather than timed, since the worker reads no clock it was not
handed.

## 5. What this does not do

- **No stop for an in-process interpretation.** The core has no interrupt and adding one would
  make a half-read page an interpretation; the close is the stop, as stated above.
- **Only the launch is a bar.** A page turn or a re-interpretation after the launch is still
  interpreted on the toolkit's thread (the core is single-threaded, its rule 4), so a second heavy
  page would hold the window as before. Moving those off the thread is a larger argument.
- **`quorra`'s processor fallback on the owner's page says nothing for its ~10 s**: the device
  refused the frame (423 MB of scene against 256 MB) and `App::mind_a_long_draw` warns only for the
  `--cpu` composer. The two toolkit windows say `still_drawing` after `WARN` for their draw.
