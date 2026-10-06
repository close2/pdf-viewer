# 1579 — A field script that is one `AF*` call runs where `ViewState` commits or shows a value

Status: accepted and **built**. Session 1371. Builds on RFC 0008 sections 4.1, 6.5 and 11 item 2
(the owner's `A193`) and on ADR 1578, the library this dispatches to.
Context: ISO 32000-2 §12.6.3 (Table 199), §12.7.3 (Table 224's `/CO`), §12.6.4.17 (Table 221),
§12.7.4.3; `CLAUDE.md` principle 1's immutable `Document`.
Code: `crates/pdf-model/src/aform/call.rs`, `crates/pdf-model/src/aform/site.rs`,
`crates/pdf-model/src/view/scripts.rs` (new), `crates/pdf-model/src/view.rs` (`set_field`,
`clear_field`, `perform`, `import`, `AnnotationView::editing`), `crates/pdf-model/src/appearance.rs`
(`Formatting`, `field_text`, `regenerate`), `crates/pdf-model/src/variable_text.rs`
(`Owed::Script`). Gate: `crates/pdf-model/tests/script_corpus.rs`.

## 1. The one shape

A `/JS` runs without an engine where its text is **textually one call**: an `AF`-prefixed name of
the library, `(`, literals separated by commas, `)`, an optional `;`, white space anywhere between
tokens. A literal is an ECMAScript number (sign and hexadecimal admitted), a string in either quote
with ECMAScript's escapes, `true`/`false`, or an array literal of strings. Nothing else: no
identifier argument, no operator, no second statement, no comment — each is the first step of an
interpreter. A `/Next` after the action is not one call. An unknown `AF` name is reported by name.
Every other script is reported as **a script this tier does not run** — the old sentence, one tier
narrower — and drawn or kept as the file has it.

## 2. When each trigger runs

The standard names the four triggers and orders `/C` by `/CO`; it does not say when a value typed
character by character has *changed*. Adobe's "Form event processing" does: keystrokes, a keystroke
with `willCommit`, validate, calculate, format. So, over a host that hands a whole value per
keystroke:

- `ViewState::set_field` with text is **`/K` in its typing form**; a refusal takes nothing and
  returns zero ("may check the added text for validity and reject" it).
- `ViewState::commit_field` is the commit: `/K`'s commit form (which may rewrite the value), then
  `/V`. A refusal puts back what each widget showed before the typing began — a reset's `/DV`, an
  import, an earlier edit — and returns the sentence; Adobe's alert is the host's to show.
- **`/F` runs where the value is laid out**, `crate::appearance::field_text`, for a text or
  editable combo value that is not a password's echo. A field between `set_field` and
  `commit_field` is laid out as typed (`AnnotationView::editing`), so a host's caret is never
  beside characters it is not editing; carets, clicks, selections and glyph boxes always ask of the
  typed characters; **a save writes the formatted appearance** whatever the state, since a saved file
  is read by nobody typing into it. `/V` keeps the value, never the format.
  `ViewState::displayed_value` answers with the formatted text for a host or an assistive client.
- **Table 224's `/CO` is walked after every value change** — `set_field`, `clear_field`, a commit,
  a reset, an import — in order and once: the strongest reading of "when the value of any field
  changes", and a calculation is a function of the other values, so running it early shows the total
  the commit would. A calculated value is the document changing its own value, so Table 227's
  `ReadOnly` does not stop it; a field being typed into is not overwritten; an entry that is not an
  indirect reference names no field; the walk stops at 4 096 entries.

Re-entrancy (a `/C` changing a field earlier in the order) does not restart the walk. Reports are
kept once each in `ViewState::script_reports` (RFC 0008 section 6.8); a `/F` not run is the page
report's `Owed::Script` sentence on the widget.

## 3. What is not done here, and whose it is

**No host calls `commit_field` yet**: `viewer-core`'s `/Bl` and Enter are where it belongs, and the
three windows are another round's. Until then a field a person types into stays as typed in
`quorra`, its `/K` filters and `/CO` follows it, and its save is formatted. A list box's selection
keystroke, the open-time sites and every non-field site are Tier 1's.

## 4. Measured

`script_corpus` over RFC 0008 section 3's population (90 763 PDFs, 46 s, 1.26 GiB): 347 documents
carry a field script at a Table 199 site; 10 703 sites run under Tier 0 and 4 237 are reported as
not run; the synthetic commit leaves 1 292 values untaken (Table 227's `ReadOnly` first) and 21
commits refused. Under two minutes, so it is `tools/batch.sh gates`' `t2-script_corpus`.

**No page moved, and the expectation that pages would did not hold.** `raster_golden` held 974,
moved 0; `render-raster`'s corpus gate at 1× 968 / 0 / 0 / 6. Both draw page one of the tracked
corpus, and a format reaches a drawn page only where the appearance is constructed or regenerated —
no `/AP`, or Table 224's `/NeedAppearances`, or a value this program changed. The only page-one
widgets of that kind with a format and a value are `160F-2019.pdf`'s five, and each states `/F 6`:
Table 167's Hidden bit, so nothing is drawn either way. The drive of `quorra` under Xvfb is where a
format is seen (two lines typed, the total drawn `$19.50`, the saved file's `/V` `19.5` beside an
appearance showing `$19.50`).
