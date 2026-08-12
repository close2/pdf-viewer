# 457 — The interpolation is on the parameter, and the streaks were the function called early

**Finding.** §8.7.4.5.5 states an *order*: a mesh shading with a `/Function` interpolates the
parametric value across the triangle and calls the function afterwards. This tree called the
function at each vertex and interpolated the colours it returned — the same picture only where the
function is a straight line, and nothing reported the difference. `issue6231_1.pdf`, whose
`/Function` is a type 3 stitching function, drew a lattice of yellow streaks across a plotted
surface that should be smooth; it now draws the surface, and agrees with `poppler` where it did
not. **No gate in this tree could see it**: the oracle's verdicts and the step-7 ink sweep are
identical before and after, and the picture is the whole evidence.

**Date.** 2026-08-13.
**ADR.** [0292](../adr/0292-the-interpolation-is-on-the-parameter-and-the-streaks-were-the-function-called-early.md).
**Touched.** `crates/pdf-render/src/{shading.rs,lib.rs}` (`Corners`, `ShadingKind::Mesh`'s `ramp`,
`MeshRaster::build`'s refusal, four dead public methods removed), `crates/pdf-model/src/mesh.rs`
(the `Corner` trait and a generic reader), `crates/pdf-model/src/shading.rs` (`breakpoints_over`),
`crates/pdf-model/tests/shadings.rs` (two tests), `crates/render-cpu/src/{lib.rs,shading.rs}`,
`crates/render-gpu/src/{scene.rs,shading.rs}`, `crates/render-gpu/tests/headless_gpu.rs` (a
parametric cross-backend scene), `crates/render-quorra/{src/scene.rs,examples/shading_probe.rs}`,
`crates/pdf-model/examples/mesh_census.rs` (new), `doc/conformance/ledger.toml` (§8.7.4.4,
§8.7.4.5.5–.8, §10.7.3, and two parent rows a sweep had left behind — §14.8.2.6 and Annex D),
`doc/performance.md`, `doc/adr/0292-*`, this file.

## Who wrote it

**The code was written by an earlier round and finished by this one.** A sibling session left it
uncommitted in the working tree; round 456 preserved it deliberately and committed only its own
hunks, naming the paths in its own record. This round was told to finish it and land it as one
commit, and judged it as its own work. What that judging changed:

- **Six `clippy::pedantic` warnings**, all `cast_possible_truncation` / `cast_sign_loss` on
  `(fraction * 255.0).round() as u8` in the new tests. Two sites in `pdf-render` share a `level`
  helper with one `#[expect]`; the one in `pdf-model` carries its own. `clippy --workspace` is
  silent in this tree again, which it had not been since the work arrived.
- **The ADR number.** The work cited **0291** in six ledger notes and its comments; round 456 took
  0291 for the staged compose pair the day before. It is **0292** now, everywhere.
- **A session number in a ledger note**: "`partial` until the four-hundred-and-fifty-sixth" was
  written by a round that was not the one that landed it. Corrected to the fifty-seventh.
- **A population nobody had re-run.** The §8.7.4.4 note leaned on `examples/mesh_census` for "the
  3 that are not are `personwithdog.pdf`'s `DeviceN`". Running it says the three `DeviceN` meshes
  are **2 in `personwithdog.pdf` and 1 in `bug1703683_page2_reduced.pdf`**. The census now prints
  the space tally *per document* so the claim is checkable rather than quotable, and its three
  `#![allow]` headers — two of them reasonless — became one `#![expect]` with a reason (trap 7).
- **One duplicated rule.** The mapping from a function's own breakpoints onto a ramp's unit
  interval existed twice, once for axial and radial shadings and once new for meshes.
  `pdf_model::shading::breakpoints_over` is the one construction, cited from both.
- **Four stale comments** in the two backends, saying a mesh carries "a colour per triangle
  corner" and, in `render-cpu`, that "the caller subdivides and fills them" — a construction gone
  since the forty-third session. And `doc/performance.md` named `Triangle::is_subpixel`, which this
  round deleted; the sentence keeps what it measured and says where the method went.

## What the round verified rather than assumed

- **The tests fail without the fix.** The old order was restored in `Triangle::paint` alone and
  both parametric tests fail with the numbers their messages predict — 134 against 70, and 129
  against 65. `a_lattice_mesh_triangulates_between_its_rows` was checked the same way, by emitting
  one triangle per lattice cell: it fails on the fourth corner.
- **The pages.** `issue6231_1.pdf` before, after and `poppler`'s, side by side at scale 2: the
  streaks are gone and the distance to `poppler` over the crop halves, 1.067 → 0.617 of a level.
  `coons-allflags-withfunction.pdf` and `tensor-allflags-withfunction.pdf` move by at most one
  level. `mesh_shading_empty.pdf` is **byte-identical**, because its meshes state colours and no
  `/Function` — the page the §8.7.4.5.5 row had been naming for four hundred sessions is not a
  page this clause can reach.

## Gates

`fmt`, `clippy --workspace` (silent), `nextest --workspace` (1634 passed), doctests, the pdf-model
corpus gate, the oracle, both text gates, dates, xmp, jpeg2000, the quorra corpus gate, and
`conformance`. **The oracle and the ink sweep were run before *and* after**, by stashing the work:
every verdict count, every per-page line and all 786 sweep rows identical. That is the round's
sharpest fact rather than a formality — the change is correct, it moves pixels on a real page, and
the whole instrument park is blind to it.

## Two environment notes for whoever is next

Both cost this round time and neither is about the code.

1. **A stale build script can outlive the checkout it was compiled in.** `pdf-font/build.rs` and
   `tools/conformance` both read `env!("CARGO_MANIFEST_DIR")` — a *compile-time* constant — and the
   shared `/home/AI/cargo-target/pdf-viewer` still held binaries compiled from a scratchpad copy of
   this tree that no longer exists. Both failed with a path under `/tmp/claude-…/scratchpad/verify`
   and neither failure has anything to do with the tree. `touch` the source and rebuild.
2. **A `\"` inside a ledger note has to survive the editing script as well as TOML.** A Python
   replacement wrote a bare quote into a basic string and the conformance gate found it — which is
   §6's rule ("check the file, not the script's exit status") arriving from the other direction:
   the script succeeded and the file was broken.
