# 1972 — The event match is the trait's, and a toolkit's answers are its required methods

Session 1568, slot 5 of batch ninety, a host round of the project owner's switch of 2026-10-14 to
"code improvements/refactoring". Status: **accepted** and **built** in `viewer-host`, `viewer-gtk`
and `viewer-qt`. Context: ADR 1908 §3 (what the two hosts spell alike), ADR 1920 (one trait over one
state), habit 103 (an arm of a host's event match is one line). Supersedes ADR 1920 §3's second and
third bullets. Code: `viewer_host::native_window::NativeWindow::react` and the methods it calls. It
cites no clause and moves no ledger row.

## 1. The decision

**`NativeWindow::react` is the one event match of the two toolkit windows**, a provided method,
exhaustive over `viewer_core::Event`, every arm one line handing the event to one method. Where both
hosts answered an event alike the method is provided (`turned`, `restate`, `needs_render`,
`fetch_policies`, `dirtied`, `refused`, `warned`, `copied`, `console`, `media`, `convert`, and the
methods ADR 1920 had already shared). Where a toolkit answers its own way — a dialogue, a clipboard,
a panel, a print — the method is a **required** one: `retitle`, `opened`, `password_required`,
`follow_the_link`, `submit`, `needs_file`, `fetch_policy`, `extracted`, `print`, `to_clipboard`,
`look_for_answers`, `ask_whether_to_proceed`, `ask_whether_to_run_scripts`, `ask_for_a_script`,
`withdraw_script_question`, `beep`, `attachments_changed`. Each host's `impl NativeWindow` forwards
each to its own method of that name, ADR 1920's pattern, or states the toolkit's one or two lines
itself (Qt's flags, GTK's clipboard).

ADR 1920 §3 refused the third of its three ways to split the match because it "turns a four-method
trait into the hosts' whole event interface". That is what the required methods now are, and it is
the reason to have them: an event added to the vocabulary fails to compile once, in the trait, and
then in both hosts exactly where its answer is a toolkit's. The exhaustiveness ADR 1920 kept by
keeping two matches is kept by one. The clause each arm answers is cited once, at the method, where
it was cited twice before (Qt's `Refused` arm carried the same sentence twice).

**The near-identical methods move too**, each with the toolkit's difference made a required hook:
`take_the_drawn`, `answered` and `take_the_windows` (`show_the_drawing`, `fit_the_window`);
`send_form` and `start_policy_fetch` (`look_for_answers`); `obey_the_catalog` (`asked_to_centre`,
`show_the_page_mode`); `take_the_opening` and `mind_the_opening` (`drain`, `open_document`,
`prepare`, `show_the_opening`, `take_down_the_bar`); and the password's supply as
`password_supplied`, which Qt's bridge entry `supply_password` calls with the document its one
prompt is about. Four fields go into `WindowState` because a moved method reads them: `presented`,
`presenting`, `anticipated` (given by `WindowState::opening`) and `awaiting`.

## 2. What moved, and the one thing that changed

Every moved body is the hosts' HEAD code token for token once comments are set aside and the trait's
`self.state()`/`self.state_mut()` is read as the hosts' `self.state`, but for the hooks above, `pump`
or `drain` as `drain`, GTK's `refresh` as `repaint`, and borrows split where a `&mut` state is read
across a call (a script the round left in its scratch). Qt's `ask` matched two variants and had a wildcard;
it is now the two required methods, and the wildcard is gone. Qt's three `NeedsFile` arms are its
`needs_file`, in the same order.

**One behaviour is GTK's where it was Qt's**: `take_the_windows` asks for a repaint once, and only
when a window landed with a picture. Qt flagged its frame for every window taken, an abandoned one
included; an abandoned window changes no picture (`Oversized::abandoned` only marks the window
declined, to be asked again), so the flag cost a frame's copy for an unchanged frame. The drive is the witness that
nothing a person sees moved.

`viewer-host`'s sweep `every_host_that_arms_a_report_asks_for_it` read each host's source for
`report_due.after_a_frame()`; that call is now the trait's, so the sweep counts every crate that
implements `NativeWindow` as asking where `viewer-host` does. Planted out of the trait, the sweep
names both toolkit crates.

## 3. What stays each host's

- **`run`**, for ADR 1920 §3's reason (`viewer_accessibility::republishes`).
- **`open_document`**, a required method now: its body names the host's `restrictions`,
  `widget_appearances` and `separations`, and a sibling round of this batch was writing into it.
- **`join_the_opening`, `show_the_opening`, `prepare`, `opened`, `password_required`**: each arms a
  timer, a dialogue or a tab swap in the toolkit's own way, which is a difference of behaviour.

**Gates.** `doc/history/1568` gives the drive before and after, the counts and the tests.
