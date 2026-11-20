# Specification audit of ISO 32000-2 §8.4 and §8.5 (graphics state, paths) — round 2155

The project's sixth specification audit, made beside the other lanes in the shape of the owner's
review 2038 ([`2038-specification-audit.md`](2038-specification-audit.md)) and of audits 2128,
2131, 2136, 2142 and 2148 ([`2148-specification-audit-12-7.md`](2148-specification-audit-12-7.md)).
Baseline: main at `0b93d22f`. Evidence: [`2155-specification-audit-8-4-8-5/`](2155-specification-audit-8-4-8-5/)
— `findings.json` (2038's schema) and `expected-failures.json` (one failing test per behaviour
finding). The same entries are in `2038-specification-audit/expected-failures-current.json`.

**This is an audit and fixes nothing.** A fix round takes the findings by the groups at the end.

## Method

1. Every `shall` and `should` of §8.4.1 to §8.4.5 (Tables 51 to 57) and §8.5.1 to §8.5.4 (Tables
   58 to 60) was read in `doc/md/ISO_32000-2_sponsored_EC3.md`, lines 4346 to 4797.
2. 2038's findings on these clauses were read first and are not repeated: B39 (a degenerate
   subpath's round caps composited apart from the rest of a transparent stroke), B59 (stroke width
   under an unequal scale), D29, D58 and the T rows beside them — all fixed.
3. The interpreter was traced through `pdf-model`'s `content/run/graphics.rs` (the line
   parameters, `ri`, `i`, path construction and painting), `content/path.rs` (`end_path`, the
   pending clip, the trailing `m`), `content.rs` (`set_dash`, `apply_dash`, `line_cap`,
   `line_join`, `miter_limit`) and `content/ext_gstate.rs` (Table 57's line keys); the oracle
   through `render-cpu`'s `stroke.rs` and `convert.rs`, and `pdf-render`'s `Stroke::device_width`,
   `closing.rs`, `degenerate.rs` and `mitre.rs`. The GPU backends were read where the CPU differed
   from the clause, and not run.
4. The lens was the earlier audits' habits, with the reader on the way out being the rasteriser:
   what the interpreter states and what the backend draws must both agree with the clause. A
   measure stated *at device resolution* (§8.4.3.2's zero width) has that origin under every
   transform; a parameter has a type as well as a range (a dash array's elements).
5. Each candidate was probed end to end — content stream, interpreter, `render-cpu`, ink read back
   — and each B finding has a failing `*_spec_audit` test with a control that passes on main:
   three in `render-cpu/tests/all/stroke_parameters_spec_audit.rs` (display lists, ADR 3036) and
   one in `pdf-model/tests/graphics_state_paths_spec_audit.rs` (a module of `content_suite`,
   ADR 3086). No portable-suite test was added: the findings are about marks one device pixel wide
   or finer, and `portable-conformance/`'s probes stand five device pixels from any edge the clause
   fixes, which none of these has.

What holds, probed and passing on main, is not listed one by one. Among it: every row of Table 55
drawn through `d` (including `[2 1 3] -2`); a closed dashed subpath joined where it starts and ends
on a dash, and dashes dispensed along the closing segment; a dash under a CTM that scales x and y
differently, measured in user space on each axis; §8.4.3.5's ratio on both sides of `2 M`, and
mitres of ratio 57 and 229 drawn where the limit admits them; degenerate subpaths (`m h`, a
degenerate `c`) as dots under round caps and nothing under square; a leading zero-length segment
neither breaking a join nor turning a square cap upright; the tangent `v` implies at a cap; `re`
with a negative width or height winding the other way; a closed subpath whose last point repeats
its first joined, not capped; `W`/`W*` after the paint and `W n`; negative `w` clipped to zero; a
miter limit under one clipped to one; the later of `w` and `/LW` winning. One reading was checked
and left: §8.4.3.6's "If the end of a dashed segment coincides exactly with a join point, then the
end cap is painted before the corner" is read by `pdf-render/src/closing.rs` as two caps and no
join, and a mid-path dash finishing at a corner draws the same; the sentence bears that reading.

## Findings

Four behaviour gaps and one decision written as a derivation. In order of how much a person would
see:

- **2155-B01** — a zero-width line is one device pixel wide only along the direction the CTM
  stretches most. `Stroke::device_width` divides one pixel by `max_stretch`, so under `4 0 0 1 cm`
  a zero-width horizontal rule is a quarter of a pixel (0.25 ink a column) while a vertical one is
  a pixel. §8.4.3.2 states the zero width at device resolution; the orientation sentence the code
  cites is about a width stated in user space. All three backends take this width.
- **2155-B02** — a valid dash pattern finer than `tiny-skia`'s limit erases its stroke. Past a
  million dashes a path, `tiny-skia`'s dasher returns `None`, its stroker then returns `None`, and
  `render-cpu` draws nothing and says nothing: `[0.00001 0.00001] 0 d` on an 80-unit line inks 0,
  where `[0.01 0.01]` inks half the solid line. `convert.rs`'s comment says a refused dash "yields a
  solid line", which is true only of `StrokeDash::new`'s refusal.
- **2155-B03** — a zero-width line is mitred: the substituted pixel is handed to the stroker with
  the state's join and limit, and §8.4.3.5 says "When the line width is zero, the miter length is
  zero." A 14° corner grows a one-pixel line's spike (2.07 pixels of ink beyond the vertex, 0.06
  bevelled).
- **2155-B04** — a dash array with an element that is not a number is read as its remaining
  numbers: `[3 (x) 1] 0 d` and `/D [[3 (x) 1] 0]` both set `[3 1]`, where the `1` that was a dash is
  now a gap. `apply_dash` draws an array outside the clause's *range* solid; one outside its *type*
  becomes a pattern the file never stated.
- **2155-D01** — `line_cap` and `line_join` send a code outside 0 to 2 to the initial value, and the
  comment presents that as §8.4.1's answer; §8.4.1 says numeric parameters "shall be clipped into
  valid range" and names no initial value. A choice, written as a derivation.

## Ledger rows contradicted

All are `implemented` and stay so until the fix round, which re-reads each: `8.4.3.2` (B01, B03),
`8.4.3.5` (B03), `8.4.3.6` (B02, B04), `8.4.5` (B04), `8.5.3.2` (B02). D01 touches `8.4.1`.

## Suggested fix groups

1. **`pdf-render/src/paint.rs` with the three backends' stroke conversion** — B01 and B03: one
   decision beside `Stroke::device_width` for a zero (or §10.7.5-substituted) width: a device-space
   hairline of the transformed path, one pixel across in every direction and with no mitre, read
   by `render-cpu/src/convert.rs`, `render-raster/src/stroke.rs` and `render-gpu/src/scene.rs`.
   Trap 2's shape: the three must not answer it differently.
2. **`pdf-render` (a dash-count bound) with `render-cpu/src/convert.rs`** — B02: a pattern past a
   stated count drawn as the coverage it implies, decided once for the three backends, and a
   dasher's `None` never a stroke drawn as nothing; the comment at `convert.rs:270` corrected.
3. **`pdf-model/src/content.rs` and `content/ext_gstate.rs`** — B04: one reading of a dash array's
   elements shared by `set_dash` and `/D`; D01 in the same file: clip the code, or name the choice
   in an ADR.

One observation for a later round: `render-raster` and `render-gpu` dash through `kurbo`, which has
no dash-count bound, so B02's pattern there is four million dashes built and filled — a budget
question (principle 3) rather than a clause one, and not measured here.
