# 2128 — The window's surface is written in one module per concern

Session 1646, slot 8 of batch one hundred, a docs refactoring round (Sonnet 5.5); `doc/todo/74`
entry 9. Status: **accepted** and **built** in `crates/viewer-ui/src/bin/quorra/surface.rs` and
`surface/`. Context: ADRs 2004 to 2112. Clauses: none; no behaviour moves.

## 1. What the census found

`surface.rs` was 1 783 lines and held one `impl App` of 1 652 lines and 30 methods. The two frame
paths (a graphics device, the processor), the bring-up of the surface and the settling after the
first frames were one block, read top to bottom as four stories.

## 2. The decision

`surface.rs` stays the root and keeps what every path reads: `Surface`, `State`, the four accessors
that end a borrow of `state`, `redraw_requested` (which routes to a path), and the free functions
`predicted`, `in_the_window` and `wake_on`. Five modules, each an `impl App`, declared with
`#[path = "surface/<name>.rs"]` as the sibling files of `quorra/` are:

| module | holds |
|---|---|
| `device_frame` | `on_the_device`, `as_built`, `put_up_as_built`, `put_up`, `present`, `without_a_page` |
| `device_adoption` | `said_taken_back`, `said_idle_work`, `adopt`, `say_the_lane`, `device_reported`, `swapchain_refused` |
| `processor_frame` | `on_the_processor`, `adopt_composed`, the `put_up_*` and `stand_in_composed`, `arrangement` |
| `bring_up` | `bring_up`, `backend_description`, `software`, `no_device` |
| `settling` | `settle_cadence`, `pipelines_compiled` |

Visibility is what a sibling reads: ten methods are `pub(super)`, each widened from the compiler's
E0624 (`on_the_device`, widened first, was narrowed back because nothing outside its module calls
it). Each module names the imports it uses; no glob.

## 3. The proof that nothing moved

The multiset of the old file's non-blank lines against the root and five modules: removed-only 11
lines (ten signatures that gained `pub(super)`, one import list), added-only 75 (module docs,
`use` and `mod` lines, five `impl` wrappers, the ten widened signatures, and `arrangement`'s
signature that `rustfmt` broke across lines). `viewer-ui` is a leaf binary crate: `cargo tree -i
viewer-ui` lists nothing above it, so it is outside the corpus arms' binaries and the arms are not
owed; `cargo test -p viewer-ui` and clippy over its targets are the proof of behaviour.

## 4. What this costs

A reader looking for `present` opens `device_frame.rs`. No ledger row, teaching diagram or
`crate-map.md` row names `surface.rs`, so none is re-pointed. `device_frame.rs` is still about 520
lines, most of it `on_the_device`'s and `present`'s documentation.
