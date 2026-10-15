# 1957 — A tick runs between frames, and a refused method is refused at its call

Session 1560, a script round of batch eighty-nine, `doc/todo/70` items 3 and 5. Status: **accepted,
built**; section 1 is `quorra`'s, and section 3 says why the toolkit windows need none of it today. Context: ADR 1950
section 5, ADR 1702 (a script's timer counted in the window's ticks), ADR 1926 (a frame that lands
wakes the window), ADR 1591 (a refused member is a property that throws). Code:
`crates/viewer-ui/src/bin/quorra/window.rs` (`App::script_timers`), `crates/pdf-script/src/surface.rs`
(`Member`), `crates/pdf-script/src/engine/bridge.rs` (`refuser`).

## 1. A due timer waits for the frame its last run asked for

**What was wrong.** A timer whose run costs more than its period — the framebuffer witness's 0 ms
interval rewriting 200 rows, whose tick held `quorra`'s window thread about 30 ms here — was ticked on
every turn of the loop. Each tick's run changed the page and asked for a render, and the render
before it, still on the render thread, was taken back: under Xvfb, 524 ticks and none of the changed
page's frames presented in 20 s, every frame "nothing to show".

**The rule.** `App::script_timers` sends no tick while a render of a page on the screen has not
reached the window — while a token of `App::requests` is in `App::unacknowledged`. The frame that
lands wakes the loop (ADR 1926) and drains the tokens, and the tick follows it. The ticker goes on
counting through the wait, so the tick carries the time the wait took, and ADR 1702's late timer runs
once rather than once per period missed. A tab behind the front draws nothing until it is brought
forward, so its renders hold no timer back. A window whose frames never land — minimised, its
swapchain lost — holds back only a timer whose run changes what it draws; the time is still counted.

**Measured**, `quorra --scripts on` under Xvfb, 20 s: at 0 ms, 53 ticks and 54 frames presented; at
250 ms, 80 ticks and 82 presented. The drive's two timer steps (`60-script-timer`,
`72-script-timer-quiet`) work.

## 2. A refused method is a function that throws when called

**What was wrong.** Every member `crate::surface` refuses was an accessor that threw on read, so
`typeof print` threw. Emscripten's runtime asks `typeof print` before it decides how to write, which
is how Doom's program stopped before any of it ran (ADR 1950 section 4); the reference has `print` be
a method, and a script that asks what a member is before it uses it is reading the surface, not
using it.

**The rule.** Each row of `EXCLUDED` and `REFUSED` states what its members are in Adobe's reference:
`Member::Method` or `Member::Property`. A method's getter answers a function of the member's name
whose call throws the row's `NotAllowedError`; a property's getter throws it, because reading it is
the use. A write throws for both. The eight property rows are the global objects (`Net`, `SOAP`,
`Collab`, `security`, `SecurityHandler`, `identity`, `XFA`, `xfa`), `app.fs`, the two `media`,
`this.dynamicXFAForm` and `this.documentFileName`; every other row is methods, and `color.convert`,
which no row lists, is one. A member waiting in `NOT_BRIDGED` states no kind and keeps throwing on
read. `tests/fixtures.rs` asks every member's `typeof` and calls it.

## 3. The toolkit windows do not starve, and why

`quorra-gtk` and `quorra-qt` tick through `viewer_host::script_timers::Ticker` too, and on the same
witness each tick was followed by its page: at 0 ms, 697 ticks and 698 renders handed back in 20 s
(GTK) and 426 and 427 (Qt); at 250 ms, 79 and 80, 84 and 80. Each draws the fields in controls of its
own (`WidgetAppearances::Delegated`), so the page it rasterises after a tick holds no field and takes
about 2 ms, and the raster is made on the window's thread before the loop turns again. Should either
come to draw a page off its thread, section 1's rule is the one to take with it.
