# 1509 — Page one is drawn while it is read, from parts the reader hands over

Slot 1 of batch eighty-one, 2026-10-12, a loop round on the owner's performance target: ADR 1845's
build. ADR 1855; no ADR 1856, no question. No ledger row moved: the partial-row word yields.

**Premise** held. **Hypothesis** held in shape, not letter: a cut between top-level commands is not
final — the interpreter takes commands back (`ET`'s clip or knockout, §11.6.4.3's seals) — so parts
are cut outside text objects and a take-back spoils the hand-over; the drawer is one surface, not a
plan per part: strips are the serial draw only very nearly (ADR 1758). **My first order did not
hold**: the read on a new thread lost a second on the owner's display and the first frame landed
later than HEAD's (ADR 1855 §4); the read keeps the opening thread now.

**Built.** `pdf_model::progress::Parts` (claimed per interpretation; `taken_back_since_asked` on the
list); `render_cpu::Underway`, byte for byte the one-strip draw; `Viewer::anticipating`/`anticipated`
with `Reader` and `foresee` (the core spawns nothing; three `OnceCell`s became `OnceLock`s);
`viewer_host::Ahead` (drawing from `GLIMPSE_AFTER`) and `Drawing::draw_ahead`; `quorra` shows each look
during the bar and finishes its first frame from it; photographed, both are the page.

**The witness in `quorra`, HEAD → this, seconds, three runs, one clock hold.** Owner's display: first
look 3.17, 3.40, 3.75 → **0.54, 0.54, 0.54**; first frame 3.29, 3.62, 4.01 → **2.91, 3.58, 3.67**.
Xvfb: first look 3.30, 3.93, 4.59 → **0.57, 0.62, 0.57**; first frame 3.43, 4.56, 4.92 → **2.91, 6.35,
3.58** (a 6.2 s read). Headless: first seen 3.38–3.74 → 0.534 s, first frame 2.99–4.28 → 2.49–3.15 s.
The finish is 77–112 ms where HEAD draws 0.62–0.80 s; the read is up to 0.6 s slower beside it.

**Files.** `pdf-render/src/{display_list,group_cost,lib}.rs`; `pdf-model/src/{progress,content}.rs`,
`content/run.rs` (one line, slot 4's), `tests/drawn_while_read.rs`, `examples/hand_over_cost.rs`;
`render-cpu/src/{glimpse,lib}.rs`; `viewer-core/src/{viewer,open,lib}.rs`; `viewer-host/src/{loading,
drawing,lib}.rs`, `examples/drawn_while_read.rs`; `viewer-ui/src/bin/quorra.rs`, `quorra/{app,renderer,
surface,window}.rs`; `tools/drive-windows.sh`; `doc/todo/69`; `doc/state-of-play.md`; ADR 1855.

**Unfinished.** The toolkit windows do not call `Ahead` (four calls each, slot 6's files, ADR 1855
§5); `--cpu`, `quorra-confined`, a page turn as before. Not run: the six arms (no draw path changed;
`raster_golden` holds all), the drive whole, the revised build's `heavy` and `launch_path` — from 08:27
no walk was granted (1511's pid 4058705 kept lane 1, 1510's clock pid 321912 the gate); mine cancelled.

**Gates.** rustfmt --check 0; clippy `-D warnings` all targets, six crates, 0; nextest `pdf-render`
279/279, `pdf-model` 2043/2043, `render-cpu` 197/197, `viewer-core` 375/375, `viewer-host` 258/258,
`quorra` 84/84; `cargo test -p conformance` 0 but `records` (slots 3, 4, 6's); the sweep (`--tree 6`,
parts of 8) exit 0, 910 identical, 97 in parts, 0 different; `raster_golden` (export) exit 0, moved
0; `launch_path` (first build, `--clock`) exit 0, 0 outside; `heavy` (first build) exit 0, 3.748 s.
