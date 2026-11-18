# 2980 — A field property a document's script set is what the field states from then on

Session 2075. Status: **implemented.** Context: review 2038 (findings B67, B69), ADR 1603 (a script's `readonly`
over the file's), ADR 2840 (`/MaxLen` holds every route's value), `doc/questions/A834`. Clauses: ISO 32000-2
§12.7.4.1 Table 227, §12.7.5.3 Table 232, §12.6.4.16, §12.7.6.2.

ADR 1603 made a script's `readonly` stand over Table 227 bit 1 where a person's input arrives. The other readers of
the same facts kept the file's: `form::fields` described the stored bits 1 and 2, a submission's bit-2 diagnostic
read the stored flag, and `held_to_max_len` held a later value to the stored `/MaxLen` although a script's
`charLimit` had changed it and a save writes the new one. Each now reads the override first
(`ViewState::script_read_only`, `script_required`, `script_char_limit`) and the file's entry where no script set
one. The override is the document changing its own field, which is why it stands over the file for a user's input,
a script's value and an import alike.
