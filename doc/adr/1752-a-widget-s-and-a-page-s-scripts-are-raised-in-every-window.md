# 1752 — A widget's and a page's scripts are raised in every window, a placed control's press included

Status: accepted and **built**. Session 1458. The windows' half of ADR 1750; builds on ADR 1602
section 4 (scripts beside the non-script actions), ADR 1616 (the `Scripts` level is the runner's)
and ADR 1702 (a site the view state exposes, a host drives).
Code: `crates/viewer-core/src/viewer.rs` (`raise`, `page_events`, `page_scripts`),
`crates/viewer-core/src/interact.rs` (`trigger`, `page_trigger`, `activate`, `handed_over`),
`crates/viewer-gtk/src/host.rs` (`presses`, `in_widget`, `pointer_through_a_control`,
`in_a_control`, `page_takes_the_keyboard`), `crates/viewer-qt/cpp/window.cpp`
(`pointerThroughControl`, the filter installed in `rebuildControls`), `tools/drive-windows.sh`
(`drive-triggers.pdf`, `script_triggers`, `--step`).
Tests: `crates/viewer-core/tests/script_triggers.rs`; drive steps 68, 69 and 70.

## 1. The premise, and what it hid

Every window already sent the pointer, and `viewer-core` already raised Table 197's six pointer and
focus events and Table 198's page pair, performing their non-script actions through
`interact::trigger` — so the requests had one caller to gain, the core, and the timer site's shape
(ADR 1702: the core calls the view state at its own event) was the one used, before slot 2's record
named it; the record then confirmed it. **What the premise did not show is that two of the three
windows never told the core of a press on a field.** GTK's own gesture inside a `GtkEntry` claims
the press before the page's drag sees it, and a `QLineEdit` accepts it; the drive's first run raised
`/E` and `/X` in GTK (motion bubbles) and no `/D`, `/U`, `/Fo` or `/Bl` there, and nothing in Qt.

## 2. Decision

- **The core runs each event's scripts where it raises the event.** `raise` hands `/E` `/X` `/D`
  `/U` `/Fo` `/Bl` to `run_annotation_scripts`, before the chain's other actions (ADR 1602 keeps no
  order between the two); a link's click hands the link's `/U` — its `/A`, by Table 197's
  precedence — from `interact::activate`; `page_events` hands a turn's leaving page to
  `run_page_scripts(Close)` after its non-script close and the arriving page to `Open` last. An
  open's own page is not run there: the open sequence runs it at the first present (RFC 0008
  section 6.5), so nothing reaches the launch path.
- **A chain whose scripts were handed over is performed without its `JavaScript` refusals**
  (`handed_over`): a refusal said beside a script that ran tells the reader the opposite of what
  happened. The test is the request's own count for the six events and a runner being supplied for
  the four page events, which slot 2's `run_page_scripts` always hands over; at `off` there is no
  runner and the refusal is the one sentence saying the script was not run (trap 5).
- **A press on a placed control is told to the core as the page's own and left to the control.**
  GTK reads the primary button's raw press and release in the capture phase
  (`EventControllerLegacy`, which takes part in no gesture's claim) and forwards those whose point
  picks a placed control or a widget it is built of — not "inside the `GtkFixed`", which holds the
  page's picture too, as the first fix found; the page's drag skips a press a control took. Qt
  installs an event filter on every control and its children and forwards a press, a release, a
  move with no button down and an entry. Neither consumes the event, and a drag inside a line edit
  is not forwarded, since it selects the control's text and not the page's.
- **A press on the page takes the keyboard out of a field's control in GTK**, as it does in
  `quorra` and Qt: the core has raised `/Bl` for the press, and a control keeping the keyboard
  would go on taking keys for a field the document was told lost it.

## 3. Consequences

- Drive steps 68 (`/E` `/D` `/U` `/X`), 69 (`/Fo` `/Bl`) and 70 (a turn forward and back, `C1 O2
  C2 O1`) work in `quorra`, GTK and Qt, each script logging and the field's adding its name to a
  read-only field the saved file holds as `E D Fo U X Bl`, drawn on each photograph;
  `quorra-confined`, pinned to `off`, runs none.
- A script that ran marks the page stale, as a timer's does: a `/E` script re-interprets the page
  under the cursor. The view state answers a count, not whether a value moved, so this is the cost.
- Table 197's `/PV` and `/PI` follow the page shown (ADR 1750); a continuous layout's neighbours
  raise neither, in every window.
- `viewer-ffi`'s session supplies no runner, so a C caller's pointer raises the events and runs no
  script, as before.
