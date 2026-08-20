# 0461 — The thread the other window did not have, and the sample a refusal spends

Status: accepted.
Session: the six-hundred-and-twenty-seventh.
Subject: `doc/todo/37`'s two open questions. The processor's window gets a composing thread of its
own, which gives it the retained low-resolution pages, makes its stand-in *beside* the frame rather
than in front of it, and leaves the two surfaces with one policy and one difference. And a
placement per page in the presenter, which is priced against a run and **declined**.

Two things came out of running it that were not in the plan and are the more interesting half: a
rendering the window held and never showed, and rule 4 locking itself shut on one sample taken while
the machine was busy — ADR 0384's own defect, one layer along.

## Item 1 — a composing thread for the window with no device

### What was there

ADR 0457 gave `--cpu` a stand-in and had to reintroduce rule 4 for that surface alone, because the
premise ADR 0391 deleted it under was genuinely back: there was no second thread, so one call
resampled, presented, then drew the true frame and presented that. `doc/todo/37` recorded the
consequence rather than hiding it — *"the retained low-resolution pages are still device-only,
because they are drawn by a render thread that is idle and this window has none. So a page turn
under `--cpu` is still `Refusal::AnotherPage`, said out loud."*

### What is there now

`crates/viewer-ui/src/bin/pdf-viewer/composer.rs`, which is `renderer.rs`'s arrangement over
`render-cpu` instead of over a graphics device:

| | the event thread | the composing thread |
|---|---|---|
| holds | `SoftwareSurface`, the base, the retained pages | the display lists of the frame it is drawing |
| does | resamples, composites the chrome, copies to the window | `viewer_ui::software::compose_pages` |
| costs | a resample | a frame |

`App::present`'s processor branch is now the device branch's *adopt, ask, place*, and the two read
alike deliberately. The startup rule binds identically and is kept the same way: the thread does not
exist until the first job asks for one, so nothing about it is on the launch path.

**The chrome stays on the presenting thread**, which is not an oversight. It is drawn in window
pixels at the identity and does not move with the page, so composing it per present is what lets a
stand-in put *fresh* chrome over an old page — and it makes the raster the thread hands back the
window's own picture without the chrome, which is exactly what the other surface's base is.

### What it bought, measured

`doc/PDF20_AN001-BPC.pdf` in `OneColumn`, an 800×1000 window, `--cpu`, twenty `Down` presses 60 ms
apart, before and after, back to back. **The machine was at a load average of 65 to 76 on 24 cores
throughout** — three parallel rounds' gates — so the absolute figures are this machine's under that
load and only the comparison is a level:

| | before | after |
|---|---|---|
| ticks the event loop ran | 19 | 53 |
| **the event thread, per tick** — median / p90 / max | 58.3 / 104.1 / 107.6 ms | **0.3 / 38.3 / 143.3 ms** |
| the event thread over the whole gesture | 1195.9 ms | **790.6 ms** |
| presents | 21 | 19 |
| median interval between presents | 70.5 ms | 71.4 ms |
| what stood in | the last frame moved, ×2 | the last frame **over a retained page**, ×2 |

The median tick fell from 58.3 ms to 0.3 ms, which is the whole of what a render thread is for: the
loop is free to answer input while the rasterisation happens beside it. The present *rate* is
unchanged, and that is the honest reading — on this document a `--cpu` frame is 50–90 ms, so the
window was already presenting about as often as it could; what moved is where the time is spent.

And the case the item was about, `SinglePage` page turns under `--cpu`, on the same document:

| | before | after |
|---|---|---|
| `another page — nothing about the outgoing page's pixels is true of the incoming one` | 3 | **0** |
| view changes with *nothing true to move* | 6 | **0** |
| stand-ins drawn | 1, from the last frame | 8, of which 7 from a retained page |

### Rule 4 survives its own premise, and that is the finding

