# 1531 — Page one is interpreted while the device comes up, the first page is divided, and a page group's cube is divided across the pool

Status: accepted. Session 1348. Amends ADR 0182 in one respect: the document's thread also
interprets the page the view opens at, where it only opened the document. Supersedes nothing.
Context: `CLAUDE.md` principle 2 (startup is a first-class requirement; an optimisation is
justified by a benchmark and explained by a comment); ISO 32000-2 §11.4.7 (the page group's
conversion), §12.5.3 (`NoZoom`, the one case an interpretation depends on the magnification);
ADRs 0147, 0182, 0777, 0884, 1395, 1406, 1441, 1513, 1532; traps 50, 94, 101;
`doc/habits/measuring.md` 52–65.
Code: `crates/viewer-core/src/viewer.rs` (`Viewer::anticipate`, `interpret_arranged` split out of
`arrange`); `crates/viewer-ui/src/bin/quorra.rs` (the document thread calls it);
`crates/viewer-ui/tests/launch_path.rs` (`draw_one_timed`, `interpreted_ms`, `frame_hash`, the
first-page phase's document thread); `crates/pdf-render/src/blending.rs` (`resolve_cube`,
`resolve_cube_in`, `CUBE_BLOCK`, `CUBE_PARALLEL_FLOOR`); `doc/checks/launch-path.toml`.
Tests: `viewer-core/tests/headless.rs`'s
`a_page_interpreted_ahead_is_drawn_as_the_first_resize_would_draw_it` and
`anticipating_with_no_document_open_does_nothing`; `blending.rs`'s
`a_run_of_one_colour_resolves_to_what_each_pixel_resolves_to_alone` and
`a_divided_raster_resolves_to_what_each_pixel_resolves_to_alone`.

## 1. What the first page was made of

The gate's first-page phase is `quorra`'s launch less the window: the document opened on one
thread, the device brought up on the other, joined, resized, drawn. It printed when the device was
up and when the document joined, and they were always the same instant — the open was hidden. It
now also prints when the viewer's render request reached the host, which divides what is left.
Wall clocks are the gate's (minimum of nine pinned children, release, warm page cache), three runs;
instructions are callgrind's on one first-page child, toggled on the post-join half
(`--toggle-collect=launch_path::draw_one_timed`), in millions.

| document | bring-up | page one interpreted after it | device's first frame | first page | open (M instr.) | interpretation (M) | of it: images, fonts | first frame, CPU (M) |
|---|---|---|---|---|---|---|---|---|
| `PDF20_AN001-BPC.pdf` | 24.6–30.1 | 1.0–1.2 | 3.5–7.4 | 33.0–36.9 | 4.96 | 9.3 | 0, 3.0 | 31.5 |
| `Well-Tagged-PDF-WTPDF-1.0.pdf` | 27.8–31.0 | 5.8–6.4 | 9.2–11.3 | 45.5–46.6 | 26.7 | 80.5 | 68.9 (DCT 66.1), 2.7 | 38.4 + 72.3 cube |
| `ISO_32000-2_sponsored_EC3.pdf` | 27.9–30.1 | 3.5–4.3 | 5.8–6.4 | 37.8–40.2 | 185.1 | 46.4 | 19.0, 13.0 | 27.1 |
| `bug1815476.pdf` | 24.4–30.8 | 18.6–20.6 | 5.7–9.3 | 52.3–62.7 | 3.4 | 318.4 | 270.0 (CCITT 247.2), 33.7 of which 27.9 the font catalogue | 110.9 |
| `xfa_filled_imm1344e.pdf` | 25.3–27.9 | 0.6–0.7 | 6.7–7.2 | 33.1–35.3 | 1.8 | 4.5 | 0, 1.8 | 12.7 |

