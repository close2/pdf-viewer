# 2280 — A mark the union declines is said by name where the draw returns

Status: accepted for integration, and its last step — a host saying the sentence — deferred to a
round that owns the hosts (§3). Session 1723, 2026-10-23, the pixels slot of batch one hundred and
eleven. Clause: ISO 32000-2 §10.7.4, and `CLAUDE.md`'s first principle; the row stays `departed`.
Context: ADRs 1831, 2122, 2253, 2268 §2, 2281.

## 1. What the brief assumed, and what the tree holds

ADR 2268 left a release build declining a mark to the library's converter silently where the
union's walk does not return to its winding or a row of its boundary does not close, because a
loud refusal needed a typed error through `scan::mask_fill`. The brief's hypothesis was one new
variant of `pdf_model::Unsupported`. That did not hold: `Unsupported` is the interpreter's
report, collected in `Interpretation::unsupported` while the page is read and worded by
`viewer_core::report::describe` before any draw; the draw happens afterwards, on a host's thread,
in a crate `pdf-model` depends on and not the other way round (`render-cpu` depends on
`pdf-render` alone). No variant of the interpreter's report can say what a later draw at a later
scale declined, and the three declines ADR 2268 names happen at one scale and not another.

## 2. Decision

- `render_cpu::UnionCondition` names each decline: off the grid, past the budget (`UNION_WORK`, or a
  run whose accumulator cannot be stated), a crossing along a common line, a contour with no point
  to start from, and the two defects — a walk that does not return, a row that does not close —
  which `UnionCondition::is_defect` names and a build with debug assertions still stops on (ADR 2268).
- `Union::settle` and `area::fill_union` return it as `render_cpu::UnionDecline`: the condition,
  the pieces the mark states, and the device pixels it reaches on the page's grid. `scan::mask_fill`
  keeps it in the band's `area::Buffers` and draws the mark by the library as before; `MaskCache`
  takes what its own buffers and a clip chain's kept, a soft mask's narrower cache hands its own to
  the cache it was drawn for, the strips return theirs, and `CpuRasterizer::declined` is the last
  draw's, each mark once. `UnionDecline`'s `Display` is the sentence.
- The planes a page is finished with — §11.4.7's fourth component, §10.8.3's spot planes,
  §11.7.5.2's shapes for a transfer — draw the page's marks on the page's geometry, so the page's
  own draw has kept every one of them, and theirs are not kept twice.
- The test plants the fault where it acts: `fill_union` takes a stretch off the boundary under a
  test-only flag, the debug stop is passed over for it, and
  `a_planted_open_row_is_said_by_the_draw_and_the_mark_is_drawn` reads `RowDoesNotClose` with the
  stroke's pieces and pixels from `declined`, the mark's centre still black; red when
  `mask_fill` does not keep the decline.

## 3. What is left, and whose it is

Nothing between `CpuRasterizer::declined` and a person carries the sentence yet. Every host's
processor draw answers `viewer_core::Rendered::Raster`, which holds pixels only — `viewer_host::
drawing`'s page and window draws, `Underway::finish` for a page drawn while it was read, `quorra`'s
processor lane, the confined worker — and `Event::Reported` is raised by the core for a draw that
failed and never for one that finished. Saying the sentence in the title and on the terminal,
ADR 1831's shape, is a change to `Rendered` and to each of those, which a round owning the hosts
makes. Until then a release build's decline is said to a caller of `render-cpu` and not to the
reader, and §10.7.4's note says so.

## 4. Cost

A cache keeps an empty `Vec` and a draw sorts what was declined; a mark the union measures pays
nothing. No pixel moves by this ADR; ADR 2281's do.
