# 1813 — The owner's drawing is profiled, and three wastes are cut

Session 1488. Status: **accepted** and **built**. The project owner asked for this on 2026-10-10,
about `/tmp/demo_heavy_engineering_A1.pdf`: "Obviously, if we find a way to improve loading, we
should improve this as well." Context: ADRs 0137–0139, 0341, 1082, 1102, 1521, 1792, 1808, 1809.
Code: `crates/pdf-model/src/content/colour.rs` (`Stated`, `set_colour_space`),
`crates/render-cpu/src/lib.rs` (`SolidOutline`, `draw_rule_as_bands`, `draw_stroked_outline`),
`crates/pdf-model/src/content/reader.rs` (`ContentReader::may_open_a_string`). Instrument:
`crates/pdf-model/examples/callgrind_rasterise.rs` (`--scale=`). Test:
`crates/pdf-model/tests/colour_paths.rs`
(`a_profile_selected_again_starts_at_its_initial_colour_and_reads_its_operands_as_before`).

## 1. Where the time went

The page is one `FlateDecode` stream of 278 579 415 bytes: 4 400 060 groups of `q 0.12 0 0 0.12 0 0
cm x y m x y' l S Q`, each a one-unit stroke under `1 J` at about half a device pixel wide, and
1 288 210 `/Cs1 CS … SC` pairs naming `[/ICCBased 5 0 R]` written in place in the resource
dictionary. Under callgrind (`RAYON_NUM_THREADS=1`, release, 150 dpi with `--scale=2.0833333`, the
scale `quorra-transform render` draws at), the whole page was **217.6 G instructions: 66.7 G
interpreting (30.7 %) and 148.3 G rasterising (68.2 %)**. So the brief's hypothesis that the
interpretation dominates did not hold; the rasteriser holds two-thirds. The five largest costs,
inclusive and as shares of the page:

| | instructions | share | what |
|---|---|---|---|
| 1 | 54.5 G | 25.1 % | `render_cpu::scan::mask_fill`, ADR 1082's exact area of each outline |
| 2 | 39.3 G | 18.1 % | `tiny_skia::Path::stroke`, **twice per stroke** |
| 3 | 19.5 G | 9.0 % | `PixmapMut::fill_rect` through the coverage mask, a pipeline built per mark |
| 4 | 17.6 G | 8.1 % | `ColourSpace::parse_at`, the profile read again at every `CS` (13.4 G `icc::Profile::parse`) |
| 5 | 11.1 G | 5.1 % | the content lexer, `next_content_token` over 78 173 320 tokens |

Inflating the stream is 2.2 G (1.0 %), the fixed cost the brief expected. Stepping over strings with
no font in force (ADR 1521) was a further 5.4 G (2.5 %) on a page that states no string.

## 2. The decision: cut what is waste, and change no pixel

Three costs are work done twice or done for nothing, and each is removed by handing back the answer
already computed, so each draws the same pixels by construction rather than by tolerance:

- **The outline is built once.** `draw_rule_as_bands` strokes a sub-pixel rule to ask
  `sub_pixel_bands` whether it is rectangles; a round cap is an arc, so it declines, and
  `draw_stroked_outline` stroked the same path, under the same style, at the same resolution again.
  `SolidOutline` is that outline, built on first use and lent to both. The stroker is a pure
  function of the three, and a dashed path still strokes its own cut.
- **A profile written in place is remembered as one behind a reference is.** `Interpreter::icc_spaces`
  keyed only `/Cs1 7 0 R`; `/Cs1 [/ICCBased 5 0 R]` is as wholly its stream, so it is keyed by the
  stream. The two identities cannot collide: no object is both an array and a stream. §8.6.8's
  initial colour is still set on every selection, which the new test holds and a planted hit that
  skips it fails.
- **A string is looked for only where one can begin.** `may_open_a_string` reads the first byte past
  the white space; only `(`, `<`, `%` or the buffer's end go on to `step_over_a_string`, which would
  have declined everything else after building a lexer to find it out.

**Not taken, and why each is the next round's rather than this one's.** The exact-area converter
(row 1) and tiny-skia's per-call pipeline (row 3) are measured constructions whose arithmetic is
the pixel; a cut there is a new construction to argue, not a waste to remove. And **the page is
drawn serially**: wall time equals user time at four rayon threads, and by `pdf_render::strips`'
own rule a stroke marks every row of its device extent unsplittable, so the page frame's
`80 80 1700 1520 re S` alone leaves `plan_strips` no legal cut. That rule was written for curves a
strip edge re-parameterises and for tiny-skia's hairline (ADR 0139); ADR 1102 has since taken every
measurable stroke off the hairline. Whether a straight-edged outline is strip-invariant under
`render_cpu::area` is a question for its own ADR and `strip_parallelism`'s scenes. It is the
largest lever left: about three times on this page's raster.

## 3. What it moved

| | before | after |
|---|---|---|
| callgrind, whole page at 150 dpi | 217.6 G | 178.0 G (−18.2 %) without the string check |
| of which interpretation | 66.7 G | 45.8 G (−31 %) |
| of which rasterisation | 148.3 G | 129.6 G (−12.6 %) |
| `callgrind_interpret`, the string check alone | 48.0 G | 45.6 G (−5.0 %) |
| `quorra-transform render`, pinned, minimum of three | 15.37 s | 11.24 s (−26.9 %) |

The wall clock was taken with the fast cores pinned (`taskset -c` on the eight cores of the higher
`cpuinfo_max_freq`) at load average 3.8, the three arms interleaved in one sitting. The PNG of
every arm has one MD5, `30776f3ec709e269990b4ce4cfad76e4`. The figures beside each cut's code are
these.
