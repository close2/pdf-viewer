# 543 — A frame that says it is stale

**Finding.** The window can now answer a view change it cannot draw in time, and the thing that
made it defensible is not the drawing but where the drawing is *allowed to exist*. A reprojection —
the pixels already on the screen, moved to where the new view puts them — lives in a private module
of a **binary**, so nothing that judges a picture can link to it even in principle. That is
`doc/todo/37`'s sharpest rule satisfied structurally rather than by remembering.

**What was built.** A view change whose last frame was slow now shows the previous view's own
pixels under `new ∘ old⁻¹`, within 23 to 51 ms of the key, and the real frame replaces it. The
pixels come from `QuorraPresenter::capture_presented`, which draws the scene `FrameSlot` is already
holding once more into a readback: quorra's `EncodeKey` covers the viewport and deliberately not the
target, so the encode is **replayed**, never made — `Captured::replayed` says so per call, and a
capture that re-encodes is reported by name and switches the feature off for the run.

**The four routes that would have been cheaper, and why none of them exists.** A viewport affine
change is inside quorra's encode key, so it is the 640 ms itself; the swapchain texture is acquired
and presented inside quorra and a host never holds one; there is no texture-backed image to hand
`upload_image`; and rasterising the page again is the cost the feature exists to hide. A readback
of an encode that already exists is not the expensive option — it is the only one whose price is a
replay plus a copy.

**The threshold is arithmetic on a measurement, which is what rule 5 asks for.** `10 ×` this run's
most expensive reprojection, with the top of the measured band standing in until there is one. Seven
reprojections over four scripted sessions of `tmp/Entwurf.pdf` under `Xvfb`/llvmpipe cost **23.1 to
50.6 ms** against frames of 492 to 1036, so the bootstrap is 51 ms and the bootstrap threshold 510.
The one ratio in the design is rule 4's "small fraction" read as a tenth; everything it multiplies
is measured on the machine that is running, and a reprojection that turns out expensive raises the
bar it must clear next time.

**Rule 5, seen rather than asserted.** In one scripted session `+`, `+`, `-` were each approximated
and the two `Down`s that followed were not — the `-` had landed on a magnification quorra had drawn
before, so its frame cost 108.7 ms and the next view change did not clear the bar. On
`doc/PDF20_AN001-BPC.pdf`, whose frames are about 43 ms, the same script draws six frames and
**zero** reprojections, and the summary says so.

**The witness, photographed.** A zoom step, `xwd` at 300 ms — while the real frame was still being
built — and again after it landed. Mid-flight the drawing is already at the new magnification:
**10.7%** RMSE against the photograph taken immediately before the key press, and **1.27%** against
the real frame that replaced it, which is the blur and the detail the old raster never had.
Magnified, the difference is softness in the thin rules and the banding.

**What a revealed edge shows.** Nothing: the reprojection draws only where it has pixels and the
presenter's own medium shows through elsewhere. Repeating the edge pixels would invent page
content, and a distinct "no information" tone would put a colour no clause states on the screen at
every zoom out. The cost of the choice is written down rather than left to be discovered — the
medium is white in this build, so a revealed strip is not visually distinct from blank page for the
half-second it lasts.

**Date.** 2026-08-15.
**ADR.** [0378](../adr/0378-a-frame-that-says-it-is-stale.md).

**Code.** `crates/viewer-ui/src/bin/pdf-viewer/stale.rs` (new: the policy, the reprojection's
display list, `MustFollow`, and nine tests including the one that walks every `.rs` outside
`viewer-ui/src/bin`), `crates/viewer-ui/src/bin/pdf-viewer/surface.rs` (`App::approximate`, the
decision before the transition, the settled view recorded after the frame),
`crates/viewer-ui/src/bin/pdf-viewer/window.rs` (`about_to_wait` will not let the loop rest on one),
`crates/viewer-ui/src/bin/pdf-viewer/timing.rs` (`Stages::approximated`, the legend row, the count
in the summary), `crates/viewer-ui/src/bin/pdf-viewer/app.rs` and `pdf-viewer.rs` (the field, the
module), `crates/render-quorra/src/present.rs` (`Captured`, `FrameSlot::capture`,
`QuorraPresenter::capture_presented`), `crates/render-quorra/src/lib.rs` (the export).

**Touched.** `doc/todo/37-a-frame-that-says-it-is-stale.md` (amended down to the processor's window,
with why that piece is smaller), `doc/todo/README.md` (its row), `doc/adr/0378-*` (new), this file.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets` silent of
lints; `cargo nextest run --workspace` **2012 tests run: 2012 passed, 15 skipped**;
`cargo test --workspace --doc` green. Corpus: `974 documents in 2.5s: 0 unopenable, 8 locked,
2 encrypted beyond us, 6 pageless, 64 incomplete, 0 slow`. Oracle: `1794 pages in 87.0s (1691 we
call complete, 103 incomplete)` — `agrees 906, contradicted 67, ambiguous 786, our geometry 1,
reference geometry 2, not comparable 13, no render 19`. `text_extraction`: 10969/11163 words in
bounds (98.26%) over 486 of 508 documents, and PDFBox's frozen extraction at 99.8%
(14257/14281) both ways; `dates` 1514 of 1545 conform (97.99%); `xmp`, `jpeg2000` and
`conformance` green (8397 citations, all naming clauses the standard has). Both quorra lanes:
default `951 pages… 931 agree, 23 differ, 2 refused, 18 not comparable`, and
`PDFVIEWER_QUORRA_COVERAGE=gpu PDFVIEWER_QUORRA_SCALE=4` `937 agree, 10 differ, 4 refused,
23 not comparable`.

**The gpu lane was run twice, and the second run is why the first can be believed.** Its triple
is not the one the previous session's file records, so the round did the A/B rather than
explaining it: the change taken off with `git apply -R` (never `git stash` — `doc/environment.md`
says why), the same lane run again on the bare tree, **the same 937 / 10 / 4 / 23 and the same
four refusals character for character**, then the change put back. Nothing this round did moves a
judged path, which is rule 2's own gate.
