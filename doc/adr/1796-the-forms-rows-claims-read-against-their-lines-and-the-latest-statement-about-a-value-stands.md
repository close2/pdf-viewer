# 1796 — The forms rows' claims, read against their lines, and the latest statement about a value stands

Session 1480. Status: **accepted** — an audit, with three defects fixed and fourteen sentences
corrected. Context: ADRs 1772 and 1784, which did the same for clauses 8, 9 and 12.5; ADR 1786.
Rows: the 30 `implemented` rows under `12.7`. Code: `crates/pdf-model/src/view.rs` (`reset_form`,
`reached_by_reset`, `forget_statements`, `import`, `write_resets`), `crates/pdf-model/src/submission.rs`
(`canonical_dates`, `date_picture`, `pdf_date`), `crates/pdf-model/src/aform/mod.rs`
(`Call::date_picture`), `crates/pdf-model/src/aform/date.rs` (`stated_date_fields`). Tests:
`crates/pdf-model/tests/value_statements.rs`, `crates/pdf-model/tests/canonical_dates.rs`; two tests
that pinned bit 10's refusal rewritten (`tests/submission.rs`, `viewer-core/tests/headless.rs`).

## 1. Why this is coverage work on `implemented` rows

Every `partial` leaf waits on an owner's answer (Q308, Q271, Q348, A66's trigger, a policy syntax no
signature names, §7.4.7's patch on the owner's fork). A false claim in an `implemented` row is a
false `implemented` (ADR 1754 §1).

## 2. The method and what it found

ADR 1784's: the notes split on `[[clause]]`, `\b(is|are) (read|applied|executed|drawn|honoured|obeyed)\b`
counted over the `implemented` rows whose clause starts `12.7` — 63 claims in 19 of the 30 rows — each
matched to the function that makes it true, every backticked name checked to exist and every cited
reader asked for a caller outside the tests (`scratchpad/r1480/claims.py`, `names.py`, `callers.py`).

61 claims hold at a named line. The two that do not are §12.7.8.3.2's "[b]oth halves of the sentence
are executed for the entries that are applied": *replace* did not hold after a person had typed (§3).
Beside the claims, fourteen sentences were false or stale, and three more were written: §12.7.5.4's
pointer to the keystroke, and the reset and the import as the later statement in §12.7.6.3 and
§12.7.8.3.2:

| row | sentence | what is true |
|---|---|---|
| 12.7.5.2.2 | "three of `/TP`'s seven codes" | all seven (ADR 1299) |
| 12.7.5.2.3 | "all three hosts build a real toolkit check button" | the two native hosts; `quorra` toggles the page's own appearance through `viewer_host::clicked` |
| 12.7.5.2.3 | a save regenerates no check box's `/AP`, "writing the new `/V` is the whole of the change" | `/AS` is written beside it (ADR 1453), and a box with no `/AP` has its constructed states written (ADR 1676) |
| 12.7.5.4 | `takes_typed_characters` "states the sentence once for all three hosts" | `quorra` asks it; the native hosts obey by choosing the widget from the same `editable` |
| 12.7.5.5 | "`FieldLock` is `FieldSelection`" | no `FieldLock` exists; `signature::field_locks` returns `FieldSelection` |
| 12.7.6, 12.7.6.2 | bit 10 reported, "which `CLAUDE.md` excludes" | applied (§5) |
| 12.7.6.3 | "nothing is written to the document" | a save writes the reset (§4) |
| 12.7.6.4 | XFDF `<annots>` "named on `FormsData::owed` rather than read" | read since ADR 1297 |
| 12.7.8 | `/Pages` and `/Annots` "named rather than applied"; XFDF "fills three of those four" | both applied; all four filled |
| 12.7.8.3.2 | "the row's debt is now one entry", "what is left … is one entry, `/RV`" | nothing is left; the row's own opening says so |
| 12.7.8.3.4 | "a replay clears and re-applies the import like any other log entry" | false, and unfixed (§6) |

