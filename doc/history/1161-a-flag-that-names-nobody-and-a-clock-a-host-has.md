# 1161 — A flag that names nobody, and a clock a host has

Date: 2026-09-21. Branch: `batch-1159-1164`, worktree `/home/AI/pdf-viewer-rounds`, shared with
five sibling rounds (1159, 1160, 1162–1164). ADRs: [1159](../adr/1159-a-flag-that-names-nobody.md),
[1160](../adr/1160-what-time-it-is-is-an-input-a-host-supplies.md).

## The two revisit notes, against the tree

**ADR 0121's note did not hold.** It argued that a filled field is saved with Table 224's
`/NeedAppearances` instead of the constructed appearance; `Update::write_appearance` has written
§12.7.4.3's stream since commit 513b128d and the ledger row has recorded it since. The note was
reading ADR 0121, which is a record and is never edited. What it found without meaning to:
`ViewState::save`'s doc comment still carried the retired reason as the current one, a paragraph
that survived the change made later the same day and 1671 commits after it. Deleted, not annotated.
**ADR 0196's note held.** `/M` was written nowhere, and its four host-supply precedents are there.
## What was built

1. **The flag names its field.** `Update` holds the owed widgets rather than a `bool`;
   `view::Written::unconstructed` names each by §12.7.4.2's qualified name and `viewer_core`'s save
   reports it. The condition is Table 224's own — this writer "has not provided appearance streams
   for all visible widget annotations" — trap 11's rule, and trap 5's third report on this path.
2. **Table 166's `/M`, from a host.** `Command::Clock(Option<pdf_syntax::Date>)` is the ninth
   host-supplied policy value; `ViewState::set_modification_time` holds it; a save stamps every
   annotation dictionary it writes, the widget of a field typed into included, which is where
   Table 166 puts it rather than on the ancestor §12.7.4.1 may keep the value on. Default absent,
   so every gate sees the bytes it saw before.
3. **The clock**, in the one place with one: `viewer_host::modification::now`, Hinnant's
   `civil_from_days` against the `days_from_civil` `pdf_syntax::Date` already orders by, stating UT
   because `std` offers no local offset. `viewer-ui` reads it immediately before a save.

## Calibrated, and left

Four tests in `crates/pdf-model/tests/saving.rs`, each reopening the file and asserting the
producer's bytes are a prefix. Three planted defects each failed the test that should (trap 13);
`form_two_pages.pdf` carries no `/M` of its own, which makes the absent case a control.

The GTK and Qt hosts send no clock, so they write no `/M` — the default, not a defect. `now` would
also fill the `None` `write_filings` passes for Table 45's `/ModDate`: a second decision about a
different table, not taken here.
