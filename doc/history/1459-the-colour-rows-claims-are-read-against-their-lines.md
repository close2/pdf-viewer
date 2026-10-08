# 1459 — The colour rows' claims are read against their lines, and a selected family keeps its start

Slot 4 of batch seventy-two, 2026-10-08, a clause round. ADRs 1754 and 1755; no row moved, no question.

**Coverage on `implemented` rows.** Every `partial` leaf waits on the owner (Q308, Q271, Q348, A66's
trigger, a policy syntax no signature names), so this round audits the thirteen `implemented` rows
`8.6.5`–`8.6.5.9`, `8.6.6.3`, `8.6.6.4`, `8.6.7` instead, as the brief says.

**Premise.** Held, with its count off by one: the notes carry 23 "is/are read, applied, executed"
sentences, not 22 (`scratchpad/r1459/claims.py`). ADR 1712's false claim is fixed at HEAD.

**The audit (ADR 1754).** All 23 claims hold at a named function and line. A name check over the
notes (`scratchpad/r1459/names.py`) found three stale names, corrected: §8.6.5.3's
`RgbRoute::from_xyz` (it is `components_with_xyz`), §8.6.5.9's `GraphicsState::black_point`
(`black_point_under`), and §8.6.6.3's "five routes", which named four — the JPEG 2000 route is two.

**The defects were §8.6.5.1's sentence** on `ColourSpace::initial_colour`, three ways. A `Lab` `/Range`
written backwards panicked `f32::clamp` on a `cs` alone; an `ICCBased` space's range was never
consulted, under a comment saying `/Range` "is not read"; and a device family under a `/Default`
space or an output intent took the stand-in's start, so `/DeviceCMYK cs` under a CMYK profile drew
the paper where §8.6.8 starts the family at `[0 0 0 1]`. One bound, `nearest`, now serves the
conversion and the start; `initial_colour_of` gives a selected family its own initial colour,
passed unchanged as §8.6.5.6 passes every value (ADR 1755 argues it against the clause's NOTE).
Fixtures: `an_initial_colour_is_held_to_the_range_however_it_is_written` (panics on the old arm),
`a_device_family_starts_at_its_own_initial_colour_whatever_stands_in_for_it` (paper 255 against
black), and `colour_paths.rs`'s `a_device_family_starts_at_its_own_black_whatever_stands_in_for_it`
through the interpreter (white against black); each fails with the old line planted.

**Found beside it.** `soft_mask.rs`'s `/BC` default takes the stand-in's start too (Table 142's "the
colour space's initial value, representing black"); not this round's file, named in `doc/todo/23`.

**Gates.** `rustfmt --check --edition 2024` on the three `.rs` files: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy -p pdf-colour --all-targets`: exit 0; `-p pdf-model --lib --test colour_paths`: exit 0
(`--all-targets` stops on `raster_golden.rs`, slot 6's file mid-edit). `cargo nextest run -p
pdf-colour`: 130 passed; `-p pdf-model`: 2005 passed, 19 skipped. `cargo test -p conformance
--no-fail-fast`: 424 passed, 1 failed — `records.rs` on sibling records mid-write (1457, then 1461),
and `bounded.rs` failed once while slot 5 edited `tools/bounded.sh`. Tier 2, each under the lock:
`raster_golden` 2 passed, exit 0; `pdf-model --test corpus` 1 passed, exit 0; the six corpus arms
against `/home/AI/arms-1456/`, exit 0 each, 0 of 5 795 page digests moved (`scratchpad/r1459/`).
