# 2367 — `alphatrans.pdf`'s rise is a tie of the stored alpha, and the device draws the exact page's level

Status: accepted — ADR 2350 §5's named rise re-stated with its cause, and nothing built: the rise is
half-level ties the two sides break apart, the device's side nearer the page in exact arithmetic, and
no oracle defect is handed on. Session 1766, 2026-10-26, the pixels slot of batch one hundred and
sixteen, under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clauses:
ISO 32000-2 §11.3.3 (the compositing formula), §11.4.7 (the page group, isolated, composited onto
the medium), §11.6.4.4 (the constant alpha), with §10.7, which leaves the conversion of a colour to
the device's bits to the device; no ledger row moves. Context: ADRs 2311, 2341, 2350, 2351;
traps 1, 213, 251, 257. Code read: `raster-gpu/src/shaders/shading.wgsl` (`fs_main`),
`render-cpu/src/lib.rs` (the page drawn onto transparency, `impose_on_medium`), `tiny-skia` 0.12.0's
`pipeline/highp.rs` (`unnorm`).

## 1. Where the rise lies: the ramp over group 87, and nowhere else

`alphatrans.pdf`'s command 89 is an axial ramp from red to blue at alpha 0.5 over knockout groups 86
(green at 0.5) and 87 (blue at 0.5) and image 88 (at 0.5). Isolated at full size on a scratch copy of
HEAD with the shading lane's rule switchable — ADR 2350's, and the rule before it, which took every
paint's level — on RADV, the arms' adapter, 1x, the `cpu` lane: the page 0.0498 before and 0.0503
now; the ramp alone 0.0131 to 0.0015; over group 86 0.0144 to 0.0097; over image 88 0.0296 to
0.0244; **over group 87 0.0131 to 0.0162**, the one pair that rises, and it rises on every lane and
scale tried (`gpu` 0.0180 to 0.0212, `compute` 0.0131 to 0.0162, `cpu` at 4x 0.0129 to 0.0160).
Group 87 alone differs from the oracle at 0 bytes and image 88 alone moves not at all, so it is
neither the knockout group's compositing on the device nor the image's sampling. Per channel over
the whole page, ADR 2350 moved the summed difference from the oracle by −3 824 in red, −1 285 in
blue and **+6 276 in green**: the rise is the green channel.

## 2. The green channel over group 87 is a tie of the stored alpha

The ramp's green is 0, so the composite's green is the backdrop's share alone. §11.4.7's page group
is isolated, so both backends draw the page onto transparency and impose the medium's white after
(`impose_on_medium`), and the white's share is `1 − α` of the alpha each side stored. Group 87's
alpha 0.5 is 127.5 levels and is stored as level 128 on both sides. The ramp over it then reaches
`0.5 + (128/255)·0.5` = 191.5 levels — an exact half in `f32` as well — which `tiny-skia`'s
store (`unnorm`, round half to even) writes 192, so green 255 − 192 = **63**, and RADV's blend
writes 191, green **64**. The rule before ADR 2350 handed the blend the level 128 for the ramp's
own alpha and drew 63 with the oracle. That is 10 044 pixels of group 87's interior at one level
each, and over the stored bytes neither side is nearer: a tie. In exact arithmetic, where no group
is stored, the page's alpha there is 0.75 — 191.25 levels — and its green 63.75, nearest **64**:
the device's. Against that exact composite (the two groups and the ramp in `f64`, at each pixel
centre's `t`), group 87's interior is off its nearest level at 7 068 channel values on the device,
22 568 on the oracle and 22 692 under the rule before. On llvmpipe, whose blend takes the source at
eight bits (ADR 2350 §4), the device draws the ramp over group 87 within 1 439 bytes of the oracle
and the page falls (0.0329 to 0.0225); the tie breaks apart on RADV alone.

## 3. What follows

ADR 2350 §5's rise is these ties, named: no device lever is owed, because the device's level is the
exact page's, and none on the oracle, whose round half to even is a tie rule rather than a defect.
The same tie stands over the bare page — the ramp's own alpha 0.5 stored as 128 leaves white 127
of 127.5, on both sides alike — which is why the ramp alone agrees with the oracle and is half a
level below the exact composite at about half its red and blue pixels; ADR 2351's half float is not
its remedy, since the tie is the stored alpha's. An oracle that drew its ramps from
per-pixel levels would store the ramp's alpha 0.5 as a byte too, and leave these ties where they
are. Looked at
(trap 1): the page, the ramp alone and the ramp over group 87, oracle and device at 1x.
