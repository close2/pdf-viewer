# 2816 — A form's limits and commits hold in every control, and an FDF file's options, pages and scripts are applied

Session 2000, the audit-fix round (group FM).
Status: **accepted and built** in `viewer-gtk` (`controls.rs`: `limits_the_line`, `limits_the_buffer`, `selects`,
`commits_on_selection`), `viewer-qt` (`cpp/window.cpp`'s multiline control; `host/forms.rs`: `choose_control`,
`Placement::commit_on_selection`), `pdf-model` (`view/fields.rs`: `set_field`, `import`, `import_options`;
`view/pages.rs`: `append_templates`, `layered_page`; `view/script_sites.rs`: `run_fdf_scripts`; `forms_data.rs`:
`FdfScripts`, `read_scripts`, `Import::options`).
It answers the owner's specification audit (`doc/reviews/1983-specification-audit.md`, lines 1217, 1229-1236, 1263,
1280, 1318) and supersedes, each for the one point named below, ADRs 0548, 0090, 0091, 0092, 1661 and 2707.
Clauses: ISO 32000-2 §12.7.5.3 (Table 232), §12.7.5.4 (Table 233 bit 27), §12.7.8.3.1 (Table 248),
§12.7.8.3.2 (Table 249), §12.7.8.3.3 (Table 251).

## 1. `/MaxLen` binds every text control, and zero is a limit

Table 232: "The maximum length of the field's text, in characters." No flag of Table 231 exempts a multiline or a
password field, and zero is a length like any other. ADR 0548's observation that the appearance enforces the limit
did not make a native control enforce it, and the GTK and Qt hosts capped only the single-line entry: a GTK
`TextView` and `PasswordEntry` and a Qt `QPlainTextEdit` accepted any length, and GTK reads a `max-length` of zero as
"unlimited". **Every text control now enforces the limit at insertion**: GTK's `GtkText` delegate takes the limit
(and refuses every insertion where it is zero), a `TextBuffer` insertion is cut to the prefix that fits, and Qt's
multiline control takes back the characters past the limit before the field hears of the change.

## 2. A choice waits for its commit, and bit 27 commits it at once

Table 233 bit 27 (`CommitOnSelChange`): "If set, the new value shall be committed as soon as a selection is made …
If clear, the new value is not committed until the user exits the field." The commit is the moment that "triggers
any actions associated with the entry or changing of this data" — the field's `/V` validation among them. The model
treated a selection as whole when made, and so ran no commit at all, and the Qt host dropped the flag. ADR 2707's
premise that a host reports a click-commit was unproven for an AcroForm choice. **A selection is now held
uncommitted exactly as typed text is** (`ViewState::set_field`), committed by `Command::CommitField` when the field is
left, and **at once where bit 27 is set**: the Qt and GTK hosts send the commit after the selection, and GTK's
drop-down and list commit when they lose the keyboard. A refused validation puts the earlier selection back.

## 3. An FDF field's `/Opt` replaces the target's

§12.7.8.3.2: "importing a field causes the values of the entries in the FDF field dictionary to replace those of the
corresponding entries in the field with the same fully qualified name in the target document", and Table 249's
`/Opt` is "\[a]n array of options that shall be presented to the user". The reader parsed `/Opt` and dropped it
before the import state, so ADR 1661's statement that imported options are drawn was not true. **The import now
states the options as the same rewrite of Table 234's `/Opt` a script's `setItems` makes** (`Property::Options`), so
one route makes the host's control list them, the appearance draw them, an index name one of them and a save write
them. A Table 249 element is an option, or an option and a default appearance string, so each becomes an option whose
label is also its export value. A field that is not a choice field takes no `/Opt`.

## 4. One FDF page dictionary is one page, whatever its template count

Table 251's `/Templates` are "the named pages that serve as templates on the page" — one page. ADR 0091 appended a
page per template. **The templates of one FDF page this document holds now make one page.** The clause states no
arrangement, so the arrangement is a documented choice that adds no geometry of its own: the page is the first
template's (boxes, `/Rotate`, group), and each template, in the array's order, is painted on it as a form of its own
content and resources in its own default user space, unmoved and unscaled; the page's `/Annots` are the templates'.
Every mark is the producer's; the only operators this program writes paint those forms in place, and the page is a
display of the import that a save does not write (ADR 2154). A single template stays the page it names.

**Left:** templates of one FDF page that Table 253's `/F` puts in another file still arrive as one page each when a
host supplies the file (ADR 1239), because they are asked for one at a time.

## 5. An FDF file's scripts run in Table 248's order

Table 248 states four moments: `/Doc`'s scripts "shall be added to those already defined in the name dictionary and
shall then be executed before the script defined in the Before entry", `/Before` "just before the FDF file is
imported", `/After` "just after", and `/AfterPermsReady` "after the FDF file is imported and the usage rights in the
PDF document have been determined". ADRs 0090 and 0092 declined the whole dictionary under the script exclusion as it
stood then; ADR 1904 and A286 drew that exclusion at this document and this viewer, which is exactly what these
scripts reach. **`ViewState::import` now runs them**, where a host supplied a runner, as document-level scripts
(`ScriptSite::Library`, labelled by the `/Doc` name or the entry's key) over the document's whole field table, under
the open sequence's budget. §12.8.2.3's usage rights are determined by the time a document is open, so
`/AfterPermsReady` follows `/After`. Without a runner the import says how many went unrun.

## 6. An FDF field with no `/V` empties the field, by choice

The audit found ADR 0090's rationale defective rather than its behaviour: Table 249 makes `/V` optional, and
§12.7.8.3.2's "importing a field causes the values of the entries in the FDF field dictionary to replace those of
the corresponding entries" speaks of entries the FDF field states, so the standard establishes neither removal
nor keeping. Principle 5's rule for a silence is a documented choice, and the choice is kept, for a reason the
standard does supply: §12.7.6.2's submit-form action, under `IncludeNoValueFields`, writes a field with no value
into an FDF file "by name only, with no associated value" — so an FDF field with no `/V` is how the standard's own
export says a field is empty, and an import that kept the target's value would not restore what was exported. The comment at the write
(`view/writing.rs`) now calls it a choice and cites this section. A flag-only FDF file that means to leave values
standing has no way to say so under this choice; that is its cost.

## Not decided here

The model's `set_field` still takes a text value longer than `/MaxLen` whole: cutting it there makes the audit's
own positive control in `xfa_saved_values_audit` (and `xfa_datasets`) fail, since `annotation-text-widget.pdf`'s
`Tekstveld2` states `/MaxLen 10` and those tests expect the fourteen characters set through the model to be saved.
Section 1 binds the controls a person types into; whether a programmatic value is cut too is the owner's.
