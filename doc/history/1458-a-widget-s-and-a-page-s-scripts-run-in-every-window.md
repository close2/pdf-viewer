# 1458 — A widget's and a page's scripts run in every window, and a C caller binds a policy copy

Slot 3 of batch seventy-two, 2026-10-08, a host round. ADRs 1752, 1753; no row moved, no question.

**Premise.** Held for the sites: no host code called `run_annotation_scripts` or `run_page_scripts`.
Built against the timer site's shape (ADR 1702) before slot 2's record existed; the record then
named the same two methods, `/PV` and `/PI` riding with the page's. Hidden by it: every window
sent the pointer and the core already raised all ten Table 197 events, but GTK's `GtkEntry` claims
a press and Qt's `QLineEdit` accepts one, so the first drive raised only `/E` and `/X` in GTK and
nothing in Qt.

**Built (ADR 1752).** The core hands `/E` `/X` `/D` `/U` `/Fo` `/Bl` to the runner in `raise`, a
link's click from `activate`, a turn's `Close` then `Open` from `page_events` (an open's own page
stays the open sequence's), and performs a chain whose scripts were handed over without its
`JavaScript` refusals (`handed_over`); at `off` the refusal stays. GTK reads a placed control's
press and release in the capture phase (`EventControllerLegacy`) and tells the core; a page press
takes the keyboard out of a field's control, not out of a note's editor. Qt filters every control
and its children. Drive steps 68–70 (`drive-triggers.pdf`, `--step` added); looked at (trap 1): the
read-only field shows `E D Fo U X Bl` in all three photographs.

**Built (ADR 1753).** `quorra_event_policy_count`, `_url`, `_bind`: the caller fetches, the library
binds and says the windows' own sentence (`viewer_host::policy::bound`).

**Found.** "Inside the `GtkFixed`" is not "on a control" — the page picture is its child; and the
whole drive's GTK steps 64 and 67 failed until a page press left a note's new editor the keyboard.

**Unfinished.** A rich note's caret stays at its end (ADR 1739): it needs `/Contents` offsets
aligned to `rich::draw`'s glyphs under §12.5.6.6's collapse and UAX #9's order. Slot 2's ADR 1751
hunk is in my `viewer-core/src/viewer.rs`.

**Gates.** rustfmt `--check` on my files: clean. `RUSTFLAGS="-D warnings" cargo clippy
--all-targets` on `viewer-core`, `viewer-host`, `viewer-ffi`, `viewer-gtk`, `viewer-qt`: exit 0.
`cargo nextest run` on those and `viewer-ui`, `viewer-confined`: 941 passed, 0 failed; the four
`script_triggers` tests fail with the core hunks reverted and the link test with `/D` planted.
`cargo test -p conformance`: 400 passed, 1 failed (`records`: siblings' 1459–1461 without gate
figures yet). Lock, `--tree 6`: steps 68–70 in four windows, 12 works (84 s). `--tree 12`: the
whole drive exit 0, 249 works, 2 wrong (GTK 64, 67), 0 to look at, 6 not offered (843 s); GTK
re-driven whole after the fix, exit 0, 76 works, 0 wrong (238 s) — combined 251 / 0 / 0 / 6, other
binaries' `md5sum` unchanged. `launch_path` not run: no open path changed and nothing runs at open
beyond the open sequence. Duration 6 350 s.
