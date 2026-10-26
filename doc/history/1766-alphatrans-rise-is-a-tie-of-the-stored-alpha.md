# 1766 — `alphatrans`'s rise is a tie of the stored alpha, and no translucent function paint exists

Session 1766, slot 7 of batch 116, a pixels round; 2026-10-26. ADRs 2366, 2367; Q629 unused.

**Premise held**: ADR 2350 left the function lane rounding at any alpha. **Hypothesis not held as
stated**: the function lane's translucent paints are not few but none — `Shading::with_alpha` drops
the program, so the lane's alpha is 0 or 1 and the shading lane's rule would move no byte — and
`alphatrans`'s rise is not the knockout groups' compositing but a tie the two sides break apart.

**Census** (ADR 2366). At the scene builder of a scratch copy of HEAD, over the corpus gate's 974
first pages (958 drawn): 12 sampled shadings on two pages, 8 device-evaluated, every one opaque. The
control, a constant alpha of 0.5 planted on those two pages: 12 of 12 printed translucent and
programless, none evaluated. Nothing built; the comment above `function_lane.wgsl`'s `fs_main` now
says why the lane rounds at any alpha.

**`alphatrans`** (ADR 2367). Isolated at full size on RADV with ADR 2350's rule and the one before:
only the ramp over group 87 rises (0.0131 to 0.0162 on `cpu` 1x; `gpu` 0.0180 to 0.0212, `compute`
0.0131 to 0.0162, `cpu` 4x 0.0129 to 0.0160); group 87 alone differs at 0 bytes, image 88 alone does
not move, the ramp over 86 and over 88 falls. The rise is all green (+6 276 summed levels; red
−3 824, blue −1 285): over group 87's stored alpha 128 the composite's alpha is 191.5 levels,
which `tiny-skia`'s store writes 192 (green 63) and RADV 191 (green 64), at 10 044 pixels. The exact
page is 191.25, green 63.75: the device's. Against that exact composite group 87's interior is off
nearest at 7 068 channel values on the device, 22 568 on the oracle, 22 692 under the old rule; on
llvmpipe the device matches the oracle there. **For slot 2**: no oracle defect — its round half to
even is a tie rule — and an oracle ramp drawn from per-pixel levels keeps the ramp's alpha a byte,
so this page's ties stay. Looked at: the page, the ramp alone and over group 87, both backends, 1x.

**Files.** New: ADRs 2366, 2367, this record. Edited: `raster/crates/raster-gpu/src/shaders/
function_lane.wgsl` (a comment), `doc/state-of-play.md` (two sentences).

**Gates.** The code change is a WGSL comment, proved by the shader's non-comment lines against
HEAD's (identical), so the six arms and `t2-turn_path` were not walked: no compiled program differs.
Worktree: `cargo fmt --check -p raster-gpu` 0; `-D warnings clippy -p raster-gpu --all-targets` 0;
rustdoc `-D warnings -p raster-gpu --document-private-items` 0; nextest `raster-gpu` 779 of 779 (2
skipped), exit 0, its fixtures on every adapter `Device::adapter_names` offers, RADV among them;
`cargo test -p conformance --no-fail-fast` 464 of 464, exit 0. I worked in `scratchpad/r1766/`
(probe export, isolation and census binaries); the census walk under the lock, exit 0, 7 s held.
