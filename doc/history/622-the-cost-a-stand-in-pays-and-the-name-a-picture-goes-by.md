# 622 — The cost a stand-in pays, and the name a picture goes by

`doc/todo/37`'s two remaining items, in the order it asked for them: the processor's window, which
had no stand-in of any kind, and the identity that made every `SinglePage` page turn show nothing.

Date: 2026-08-20.
ADR: [0457](../adr/0457-what-a-stand-in-costs-and-what-a-picture-is-of.md).

Touched: `crates/viewer-ui/src/bin/pdf-viewer/{stale.rs, surface.rs, renderer.rs, timing.rs}`;
`crates/viewer-core/src/{event.rs, open.rs, viewer.rs}`; `crates/pdf-model/src/appearance.rs` and
`tests/annotations.rs`; `doc/conformance/ledger.toml` (§12.5.6.11), `doc/ui-boundary.md`,
`doc/todo/37`, `doc/todo/README.md`, the ADR and this file.

## Item 1 — the processor's window

A resample of one window of RGBA under the same `new ∘ old⁻¹` affine, in `crate::stale::Canvas`, a
private module of the binary. The pixels were already in hand: `compose_pages` produces them on the
way to the window, so `on_the_processor` hands the raster back instead of dropping it and the base
and `Stale::settled` are adopted by one call site.

**Rule 4 comes back for that surface and only that surface**, which is the argument `doc/todo/37`
said this round would owe. ADR 0391 deleted it because a device-path reprojection is three textured
quads issued while the render is on another thread — a cost that is structurally zero. There is no
other thread here: one call resamples, presents, and then draws the true frame and presents that. So
the premise is genuinely reintroduced and the rule returns in ADR 0384's form,
`resample + period ≤ frame`, with no constant and with *unmeasured permits*. `Standing::Beside` and
`Standing::InFrontOf` are what ask one rule of two arrangements; the device path gains no gate.

**Rule 1 is met sooner than a clock could meet it.** `MustFollow::drawn_in_the_same_frame` is the
second discharge, and the alternative — standing in on one tick and rendering on the next — would
have needed a second mechanism to stop the loop standing in for ever.

## Item 2 — what identifies a page's picture

`(document, page, ink)`, and not the address of the `Arc<DisplayList>`. A page returned to is
interpreted again and arrives as a new address over the same commands, which is why every
`SinglePage` page turn printed `another page — nothing to show`. `RenderRequest::ink` carries the
third, from `Open::stale` — already the one place that decides what a change to the ink *is*.

A picture of superseded **ink** is dropped, and a picture of a superseded **interpretation** is kept.
That is the decision 608 said was owed rather than a key changed in passing, and the distinction it
turns on is the one that file drew: blur says *approximation* by itself, and a §8.11 layer switched
off going on being drawn sharp says nothing and asserts something false.

## What running it said

**The absolute cost of a resample is the machine's and is not in the code.** This machine was
running three parallel rounds' gates at a load average over 70 on 24 cores throughout, so the
figures in the ADR are quoted for legibility rather than as a level — and the one measurement that
survives that, taken by timing two forms **alternately in one process**, is that `f32::floor` and
`f32::round` are `libm` calls on this target and cost 1.3 to 1.8 times the sampling. A fixed-point
form was tried the same way and rejected: no gain outside the noise, at two levels of 255.

On screen, `SinglePage` on the graphics device: `Right` to page 4 and `Left` back to page 3 both
printed `approximated from a retained page`, the return being the case that showed nothing before
this session. On `--cpu`, the policy was seen doing both halves of its job — standing in against a
frame of 224 ms and refusing against one of 127 with both numbers in the refusal.

**What could not be photographed** is the processor's stand-in itself: a screen capture on this
machine under that load takes longer than the stand-in is up, so a burst of them shows the window
flip from the old view to the true frame without catching what was between. What stands in for the
photograph is the presenter's own report and two unit tests over the resampled pixels.

Launch, three runs of the 1023-page specification: 136.3, 136.6 and 139.2 ms to first present —
inside the band 607 and 608 measured, which the two items had to leave alone and did.

## The spec-driven half — §12.5.6.11, and it is the sixth refusal shape

The row said what was owed was "a reader for `/Sy` and `/RD`, which this tree has none of — no
source names either key". `/Sy` is unread. `/RD` has been read since §12.5.6.8 was implemented:
`appearance::insets` applies it in Table 183's own left, top, right, bottom order. Three tables
state that entry in that order, two are read, and the row said none was — `doc/habits.md`'s sixth
shape, **grep for the entry rather than for the capability**.

What it cost was the sentence a person reads: a caret fell to `construct`'s catch-all, *"its clause
states no geometry"*, which is false of a table stating four numbers of geometry and a
`shall`-bearing code point. It has its own arm now with the reading the row had already got right —
the artwork of the caret is stated nowhere, and Table 183's pilcrow is "displayed along with the
caret" rather than instead of it — and the test asserts the sentence rather than only the behaviour.

**A lead for a later round, not acted on here.** The catch-all still covers `Redact`, `Screen`,
`Movie`, `PrinterMark`, `TrapNet` and `Watermark` with one sentence between them, and §12.5.6.23's
redaction states `/QuadPoints`. Whether that sentence is true of the other six is a question this
round did not open.

## The hosts

Level, and structurally. `viewer-gtk` and `viewer-qt` are tier 1: the core hands them a whole-page
raster per page and the toolkit scales it, so the reprojection does not exist there in this form at
all — which is the same answer 608 and 609 gave and is still the right one, because what a tier-1
host would reproject *with* is its toolkit's scaler rather than this module. Both compile against
the widened `RenderRequest` and ignore the field, as a host that keeps no pixels of its own may.

## What is left

`doc/todo/37`'s two open questions, neither of which is a defect: a render thread for the
processor's window, which is what would give it the retained low-resolution pages and make rule 4 a
question again rather than an answer; and a placement per page in the presenter, which would remove
`Refusal::Rearranged` rather than bound it.