The brief's hypothesis held for the sentences: they were reach claims and stale statuses. It did not
hold for the defects, which sat behind the verbs *performed* and *replace* in rows whose claims a regex
counts as true, because they are true of a document nobody has typed into.

## 3. Defect one: a reset or an import left what a person typed standing

`ViewState::annotation` reads a widget's four statements about its value in a fixed order — a
person's edit, an import, a reset, Table 226's `/V`. `set_field` removed the other three when it took
a value, so the latest statement stood. `reset_form` and `import` removed only each other's, so a field
typed into and then reset still drew what was typed, and the same for an import. §12.7.6.3 sets `/V`
to `/DV` whatever it was; §12.7.8.3.2's import makes the FDF's entries "replace those of the
corresponding entries in the field". **Each of the four is now the later statement and stands
alone**: `forget_statements` takes the other three away from every widget a reset or an import
reaches, with the field's typing in progress and §12.7.5.3's chosen file, which belong to the value
replaced. Three tests fail with the old two lines planted back; the control, typing after either, passes
both ways.

## 4. Defect two: a save did not write a reset

`ViewState::save` wrote edits, imports, additions and retypings, and no reset, so a file saved after one
opened showing the old `/V`. `write_resets` writes `/V` as the `/DV`, or removes it where there is none,
on the dictionary `holder` finds, with the appearance, `/AS`, `/RV` and `/M` a typed value gets, and
removes `/I`, which described the old value. **A field whose `/V` already is its `/DV` is not
written**: the reset changed nothing there, and a form reset whole would otherwise be saved as a change
to every field. `a_reset_that_changes_no_value_writes_nothing` fails with that guard planted out.

## 5. Table 240 bit 10 is applied

The bit: "any submitted field values representing dates shall be converted to the standard format
described in 7.9.4". Its NOTE 1 says where a date is stated — "only in the ECMAScript code that
processes it" — and the composition named the bit as owed because that code was excluded. Since
RFC 0008's Tier 0 this tree reads one-call library scripts, and the date functions are among them, so
the reason no longer held. A field whose `/F` script, or failing that `/K` script, is one call of
`AFDate_Format`, `AFDate_FormatEx`, `AFDate_Keystroke` or `AFDate_KeystrokeEx` holds a date. Its value
is read under the call's picture by `aform::parse_date`, which is the reading the field is displayed
with, and written as `D:` and the fields the picture states, from the year. §7.9.4 lets a date stop
early: "all other fields may be present but only if all of their preceding fields are also present".
The choices, each the clause's or written down:

- **No offset from UT.** The field states none, and inventing the machine's would be a fact this crate
  has no clock for. §7.9.4 reads that absence as "GMT", and the composition says so where the picture
  states a time.
- **Named on `owed` and sent as it is**: a value the picture does not read, a picture with no year
  (§7.9.4 requires one), a year outside four digits, and a field whose script is longer than one call,
  whose ECMAScript this tier does not interpret (trap 5).
- **A field with no script is not a date**, by NOTE 1's own test, and nothing is said about it.

The bit is restricted to no format, so FDF, XFDF and HTML Form format all carry the converted value.

## 6. What is left, and whose it is

**An undo after an FDF import loses the annotations it placed.** `viewer_core::Open::replay` clears the
additions and replays the edit log, and an import is not an entry of that log, so the placed
annotations go while the imported values, which `clear_all_fields` does not touch, stay.
`ViewState::clear_all_additions` cannot simply keep them: the allocation counter would not reset, and
the log names a person's annotations by number. The fix is a `Done::Import` entry in `viewer-core`,
which this round did not own.

**An FDF field stating no `/V` empties the field it names.**
`an_imported_field_with_no_value_empties_the_one_it_names` holds that reading. A field that states
only `/SetFf` therefore clears a value as well as setting a flag. The replacing sentence is about "the
entries in the FDF field dictionary", so whether an absent entry replaces anything is a reading a later
round should take back to §12.7.8.3.2 and §7.3.7.
