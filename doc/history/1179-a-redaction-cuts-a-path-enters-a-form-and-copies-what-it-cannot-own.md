# 1179 — A redaction cuts a path, enters a form, and copies what it cannot own

§12.5.6.23's second phase requires a processor to "remove all content identified by the redaction
annotation" and to "remove all traces of the specified content". Three of the cases ADRs 1124 and
1126 refused are built.

**A painted path is cut** (ADR 1195). The region's four edge lines divide the plane into nine
cells; the middle one is the region and the other eight tile its complement with disjoint
interiors, so `P \ R` is the union of eight Sutherland–Hodgman clips against convex windows —
exact, orientation-preserving, taken subpath by subpath, so a nonzero and an even-odd fill both
survive it and the algorithm's degenerate boundary edges enclose no area either rule can see. The
surviving geometry is written back as fresh §8.5.2 operators, so the coordinates that described the
removed marks are gone rather than clipped. A surviving vertex is the producer's own pair, copied;
only a created one is arithmetic, and the page is **refused** unless the worst displacement this
writer's decimals and §7.3.3's single precision can together produce is provably smaller than the
hundredth of a point the region is widened by. Refused by name: a stroke (§8.5.3.2's marks are the path's outline), a Bézier segment, a
path that is also §8.5.4's clipping boundary, an interrupted path object, a singular transform.

**A form is entered** (ADR 1196), and always — the interpreter runs a form's content inline, so its
codes are in the placed-code count ADR 1124 calibrates the walk against, and the walk that skipped
forms **refused every page whose text was inside one**, a latent over-refusal nothing measured.
`text_inside_a_form_clear_of_the_region_does_not_refuse_the_page` holds it.

**A shared object is copied, not refused** (ADR 1196, amending ADR 1126 §2). An object another page
also draws carries marks the annotation did not identify, so the removal goes into a copy the
redacted page's own `/Resources` names and the original stays byte for byte. Ownership is not the
reference count alone: an object reached only through a shared form is that page's too, and the
first version of this got that wrong — planting it back fails
`a_form_nested_in_a_shared_form_is_copied_with_it`, which was written for it.

**The corpus cannot rank any of this.** Nine documents on this disk carry a `/Redact`, all veraPDF
and Isartor fixtures, five reaching a region; not one has a path, a form or an image under it, so
the verb applies nine of nine before and after. The census was calibrated rather than believed: a
planted fixture with a stroke under the region is named by the same command (trap 13), so the zero
is a fact about the corpus.

Gates: `rustfmt --check`, `clippy -D warnings`, `cargo test -p pdf-transform` (lib 62, redact 31,
the other seven targets), `cargo test -p conformance`, and tier 2's `--test gate` behind the lock,
all exit 0. `archive_corpus.rs`, `archive.rs` and `quorra-transform.rs` fail lint from a
neighbour's in-flight edit.
