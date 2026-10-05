# 1519 — A text turn spreads with the processor's clock, and a `/Matte`'s opacity is written in the inversion's pass

Status: accepted. Session 1342. Takes up the two items ADR 1513 section 4 and ADR 1481 section 6 left:
the text page's two turn figures, and the inversion and the multiplication on `issue13931.pdf` as one
pass. Amends ADR 1513 on one point: the text row's spread is the processor's, not the process's.
Supersedes nothing.
Context: ISO 32000-2 §11.6.5.2 and Table 144; `CLAUDE.md` principle 2 and the rule on optimisations;
ADRs 1420 (decline where a rounding moves), 1433, 1457, 1469, 1481, 1505, 1513; traps 50, 66, 94,
98; `doc/habits/measuring.md` 52–63.
Code: `crates/pdf-model/src/image.rs` (`Prematte::opacity`, `Prematte::of`, `Prematte::carried`,
`invert_matte_in_place`, `matte_through_unpack`, `samples_of_frame`, `SamplesOnGrid::mask_in_alpha`,
`EagerMask::InTheSamples`, `soft_masked`, `apply_soft_mask`, `matte_before_samples`);
`doc/checks/turn-path.toml` (the text row's reason, `issue13931.pdf`'s band).
Tests: `image.rs`'s `a_matte_inverted_with_its_opacity_is_the_inversion_then_the_multiplication`
(watched failing with the alpha byte off by one, trap 13); `raster_golden` held 974, moved 0.

**`crates/render-cpu/` was not opened** (`doc/questions/A76`).

## 1. The text page's turn: the clock, not the process

Every measurement below is ISO 32000-2 page 101 by the gate's own child (`turn_probe`), pinned to the
four Zen 5 cores and their siblings, on exports of the tree with their own target directories
(trap 50), with scratch instruments removed since.

- **The stage.** Of the turn, only `encode` moves (4.0 to 7.0 ms); `interp` and the rest hold. On one
  core, where the fan-out does not run, fourteen processes read 11.9 to 12.7 ms and one 13.9.
- **Not the process.** Thirty rounds inside one process wander 4.4 to 9.9 ms of encode, in runs of
  seconds; so do the processes. With address randomisation off (`setarch -R`, the same addresses in
  every process) eight processes spread as eight with it on, 7.7–8.4 against 7.7–9.1.
- **Not the threads' placement or the allocator.** A probe in `rasterise_all` logged each scope's
  threads, their cores, start, jobs and run-queue wait: five scopes a turn, ~3 000 jobs; two
  threads starting on one core does not track the slow rounds; page faults are 0–90 a scope; glibc's
  trim and mmap thresholds raised changed nothing. **What doubles is the work per job** — 5.6 to 11.9
  µs a glyph between rounds of the same process.
- **The clock.** Interleaved, quiet (load 0.4–1.8), eight processes each: as the gate runs, 7.6 to
  9.9 ms; with 30 ms of spinning on the pinned cores just before the turn, 7.4 to 8.5; with 1.5 s of
  idle before it, 8.8 to 10.8. The governor is `amd-pstate-epp` at `balance_performance`, the deepest
  idle state's exit latency 350 µs, and the fan-out's threads wake on cores that were idle while the
  walk's thread was busy — the one-core figure is steady because its core never idled.

So the spread is the machine's power management, and nothing in the tree's layout removes it:
pinning threads would land them on the same cold cores, and a spin before a frame is work done to
move a clock, which this project does not do. A real page turn comes after reading, which is the
idle case. The band stays spanning the spread, and `turn-path.toml` now says why.

## 2. `issue13931.pdf`'s matte in one pass, exactly

§11.6.5.2: "To derive c from c′ , the PDF processor may sometimes need to invert the formula shown
previously" — `c = m + (c′ − m) ÷ α` — and the soft mask then supplies the opacity. The tree inverts through a 65 536-entry table per component
(ADR 1457), writing each pixel's alpha as 255, and the multiplication writes `(a × α + 127) ÷ 255`
(ADR 1469). With `a = 255` that is `(255 × α + 127) ÷ 255`, which is `α` for every `α` from 0 to 255
because 127 is less than 255. So writing `α` in the inversion's own pass is one rounding where there
was one — none — and the bytes are the same. ADR 1420's decline does not arise.

It is taken only where the multiplication would read exactly that plane: the inversion's `α` is the
eager mask's own plane (`Arc::ptr_eq`, so the mask is on the raster's grid), and the frame covers the
grid (`invert_matte_in_place`). `apply_soft_mask` then multiplies nothing and still reports the mask
applied and its shortfall. A `/Matte` is never routed to the device scale (`soft_mask_entry`), and a
stencil never reaches the inversion, so neither path can meet an alpha already written.

Measured, interleaved process by process against the tree without it (exports, `md5sum`-distinct,
six each, pinned, load 2.3): turn 23.55 → 21.73 ms at the minimum, 24.3 → 22.4 at the median, all of
it in `interp` (18.9 → 17.1). Ten gate runs after: 21.55–22.38, re-banded 18.3 .. 26.9.

## 3. What is left

The text row's band spans the clock, and a regression of a third still fires. `issue13931.pdf`'s
interpretation is now its frame and the inversion, and the copy into the `Arc` an `Image` holds.
