# 3612 — `quorra`'s floating window is tested under winit's running loop

Session 2227. Status: accepted. Context: ADR 3587.

## Decision

`a_floating_window_opens_shows_its_clip_and_closes_with_it_under_a_running_loop` builds an X11
`EventLoop` on the test's thread (`with_any_thread`), hands `App` its proxy as the launch does, and
pumps it (`pump_app_events`): `resumed` makes the document window, the screen annotation is
activated, and the test reads between turns that the clip's window opened at Table 295's `/D`
size, is titled with the clip, holds the clip's own picture, and closes when the clip ends. A
display is required and its absence fails the test rather than skipping it, so CI runs
`viewer-ui` under `xvfb-run` as it runs `viewer-gtk`. winit admits one event loop per process,
so this is the only test of the binary that builds one.