`doc/todo/37` expected this round to make rule 4 "an answer rather than a question" on that surface,
as ADR 0391 made it on the other. **It does not, and the argument is worth more than the
expectation.**

On the device a stand-in is three textured quads issued by the presenting thread while the device
draws on another: tenths of a millisecond, taken from nothing anybody was going to use. A resample
on the processor is a window of pixels walked by the processor, on the thread that presents — 28 to
98 ms in the runs above. It is not structurally zero, so there is still something to bound. What the
thread changed is *what* it bounds:

- the render begins at the tick that noticed the view change and finishes `frame` later, on the
  other thread, whatever this one does;
- the resample occupies the presenting thread for `resample`, and the true frame is presented at the
  first tick after **both** are done.

So standing in costs the real frame `max(0, resample − frame)` rather than `resample`. **The
inequality does not move** — `resample + period ≤ frame`, ADR 0384's form, no constant in it —
because *buys the refresh it spends* and *is up a refresh before the true frame lands* are the same
arithmetic read from opposite ends. What moves is the size of being wrong about it, and that is a
reason to keep a rule that costs one comparison rather than to delete it.

It fires on real runs, which is what settles it rather than the argument: `standing in would cost
91.1 ms on this thread against a frame of 100.1 ms being drawn on another`.

**What did go is the second policy.** `Standing::Beside`/`InFrontOf` described an *arrangement* and
both surfaces now have the same one; the type is `Standing::Quads`/`Standing::Resample` and
describes the price. `MustFollow::drawn_in_the_same_frame` is deleted — rule 1 is discharged of the
clock on both surfaces — and `Stages::stood_in` with it, because a tick no longer presents twice.

### Two defects running it found

**A rendering the window held and never showed.** With the thread, a frame lands at a tick where the
view has already moved on; the plan is then a view change rather than `Plan::Render`, and where the
stand-in was refused the tick presented *nothing* — so the window went on showing a picture older
than the one in its own hand, for as long as the refusal lasted. The first scroll measurement caught
it: 6 presents where the synchronous path managed 22 over the same gesture, with `composed 48.7`,
`composed 37.2` and `composed 43.3` on ticks that showed nothing. Three renderings finished and
thrown away inside one gesture.

A refusal keeps its meaning — *do not stand in* — and stops meaning *show nothing*: a rendering
nobody has seen is the truest picture this window has, at the placement it was drawn for, which is
what every viewer showed before `doc/todo/37` existed. `Composer::presented` is what notices, and it
is set by **both** ways of reaching the window, which is what keeps this from being a step
backwards: a stand-in *is* those pixels moved, so a tick that stood in has shown them and putting
them up unmoved afterwards would move the picture back to where the view no longer is.

The other surface does not need it and the reason is worth stating rather than assuming: there the
only *judged* refusal is `InsideTheRefresh`, which says the true frame is expected within one
refresh — waiting bounds itself. Rule 4 refuses precisely when the frame is slow.

**Rule 4 locked itself shut on one sample.** `Stale::resampling` held the last resample for the rest
of the run. A stand-in measured at 348.7 ms while the machine was busy then refused every view
change after it — against frames of 45 ms — and refusing is exactly what stops another sample being
taken. **That is ADR 0384's defect: a self-calibrating threshold whose own gate blocks its only
sample.** "Unmeasured permits" answered the *first* sample and nothing after it.

So a refusal gives up its own reason: `Stale::declined` clears the measurement a `TooDear` refused
on, the next view change permits and measures, and the decision after that is about the machine as
it is now. At worst every other view change draws a stand-in rule 4 would have refused, and what
that costs is what rule 4 bounds — `max(0, resample − frame)` on one view change — against a feature
that otherwise switches itself off for the length of a run.

## Item 2 — a placement per page in the presenter: priced and declined

`doc/todo/37` has carried this since 607 as what would **remove** `Refusal::Rearranged` rather than
bound it. Session 609 bounded it at half a device pixel (`stale::AGREEMENT`), derived from the
raster's own quantisation; the refusal remains correct for a zoom, because `viewer_core::layout`'s
inter-page gap is stated in logical pixels and does not scale while the pages do.

