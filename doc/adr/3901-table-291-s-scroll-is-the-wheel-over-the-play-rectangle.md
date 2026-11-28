# 3901 — Table 291's scroll is the wheel over the play rectangle

Session 2300. Status: **accepted and built**. Code: `viewer-host`'s `video/scroll.rs`
(`placed_scrolled`, `Placed::overflow`, `Placed::in_samples`, `Placed::within`), `video.rs`
(`VideoPlayer::scrolls`, `VideoPlayer::scroll_over`), `video/window.rs` (`Pictures::scroll_over`);
each window's `scroll_the_picture` (`quorra`, GTK 4, Qt, the C ABI's `quorra_window_wheel`) and
the media windows of `quorra` and GTK; `pdf-model`'s `media.rs`, where `unscrolled` is gone.
Amends ADR 3553 decision 2. Context: ISO 32000-2 §13.2.5 Table 291, §13.2.2.

## Context

Table 291's `/F 3`: "The media shall not be scaled. A scrolling user interface shall be provided
if the media rectangle is wider or taller than the play rectangle." ADR 3553 found no window with
such an interface, so an `MH` `/F 3` made a clip with pictures non-viable and a `BE` one was set
aside, said, and drawn as hidden.

## Decision

1. **The interface is the wheel over the play rectangle**, a choice: the table states the
   requirement, not its form. Every window already reads the wheel, and a touchpad's two-finger
   travel arrives as the same event, as a distance in pixels. Over the rectangle of a picture that
   overflows it, that distance moves the part shown, one sample a pixel because the picture is
   unscaled, and it stops at the picture's edges. While the pointer is over such a picture, every
   wheel event belongs to the picture, even at an edge, so the wheel does not start scrolling the
   page halfway through a gesture. Rejected: a drag, which competes with a press on the
   page's annotations and on the controller bar (ADR 3610); and drawn scroll bars, which cover
   the picture they scroll, in a rectangle that may be a few dozen pixels across.
2. **No overflow, no interface.** The requirement's condition is the picture's size against the
   rectangle's, so a picture that fits takes no wheel, and no other fit does either.
3. **One geometry.** `placed_scrolled` is `placed_turned` with the part's first column and row
   moved by the samples scrolled and held inside the picture. Each window draws the `Placed` it
   returns exactly as it draws every other fit, so the device layer (`Layer::within`), the
   processor's overlay, GTK, Qt and the C ABI all scroll through the code that places the
   picture. A wheel distance enters through the inverse of the placement's linear part, so a
   picture turned by Table 306's `/Rotate` scrolls along its own rows. The position is kept
   fractional, so a touchpad's travel of less than a pixel per event is not lost.
4. **Where the pointer is.** `quorra` and GTK keep the pointer's last position, and Qt now does
   too (`Host::pointer_at`), because a scroll event in those toolkits does not carry one. The C
   ABI gains `quorra_window_wheel(dx, dy, x, y, *picture)`. `quorra_window_scroll` stays for scroll
   bars, which never scroll a picture. A media window of its own (Table 294's `/W` 0 or 1) is
   entirely play rectangle.
5. **The judgement no longer refuses `/F 3`.** The requirement is conditional, and every window
   that shows pictures meets it whatever the picture turns out to be: with an overflow, the
   interface is there; without one, nothing is required. Viability (§13.2.2) is therefore
   decided at judgement time, before decoding, and needs no knowledge of the picture's size. A
   host that shows no pictures hands the clip to this machine's player, which ADR 1960 judges as
   before. So `/F 3` in `MH` is honoured and in `BE` is not set aside, and `Honoured::fit_must`,
   which existed only for that refusal, is removed.

## Consequences

- §13.2.5's last owed sentence is met; the row and §13.2's aggregate take the state the tests
  give them.
- The WinUI host (`hosts/windows`) calls `quorra_window_wheel` for the wheel, its interop
  declaring it (`header_and_interop_agree`); no build of it was possible on this machine, which
  has no .NET toolchain, so its first build is that host's own check.
