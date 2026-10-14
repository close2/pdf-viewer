# 1910 — `quorra`'s renderer and its stand-in are module trees by concern

Session 1537, slot 4 of batch eighty-five, the first batch of the project owner's switch of
2026-10-14 to "code improvements/refactoring". Status: **accepted** and **built**. Context: ADR
0391 (the render thread), ADR 0443 (the retained pages), ADR 0699 (the sharp pass), ADRs 1821, 1831
and 1886 (the lane a frame takes), ADR 1855 (page one as it is read), ADR 1023 (a comment carries
its current reason). It cites no clause and moves no ledger row.

## 1. The decision

`crates/viewer-ui/src/bin/quorra/renderer.rs` (3 569 lines) and `stale.rs` (3 686) are each a root
and children. A thing is found by what it is about. Each root keeps its module comment, and gains a
table of where each part is.

| `renderer` | what it holds |
|---|---|
| the root | what crosses between the threads: `Pair`, `Job`, `Done`, `Finished`, `Prepared`, `Partial`, `Link`, `Shown`, `Landed`, `same_pages` |
| `window` | the event thread's half: `Flight`, `Window`, `Ungrounded`, `affine`, the whole `impl Window` |
| `lanes` | `Prediction`, `Costs`, `Measured`, `Lane`, `Refusals`, `Refused`, `remembered`, and their tests |
| `thread` | `Idle`, `spawn`, `draw_until_told_to_stop` |
| `device`, `processor` | `draw` and `prepare`; `processor_half`, `ProcessorHalf`, `draw_on_the_processor` |
| `retained`, `sharp`, `as_read` | the stand-in's layer drawn while idle; ADR 0699's pass and its bound, with its tests; page one's looks uploaded |
| `gesture` | the zoom-gesture measurement, the old `mod gesture`, in a file of its own |

| `stale` | what it holds |
|---|---|
| the root | `PROXY_EDGE`, `Picture`, `Placed`, `Depicts`, `MustFollow`, `Settled`, `Standing`, `Stale` |
| `plan` | `Plan`, `Stand`, `Missed`, `Source`, `Refusal`, `Refusals`, `Drawn` |
| `placement` | `depicts`, `disagreement`, `AGREEMENT`, `one_placement`, `absorbed`, `Carried`, `snapped`, `carrying`, `could_stand_in` |
| `retained`, `canvas` | `PROXY_PAGES`, `Retained`, `proxy_target`, `Proxies`; the processor's resample, with its five tests |
| `tests` | the module's other tests, one indent shallower |

## 2. What moved and what did not

- **No code line changed.** Compare HEAD's two files with the two trees as multisets of lines. All
  that differs is imports, `pub(super)`, rustfmt joining five lines that de-indenting let fit,
  `super::` and `pdf_render::` dropped from the canvas tests (now `unused_qualifications`), the
  module comments, and seven comments that said "this file" or "this module" of a place they have
  left. The instrument names a planted change of one constant.
- **Callers' paths are unchanged.** The roots re-export what the binary reads (`crate::renderer::
  {Window, Prediction}`, and `crate::stale`'s plan, placement, retained and canvas items).
  `PROXY_EDGE` went to `stale`'s root because `renderer` and `composer` name it. The one path a
  comment wrote that is now private is written `crate::renderer::window::Ungrounded`.
- **Visibility.** Nothing is wider than `pub(super)`. Each of the 45 widenings was checked by
  taking it away and compiling, and all 45 failed without it. The same check calls a planted
  needless widening needless. Doc links to names a
  child no longer imports go through reference definitions, so the prose is unchanged.
  `cargo doc --bin quorra --document-private-items` prints HEAD's 24 warnings, the same ones.
- `#[path]` on every child, for the binary root's reason: a module reached through `#[path]` owns
  its directory, so a bare `mod x;` inside `renderer.rs` would resolve beside it.

## 3. Gates

`cargo nextest run -p viewer-ui` runs 187 tests before and after, all passing. They are the same
function names. Seventeen now sit under the module of their subject: nine in `renderer::lanes`,
three in `renderer::sharp`, five in `stale::canvas`. Clippy pedantic is clean with `-D warnings`.
The drive, `launch_path` and `turn_path` figures are in the round's record (`doc/history/1537-…`).
