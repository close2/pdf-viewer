# ADR 0498 — The setting the gate turned off

Status: accepted, 2026-08-22. Session 673. Moves the `quorra-gpu`/`quorra-scene` pin from
`cad50156` to `97ad95ac`; makes `render-quorra`'s corpus gate run at the shipped glyph quantum and
adds `PDFVIEWER_QUORRA_GLYPH_QUANTUM`; re-derives both sets of constants in `tests/real_pages.rs`;
retires a claim in `tests/corpus.rs`; amends `QUORRA_FEEDBACK.md` §31 and answers it in §37.
**133 corpus pages move onto the CPU oracle and no page regresses.**

## 1. What arrived, and why it is not an ordinary bump

`doc/QUORRA_GLYPH_PHASE_CARRY.md` is the quorra team's report, their ADR 0073. `GlyphPlacement::of`
split a placement's device translation into an integer origin and a fractional phase, and quantised
the phase with

```rust
let nx = (fx * fq).round() as u16 % q;
```

`(fx · q).round()` reaches `q` itself for any `fx ≥ 1 − 1/2q` — 3.1% of sub-pixel phases per axis
at the default quantum of 16 — and the `% q` sent that to bucket 0 of the **same** pixel while the
integer origin stayed put. Such a mark was rasterised at phase zero and seated at `floor(e)` where
the placement asked for `floor(e) + 1`: a whole device pixel, in x, in y or in both, on the lane
that draws text. The fix carries into the origin instead of wrapping.

**It is in the configuration this product ships.** `render_quorra::options()` leaves
`glyph_quantum` at quorra's default of `Some(16)`, so every page this viewer has ever drawn went
through that arithmetic — and no gate in this tree ran there.

Principle 5's rule for a sibling tree is the rule for a reference renderer: their report is
evidence about their measurement. Every claim below was re-run here.

## 2. What reproduced

One worktree, `Cargo.lock` flipped between the two revisions in the same copy, `tests/corpus.rs` at
the shipped quantum in both arms, RADV, page one of the corpus at scale 1. Load averages at the
start of each run are in the logs and were between 4 and 10 on a 24-core desktop with a parallel
round building beside it; nothing here is a timing claim.

| | agree | differ | refused | not comparable |
|---|---:|---:|---:|---:|
| `cad50156`, quantum `Some(16)` | 800 | 155 | 2 | 17 |
| `97ad95ac`, quantum `Some(16)` | **933** | **22** | 2 | 17 |
| `97ad95ac`, quantum `off` | 933 | 22 | 2 | 17 |

Their §2's table, to the page. **And their measurement stopped where they said it did**, which is
the reason `doc/todo/02` §2 asks a release round for the other lane and the magnified population:
they did not run those, and both are worth more than the column they did run.

| lane, scale, shipped quantum | `cad50156` | `97ad95ac` | pages moved |
|---|---|---|---:|
| scale 1, `cpu` — the gate's own line | 800 / 155 / 2 / 17 | **933 / 22 / 2 / 17** | 133 |
| scale 1, `gpu` | 800 / 155 / 2 / 17 | **933 / 22 / 2 / 17** | 133 |
| scale 4, `cpu` | 726 / 223 / 3 / 22 | **938 / 11 / 3 / 22** | **212** |
| scale 4, `gpu` | 730 / 219 / 3 / 22 | **939 / 10 / 3 / 22** | **209** |

Agree / differ / refused / not comparable. **No page regresses on any of the four** — `comm` over
each pair of sorted name lists, nothing only in the second — and no refusal moves. The magnified
population is where the defect was worst, which follows: four times a page's scale is four times as
many glyph placements and the same 3.1% of them per axis in the top bucket.

Three further claims were checked rather than inherited, because a total hides all three:

- **No page regresses.** `comm` over the two sorted name lists: the 22 are a strict subset of the
  155, and nothing is only in the second run.
- **`Some(16)` and `None` give the same verdicts after the fix.** The two 22-name lists are
  `diff`-identical. So their §4 is right, and the page-level cost this tree has attributed to the
  quantum's trade since the atlas landed was this defect.
