# 1391: a composite is drawn in the pass after it, and the regions floor stands on a lived device

Raster slot, batch sixty-one. ADRs 1618 and 1619; no ledger row moved, no question. Threads of
user AI beside the first heavy run: 125 before the first build, 481 before the corpus walk.

**Counted** (habit 63), every `begin_render_pass` logged by label (scratch, removed):
`bug1721218_reduced.pdf`'s first frame is 222 passes — two group renders of 107–108 and a page
frame of 7. Per group render: 32 child and mask contents, 29 backdrop copies, 29 composites, 3
reductions, 1 blit — each a change of attachment — and 12 runs resumed after a composite, 1 empty
clear and 1 end stamp, which change none. Flat pages are 2 passes; `personwithdog.pdf` 28,
`22060_A1_01_Plans.pdf` 42, `images.pdf` 6. No pass ends for a blend mode, knockout, clip or barrier.

**Built** (ADR 1618). A child's composite is carried to the next pass onto its accumulator and
drawn there first, under its own scissor; an isolated child that opens a plan has its copy cleared
and its composite's pass load with `Clear`; the end timestamp is written by each route's last pass.
193 passes where 222, flat pages 1 where 2. Three new tests in `a_composite_shares_the_pass_after_it.rs`,
each watched failing (scissor not reset; composite loading `Keep`).

**Priced, not built**: the copies as `copy_texture_to_texture` (58 a turn, exact, owes a measurement
of the copy usages on RADV); consecutive disjoint composites in one pass (11 of 16 pairs, past the
priced texture peak). **Decided** (ADR 1619): in one sitting the seventh frame is the quicker by its
first uses, so the walk ranks the same on either device and the polygonal fill stays unbuilt.

**Measured**, pinned, interleaved exports, md5-distinct: `frame_budget` turn 153.10–154.33 →
151.86–153.06, seventh 152.04–153.14 → 151.16–152.37, step 84.81–85.59 → 84.08–84.30;
`zoom_frame` inside its spread, 1.67× and 1.36× the CPU backend for both trees, so no feedback
section. `doc/performance.md` 3e's three rows re-taken, `turn-path.toml`'s bands by its rule
(turn 129.0–187.6, seventh 128.4–186.5, step 71.4–101.9), `doc/todo/36` row kept true.

**Gates.** `rustfmt --check` on the seven source files: 0. Clippy `-D warnings`, `raster-gpu` and
`render-raster`, all targets: 0. `cargo nextest run -p raster-gpu -p render-raster`: 0, 800
passed. `cargo test -p conformance`: 0. Behind the lock, exports of HEAD and of the change in their
own target directories, sandbox rebuilt inside: `render-raster --test corpus`, six arms each, all
exit 0, digests by name 0 moved of 968 / 964 / 968 / 963 / 968 / 964, verdicts equal, 1-vs-N 0, in
1699 s and 1502 s; `raster_golden` on the worktree: 0, held 974, moved 0, in 29 s;
`tools/batch.sh raster-examples`: 0, 14 passed, in 148 s; `turn_path` on the old bands: 0, 33 of 33
judged, in 169 s; on the new bands twice: 0 and 0, 33 of 33 judged, 0 outside, in 94 and 94 s.
