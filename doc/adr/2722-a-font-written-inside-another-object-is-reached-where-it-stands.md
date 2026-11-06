# 2722 — A font written inside another object is reached where it stands

Session 1954, a single archive round.
Status: accepted and built. In `pdf-transform`: `archive/fonts.rs` (`descriptor_dict`,
`descriptor_entry`; `embedded_program` reads a descriptor dictionary), `archive/sites.rs`
(`cid_to_gid_maps`' `holders`, `mapped_cid_fonts`, and `direct_fonts_edited`, the walk
`typed_fonts` now shares), `archive/rewrite/values.rs` (`write_cid_to_gid_map`). In `pdf-archive`:
`direct_cid_fonts_stating_no_cid_to_gid_map` returns every holder, and `owes_a_cid_to_gid_map` is
public. Tests: `twin_codes::a_program_behind_a_descriptor_written_inside_its_font_is_restated`,
`twin_codes::a_cid_font_inside_a_type0_font_written_in_a_page_is_given_the_identity_map`.
Context: ADRs 2162 (a direct font written out as an object), 2432 (a `CIDFont` direct in its
`Type0` object's `/DescendantFonts`), 2608 (`/Type /Font` written into a holder), 2716, and
`doc/pdfa-blocking-issues.md` R10.

## The case

`hesr13613-sup-0001-Authormatrix` writes four `Type0` fonts directly in form XObjects' resources,
each with its `CIDFontType2` and that font's descriptor direct, and each descriptor naming a
`/FontFile2` stream object. Two rows refused it at both parts: `fonts/widths-agree-with-the-program`,
because `restate_metrics` reached a program only through a descriptor that is an object, and
`fonts/cid-to-gid-map-present`, because the entry was written only into a `CIDFont` that is an
object or one direct in a `Type0` that is an object.

## The reading

ISO 32000-2 §7.3.10: "Except where documented to the contrary, any object value may be a direct or
an indirect reference; the semantics are equivalent." Table 109 asks that `/FontDescriptor` be an
indirect reference, so a direct descriptor is a defect of the source, but not one that changes what
it names: the program is a stream, and §7.3.8.1 makes every stream an indirect object. So the
program is restated where it is, exactly as for a descriptor that is an object; only the copy that
a shared program needs (ADR 2640) writes a descriptor, and that keeps its refusal by name where
there is no descriptor object to copy.

`/CIDToGIDMap`'s absent entry said `Identity` wherever the `CIDFont` is written (ADR 2432's
argument, unchanged). The entry is now written into the dictionary the font stands in, whatever
that object is, by the same walk ADR 2608 types fonts with: through `/Font` resource dictionaries,
`/DescendantFonts` arrays and every direct dictionary and array, never through a reference. The
refusal stays where the holders do not account for exactly the findings the row reported at no
object, or a holder is not a dictionary.

**Not done**: a substitution into a descriptor written inside its font (`SimFang-variant`,
`XiaoBiaoSong`), which needs an object to add `/FontFile2` to, so ADR 2162's writing-out reaches
the descriptor. Both documents would still stop at R03 (`bound-names-are-valid-utf8`) and R11
under both profiles, so the lift frees neither.

## Result

`hesr13613` passes both rows under every profile and part. It now stops, under `as-if-printed`, at
the output's `graphics/only-operators-the-base-standard-defines`, which the R10 refusals hid;
under `as-shown` at R03 and R09 as before.
