# 627 — The thread the other window did not have

`doc/todo/37`'s two open questions, which 622 named and left as "neither a defect": a render thread
for the processor's window, and a placement per page in the presenter. One is built and one is
priced and declined.

Date: 2026-08-20.
ADR: [0461](../adr/0461-the-thread-the-other-window-did-not-have.md).

Touched: `crates/viewer-ui/src/bin/pdf-viewer/composer.rs` (new), `{stale,surface,timing}.rs` and
`pdf-viewer.rs`; `crates/pdf-model/src/appearance.rs` and `tests/annotations.rs`;
`doc/conformance/ledger.toml` (§12.5.6.23), `doc/todo/37`, `doc/todo/README.md`, the ADR and this
file.

## Item 1 — a composing thread for the window with no device

`crate::composer` is `crate::renderer`'s arrangement over `render-cpu`: the event thread adopts,
asks and places on the clock's tick, and `compose_pages` walks the arrangement beside it. The
processor's window gains the retained low-resolution pages — drawn one per idle turn, exactly as the
other window's are — and `--proxy-pages` reaches both.

**The two surfaces stop having two policies.** `MustFollow::drawn_in_the_same_frame` and
`Stages::stood_in` are deleted, because a tick no longer presents twice; `Standing` stopped naming
an arrangement (`Beside`/`InFrontOf`) and names a price (`Quads`/`Resample`), because the
arrangement is now the same and the price is four orders of magnitude apart.

**Rule 4 stayed a question and this round expected it not to.** A resample is no longer *added* to
the frame — the render is on another thread — but it is still tens of milliseconds of the
**presenting** thread, so standing in costs the real frame `max(0, resample − frame)` rather than
nothing. The inequality did not move; its derivation did. It fires on real runs.

## What running it said

**The machine was at a load average of 47 to 76 on 24 cores for the whole session** — three parallel
rounds' gates — so every absolute figure below is that machine under that load and only the
comparisons are levels. Both binaries were built from this worktree and run back to back.

`doc/PDF20_AN001-BPC.pdf`, `OneColumn`, 800×1000, `--cpu`, twenty `Down` presses 60 ms apart:

| | before | after |
|---|---|---|
| the event thread, per tick (median / p90 / max) | 58.3 / 104.1 / 107.6 ms | 0.3 / 38.3 / 143.3 ms |
| the event thread over the gesture | 1195.9 ms | 790.6 ms |
| presents / median interval | 21 / 70.5 ms | 19 / 71.4 ms |

Same document, `SinglePage`, `Right Right Left`: `another page — nothing about the outgoing page's
pixels is true of the incoming one` three times before and **none** after; six view changes with
nothing true to move before and **none** after; one stand-in before and eight after, seven of them
from a retained page.

The device path, driven through the same two gestures with both binaries, is unchanged in kind: 22
against 24 presents on the page turns, 13 against 18 on the scroll, no refusal on either.

**Two defects came out of running it and both are in the ADR.** A rendering the window held and
never showed — three finished frames discarded inside one scroll, because a refused stand-in
presented nothing at all — and rule 4 locking itself shut on a single sample taken while the machine
was busy, which is ADR 0384's own defect a layer along. A refusal now gives up the sample it refused
on.

**What could and could not be photographed.** 622 could not catch the processor's stand-in at all.
This round did: `--cpu`, `SinglePage`, three zoom steps and then `Right` on
`doc/ISO_32000-2_sponsored_EC3.pdf`, photographed every 80 ms — one frame is page 2 unmistakably
blurred with the sidebar sharp beside it, which is the retained page under fresh chrome, and the
Laplacian standard deviation over the page area separates it from the true frame by a factor of five
(0.0223 against 0.115). What could **not** be captured is a stand-in on a *light* page: it is up for
less than one capture, which is the feature working.

**Launch**, ten runs of the 1023-page specification on each binary, at a load average of 59 to 71:

| | best | median | worst |
|---|---|---|---|
| before | 802.4 ms | 930.1 ms | 1152.2 ms |
| after | 631.0 ms | 817.7 ms | 1217.0 ms |

**Five to eight times the band 607, 608 and 622 measured**, and the load is the whole of the reason
— llvmpipe under `Xvfb` on a machine running three other rounds' gates, where the launch path is
mostly one thread. The *best of ten* is the least-contended sample either binary got and the after
binary holds it, so what the pair says is that this change did not move the launch; what it cannot
say is where the level is. **A run on a quiet machine is owed and this round could not get one** — a
waiter polling for a load average under six never fired in the hour it ran.

## The spec-driven half — §12.5.6.23

622's lead, taken. `appearance::construct`'s catch-all told a person "its clause states no geometry"
about a clause that states the region twice — Table 195's `/QuadPoints`, referred to Table 182's own
words, and `/Rect` where that is absent. The ledger row had the reading right and the sentence a reader
gets had it backwards, which is the caret's defect of the session before, in the same arm. `Redact`
has its own arm and a test asserting the sentence; the behaviour is unchanged.

## The hosts

Level. `viewer-gtk` and `viewer-qt` are tier 1: the core hands them a whole-page raster per page and
the toolkit scales it, so there is no reprojection there to keep level with — checked again rather
than assumed, and still the answer 608, 609 and 622 gave. Nothing this round touched crosses the
boundary: `composer.rs` is a module of `pdf-viewer`'s binary, and `stale.rs`'s rule-2 test still
walks every `.rs` outside `viewer-ui/src/bin`.
