# 2290 — A mark the union declines is told to the viewer and said in every window

Status: implemented. Session 1728, 2026-10-23, the host slot of batch one hundred and twelve.
Clause: ISO 32000-2 §10.7.4, and `CLAUDE.md`'s first principle; the row stays `departed`.
Context: ADRs 1831, 2268, 2280, 2281; `doc/ui-boundary.md`'s freeze.

## 1. What the brief assumed, and what held

The brief's hypothesis was one field on `viewer_core::Rendered`. It did not hold, for two reasons
found in the code. `Rendered::Raster` is one of four answers and `quorra`'s processor lane answers
`Rendered::Presented`, one acknowledgement per page of a composed frame; and the largest marks are
drawn as **windows** of a page past its window (ADRs 1892, 1930), whose draws after the first
answer no `RenderToken` at all — `inks.pdf` at sixteen times is such a page. A field on an answer
would have carried a decline from neither.

## 2. Decision

- **`viewer_core::Command::Declined { document, page, declined }`**, keyed by the page, carrying
  `viewer_core::Declined` — the condition and the sentence, as words, because the core names no
  backend. It passes the freeze's test: what a draw declined is a value only the host has. The core
  answers it with `Event::Reported` for the page **once for each condition while the page's reading
  stands** (`Interpreted::declined`), and keeps the sentence with the page's notes, so
  `Query::Reports` says it again; a zoom gesture's frames at every magnification say it once. A
  page not open, or whose reading has gone, takes nothing.
- **Every processor draw hands its declines on**: `render_cpu::Underway::finish` answers
  `render_cpu::Finished` with the strips' declines; `viewer_host::drawing`'s page and window draws
  carry them on `Finished::declined` (`viewer_host::drawing::said` words them); the native window
  sends the command for a page and for a window; `viewer_device::compose::compose_pages_watched`
  answers `Composed` with each arrangement page's, which `quorra`'s two processor lanes (the
  device's processor half and `--cpu`'s composer) turn into one command per page; the confined
  worker tells its own viewer for a page and a window it drew, and `quorra-confined` sends the
  command for a page it drew from crossed marks — so it crosses the pipe (`DECLINED`, 44); the C
  ABI's `quorra_raster` is `viewer_ffi::session::Drawn`, whose declines
  `quorra_render_ready_raster` tells the viewer after the frame.
- **Said the way ADR 1831's lane sentence is**: the terminal gets the core's sentence once; the
  title names the pages — `viewer_host::status::declined_pages`, "marks drawn at the library's
  sixteenths on page N", through `WindowState::title_notes` in GTK and Qt and in `quorra-confined`;
  `quorra`'s title counts it among the page's items, as it counts every page note.
- **A drive plants the fault**: `PDFVIEWER_PLANT_OPEN_ROW` in a process's environment plants ADR
  2280's open row where `fill_union` checks it, read once; the drive's `89-union-declined` launches a
  folding stroke planted and then unplanted, the control that must stay silent (trap 13).

## 3. What it measured

`inks.pdf` at sixteen times declines **two** marks past `UNION_WORK`, not one: 3 493 pieces from
(5892, 5143) and 19 743 from (528, 5255) (ADR 2281 named the larger). A window over the larger one
names it as the window's part, 602×72 pixels from (528, 5257); `viewer-host`'s `union_declined`
test holds that it comes back with the window and is said once for two draws.

## 4. What is left

The device lane measures no union and declines nothing. A raster nobody is shown at its size —
`viewer_device`'s retained proxies, `compose_pages` for a stand-in — says nothing of a decline. A
C caller's window raster (`quorra_render_request_rasterise_window`) carries its declines in the
handle, and no call yet hands them back. A page the confined worker drew as pixels is said on the
terminal and not named in that window's title, which knows only the draws it made.

## 5. Cost

A `Vec` per draw, empty for nearly every page; one cell load per union fill for the plant. No pixel
moves.
