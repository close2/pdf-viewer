# 3470 — The synthetic commit enters a selection-only field by its selection

Session 2192. Status: **accepted** and built. Builds on ADR 3453 (the ten fields `cca7b2660` left untaken), ADR
3311 (typed text into a combo box without Table 233's `Edit` flag is refused unless it is a label) and ADR 0248
(a `/V` naming an export value selects nothing). Code: `crates/pdf-model/tests/support/synthetic_commit.rs`, used by
both `script_corpus` walks.

## Context

Both walks commit every scripted field's own value again, so that its `/K`, `/V` and the `/CO` walk run. They
entered every field as `Entered::Text` of the text it shows. Since ADR 3311 a combo box without bit 19 refuses text
that is not one of its labels, so a choice field whose value is empty or names no option was no longer taken, and
its scripts no longer ran (the `untaken` column of ten held documents, ADR 3453).

## Decision

1. A choice field that takes only a selection — a list box, or a combo box with bit 19 clear, which "shall include
   only a drop-down list" (§12.7.5.4, Table 233) — is entered as `Entered::Chosen` of the options
   `pdf_model::form::fields` says its value selects: what a reader choosing in its list sends. A text field and an
   editable combo box are entered as text, as before.
2. Where such a field's value names no option, no choice in its list makes that value, and the field is counted
   untaken rather than entered as anything else. §12.7.5.4's `/V` is "a text string representing the selected
   item, as given in the field dictionary's Opt array", the name string of a pair; ADR 0248 reads a `/V` that
   holds an export value as selecting nothing, and an empty selection would clear the value, which is not
   committing it again. An empty value is the clause's "no item is currently selected", entered as the empty
   selection.

## What it moved

Over the held documents (`PDFVIEWER_SCRIPT_CORPUS_HELD=1`, ADR 3471), six lines of Tier 0's held file, each in the
`untaken` column alone and no digest: `FOP-2216-4`, `-5`, `-7`, `-8`, `-9` (one field each, empty) and pdf.js
`pdf.js-8048-0` (three of six). Of ADR 3453's ten, the five `FOP-2216` fields are taken and their scripts run;
`PDFBOX-923-0`, `-2`, `-3` (`NA`) and pdf.js `issue15815`, `issue17998` (`/V` an export value, `/I [0]`) stay
untaken under decision 2: the files contradict §12.7.5.4's parenthesis, and ADR 0248 reads them by the clause.

## Cost

Five of the ten fields' scripts still do not run under the walk. Reading an export-value `/V` as its option would
run them, and is a change to `form::selected`, which ADR 0248 decided against.
