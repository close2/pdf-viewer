# 1665 — A style redraws the check box glyph, and the full-screen preferences are refused by name

Status: accepted and **built**; section 3 superseded by ADR 1676, its found gap closed by ADR
1677. Session 1414. Builds on ADR 1652 section 3 (the `style` and `cursor`
constants), ADR 1617 (a script's properties drawn as the entries the standard draws a widget from)
and ADR 1591's refusal rule.
Code: `crates/pdf-model/src/view/script_model.rs` (`Glyph`, `Property::Style`),
`crates/pdf-model/src/appearance.rs` (`constructs_for_script`, `for_saving`, `caption_entry`,
`draws_dingbats`, `is_toggling`), `crates/pdf-model/src/annotation.rs` (`stored_set_aside`'s call),
`crates/pdf-model/src/view/scripts.rs` (`apply_property`), `crates/pdf-script/src/engine/bridge.rs`
(`FieldProperty::Style`, `style_of`), `crates/pdf-script/src/surface.rs` (`EXCLUDED`'s `app.fs` row),
`crates/pdf-script/src/wire.rs` (property tag 11).
Tests: `crates/pdf-script/tests/realm.rs` (`a_style_is_the_glyph_code_of_the_normal_caption`,
`the_full_screen_preferences_are_refused_by_name`), `crates/pdf-model/tests/script_properties.rs`
(`a_style_is_the_check_box_caption_and_its_states_are_owed`,
`a_style_in_another_font_is_reported_and_not_drawn`).

## 1. `Field.style` is Table 192's `/CA`

Adobe's *JavaScript for Acrobat API Reference*, "Field properties" (`adobe/dc-acrobat-sdk-docs` at
`ab3b42a7`), names six glyph styles of a check box or radio button and draws none of them. ISO
32000-2 says where such a glyph comes from: Table 192's `/CA` is the widget's normal caption and
"may be used with any type of button field, including check boxes … and radio buttons", drawn in the
font Table 228's `/DA` selects. Annex D's Table D.6 is `ZapfDingbats`' built-in encoding with each
glyph's picture. **The choice**: each style is the one-byte code Table D.6 gives the solid glyph of
that shape — check `a20` ✔ (octal 064), cross `a24` ✘ (070), diamond `a78` ◆ (165), circle `a71` ●
(154), star `a35` ★ (110), square `a73` ■ (156). Where the table has several of a shape (✓ ✔, ✕ ✖ ✘)
the solid one is taken, which is a choice too: neither source pairs a name with a code. A read
answers the style whose code the widget's `/CA` holds and `undefined` for any other caption.

## 2. Drawn: the widget is constructed, its on state the glyph

§12.7.5.2.3 keeps a check box's states as streams the value selects among, so ADR 1617 drew a
toggling button's scripted properties from its stored states. A style changes the mark the on state
*is*, so the stored stream no longer draws it: `constructs_for_script` now constructs a toggling
button whose script set its style — its on state the `/CA` glyph in the `/DA` font, its background
and border from `/MK`. Rendered at 6 px per unit and looked at (trap 1): a 20-unit box drew ✘, ★
and ● for `cross`, `star` and `circle`, each centred and auto-sized inside its border.

**In any other `/DA` font the code is a letter**, so `cross` in Helvetica would draw an `8`. The view
state checks the `/DA` font's `/BaseFont` (through `/DR`, or the name that denotes it where `/DR`
defines none) before it records the style; where it is not `ZapfDingbats` nothing is recorded and
the report says why.

## 3. Saved: the glyph is stated and the states are owed

One constructed stream would replace the appearance dictionary's states, which is the loss ADR 1617
avoided. So the saved file writes `/MK /CA` and leaves `/AP` alone, and `for_saving` answers `Owed`
for that widget: the save names the field in `Written::unconstructed` and sets Table 224's
`/NeedAppearances`, which asks the next reader to construct its appearances. The cost: a reader
that ignores the flag draws the producer's glyph. Writing the on state's stream inside the
appearance dictionary is the build that would remove it, and it is not this ADR's. One gap found on
the way and not changed here: `/NeedAppearances` is written only where `/AcroForm` is an indirect
object (`view::interactive_form`), so a form dictionary held directly in the catalog gets the
report and not the flag.

## 4. `app.fs` is refused by name

The reference's "FullScreen" page makes `app.fs` the interface to the application's full-screen
(presentation mode) preferences. RFC 0008 section 4.2 admits no member of it to `app`, and a
document asks for full-screen mode through Table 29's `/PageMode`, which the host reads. Until now
`app.fs` was absent, so `app.fs.cursor = cursor.hidden` threw a `TypeError` about `undefined`;
`surface::EXCLUDED` now carries it with that reason, and the script meets a `NotAllowedError` naming
`app.fs`, as RFC 0008 section 4.3 has every excluded call fail. It is a row of its own rather than a
member of the chrome row because its reason is not the chrome's.
