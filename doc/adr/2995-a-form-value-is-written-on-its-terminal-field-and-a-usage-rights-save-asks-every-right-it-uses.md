# 2995 — A field's value is written on its terminal field, and a usage-rights save asks every right it uses

Session 2078. Status: **implemented.** Amends ADR 1796 (where a reset is written) and ADR 0159 (which rights a
save checks). Context: review 2038 (findings B13, B40). Clauses: ISO 32000-2 §12.7.4.1, Table 226, §12.7.6.3,
§12.8.2.3, Table 258.

**Where the value goes.** `writing::holder` walked a widget's `/Parent` chain to the first dictionary stating a
`/V` or a `/FT`. Where two terminal fields inherit one `/V` from a nonterminal ancestor, a reset — or a typed value,
or an import — of one wrote the ancestor's `/V`, and the sibling's value changed with it on reopening. §12.7.4.1
gives a value to a *field*, and the first partial name (`/T`) on a widget's chain is the terminal field the widget
belongs to, so the walk now stops there as well. One case keeps the ancestor: a reset to no default of a field that
states no `/V` of its own, where removing the inherited entry is the only removal §12.7.6.3's "its V entry shall be
removed" can mean.

**Which rights a save asks.** §12.8.2.3: a processor that modifies a PDF "with a UR signature in excess of the
rights that are granted by that signature, should remove that signature prior to writing". ADR 0159 checked
`/Form /FillIn` and `/Document /FullSave`, the two operations a save then carried out. A save now also writes the
annotations a person added and the notes a person retyped, so it asks Table 258's `/Annots /Create` and
`/Annots /Modify` for each (`UsageRights::permits`) and withdraws `/UR3` where either is withheld. Embedded files
(`/EF`) are not asked: `UsageRights::permits` names no operation for them (`revision::RIGHTS_NOT_RECOGNISED` refuses them by name), and that remains the
recorded gap.

Tests: `pdf-model` `inherited_reset_defaults_audit` `reset_saves_each_terminal_fields_effective_default`,
`usage_rights_annotation_save_spec_audit`, `forms_data` `a_save_beyond_the_granted_usage_rights_withdraws_the_signature`.
