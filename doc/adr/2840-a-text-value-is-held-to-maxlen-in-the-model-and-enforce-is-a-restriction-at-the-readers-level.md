# 2840 — A text value is held to `/MaxLen` in the model, and `/Enforce` is a restriction at the reader's level

Session 2011. Status: **accepted and built** in `pdf-model` (`view/fields.rs`: `held_to_max_len`, `set_field`,
`import_fields`; `view/pages.rs`; `view/scripts/calculation.rs`: `write_text`; `restriction.rs`:
`Operation::OverridePreference`, `Restriction::PrintScalingEnforced`), `viewer-core` (`RestrictionPolicy`,
`viewer/printing.rs`: `print_intent`, `grant_print`; `Event::Printing::scaling_held`; `Held::PrintScaling`), the
wire and ABI vocabularies (`viewer-confined`, `pdf-vfs`, `viewer-ffi`'s `QUORRA_RESTRICTED_ENFORCE`) and the
windows (`viewer-host::printing::Defaults::held_note`; `viewer-qt` locks its scaling control).
It carries out the owner's answer `doc/questions/A834`, items 1, 2 and 4, to round 2000's `Q834`, and supersedes
ADR 2816 section 1's last paragraph and ADR 1180's "reported rather than obeyed" for `/Enforce`.
Clauses: ISO 32000-2 §12.7.5.3 (Table 232), §12.2 (Tables 147 and 148), Annex I.2.

## 1. `/MaxLen` is the field's, so the model cuts every value to it

Table 232: `/MaxLen` is "The maximum length of the field's text, in characters." ADR 2816 made every native
control hold to it and left the model taking a longer value whole, because two audit tests expected fourteen
characters saved into `Tekstveld2`'s `/MaxLen 10`. The owner answered (A834 item 1): cut in the model too, and
revise those two expectations to ten characters. `held_to_max_len` is the one cut, applied where a value enters
the edit log — `set_field`, a script's `write_text` (calculation, keystroke and format rewrites), an FDF/XFDF
import and a template page's fields — to Table 226's `/FT /Tx` with an inherited `/MaxLen`, counting Unicode
scalar values of the §7.9.2.2 text string. A choice field's `/V` names an option and is never cut; a value that is
not a text string is left whole. The two audit tests' expected values are the one sanctioned edit to the owner's
audit tests, and only in that expectation.

## 2. `/Enforce` is `OverridePreference`, default `warn`

Table 147's `/Enforce` names preferences "that shall be enforced by PDF processors and that shall not be
overridden by subsequent selections in the application user interface"; Table 148 defines one name,
`/PrintScaling`. A document restricting its reader's choice is under `CLAUDE.md`'s four levels, asked once in
the host-supplied place, and A834 item 2 set its default at `warn` — the only entry of `RestrictionPolicy` whose
default is not `off`. The operation is the reader's choice in the dialogue, not printing, so no level stops a
print: `viewer_core` asks it last, at the grant, and `Event::Printing` carries `scaling_held` — `off` and `warn`
leave the scaling the reader's (`warn` raising `Event::Warned`), `on` holds it and raises `Event::Refused`, `ask`
holds the print on `Event::Asking` and either answer starts it, a `no` holding the scaling.

**What is left.** `quorra-qt` locks its scaling combo when held. GTK's `GtkPrintOperation` dialogue and a C
caller's own dialogue offer no lock, so there the hold is `Defaults::held_note`, said before the dialogue opens
on the document's scaling, and a reader can still choose another scale in it. That gap keeps §12.2's row
`partial`.

## 3. Annex I.2's first error stays declined

A834 item 4 confirms ADR 2815 section 2: every unrecognised feature is reported, never only the first.
`objects_not_understood.rs::every_object_not_understood_is_said_not_only_the_first` pins it.
