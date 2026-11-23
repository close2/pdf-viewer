# 3461 — A button whose appearance states name no stream is written drawing nothing

Session 2190. Status: **accepted** and built. Context: ISO 32000-2 §12.5.5, §12.5.2 (Table 166),
§12.7.5.2.3, §7.3.7; ISO 19005-2 and ISO 19005-4 section 6.3.3; ADRs 3340 (a toggle the window draws
nothing for), 3374 (a subdictionary that defines no stream), 2752; A48; R16 of
`doc/pdfa-blocking-issues.md`.

## The question

`issue15557` (pdf.js) was R16's one document that stopped under both shipped profiles at 2b and 4,
at `annotations/normal-appearance-shape` (ADR 3374 moved it there from a false "conforms"). Its
check box states `/AP << /N << /Yes /null /Off /null >> /D << /Yes /null /Off /null >> >>` — each
state the *name* `null` — no `/AS`, no `/V`, a caption `/MK << /CA (8) >>`, and the form asks for
`/NeedAppearances true`. The window draws nothing and says so: "/AP states appearances by state and
no /AS names one, which Table 166 requires there, so nothing is drawn".

## Reading

§12.5.5 defines the subdictionary by its values: "the subdictionary shall define multiple appearance
streams corresponding to different appearance states of the annotation". One whose every value is
something other than a stream defines no appearance for any state, so whichever state Table 166's
`/AS` selected a reader would draw nothing, and with none selected it draws nothing either. So
writing each state as a stream drawing nothing changes nothing this viewer, or any reader of the
file's own states, draws. What it costs is what ADR 3340's toggle costs: an appearance another reader
might have built — here from the caption, under the flag section 6.4.1 removes — is not built, which
is `field-appearances`' sentence: a field whose appearance its producer never wrote is blank in the
archive.

## Decision

1. `sites::appearance_states` takes a button widget (`Widget`, inherited `/FT /Btn`) whose `/N`
   subdictionary maps no state to a stream and names at least one (a null value is §7.3.7's absent
   entry and names none; a subdictionary of nulls stays with the rows about an appearance being
   present, ADR 3374), whatever its `/AS` says. One spare object becomes ADR 3340's form drawing
   nothing over the widget's rectangle, and `/N` is rewritten as a subdictionary of **the file's
   own state names**, each that form.
2. **No state is chosen.** The `/AS` is left as the file states it — here absent, which Table 166
   requires and which A48's "never fill in an absence" forbids this conversion to supply. Neither
   ISO 19005 part asks for `/AS`.
3. The answer is per document (`CONDITIONAL`): `Stated` where every collapse selects a producer's
   stream, the loss `field-appearances` where a button's states named none. The shape is
   `states-naming-no-stream`; `no-appearance-state` (a subdictionary *with* streams and no `/AS`)
   keeps its refusal.

## Result

`issue15557` converts under `as-if-printed` and `as-shown` at 2b and 4; veraPDF 1.30.3 passes all
four outputs and page 1 renders identical at 144 dpi. The fixture test
`a_buttons_states_naming_no_stream_are_each_written_drawing_nothing_as_field_appearances` holds the
refusal without the loss and the written shape with it, at both parts.
