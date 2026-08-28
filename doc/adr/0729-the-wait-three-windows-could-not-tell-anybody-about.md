# ADR 0729 — The wait three windows could not tell anybody about

Status: accepted, 2026-08-28. Session 795. Takes `doc/todo/15`'s longest-standing remainder: the
owner's *"warn the user and allow the user to abort, however don't block"* reaching the three
established windows through `viewer_host::keys`. Cites no clause of its own — this is
`CLAUDE.md` principle 3's bound reaching a person — and the ledger is untouched; the one clause it
leans on, §7.7.2's Table 29, is quoted where it takes a binding away.

## What was owed, and what had already been decided

Everything except the person. `pdf_render::Interrupt` is the mechanism (ADR 0650); ADR 0657
decided *when a host raises it by itself* and measured the case against a deadline; ADR 0668 put
the rule in `viewer_host::drawing` so that two native windows share one arrangement. All of it is
about draws whose answer the viewer has **stopped wanting** — a page turn, a resize, a
re-interpretation — and every one of those is provable without asking anybody.

What none of it covers is a draw whose answer is still wanted and which somebody is watching not
happen. `doc/todo/15` has carried that as owed since ADR 0657, in the owner's own words, and ADR
0713 built it for the *fourth* window on the confined boundary while deliberately leaving the
three established ones: the input belongs in `viewer_host::keys`, which is where
`doc/todo/30`'s levelness rule makes a binding all three windows' at once.

## Decision 1: a clock is admissible for a warning, and was not for a deadline

`viewer_host::drawing::WARN` is one second. ADR 0657 refused a duration and this adds one, so the
difference carries the whole argument: **that decision was about abandoning work and this one is
about saying a sentence.**

The measurement was taken again rather than quoted, with `viewer-confined`'s `examples/host_draw`
at twice device scale over `doc/pdf.js`'s first pages:

| | |
|---|---|
| documents priced | 957 |
| median | 1.8 ms |
| p90 / p99 | 10.6 ms / 85.6 ms |
| slowest | 315.9 ms (`issue1905.pdf`) |
| over one 60 Hz period | 6.06% |
| over 500 ms | **0.00%** |
| the amplification fixture (1567 bytes, ADR 0650) | 27 600 ms |

The legitimate population runs continuously up to its own end, which is why no *deadline* can sit
between the two: one placed low enough to catch the fixture throws away pages somebody asked for,
and one placed above the tail catches nothing a document cannot step under. A **warning** has the
opposite failure cost — a line of chrome a person ignores — so a number that merely clears the
legitimate population is enough, and a second is three times the slowest of the 957 with nothing
at all within half of it.

Nothing raises an interrupt when it passes. `Drawing::overlong` answers a question and
`Composer::overlong` answers the same one about a frame; both are read on a poll the window
already runs while something is being drawn, so a window at rest wakes for none of it.

## Decision 2: Escape, and only while the window is saying so

`viewer_host::keys::Waiting` is the fourth argument to `meaning`, and it decides one row.

- **Escape rather than a key of its own.** The situation this exists for is somebody in front of a
  window that is not answering, which is the worst moment to need a key one has learned. Escape is
  what every program means *not that* by, `keys.rs` already argued exactly that when it took the
  key off `viewer-ui`'s exit, and the confined window has meant this by it since ADR 0713 — so
  four windows agree rather than three.
- **Only while the warning is up.** A binding that means two things is a guess unless the window
  has said which; `Waiting::Warned` is the state a host enters by *saying*
  `viewer_host::still_drawing`, which names the key, and leaves when the draw ends. With no
  warning up Escape clears the selection exactly as before, and a unit test walks the whole of
  `Key::ALL` to show that no other row moves.
- **A presentation still leaves full screen.** Table 29's `FullScreen` shows "no menu bar, window
  controls, or any other window visible", so the sentence offering the key is not on the screen at
  all while one is running, and a key doing what an unseen sentence offered is the guess above.
  Escape leaves full screen, the sentence appears, and the next Escape stops the draw.

## Decision 3: what an abort costs, which the sentence has to say

Nothing is reported for it. `viewer_core::Rendered::Failed` records a page as *answered* and stops
the scheduler (trap 20), so an abandoned draw stays unanswered: the page keeps whatever picture it
had, and it is asked for again the moment the view changes, because a changed view is a new
request. `viewer_host::stopped_drawing` says so in as many words, since a person told only that
something stopped would reasonably conclude the program had given up on the page.

