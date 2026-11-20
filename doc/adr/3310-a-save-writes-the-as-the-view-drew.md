# 3310 — A save writes the `/AS` the view drew

Session 2152. Status: accepted. Context: audit 2148-B03, ADR 0235, `radios_in_unison.rs`.

## Decision

§12.7.5.2.3 makes `/AS` decide which appearance a button shows ("the value of the AS key shall be
used instead of the V key"), so the `/AS` a save writes is what the next reader shows. The view
decides which buttons of a set are on in one place, `appearance::Field::is_on` — with
`RadiosInUnison` clear "at most one radio button in a field shall be set at a time" — and the save's
`Update::write_state` now asks it, with the same statement about the value the appearance was built
from (`FieldValue::Edited`, `Default` or `Imported`). Before, `write_state` tested only that the
widget's `/AP` `/N` held the state's name, so two buttons sharing a name were both saved on while the
view showed one.

The rule is one function read twice rather than two readings of one clause, which is what keeps the
saved file and the picture from disagreeing again.