- **Only one commit of the nine touches `src/`.** Read off `git log --stat` over
  `cad50156..97ad95ac` rather than off their §7: `37f1cc5` is `crates/quorra-gpu/src/atlas.rs` plus
  an example, a unit test and CI; `6d796da` and `6157000` are that crate's own tests and examples
  (their ADRs 0071 and 0072); `266747f`, `8c1edcc`, `b0dec96`, `f4065d8`, `3a96718` and `97ad95a`
  are `doc/`. No API change and no line of this tree needed adapting.

## 3. The decision: the gate runs the setting the product ships

`tests/corpus.rs` set `glyph_quantum: None` for the whole of its life, under a comment saying the
run "isolates the backend's fidelity from the deliberate sub-1/32-pixel trade `real_pages.rs` gates
separately". The reasoning is not silly, and it is how a whole-pixel error in the text lane lived
in the shipped configuration where the 974-page instrument could not reach it.

**So the default is the shipped quantum and the isolation run is the override**, not the other way
round. `PDFVIEWER_QUORRA_GLYPH_QUANTUM=off` — or a bucket count — is the fourth knob, alongside
`PDFVIEWER_QUORRA_SCALE`, `PDFVIEWER_QUORRA_COVERAGE` and `PDFVIEWER_QUORRA_ONLY`, and like the
first two it turns the ratchets off, for the same reason: a list measured at one setting held to
equality under another reports that setting's difference as a change in the backend.

Three things about the cost, because `doc/traps/instruments-and-reports.md`'s trap 11 asks a new
column what it costs and what it would have caught:

- **It is not a new column.** It is the same invocation at a different setting, so the sequence
  gains nothing to run.
- **It is the *faster* setting.** 26.2 s against 37.7 s for the quantum-off run of the same corpus
  on the same machine minutes apart — the atlas reuse is what the quantum exists for, and a gate
  that turned it off was paying for the isolation twice.
- **It would have caught this on the day the quantum landed.** At `cad50156` the gate reports 155
  differing pages against a ratchet holding 22 and fails on the *names*, which is what a gate
  should do with a whole-pixel error. The one gate that could see it at all was an envelope, and
  §4 is what the envelope did.

**The general rule outlives the instance, and it is the part worth keeping**: a gate that turns a
shipped setting off is measuring a configuration nobody runs. Isolation is worth having as a second
column and never as the only one.

## 4. The bound that had stopped catching things

`real_pages.rs::the_glyph_quantum_cost_stays_bounded` asserts a mean, a worst tile and an SSIM over
eight page/scale combinations. Its constants were 2.5 / 30.0 / 0.95 against a doc comment recording
a worst observed of mean 1.85 / worst tile 20.2 / ssim 0.9670.

Those were the defect's numbers, not the trade's. Measured at `97ad95ac`, the quantum's whole cost
on those eight cases is **mean +0.02, worst tile +0.07, ssim −0.00013** against the same pages with
the quantum off — which is what a bound of half a thirty-second of a pixel per axis should look
like. The envelope was between four and nine times what it was holding, and a 3% population of
whole-pixel misplacements moved it without breaking it: at `cad50156` the worst of the eight is
mean 0.6865, worst tile 20.10, ssim 0.99155, and all three sit inside 2.5 / 30.0 / 0.95.

Both sets of constants are re-derived from the run, and **forced both ways**:

| | worst of the eight | constant |
|---|---|---|
| fidelity, quantum off | mean 0.4989 / worst tile 3.15 / ssim 0.99870 | 0.75 / 4.5 / 0.9975 |
| the quantum's envelope | mean 0.5194 / worst tile 3.22 / ssim 0.99857 | 0.80 / 4.8 / 0.9970 |

At `cad50156` the new envelope breaks on **all eight** cases, at least two constants apiece; the old
one passed all eight. The fidelity gate is unmoved between the pins, as it must be — the carry is a
no-op with the quantum off — and it was tightened for the neighbouring reason: its own doc comment
claimed a worst case of mean 1.18 / worst tile 5.4 / ssim 0.9944, which had decayed to a third of
that without anybody re-reading it.

