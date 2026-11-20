# 3340 — A check box the window draws nothing for is written drawing nothing

Session 2159. Status: accepted. Context: ISO 19005-2 and -4 sections 6.3.3 and 6.4.1; ISO 32000-2
§12.7.3 (Table 224), §12.7.5.2.3; ADRs 1257 (a toggle never constructed for), 2751 (a push button),
2858 (`stand-in-face`, the same site's other loss); R16; A681.

## The question

pdf.js `checkbox_no_appearance` is R16's one document under `as-if-printed`: two check boxes,
`cbOn` (`/V /Yes /AS /Yes`) and `cbOff`, no `/AP`, no `/MK`, and `/NeedAppearances true`. Printed at
the refusals: section 6.3.3's "naming them would be inventing the control's own vocabulary", and
section 6.4.1's fence, since no appearance could be built. The window draws nothing for either and
says of `cbOn` that §12.7.5.2.3 puts its on state in an `/AP` it does not have; a print carries
the same nothing.

## The decision

Where the configuration grants `field-appearances`, a check box's or radio button's widget with no
appearance dictionary and no Table 192 caption (`/MK` states none of `CA`, `RC`, `AC`) is given an
`/N` subdictionary of the states the file and the clause name — the on state its `/AS` states, and
`Off`, §12.7.5.2.3's — each one form `XObject` over its rectangle drawing nothing.

- **It invents no name and no mark.** The on state is the widget's own; `Off` is the clause's. No
  state is named for a widget whose `/AS` states none.
- **The loss is `field-appearances`, whose sentence is already this**: a field whose appearance its
  producer never wrote is blank in the archive, and a printed copy showed the same empty box. So
  the site's answer is `Authorised { field-appearances }` (with `stand-in-face` beside it where a
  stand-in was written too), both appearance sites list the loss among their per-document losses,
  and the report names each widget's states. `as-if-printed` and `as-shown` grant it; with it not
  granted, the widget is the flag's to keep, named under `unconstructed`, as ADR 1257 has it.
- **Then the flag may go**: every visible widget has an appearance, so Table 224's claim is true.

## Result

`checkbox_no_appearance` converts under both shipped profiles at 2b and 4, veraPDF 1.30.3 passing
each, page 1 at 72 dpi pixel-identical (white). Left in R16: a `FreeText` in the stand-in for a
`/DA` font (`poppler-395-0-fuzzed`), an appearance kept on an appended page of a tagged document,
`issue7446`'s annotation with no normal appearance for its appended page, and a 3D annotation at 4
— each `as-shown` only.
