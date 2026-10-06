# 1565 — The adapter is told which window has the keyboard

Session 1365. Status: **accepted**. Amends nothing; closes `doc/todo/31`'s window-focus item.
Context: ADRs 0214, 0623; `crates/viewer-accessibility/src/bridge.rs`; `accesskit_unix` 0.22.1
and `accesskit_atspi_common` 0.19.1 as this tree binds them.

## The question

`accesskit_unix::Adapter::update_window_focus_state` existed and nothing called it, in any of the
three windows. What that costs is read off the adapter, not guessed: `accesskit_atspi_common`'s
`NodeWrapper::state` gives the root `Window` node AT-SPI's `ACTIVE` only while the host is focused,
`Tree::focus_id` answers `None` while it is not (so no node is ever `FOCUSED`), and `window:activate`
is emitted only from `focus_moved`, which fires only when that answer changes. With the call
missing, every frame this program published read as a window nobody is in, and no activation event
was ever sent. A screen reader that follows the active window between applications, which is how
Orca and its peers decide what to speak, never reaches the page.

## Decision

`Bridge::focused(active)` passes the one fact on. Each window calls it when the bridge comes up,
with what its toolkit already knows, and on every change the toolkit reports:

- `quorra`: `winit::window::Window::has_focus` at bring-up and `WindowEvent::Focused`.
- `quorra-gtk`: `GtkWindow:is-active`, through an idle because GTK raises the notification inside
  calls that can hold the host borrowed.
- `quorra-qt`: `QEvent::ActivationChange` with `isActiveWindow`, sent across the bridge as
  `window_activated` and kept on the Rust side until the adapter exists, as `window_placed` is. A
  change that arrives while another call into the host runs is sent once that call has returned,
  with the state at that moment.

The bridge is still created after the first frame (ADR 0214). A window activated before that is
told at bring-up, and the adapter keeps the value until a client attaches. `quorra-confined`
publishes no accessibility tree, so it has nothing to tell.

## What it was driven against

`tools/drive-windows.sh` step `32-window-active` reads the state set of the frame above the
`DocumentFrame` over AT-SPI. It reads it once with the keyboard given to the window and once with
the keyboard given to the root window, on two documents. Before the change all three windows
answered `inactive` both times. After it all three answer `active`, then `inactive`. Two toolkit
behaviours shaped the step:

- With the keyboard on the root, X delivers keys to the window under the pointer, and GTK counts
  that as having the keyboard (it went inactive and active again in the same millisecond). So the
  pointer is moved off the window first.
- Without a window manager GTK still tracks focus from `FocusIn` and `FocusOut`. The step therefore
  needs no window manager, and this machine has no X11 one to run.
