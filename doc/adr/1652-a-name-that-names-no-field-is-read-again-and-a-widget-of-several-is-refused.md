# 1652 — A name that names no field is read again, cut; one widget of several is refused by name; and three members of Adobe's library are carried

Status: accepted and **built**. Session 1408. Builds on RFC 0008 section 4.2 (`A193`), ADR 1591's
refusal rule, ADR 1603's realm table and ADR 1578's library; closes `doc/todo/56`'s Tier 1 column
classes 4 and 7 as far as each goes.
Code: `crates/pdf-script/src/engine/bridge.rs` (`get_field`, `spoken_name`, `widget_address`,
`constants`), `crates/pdf-script/src/engine/members.rs` (`library`, `exact_match`),
`crates/pdf-script/src/surface.rs` (`Field.style`).
Tests: `crates/pdf-script/tests/realm.rs`
(`a_name_that_matches_no_field_is_read_again_without_its_spaces_and_trailing_periods`,
`one_widget_of_a_field_is_refused_by_name_rather_than_answered_null`,
`the_glyph_styles_and_the_pointer_behaviours_are_the_reference_s_tables`,
`an_exact_match_is_of_the_whole_string_and_answers_a_position_from_one`).

## 1. The name, read twice

Adobe's "Doc methods" page says of `getField`'s `cName` only that it is the name of the field of
interest. ISO 32000-2 §12.7.4.2 says what a name is: partial names joined by PERIODs, and "a partial
name shall not contain a PERIOD character". The census (round 1402) found about 166 runs in nine
documents asking for a name the document holds with something more — trailing spaces
(`"NUM_013    "`, five copies of one form; `"ca8 "` in a calculation Acrobat generated from
simplified field notation) or a trailing period (`"auo."`, `"stpfl.kt.bank."`). Each answered
`null` and the next property read threw.

**The choice**: `getField` matches the name **as written** first — itself, then every field below
it. Only a name that matches nothing is read a second time, with the white space at either end and
the PERIODs after it removed, and a field found that way is answered and said in the run's notes
(the name asked, the name given, this ADR). Two arguments, one per character:

- A trailing PERIOD separates the name's last part from nothing. The clause rules a PERIOD out of a
  partial name, so the only field a name ending that way could name is a child stating an empty
  `/T` — which the exact reading finds first.
- White space *is* a character a partial name may hold, so it is not meaningless by the clause.
  It is cut only after the exact reading has failed, so a field the document names with its spaces
  is never reached by the cut; what the cut changes is a `null` that every census script then threw
  on. It is the cut Tier 0 already makes of `AFSimple_Calculate`'s list (`aform::Arguments::names`),
  so one spelling is read alike by the library and by a script.

Acrobat's own generated calculation is **evidence** that the producer's viewer answers these, and
nothing here is taken from it. What the choice costs: a script that tests `getField("x ") == null`
to ask whether a field *with* the space exists gets the field without it. No census script does.

## 2. One widget of several: refused, not `null`

The reference's "Field" page has `getField("name.N")` answer a `Field` of the Nth widget (from zero)
of a terminal field, whose widget properties are that widget's alone. The realm holds a field as its
first widget (ADR 1603's `FieldState`), so it cannot answer the second. Such a name — no field of
its own, a terminal field before its final PERIOD, digits after — is **refused by name** under ADR
1591 (`NotBridged`), because `null` would tell the script the field does not exist. Carrying it is
a `FieldState` per widget across the wire and a widget-scoped `ScriptEdit::Property`; the view
state already keeps a script's drawn properties per widget (`scripting.drawn`), so the second half
is in place. **One census run reaches the address, and it is the cost of the choice**: `5712688.pdf`'s
format script `TFTemplate_Format` asks for `"@@b12c96nfMM2_3m.0.0"`, the first widget of a template
field; it finished on the `null` before and is now refused, so that field's format is not applied
and the report says why. A `null` there was a plausible answer to a question this realm cannot
answer, which is the silent fallback `doc/traps` trap 5 is about.

## 3. `AFExactMatch`, `style`, `cursor`

- **`AFExactMatch(rePatterns, cString)`** is documented nowhere Adobe publishes: the whole of
  `adobe/dc-acrobat-sdk-docs` at `ab3b42a7` was searched and no page names it (the JavaScript
  reference, the Interapplication Communication guide's argument menus, the rest). So every rule is
  a choice from its name and its neighbours: *exact* is the whole string — the first text
  `String.prototype.match` finds is the string itself; a list answers the matching pattern's
  position counted from one, so 0 is the false answer; one pattern is a list of one and answers 1 or
  0. It is a native of the engine, not of `pdf_model::aform`, because its argument is a pattern and
  Tier 0 reads only literals.
- **`style`**: the "Field properties" page's table, six keywords `ch cr di ci st sq` for check,
  cross, diamond, circle, star and square, typed as a string; each constant is the style's name as
  the table writes it, a choice since the page states no values. `Field.style` itself is **refused
  by name** (`surface::NOT_BRIDGED`): it was simply absent, so `f.style = style.cr` — the census's
  one use, in `6942042.pdf`'s will-print script — would have set a property nothing reads, in
  silence. Writing it is a check box's glyph redrawn (Table 192's `/CA`), which is the appearance's.
- **`cursor`**: the "FullScreen properties" page's table, `hidden`, `delay` and `visible`, typed as
  a number; numbered 0, 1, 2 in the table's order, a choice for the same reason. `app.fs` is no
  member of RFC 0008 section 4.2's `app`.

## 4. Measured

The Tier 1 column over the census population, before and after this ADR and ADR 1653 together: threw
9 541 → 9 319 runs, finished 11 311 → 11 773, refused-and-finished 0 → 0, runs 20 860 → 21 100; the
column's ceiling and floor are ratcheted to the new figures. Of the classes, the trailing-name forms
(`PDFBOX-3094-*`, `4605262.pdf`, `PDFBOX-4399-2.pdf`, `PDFBOX-4496-0.pdf`, the `auo.` form) and
`AFExactMatch` (`evince-1314-0.pdf`) throw no more; `cursor`'s document now throws a `TypeError` on
`app.fs`, which the realm does not have, and `style`'s the `NotAllowedError` of `Field.style`.