**And the queue behind it is left alone.** Table 29's column asks for every page it shows; the
pages waiting behind the expensive one have cost nothing yet, and dropping them would leave each
with an outstanding request nothing will ever answer. A person who meant to stop those too has the
key again a moment later.

## Finding 1: `viewer-qt` had been swallowing Escape entirely

The key reaches that window through a `QAction` shortcut, because a shortcut consumes a key before
`keyPressEvent` sees it. The action forwarded Escape to the table **only** in full screen, closed
the find bar if one was open, and otherwise returned — so §12.4.2's "clear the selection", which
`viewer_host::keys` has stated for all three hosts since ADR 0526, never reached the table in this
host at all. It was found by pressing the key at a window that was showing this round's warning and
watching nothing happen.

The fix is to forward in every case the window does not legitimately own. What it does own is the
*ordering* of chrome over the page — `keys.rs`'s "What a host still owns" — so the find bar keeps
the key while it is visible and Table 29 permits one, and everything else goes to the table.

**The general shape is worth more than the instance**: a shared key table is only as level as the
narrowest path a key takes to reach it, and nothing in this tree looks at that path. Two of the
three hosts call `meaning` from one place; the third calls it from two, and the second of them had
a guard.

## Finding 2: on the flagship, an abort without a record is a loop

`viewer-ui`'s composing surface is not a tier-1 host in this respect: its outstanding request is
its **own**, asked once a tick for as long as the pixels on hand are not of this view. So the
first version of this change stopped a frame and the next tick asked for the same frame again,
warned about it a second later and stopped it again — photographed doing exactly that, three
times over, before `Composer::declined` existed.

The record is the arrangement the person stopped, compared exactly as `depicts` compares the one
on the screen (each page by its `Arc`, the targets by value, the count with them), and cleared by
any other arrangement being asked for. That is precisely the "when the view changes" the sentence
promises. A native window needs no such field, and the reason it does not is the sharper half of
the finding: **a viewer's token never answered is never re-issued, and a host's own ask is.**

## Proof, driven under Xvfb on the release programs

The fixture is `tests/support/amplification.rs`'s document at four levels — 1567 bytes, ten
thousand page-covering fills — written out to a file so that a window can open it.

- **`pdf-viewer-gtk`**: the status bar reads *page 1 is taking a long time to draw — press Escape
  to stop drawing it*, photographed; Escape gives back the drawing thread after 14.3 s of drawing
  and the bar reads *stopped drawing page 1 — it will be drawn again when the view changes*,
  photographed. `+` then re-asks the page at 637×824, the warning returns a second later, and a
  second Escape stops that one after 3.0 s. An ordinary five-page document (`PDF20_AN001-BPC.pdf`)
  warns about nothing at all.
- **`pdf-viewer --cpu`**: the sentence is in the title bar, which is where this window already
  puts what the pages on the screen could not draw — it has no status band and never had one — and
  on standard output. The view is named rather than a page, because this thread draws Table 29's
  whole arrangement at once. Escape stops it, the title comes back clean, and nothing asks again
  until `+`.
- **`pdf-viewer-qt`**: both sentences photographed in the `QStatusBar`, after Finding 1's fix.

The window answered a key press throughout in all three, which is the "don't block" half and is
what the drawing thread was built for.

## Trap-13 calibration

Every new test was run against an injected defect before being believed; all seven failed, and the
suite is green as committed.

| injected defect | failed |
|---|---|
| Escape's warned row deleted | `escape_stops_a_draw_only_while_the_window_is_saying_that_it_can` |
| the warning also moves `w` | `the_warning_changes_escape_and_no_other_key` |
| the wait is not read (`>= ZERO`) | `a_draw_is_warned_about_only_once_it_has_outlasted_the_wait` |
| a stopped draw is still warned about | `the_persons_abort_takes_the_thread_back_and_owes_nothing` |
| a finished draw is left in flight | `a_window_that_is_not_drawing_warns_about_nothing` |
| the abort empties the queue behind it | `an_abort_leaves_the_pages_queued_behind_it_to_draw` |
| the abort answers for a draw that does not exist | `an_abort_with_nothing_in_flight_stops_nothing` |

## What this does not close

The device path warns about nothing, and that is stated rather than guarded: quorra has no
interrupt, so a frame on the flagship's render thread cannot be taken back at all (ADR 0725) and
offering a key for it would be a sentence naming a key that does nothing. `--cpu` is the surface
with the interruptible thread. `doc/todo/15` keeps the rest — breach-as-refusal, moving the
established windows onto the confined boundary, and the real-adapter measurement ADR 0725 owes.
