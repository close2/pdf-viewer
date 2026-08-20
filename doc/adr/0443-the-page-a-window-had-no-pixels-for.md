# 0443 — The page a window had no pixels for

Status: accepted.
Session: the six-hundred-and-eighth.
Subject: `doc/todo/37`'s second surface — a retained low-resolution picture of the whole page, so
that a view change reaching area the last frame does not cover has something true to show; where
its extent is configured and why it could be nowhere else; the two numbers chosen from
measurement; and the three things looking at the screen said that no test would have.

## What was owed

`crate::stale`'s own module doc has named this gap since it was written: *"a scroll reveals an edge
the old raster has no pixels for, and anything the new view would draw that the old one did not is
simply absent."* The base is a picture of the **window**, so what it does not cover it has nothing
to say about, and the window put its own background there — never page white, which would assert
that the page is blank (ADR 0378).

The project owner asked for the second layer on 2026-08-20:

> Another thing we should consider: keep a version of the page (possibly low resolution) so that we
> can reproject onto it, when zooming out, this would allow us to display something onto the newly
> appearing area.

and, on the one question `doc/todo/37` left open:

> regarding the question if we should have only one page or a small window of neighbouring pages.
> Make it configurable.

Sessions 606 and 607 made it matter much more rather than less: all three hosts do Table 29's
continuous and two-page arrangements now, so a view reveals new area constantly.

## The construction, and why it is complementary rather than a filler

A `Proxies` entry is a **whole page** drawn once into a raster whose longer side is `PROXY_EDGE`
pixels, produced on the render thread when it has nothing else to do, placed under the base by the
same `settled⁻¹ ∘ asked` composition the base uses. The presenter's layer list becomes: the medium,
the retained pages, the base, the chrome.

**The error is smallest exactly where it is needed**, which is the argument to keep. A whole-page
picture is near the right resolution when the view is zoomed out — which is when the margin appears
— and badly blurred when the view is zoomed in, which is when the sharp base already covers it. The
two are complementary rather than competing, and **the base's own opacity is what composes them**:
`render-quorra` draws the window's medium under the page into that texture, so wherever the base
covers the window it wins outright and neither layer has to know about the other.

**One placement per page rather than one for the picture**, and that is the difference that earns
the layer its keep beyond the margin. The base is one texture of the window, so it needs a single
affine true of every page in it and refuses outright where there is none (`Refusal::Rearranged`,
ADR 0442). The retained pages are one texture *apiece*, so a zoom in a column, a resize and a page
turn — every case the base calls impossible — place correctly, one composition each.

## Where the extent lives, and why it could be nowhere else

**Rule 2 decided it.** A stand-in is a deliberately wrong picture, so everything that makes one is a
private module of a binary and nothing that judges a picture can link to it. A neighbour count that
became a `Command`, a field of a boundary type or anything below `crates/viewer-ui/src/bin` would
make a wrong picture visible to the tree that judges pictures, which is the one thing rule 2 exists
to prevent. So it is the **host's** setting — `--proxy-pages N`, beside `--cpu`, `--backend` and
`--no-sandbox` — and `--proxy-pages 0` turns the whole of it off.

## The two numbers, and the measurement that chose them

Taken with `cargo run --release -p render-quorra --example zoom_frame` on this machine's Radeon
890M — a sequence of magnifications drawn against one warm device, which is exactly a proxy drawn
after a real frame of the same page.

**The scale is free in time, which is the opposite of what `doc/todo/37` assumed.** It illustrated
the idea with "an eighth of device scale"; a sixty-four-fold range of pixels moves the frame by
nothing:

| page | 91 × 128 | 362 × 512 | 724 × 1024 |
|---|---|---|---|
| `doc/ISO_32000-2_sponsored_EC3.pdf` p60 | 21.8 ms | 24.6 ms | 24.6 ms |
| `doc/PDF20_AN001-BPC.pdf` p1, 160 commands | 4.2 ms | 1.9 ms | 3.1 ms |

and on the owner's own witness, 58 029 commands, 406.4 ms at 254 × 72 against 408.2 ms at
2027 × 576. **What a frame of a page costs is walking its display list, not covering its pixels.**
So the binding constraint is memory: `PROXY_EDGE = 512` gives 362 × 512 × 4 = 741 376 bytes for A4,
against 8 192 000 for *one* of the two window textures a single frame already holds, and half the
linear resolution of a page filling an 800 × 1000 window.

**A fixed edge rather than a fraction of the magnification**, for a second thing the ladder showed:
a run that drew the same page at seven scales made quorra throw its whole glyph atlas away, and the
real frame after that repack cost 411 ms where the one after a single proxy cost 82. A scale that
followed the zoom would put a new glyph size in the atlas at every step; one fixed edge puts in one,
once, per page.

**The extent's cost is memory alone, and that is the separation the measurement makes.** Drawing a
page's picture is paid once per page *shown*, whatever the extent — and the frame after it pays a
rebuilt scene rather than a replay:

| page | the proxy | the frame after it | that frame as a replay |
|---|---|---|---|
| `doc/ISO_32000-2_sponsored_EC3.pdf` p60 | 21.9 ms | 2.1 ms | 1.1 ms |
| the owner's witness, 58 029 commands | 411.0 ms | 82.0 ms | 1.9 ms |

A larger extent is therefore strictly cheaper in renders — a page evicted and revisited is drawn
again — and strictly dearer in bytes. `PROXY_PAGES = 8` is where the whole retained set, 5.9 MB for
A4, is still smaller than one of the two window textures a frame holds anyway: a budget stated
against something this program already spends rather than a round number.

**The price to know about is neither of those two**: a view change arriving while a page's picture
is being drawn waits for it to finish, up to 411 ms on that witness. The window still answers at
once — the stand-in is three quads on the presenting thread — and what is late is the real frame.
One page per idle turn is what bounds it.

## Nothing on the launch path

The render thread does not exist until the first job (ADR 0391), it draws that job first, and it
looks for a page with no picture only when the job channel is empty. There is no timer, no
pre-render and no thread of its own. `viewer-ui`'s launch measurement is unmoved; the round that
takes the next one has the same three milestones to compare against.

## Rule 3 grew a distinction, because there are two amounts of wrong now

"Approximated from the last frame" and "approximated from a low-resolution page" are not the same
claim, and a frame line saying only `approximated` for both had stopped saying what it was showing.
`Plan::Approximate` carries a `Source` — the last frame, the last frame over retained pages, or
retained pages alone — the frame line prints its word, the summary counts the three separately, and
a refusal of the *sharp* layer that the retained pages stood in for is counted apart from a refusal
that showed nothing (`Refusals::proxied`). The presenter cannot draw a stand-in without naming what
it is made of, because the name is in the value.

## Three things the screen said that no test would have

Driven under `Xvfb` per `doc/environment.md`, `--trace=frames`, with `--proxy-pages 0` beside every
run as the control.

**The first corrects this file's predecessor.** `doc/todo/37` said a page turn "has no base at all",
and that is true only under Table 29's `SinglePage`. Both spec documents open in `OneColumn`, where
the incoming arrangement shares a page with the outgoing one, so the base carries and what the
retained layer adds is *the other page of the pair* — which before this round simply vanished for
the length of the render. The trace says so in the source word: `approximated, over a retained
page`, eight of eight stand-ins in one run.

**The second is a limit of the identity, and it is the honest reason the `SinglePage` page turn is
still not answered.** A retained picture is keyed by the `Arc<DisplayList>` it was drawn from — the
ABA-safe identity of ADR 0351 — and `viewer_core::open` keeps "one entry rather than a cache of
them", bounded by the arrangement. So a page turn in `SinglePage` drops the outgoing page's
interpretation, and returning to it produces a *new* `Arc` with no picture held. Driven three times
over with four-second gaps, every such turn printed `another page — nothing to show`. What would
answer it is an identity that survives re-interpretation, which means deciding whether a stand-in
may be a picture of a *superseded* interpretation of the same page — a different kind of wrong from
blur, and one that deserves its own argument rather than being taken in passing. It is written down
in `doc/todo/37` as what is owed next.

**The third is about the base and not about this layer at all.** ADR 0442's `Refusal::Rearranged`
compares the per-page compositions exactly, on the argument that a threshold here would be a number
nobody measured a purpose for. On a real column it fires on *scrolls*, where every page genuinely
moves by the same distance: one trace line reads

> the first page of the picture held moves by (1.000 0.000 0.000 1.000 0.000 -371.000) and a later
> one by (1.000 0.000 0.000 1.000 0.000 -371.000)

— two placements that print identically and are not equal, because the composition goes through an
inverse in `f32`. The base is therefore refused for most view changes in a continuous layout, and
this round's layer hides the symptom while leaving the cause. **Not fixed here, deliberately**: the
fix is a bound on how far a placement may be wrong before a pixel moves, which is a threshold with a
derivation and needs its own measurement, and taking it in the same round as a new layer would put
two arguments in one commit. `doc/todo/37` carries it with the evidence.

## What the other two hosts owe, precisely

Nothing, and the reason is structural rather than a deferral. `viewer-gtk` and `viewer-qt` are
**tier-1** hosts: `viewer-core` hands them a whole-page raster and the toolkit scales it, so their
base *is* a picture of the page and the gap this round closes does not exist there in that form.
What they do not have is the reprojection at all, which is `doc/todo/37`'s standing item about the
processor's window and not this one. The owner's rule that all three hosts stay level is about a
capability a host lacks, and this is a capability the shape of their pipeline already supplies.

## What this cost in code

`crate::stale` gains `Proxies`, `Retained`, `proxy_target`, `Source` and `Stand`; `Plan::Reproject`
becomes `Plan::Approximate(Stand)`; `crate::renderer` gains the idle-turn producer, a `Finished`
enum on the channel it already had, and a layer list that is built rather than fixed at three.
`Stages::approximated` is an `Option<Source>` rather than a `bool`. Everything that makes an
approximate picture is still inside `crates/viewer-ui/src/bin`, which the test at the foot of
`stale.rs` walks the tree to say.