Bring-up was 171.4 M instructions in every child, 156.2 of them `wgpu::Instance::new` — ADR 1532.
The open reads 108, 350, 4320, 84 and 99 KiB in 30, 88, 1075, 24 and 26 calls (the gate's
`read_kib`, `read_calls`) and is behind the device on every row, warm: ISO 32000-2's 12.5 ms warm
open is its largest, and it holds 69.5 M of cross-reference reading and 35.5 M of `Outline::read`
(`Open::around`, eager, on the document's thread). Fonts are small everywhere but where a page
names one it does not embed: 1.8 to 13.0 M per page, `skrifa` reading each program where it lies,
so a font parsed lazily per glyph has nothing to take here. The first
frame's excess over a second frame of the same page is 2.8 to 4.9 ms (`FrameCost`: the driver's
first submission 0.9–1.3 ms more, the encode 0.5–0.9, the readback 0.1–0.4, and on three rows one
pipeline compiled on first use, 0.3 ms).

## 2. Decision: page one is interpreted on the document's thread

The largest stage after bring-up was page one's interpretation on three of the five rows, and it
stood after the join only because nothing asked for it before a viewport existed. Nothing in it
needs one: the display list is in the page's space, and the one input that is the viewport's —
§12.5.3's `NoZoom` placement, which reads the magnification — is already re-asked by every
`settle` (`Open::reinterpret` drops a view-dependent interpretation, ADR 0777's `replace` runs the
annotation pass again, `Open::stale` drops it where the ink moved).

So `Viewer::anticipate` interprets the page the focused document stands at into the arrangement's
own cache (`Open::on_screen`), through the same `interpret_arranged` the first `arrange` would have
called — its reports, its readback, its revision — and `quorra`'s document thread calls it after
`Command::Open` and any `--page`. The first `Command::Resize` places the page, finds it
interpreted, and asks for its render. A method rather than a `Command`, so that a host that does
not call it, and the confined viewer's wire, have no variant to carry.
`quorra-confined`, `quorra-gtk` and `quorra-qt` do not call it yet.

## 3. Decision: the page group's cube is divided, and a repeated pixel copies its answer

With the interpretation gone from behind the device, `Well-Tagged-PDF-WTPDF-1.0.pdf`'s first frame
was the largest remainder, and 72.3 of its 226.7 M instructions were `pdf_render::resolve_cube`:
§11.4.7's conversion out of an `ICCBased` page group, over the whole raster, on every frame. A
pixel's answer is a function of its four bytes alone, so the raster is cut into blocks of 16 384
pixels and converted across rayon's pool above four blocks (ADR 0147's argument), and within a
block a pixel equal to the one before it copies that one's answer. The copy alone took the cost to
55.2 M — the page is a photograph, so runs are short — and the division took the frame from 12.0
to 9.1–10.5 ms. The CPU backend runs the same function and gains the same.

## 4. What it bought, and the bands

Three runs after both decisions and ADR 1532, frames hashed: every row's frame identical to the
frame before (`cc8474db…`, `a85460f8…`, `8f0d1ec4…`, `888b282d…`, `7ee86348…`). First page 24.7–26.2,
28.9–30.6, 28.1–30.0, 34.1–34.6 and 24.5–26.7 ms. On `bug1815476.pdf` the A/B with ADR 1532 left
out isolates this ADR's share: 57.1 and 52.7 ms before, 41.3 and 40.9 after. On that row the document's thread is now the longer — joined at 26.6–27.4 ms against a
device up at 22.0–24.0 — and what holds it is a CCITT image and the font catalogue, neither in
this round's files. `launch-path.toml`'s `first_page_ms` and `peak_anon_mib` were re-taken by its
own rule, floors and ceilings both, so that losing either lever fails the gate as itself.

## 5. Left

The open's eager `Outline::read` (35.5 M instructions on ISO 32000-2) is behind the device warm and
is not cold: that document's cold open is 22.5 ms against a 17–18 ms device now, so a cold launch
of it waits for the open. `bug1815476.pdf`'s CCITT decode and catalogue walk now decide its first
page. The first frame's 2.8–4.9 ms over a second frame is the driver's and the encode's. The
`xfa_filled_imm1344e.pdf` row's clocks are still unbanded.
