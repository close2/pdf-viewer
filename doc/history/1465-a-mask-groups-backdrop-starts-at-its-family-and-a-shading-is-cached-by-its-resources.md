# 1465 — A mask group's backdrop starts at its family, and a shading is cached by its resources

Slot 4 of batch seventy-three, 2026-10-08, a clause round. ADRs 1764 and 1765; no row moved, no question.

**Coverage on `implemented` rows**, since every `partial` leaf waits on the owner (Q308, Q271, Q348,
A66's trigger, a policy syntax no signature names): a found defect, and the 33 rows under `8.7.`/`8.9.`.

**Premise.** Held in the code and the counts: `space.initial_colour()` at `soft_mask.rs` 550,
`initial_colour_of` at `colour.rs` 3327, 33 rows and 24 claim sentences (`scratchpad/r1465/claims.py`).
It did not hold in one name: the `/BC` sentence is §11.6.5.1's row (Table 142), not §11.6.5.2's,
which says nothing of a backdrop — §11.6.5.1's note is the one corrected.

**The backdrop (ADR 1764).** `backdrop_values` now defaults to `initial_colour_of` of the `/CS` the
group states, so a `/DeviceCMYK` luminosity group under a CMYK intent is backed by `[0 0 0 1]`.
Fixture `a_cmyk_mask_groups_default_backdrop_is_the_familys_black_under_an_intent`: 230 against the
old line's 0, equal to `/BC [0 0 0 1]`. `doc/todo/23`'s row is closed.

**The audit (ADR 1765).** All 24 claims hold at a named line. `scratchpad/r1465/names.py` found
§8.7.4.4's `transferred_corners` (no such function, in a sentence putting §10.5's transfer inside the
mesh conversion, false since ADRs 1266 and 1279, as is §8.7.4.3's `/Background` twin) and §8.9.5.1's
`parse_with_output_intent` (`image::colour_space` calls `parse_under`). Corrected in the notes.

**The defect it found.** `shading::Cache` keyed a build without the resources' `/ColorSpace`, so one
shading painted from the page and from a form with a `/DefaultRGB` took the first painting's colours —
for `[/DeviceRGB]`, a reference to `/DeviceRGB`, and a `Separation` over it (§8.6.5.6 remaps all
three). The key holds that entry, interned once, and the build is handed it alone; a named space is
cached too. `shadings.rs`'s fixture: all three red where green is owed with the old key planted.

**Left.** §10.5's note says a shading under a transfer is not cached and a named space neither;
both halves are false and it is not this round's row.

**Gates.** `rustfmt --check --edition 2024` on `soft_mask.rs`, `shading.rs`, `tests/shadings.rs`:
exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-colour --all-targets` and `-p pdf-model
--all-targets`: exit 0 (the second once slot 2's `view/scripts.rs` compiled). `cargo nextest run -p
pdf-colour`: 130 passed; `-p pdf-model`: 2012 passed, 19 skipped. `cargo test -p conformance
--no-fail-fast`: 425 passed, 1 failed — `records.rs` on slot 3's record mid-write (`batch.rs` failed
earlier on slot 5's edit). Tier 2 under the lock: `raster_golden` 2 passed, held 974, moved 0, exit 0;
`pdf-model --test corpus` 1 passed, exit 0; the six arms of HEAD plus this patch, built in its own
`CARGO_TARGET_DIR` from an export of HEAD, against `/home/AI/arms-1462/`: exit 0 each, 0 of 5 795 page
digests moved (968, 968, 968, 964, 963, 964), 921 s queued and 2 187 s held.
