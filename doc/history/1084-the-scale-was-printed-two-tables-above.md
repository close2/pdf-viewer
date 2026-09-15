# 1084 — the scale a Lab profile needed was printed two tables above

Date: 2026-09-15. ADR 1098. Clause 8's `partial` rows, read against the clause and not the note. One
row moved by code; four read and kept, three with the residue corrected.

**§8.6.5.5 `partial` → `implemented`.** The row was `partial` for "`/Range` and the Lab class it
gates, which are one item", the block being that a Lab *device* value's scale onto an ICC table's
input is not the ICC's to state (ADR 1008). §8.6.5.5 states it twice and in an order, and the second
needs no file: the prose under Table 67 says the range "is a function of the colour space specified
by the profile and is indicated in the ICC specification", **Table 68 prints it** — "𝐿 ∗ : [0 100] ;
a ∗ and 𝑏 ∗ : [-128 127]" — and Table 65's `/Range` restates it from the file. So `Profile::range`
is the profile's own, `with_range` takes a document's restatement over it, and `Profile::encoded` is
the one place a device value crosses onto a table's unit interval — clipped to the range first,
which is §8.4.1's clip where the bound is known. `'Lab '` is admitted on that, its `mft` matrix
dropped (ISO 15076-1 section 10.10 wants a PCSXYZ *input*). **Table 88 is the image's side of the
same fact**, so `ColourSpace::component_range` answers with the profile's range.

Six tests over a hand-built `'Lab '` profile whose output depends on `L*` alone; expectations
derived rather than read off the raster — `L* = 50` is half of D50 white, linear sRGB 0.5, or 188 of
255, where the unit-interval reading gives white. Each was calibrated against a planted defect: the
range forced to `UNIT_RANGE`, `with_range` made a no-op, `encoded`'s clamp removed, the `'Lab '` arm
refused, `component_range`'s `Icc` arm deleted. **The population is zero** — 333 profiles in 1249
documents, none `'Lab '` (session 987), and four uncompressed `/N`-and-`/Range` dictionaries over
1477 files, all the identity — so the fixture is hand-built (trap 8).

**§8.7.4.5.7 and §8.7.4.5.8 kept, residue corrected.** Both said the fineness is not "deriv[ed] from
§10.7.3's smoothness tolerance", and §10.7.3's is a *colour* tolerance ADR 1058 already derives cell
by cell. What `PATCH_STEPS` fixes is geometry: a silhouette, whose tolerance is §10.7.2's and
"measured in device pixels", beside a colour error of the warp, which is scale-free. `pdf-model` has
no device pixels — `shading::Cache::build` takes the transform into the *display list's* space and
`TargetSpec::for_page` picks the raster's scale afterwards — which makes ADR 0919's "larger than a
round" architectural. **§8.10.4.1 kept** (a `may` mistaken for a debt? no) and **§8.6.6.5 kept**, one
case named: an `NChannel` space with no spot colourant needs no blending.

Siblings had `pdf-signature` and `render-cpu` mid-edit most of the round, so the gates ran in a
private worktree at the same HEAD with only this diff, mirrored back afterwards. fmt 0; clippy `-D
warnings` 0; `nextest --workspace` 4858/4858; doc 0; fuzz fmt and clippy 0; conformance 0 (a first
pass failed `every_quotation_is_the_standards_own_words` on an uncited blockquote). `raster_golden`
**held 974, moved 0**; `pdf-model --test corpus` 0, every ratchet at its ceiling; oracle 3/3. **Trap
1 paid by eye**: both fixtures at 8×, mid grey and white, nothing reported.