**The bound is close to the measurement, and that is only safe because it was measured twice.** The
whole file runs under `cargo test --workspace`, which on CI means lavapipe rather than this
machine's RADV. Both adapters were measured, with the test temporarily pinned to `llvmpipe`, and
the sixteen figures agree to the fourth decimal — so a margin of about 1.5× on the two error
measures is a margin against drift rather than against the adapter.

The gate also **prints** its comparisons now. It asserted three constants and printed nothing for
its whole life, so the round that found them oversized had to add a line before it could see it. A
bound is only re-derivable from what it is holding.

## 5. What went back to quorra, and where their report is wrong

`QUORRA_FEEDBACK.md` §37. Their §6 corrects our §31 in two bullets and they do not fare alike.

**Their first bullet is right and we were wrong.** `examples/lane_diff.rs` sets
`glyph_quantum: None` at the line they name, so §31's per-command offsets are not the quantum and
ADR 0073 does not touch them. The example's comment said "the gate's own options" — which was true
when written and is why the mistake was available to make. It now says what it holds and why, and
that it deliberately does not follow the gate.

**Their second bullet does not hold for §31's population.** They write that `take_gpu_lane` declines
the device lane for anything `worth_caching`, so on a default atlas the two coverage settings are
the same lane for a hairline. That is true of a solid fill and false of a stroke:
`Encoder::push_coverage_styled` passes `CacheProspect::TooLarge` at the call site — its own comment
says why, the atlas keys outlines and a stroke arrives as polylines — so `worth_caching()` is
`false` by construction for every stroke and declines nothing. All four of §31's pages draw
axis-aligned *rules*, and so does their own `lane_placement.rs` fixture. What kept their hairline
off the sampled grid is their third bullet, the triangle floor.

The pixels agree with the reading: re-run at `97ad95ac`, the two lanes differ on all four pages
(mean 2.5978, 1.5174, 1.1683, 0.2539), and §31.2's centroid table is per-rule. §31 keeps its
finding and gains the **converse** caveat, which is the one that is true: on a page whose marks are
cached glyph fills the two settings are one rasteriser under two names, so a page-wide lane
comparison averages marks the setting moved with marks it could not.

## 6. The fourth hiding place

Their §5 names three places the defect hid — no unit test on `GlyphPlacement::of`, every sweep
aliased with the quantum, and our 974-page instrument running with the setting off. There is a
fourth on this side, and it is the least comfortable of the four.

`examples/quantum_diff.rs` sweeps the quantum itself on a page of solid text and has printed the
defect on every run since the atlas landed: at `cad50156` it reports mean 0.6624 at `Some(16)`
against 0.3197 at `None`, twice the error for a setting whose stated cost is a thirty-second of a
pixel. At `97ad95ac` the same run reads 0.3356 against 0.3197. **The instrument worked, was run,
and was read** — and the number was filed under "the quantum's deliberate trade", which is a story
plausible enough to stop a number being evidence.

That is a shape worth naming beside trap 1's: not an instrument that could not see, and not one
nobody ran, but one whose output had an explanation waiting for it. The tell available at the time
was arithmetic — a factor of two where the setting's own stated bound is a thirty-second of a pixel
— and nobody did the division. ADR 0131 had even done it the other way round in `render-cpu`, where
a correctly-rounded 1/16 quantum contradicted the oracle on **no** page; the two measurements
disagreed for eleven months and neither was put beside the other.

## 7. What this does not decide

- **The quantum's value.** `render_quorra::options()` still takes quorra's default of `Some(16)`,
  which is ADR 0131's number and is now corroborated rather than merely inherited: at 1/16, with
  the carry fixed, the shipped setting costs no page its verdict on the 974-page gate.
- **§31's two questions.** The default lane's per-command offset and the sampled lane's y coverage
  are still open; quorra's §6 says their next round starts at the second.
- **Their §33.** `upload_outline`'s eager quadratics are ours to have asked and theirs to answer.
