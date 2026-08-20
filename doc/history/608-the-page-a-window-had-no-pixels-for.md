# 608 — The page a window had no pixels for

The third round of the block on the owner's decision that **the UI is now work**, and the one that
closes the gap `crate::stale`'s own module doc has named since it was written: a view change that
reaches area the last frame does not cover now has something true to show there.

Date: 2026-08-20.
ADR: [0443](../adr/0443-the-page-a-window-had-no-pixels-for.md).

Touched: `crates/viewer-ui/src/bin/pdf-viewer.rs` and
`bin/pdf-viewer/{arguments.rs, app.rs, renderer.rs, stale.rs, surface.rs, timing.rs}`;
`doc/conformance/ledger.toml` (§14.8.2.3), `doc/todo/37`, the ADR and this file.

The spec-driven half is in the same commit and is its own thing: **§14.8.2.3's soft hyphens**, read
against the code with `doc/todo/01`'s sweeps — see below.

## What was built

A retained low-resolution picture of the whole page, under the base. `crate::stale` grows
`Proxies`, `Retained`, `proxy_target`, `Source` and `Stand`; `crate::renderer`'s thread draws one
page per idle turn and sends it back on the channel it already had; the presenter's layer list is
built rather than fixed at three. `Plan::Reproject` became `Plan::Approximate(Stand)` because rule 3
needed to say *which* layer filled the picture.

The extent is `--proxy-pages N`, the host's flag beside `--cpu` and `--backend`, and it is there
because rule 2 leaves nowhere else: a stand-in is a deliberately wrong picture, so a count that
became a `Command` or a boundary field would make one visible to the tree that judges pictures.

Both constants carry the measurement that chose them, in their own doc comments, with the tables.
The finding worth keeping is that **the scale is free in time** — what a page costs is walking its
display list, not covering its pixels, flat over a sixty-fourfold range of raster sizes — so the
edge is bounded by memory and by quorra's glyph atlas rather than by the clock. What the *extent*
costs is bytes alone, because a page's picture is drawn once per page shown whatever the extent.

## What running it said, and what it corrected

Four claims of `doc/todo/37` were corrected by driving the thing under `Xvfb` with
`--proxy-pages 0` beside every run as the control, and the corrections are in that file and in the
ADR. The two that matter most:

**"A page turn has no base at all" is true only under `SinglePage`.** Both specification documents
open in `OneColumn`, where the incoming arrangement shares a page with the outgoing one — so the
base carries, and what this layer adds is the *other* page of the pair, which used to vanish for the
length of the render.

**The `SinglePage` page turn is still not answered, and the reason is the identity rather than the
mechanism.** A retained picture is keyed by the `Arc<DisplayList>` it was drawn from, and
`viewer_core::open` keeps "one entry rather than a cache of them" — so returning to a page produces
a new `Arc` with no picture held. What that needs is a *decision* — whether a stand-in may be a
picture of a superseded interpretation of the same page — and it deserves the argument its siblings
got rather than a key changed in passing.

**And one finding is about the base rather than this layer.** ADR 0442's `Refusal::Rearranged`
compares each page's composition exactly. On a real column it fires on *scrolls*, where every page
genuinely moves by the same distance: one trace line prints two placements identical to three
decimals and refuses them, because the composition goes through an inverse in `f32`. This round's
layer hides the symptom and leaves the cause; `doc/todo/37` carries it with the evidence and with
the derivation a bound would rest on — a placement wrong by less than half a device pixel moves no
pixel.

## The other two hosts

Level, and structurally rather than by deferral. `viewer-gtk` and `viewer-qt` are tier-1: the core
hands them a whole-page raster and the toolkit scales it, so their base *is* a picture of the page
and the gap this round closes does not exist there in that form. What they do not have is the
reprojection at all, which is `doc/todo/37`'s standing item about the processor's window.

## The spec-driven half — §14.8.2.3, soft hyphens

Three of `doc/todo/01`'s sweeps were run and the `owed` one's reading list taken: `partial` rows
whose every stated debt this tree already names. §14.8.2.3 is one of them, and reading it against
the code found the row citing only the independent-extractor *gate* while a unit test written
against the clause's own sentence exists and was not named — `text_render_modes.rs::
actual_text_replaces_what_a_sequence_reads_back`, whose fixture is `issue13226.pdf`'s shape, a space
glyph whose `/ActualText` is U+00AD. The row now cites it, names `content/marked.rs` beside
`content.rs` as the code that reads the entry, and says out loud the thing a later round would
otherwise get wrong: the tree's own *harness* folds U+00AD (`text_extraction.rs::without_hyphens`,
witness `bug1997343.pdf`) as a comparison rule, and that is not the product doing the repurposing
the clause's distinction exists for. The status stays `partial` because that half is genuinely
owed and belongs to a consumer.

## What is left

`doc/todo/37`'s two standing items and one new one: the processor's window, which still has no
stand-in of any kind; the identity that would let a `SinglePage` page turn use a retained page; and
the base's exact comparison, which refuses scrolls in a column for a difference no pixel can see.
