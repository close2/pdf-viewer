# 3021 — Two host policies the windows have moved past: when a chosen file is read, and the confined window's restriction control

Session 2082. Status: **accepted, not built.** Both changes are in the viewer crates, which another round holds
while this one runs. Restates ADR 1216's timing rule against the code. Applies ADR 1190's own rule to the operations
ADRs 1592 and 1987 gave the confined window. Context: review 2038 (findings D55, D59). Clauses: ISO 32000-2 §12.7.5.3
(`FileSelect`), §7.6.2 and §12.7.3 (the operations a document's restrictions name). Principle 3 also applies:
*a document's restrictions are the reader's to set*.

## 1. A file-select control's file is read once, when the person finishes with the field (D55)

ADR 1216 decided: "once, when the person finishes with the field. Not per keystroke — that would open a file sixty
times a second." ADR 1240 kept that. The code does not do it in two windows. `viewer_host::form::edit_of` reads the
path's contents (`policy::read_chosen`) whenever it is handed text for a file-select control. GTK hands it text from
the entry's `changed` signal through `FieldChange::Set`. Qt hands it text from `set_control` on every typed value.
Both happen before, and apart from, the commit callback. So each keystroke of a typed path opens a file, and each
prefix that names no file is refused out loud. `viewer-ui` reads at Escape, as the ADR says.

No clause times the read. §12.7.5.3 says only that the field's text "represents the pathname of a file whose
contents shall be submitted as the field's value". The rule is therefore this project's, and ADR 1216 stated it.
**Decision**: the rule stands and the code is brought to it. While a person types, a file-select control's text is
an ordinary `SetField` of the path. `edit_of` turns it into `ChooseFile` only at the control's commit: GTK's
`FieldChange::Commit`, Qt's `editingFinished` (ADR 1592), `viewer-ui`'s Escape. A chooser's answer is a commit in
itself. A test counts `read_chosen` calls for a typed path and a commit in each window, and expects one.

## 2. The confined window now owes the restriction control (D59)

ADR 1190's rule is "a host owes the control for an operation exactly when it can perform that operation". It
found that `quorra-confined` performed none of the restricted operations, so it owed no `--restrictions=` setting
and ran at `Off`. ADR 1190 also said what would arrive with each feature: "selection or copy brings `--restrictions=`
and the ask dialogue … anything that writes a file brings the save question". ADRs 1592 and 1987 gave the window
form filling, committing and saving. Its arguments still refuse a restriction setting ("this window sets no level
for a document's restrictions"), and its worker is sent no `Command::Restrict`.

**Decision**: by ADR 1190's own rule, the window owes `--restrictions=` for the operations it performs (form fill
and save, plus selection and copy where it has them), and the ask dialogue already lazily loaded since ADR 0718.
`Off` remains the default and the reader-favouring end, as principle 3 asks, so nothing is imposed on a reader
meanwhile. What is missing is the reader's ability to turn a restriction on in this window. No ISO clause requires
that, so this is the project's own principle and not a conformance defect. The protocol's `Command::Restrict` is
the route, so the worker answers as the other three windows' workers do.

## Consequences

D55 and D59 stay `open` in `fixes.md` with this ADR as the construction, for the viewer round that takes them.
§12.7.5.3's ledger row does not say when the file is read, and it stays as it is.
