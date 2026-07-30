# ADR 0047 — The three modes the correctness oracle got wrong

Status: accepted, 2026-07-30.

## Context

ADR 0046 wrote the cross-backend scene clause 11 had never had and found that `Hue`, `Color`
and `Luminosity` differ between the two backends by 113 of 255, with §11.3.5.3's closed form
saying the **CPU backend** is the wrong one. It stopped there deliberately: taking the four
non-separable modes back from `tiny-skia` means `render-cpu` reading the destination, which is
a change to the backend's structure rather than the tail of a clause review.

This is that change.

## Decision

**`render-cpu/src/blend.rs` implements §11.3.5.3.** `Lum`, `ClipColor`, `SetLum` and `SetSat`,
Table 135's four blend functions over them, and §11.3.6's compositing formula with §11.3.7.3's
union — the whole of it about 150 lines, because the clause states the arithmetic exactly.

**A command under one of the four is drawn onto a transparent layer and composited by hand.**
The layer is band-sized, so it costs what the command's clip admits and nothing on any page
that does not use these modes. Drawing onto transparency loses nothing, and that is a
derivation rather than a hope: §11.3.6's formula with α<sub>b</sub> = 0 collapses to the source
colour whatever `B(Cb, Cs)` is, which is also why `convert::blend_mode` maps all four to
`SourceOver` instead of to the library's own versions. §11.6.6's groups take the same two steps
through `draw_group`.

**The formulas stay in `render-cpu` and are deliberately not shared with the GPU backend.**
Trap 2's rule sends a *decision* into the crate both backends share, because a decision either
one can make alone is a decision neither has made. This is not a decision: the clause states
the arithmetic, Vello implements it in its own shader, and
`cpu_and_gpu_agree_on_every_blend_mode` compares the two. Hoisting these functions into
`pdf-render` would make that scene compare one implementation against itself — the thing a
cross-backend comparison exists not to do.

**`NonSeparable` is its own enumeration rather than a checked `BlendMode`.** It has four
variants, so `NonSeparable::blend` is total: there is no arm a separable mode can fall into,
silently or otherwise, and the single place the two paths part is `NonSeparable::of`. That is
trap 5 applied to a type rather than to a report.

## Result

All sixteen modes now agree between the backends **to the channel** — not within a tolerance —
and `DISAGREE` in `headless_gpu.rs` is empty. It stays as a list rather than becoming a
tolerance, because a fourth mode joining it should be a failure and not a footnote.

## Three things this cost, and what they taught

**The corpus cannot see this fix at all, and that was measured rather than assumed.** One
`eprintln!` in `blend_mode` over all 974 documents' first pages: **five uses, in two
documents** — `issue21570.pdf` selects `Color`, and `bitmap-template1-customat.pdf` selects
all four. Neither page's raster moves by a byte, verified by rendering
`issue21570.pdf` before and after and comparing the SHA-256 of `ours.png`: identical. Both
oracle verdicts are unchanged, and the whole gate is 811 agreeing and 88 contradicted before
and after. A clause every renderer implements, wrong on this backend for the project's whole
life, and the demand curve's answer is *zero*.

**The debug-build overflow panic was not the evidence ADR 0046 said it was.** That ADR called
`tiny-skia`'s "attempt to multiply with overflow" in `wide/u16x16_t.rs` "the sharpest evidence
available" that the defect was an intermediate wrapping. With the three modes no longer
reaching the library at all, the same panic still fires — from `lowp::overlay`, and `Overlay`
agrees with Vello *to the channel* in release. The lanes are meant to wrap; that is what the
SIMD instruction they stand in for does, and Rust's checked arithmetic firing inside a
dependency whose arithmetic is modular says nothing about whether the answer is right. What
settled these three was the closed form, and only the closed form. **Being right for the wrong
reason is worse than being wrong** — the handover has had that sentence since ADR 0042, and
this is the same shape one ADR later.

**`ClipColor` is only defined where `Lum(C)` is itself in range.** Fed a colour of luminosity
−0.39 the clause's own arithmetic returns −3.55 for a component. Nothing can do that, because
`ClipColor` is reached only through `SetLum`, whose second argument is a `Lum` of a colour
already in `0..=1` — but the constraint is a property of the *clause's structure* rather than
of the function, so it is written on the function and the test that found it uses only colours
`SetLum` can produce.

## Cost

Measured with callgrind on `bitmap-template1-customat.pdf`, the one corpus document that
selects all four modes: **408 869 470 instructions before, 408 869 830 after** — 360 more, or
0.00009%. The layer path is one band-sized pixmap and one pass over it per non-separable
command; every other page pays one enum check per command, which is inside the noise of that
measurement.

## Consequences

- §11.3.5.3's ledger row goes from `partial` to `implemented`, and §11.3.6's from "the formula
  is tiny-skia's and Vello's" to that being true of the separable half only.
- The tree's references to "Table 136's four non-separable modes" were wrong in three places:
  Table 135 is *Standard non-separable blend modes* and Table 136 is *Variables used in the
  source shape and opacity formulas*. Fixed. The table-title checker prints every table this
  tree cites and would have shown it to anyone reading the output — which is what that check
  was built for (`doc/PLAN.md` §5a) and is a reminder that a gate nobody reads the output of
  is half a gate.
- **§11.3.5.3's formulas cannot be quoted.** They are *images* in the specification PDF, so
  `doc/md/` holds `<!-- formula-not-decoded -->` where each one belongs and the quotation
  checker rejects any blockquote claiming to be one — correctly. Table 135 survives, because it
  is a table. So the arithmetic is transliterated in prose and said to be, which is what
  `CLAUDE.md`'s rule about quotation marks requires and what its first draft here got wrong.
