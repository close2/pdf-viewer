# 0706 — The image crosses the boundary with its flag, not its answer

**Status.** Accepted — the third §4.5 amendment (quorra's ADR 0089; the stroke's ADR
0701 and the collapse's ADR 0703 are its siblings), driven by ADR 0702's ledger:
an image command carried the filter and the area-averaged reduction resolved at one
placement, so any page with a picture on it could never keep its page-space scene.

## What changed, on this side

An ordinary image now crosses as its own samples — cached under the source's
identity, as ever — plus §8.9.5.3's `/Interpolate` flag
(`quorra_scene::ImageFilter::Auto`), and quorra's encode resolves the filter and the
reduction per placement, mirrored statement for statement from `pdf_render`
(`is_smoothed`, `factor`, `Reduction`, `area_averaged` — integer arithmetic
throughout, so the mirror is byte-identical, not close). The reduced variants become
resident on the device, once per `(image, factors)`, replacing this crate's own
reduced-upload cache (ADR 0297's mechanism moves down a layer and widens: the
variant now survives scene rebuilds too). The image command reads nothing of the
view, and `consume_view` at the image site is gone.

The **deferred** image — §11.6.5.2's sampled soft-mask product, produced at this
placement's grid — keeps its per-placement upload and keeps consuming the view: its
samples have no identity for any cache to key.

`render-cpu` and `render-gpu` are untouched, resolving through `pdf_render` as
before — the oracle keeps the originals the mirror is compared against, and the sixty
oracle gates passing unmodified is that comparison.

## Measured

An image-bearing corpus page (the fully-featured sample), alternating placements on
the 890M: scene 0.0 from frame 1, `RecordReplayed`, whole zoom step 1.6–2.7 ms.

## Held by

Quorra's suite (629) with their `tests/auto_image_filter.rs` — one scene resolving
per viewport is the property bought — and this side's sixty `render-quorra` tests
and the full workspace against the local quorra. The commit of this side waits on
the pin reaching ADR 0089, as the lock discipline requires.