**What it would buy, measured.** `doc/PDF20_AN001-BPC.pdf` in `OneColumn`, six zoom steps:

| | `Rearranged` refusals | what was shown instead | view changes that showed nothing |
|---|---|---|---|
| the device | 7 | 7 stand-ins from a retained page, 8 over one | **0** |
| `--cpu` | 3 | 3 from a retained page, 3 over one | 0 (3 refused by rule 4) |

Every one of those seven was answered by the layer under it. So what a placement per page would buy
is **the sharp layer instead of a 512-pixel page** on the frames of a zoom in a multi-page
arrangement — not a picture instead of a blank window, which is what it would have bought before ADR
0443 built the retained pages and is presumably why the entry reads as it does.

**What it would cost, priced rather than guessed.**

- `render-quorra` draws every page of the arrangement into **one** window-sized texture
  (`PresentFrame` into `WindowTextures`). One placement per page needs one texture per page, which
  is either N renders per frame or an ask to quorra — the shape of ADR 0386 again, for a smaller
  return.
- Memory: the window's page texture is 8 192 000 bytes at 1280×1600 and there is a spare pair
  travelling back to the render thread. A column of three pages makes the page half three of those,
  and `Pair::fits` — which reuses a pair by the *window's* size — stops being the right pool.
- `stale::AGREEMENT` and 609's `a_scroll_of_a_real_column_is_carried_and_a_zoom_of_one_is_still_refused`
  would go with `one_placement`, and with them the printed measurement that a scroll's placements
  differ by 0–0.000183 px where a zoom's differ by 1.25–2.75 — four orders of magnitude apart, which
  is a fact about `viewer_core::layout` that nothing else in this tree states.

**Declined for now.** `AGREEMENT` and 609's test are untouched, the refusal keeps its bound, and
`doc/todo/37` carries the price so that the next round asking the question starts from it rather
than from the sentence.

## The spec-driven half — §12.5.6.23, and it is the caret's defect a second time

622 left a lead: `appearance::construct`'s catch-all covered `Redact`, `Screen`, `Movie`,
`PrinterMark`, `TrapNet` and `Watermark` with one sentence — *"its clause states no geometry"* — and
§12.5.6.23's redaction states `/QuadPoints`. It does, twice over: Table 195's entry is an array
"specifying the coordinates of n quadrilaterals in default user space", referred by that row to
Table 182 — which is the entry §12.5.6.10 is *drawn* from in this very module — and the same row
names the fallback: "If this entry is not present, the Rect entry denotes the content region that
is intended to be removed".

What is genuinely unstated is any artwork for an **unapplied** redaction, which is the only state
this program sees one in: every overlay entry in Table 195 — `/IC`, `/RO`, `/OverlayText`,
`/Repeat`, `/DA`, `/Q` — is conditioned on "after the affected content has been removed", and that
phase's verb is *destroy* ("that portion of the image data shall be destroyed"), which §7.5.6's
incremental update cannot express.

The behaviour was right and is unchanged; the *sentence* was wrong, and the sentence is the half a
person reads. `Redact` has its own arm with the reading, and a test asserts the sentence rather than
only the behaviour.

**The shape generalises and no sweep can see it.** §12.5.6.23's ledger row had reasoned all of this
out — it says outright that "`/QuadPoints` names the region to be removed rather than anything to
paint" — and the code said the opposite of its own project's reading. A catch-all names no blocker,
no missing vocabulary and no absent architecture, so it passes every sweep `doc/habits.md` lists
while being wrong about whichever member is least like the rest. Two of the six have now been read
against their own tables and two of the six were wrong. **Read the table of one grouped subtype
against the sentence the group shares** — it is one clause's worth of reading and it has a fifty per
cent hit rate so far.
