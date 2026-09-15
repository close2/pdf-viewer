# 1066 — The lattice is snapped where a device pixel has been stated

Date: 2026-09-15. Branch: `batch-1062-1067`, shared with five sibling rounds. ADR:
[1080](../adr/1080-a-tiling-lattice-is-snapped-where-a-device-pixel-is-stated.md). Rows §8.7.3.1 and
§8.7.3, both `partial` → `implemented`.

Table 74's `/TilingType` had been read and given code 2's treatment whatever it said (ADR 1031).
`crates/pdf-render/src/lattice.rs` is now the clause's arithmetic — `TilingType`, `Lattice`,
`snap_lattice`, `MAX_DISTORTION` — and `pattern::Interpreter::lattice` is where a device pixel comes
from. The lattice is two step vectors; "spaced consistently … by a multiple of a device pixel" is
rounding each of their components to a whole pixel and paying for it on the pattern matrix, leaving
`/XStep`, `/YStep` and the matrix's translation — §8.7.3.1's *phase* — untouched. The distortion is
measured at the cell's corners; a snap past one device pixel is declined, as is one collapsing the
lattice.

**ADR 1031 guessed that only a backend knows the pixel, and that is the half this round revises.**
By the time a list reaches a rasteriser the tiling is gone — the cell is expanded into displaced
copies during interpretation (ADR 0430) — while `ViewState`'s magnification is *already* read
there, put in place by §12.5.3's `NoZoom`. `None` is *nobody has said*, so every gate places tilings
at the file's own geometry and none of them moves; the viewer states a magnification every frame. A
page placing such a lattice is `view_dependent` and keeps no ADR 0777 replacement: §12.5.3's pass
cannot re-place what the content stream laid.

`examples/tiling_type_census` now measures the lattice beside the codes, with a control over the
documents asking for neither code 1 nor 3. Its first honest run cost a defect:
**`DisplayList::geometry_digest` does not hash transforms**, though its doc comment said it did, so
the census's first answer — a third of the real movers — was a sentence about the instrument
(trap 13). `issue16038.pdf` page 1 moves an eighth of its pixels with the two digests equal. The
comment is corrected and names that witness; the census compares whole lists.

Six movers were opened and looked at (trap 1) and each is what its `/TilingType` predicts. The two
largest are improvements a person can see: `issue16038.pdf`'s ruled grid loses the notches its cell
boundaries left, and `issue11473.pdf`'s hatches come out evenly spaced rather than beating against
the pixel grid. The rest move a fraction of a percent, and one mover moves no ink at all.

**Cost: nothing the gates pay.** `callgrind_pages` on `issue16038.pdf` page 1 is two instructions
cheaper than the same binary with the snap patched out, and `22060_A1_01_Plans.pdf` page 1 — 6.76 G
instructions of pattern — differs by less than the instrument's spread. What it does cost is a
re-interpretation per zoom of a page that places one, which is §12.5.3's own price.
