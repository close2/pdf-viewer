# 1348 — Page one is interpreted while the device comes up, and GL is left unloaded

The PERFORMANCE slot of batch fifty-four: time to first page. ADRs 1531 and 1532. No ledger row
moved; no question written.

**Division (ADR 1531 section 1).** `launch_path`'s first-page phase prints when page one's render
was asked for and a hash of the frame. Before: bring-up 25–31 ms on every row and the open always
behind it; then page one's interpretation 1.0, 5.8–6.4, 3.5–4.3, 18.6–20.6 and 0.6 ms; then the
device's first frame 3.5–11.3. Callgrind by function: bring-up 171.4 M (156.2 `Instance::new`, 65.9
of it `eglInitialize`); `bug1815476.pdf`'s interpretation 318 M (CCITT 247, catalogue 28); fonts
1.8–13 M a page, so no lazy-parse lever; WTPDF's frame 72.3 M in `resolve_cube`.

**Levers, each A/B'd, every frame hash unchanged.** (1) `Viewer::anticipate` interprets the opening
page on `quorra`'s document thread: `bug1815476.pdf` 57.1/52.7 → 41.3/40.9 ms, WTPDF 45.5/46.6 →
42.2/37.9. Kept. (2) `raster_gpu::create_launch_instance`: primary backends, GL only where they have
no hardware adapter; bring-up 26.9/25.2 → 16.6/19.0 ms, device allocation 11.0 → 6.4 MiB. Kept;
ADR 0179's "not the lever" no longer holds on this driver stack. (3) `resolve_cube` copies a repeated
pixel's answer (72.3 → 55.2 M) and divides the raster across the pool: WTPDF's draw 12.0 → 9.1–10.5
ms. Kept. Warm-up contention with the first frame (a settled device) moved 0.5–1.7 ms, mixed sign:
not taken.

**After, three gate runs**: bring-up 17.1–18.5 ms; first page 24.7–26.2, 28.9–30.6, 28.1–30.0,
34.1–34.6, 24.5–26.7. Bands moved down, floors and ceilings, by the file's rule:
`bring_up_ms` 14.5..22.2, `bring_up_anon_mib` 5.5..7.5, four `first_page_ms`, four `peak_anon_mib`
(`bug1815476.pdf`'s inside, left). Driven under Xvfb, three documents, both instance arms: same
adapter (llvmpipe; RADV cannot present there), same screen, 0 pixels apart on re-runs.

**Gates.** rustfmt --check on my files 0; clippy -D warnings viewer-core, viewer-ui, pdf-render,
render-raster, raster-gpu --all-targets 0; nextest of the same five 1490 passed; conformance 0.
Behind the lock: `launch_path` no clocks 0 (26 banded, 0 outside); with clocks 0 once, 101 twice
on BPC's warm open (0.70, 0.77 ms against 0.57; the base binary read 0.58 in the same A/B, an open
untouched); `raster_golden` 0 (974 held, 0 moved); `pdf-model --test corpus` 0; `turn_path` 0 (20
judged, 0 outside); `render-gpu --test headless_gpu` 0 (39 passed). pdf-syntax/model/font untouched.

**Left.** `bug1815476.pdf`'s document thread is now the longer by 3–5 ms (CCITT, the catalogue).
`Open::around`'s eager `Outline::read` (35.5 M on ISO 32000-2) puts a cold launch of it behind its
22.5 ms cold open. `quorra-confined`, -gtk, -qt do not anticipate. 1347's `bug1721218` figure did
not reach me. Hunks in others' files: `viewer-core/src/viewer.rs`, `tests/headless.rs`, `quorra.rs`
(1346); `render-raster/src/present.rs`, `raster-gpu/src/{startup,lib}.rs` (1347);
`pdf-render/src/blending.rs`.
