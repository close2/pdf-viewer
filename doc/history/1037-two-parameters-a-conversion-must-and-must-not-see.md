# 1037 — two parameters a conversion must see, and one it must not

Date: 2026-09-13. ADR 1054. Clause 11's `partial` rows in §11.4, §11.6 and §11.7, read against the
clause and not the note. Touched `colour.rs`, `image.rs`, `content/transparency.rs`, `soft_mask.rs`,
two test files, `examples/press_census.rs` and the ledger. Three rows moved.

**§11.6.5.2 — a soft-mask image is read against no resource dictionary.** ADR 1008 found in session
987 that `apply_soft_mask` handed the mask's own decode the resources in force, so §8.6.5.6's
`/DefaultGray` remapped its samples through a tone curve; it priced the fix and did not take it. The clause settles it in its own arithmetic: "[c]olour values in the original device
colour space shall be passed unchanged to the default colour space", so a remapping changes which
colour a value denotes and never the value — and §11.6.5.2 uses the value, Table 143 fixing the
entry at `DeviceGray` and stating no conversion from a sample to a mask value.
`image::mask_colour_space` is that reading, taken by all three of the mask's own routes;
`matte_colour` keeps the parent's resources, Table 144 stating `/Matte` in the *parent's* space. A
mask naming its space by a resource key — Table 143 forbids one — is now named unusable rather than
resolved against a dictionary the clause does not point at. Row: **four residues → three**.

**§11.7.5.3 — the intent at the `Do` selects a group's conversion out.** The clause's second bullet
asks for "the current rendering intent in effect at the time the Do operator is applied to the
group", and `colour::sample_press` took the whole grid through `A2B1`-else-`A2B0` with compensation
on, whatever the file said. The reason was structural: the press was cached on the profile alone,
so there was nowhere to put a second answer. `PressIdentity::Profile` now carries a
`crate::icc::Rendering` beside the identity, `Interpreter::group_press` reads it off `outer` — the
state at the `Do`, which §11.6.6 has not reset — and `xyz_to_ink` asks `Press::rendering` for the
black point so the two directions stay inverses by construction. The page group, a mask group and
the assumed inks each take `compensating()` for a reason the ADR names. Row: **two debts → one**.

**§11.4.1 — `partial` → `implemented`, by reading the clause.** The row was held for "[a]n isolated
group may specify its own blending colour space", a **permission**, in a list this clause opens
with "the group as a whole may have several further attributes"; §6.3.1's row already says a `may`
is not a debt. Its five `shall`s and two `shall determine`s are all executed, and the requirement
that a group *composite in* that space is §11.6.6's and §11.7.2's, both `partial` and both carrying
the remainder by name. §11.4.8 was left alone: it restates §11.4.4 and states nothing of its own.

Gates: tier 1 green but for clippy, whose three findings are all in a sibling's in-flight
`tests/silent_fonts.rs`; `nextest --workspace` 4595/4595. `raster_golden` holds **973 of 974**; the
one that moved is another round's font-error reclassification, confirmed by planting both changes
back and watching that document's reports stay identical — trap 37 answered, not a digest
regenerated. Both new tests were calibrated by planting their own defect: one fails of 18 in
`image_masks.rs`, one of 59 in `transparency_groups.rs`.
