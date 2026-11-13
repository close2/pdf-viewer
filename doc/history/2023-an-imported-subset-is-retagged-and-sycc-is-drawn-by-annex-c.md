# 2023 — An imported subset is retagged against the file, and sYCC is drawn by PIMA 7667's Annex C

Session 2023, a single round, the ledger lane, on `doc/todo/65` section 6's two buildable rows. ADRs 2862 and
2863. No Q851.

**The slice.** §9.9.2's one residue (appearances imported whole from an FDF keep their producer's subset names
unchecked), then §7.4.9's sYCC (drawn by the codec, owed by Annex C, ADR 2793).

**The premise.** Both held, and effort was the only obstacle. Section 6's row still said the archive's writers
did not share one record; ADR 2838 had already done that. The code showed two more things. `SubsetNames` knew a
Type 0 font by its `CMap`-suffixed `/BaseFont` alone. And `retag` wrote that suffixed name onto the `CIDFont` and
the descriptor, which left the compact program's Name INDEX holding the old name against Table 117.

**Built, §9.9.2.** `pdf_font::subset_tag::retagged` writes a new tag over the old one in place: in a compact
font's Name INDEX, a Type 1 clear-text `/FontName`, an sfnt's `name` records (checksums restated), and an OpenType
`CFF ` table. A subset is now known by every tagged name it states. `retag` moves each name's six letters and each
descriptor's program. `SubsetNames::name_imported` names every font held directly under a `/Font` resource of an
imported `/AP`, for a field (§12.7.8.3.2) and for an annotation (§12.7.8.3.4), through the save's one record.

**Built, §7.4.9.** `ExtendedEncoding::SrgbYcc` reads C7 and C8, then shares B9 onward. 18 is drawn through it
wherever its three components share one unsigned depth. A writer gets the codec's RGB when every pixel lies
inside sRGB's gamut (C.2's "exact correspondence"), and is refused by name otherwise. The margin is the half-code
rounding worked through C8 and B9. Two older expectations moved, each re-derived by hand. (255, 128, 96) has
`Y` = 1.077, past section 4.3.4's white, so ADR 2292's `L*` held at 100 draws (203, 255, 248). The precedence
test's grey is C8's (151, 149, 151). `G′` there is 0.494864, not 0.494873.

**Plants.** `name_imported` returning its input fails the three imported tests. A compact program left
unretagged fails `a_compact_fonts_name_index_takes_the_new_tag`. Routing 18 back to the codec fails
`sycc_is_drawn_…` and the writer test. Restored, all pass.

**Rows.** §9.9.2 and §9.9 `partial` → `implemented`. §7.4.9 `partial` → `departed` on CIEJab alone (A348,
ADR 2793's decision 2). §7.4 `partial` → `implemented`.

**Gates.** `cargo fmt --check` and clippy `-D warnings --all-targets` on `pdf-font`, `pdf-colour`, `pdf-model`: exit 0.
nextest of the three: 3238 of 3239, the one failure the precedence test above, re-derived; its binary then 19 of 19,
and the plants' restored run 39 of 39. rustdoc `-D warnings --document-private-items` of the three: exit 0. `cd fuzz &&
cargo check --bins`: exit 0. Conformance after `git add -N`: exit 0, once this paragraph was written. Tier 2 at the
safedocs-restored cache: `corpus` (974 documents, 61 incomplete) and `raster_golden` (23 moved) both exit 101, and
both name exactly round 2021's documents, so no page moved. `state.sh docs`: exit 0.
