# Round 2226 — erratum #77 is said in the verdict, and `check` reads the strike's page

A clause round in `lane3`, on the owner's A1043 and the errata leftovers of round 2225.

- **#77's note (ADR 3605).** `pdf_archive::errata::Note` and `SUPPLEMENT_DIRECTION`. A predicate
  states a note with `Findings::note`, and the judgement carries it as `Judgement::noted`, met or
  failed. `cid_system_info_agrees_with_the_cmap` notes every non-Identity `Type0` font whose
  supplements differ. `Report::render` prints the note under the failed row, or under `met, under
  a reading that is noted:`. `quorra-retrieve archive`'s JSON adds `notes` and `met_with_notes`.
  The converter's `Decided::noted` carries it beside `collection-restated`, in text and JSON. Tests:
  `the_supplement_direction_is_noted_wherever_it_decides_the_row` (both parts, above, below and
  equal), the `EUC-H` converter test, and `a_rows_notes_are_in_its_json`.
- **#226 (ADR 3606).** Corrected §14.7.6 does not rank two owners in one `/A` band. The later
  object keeps the attribute, as a stated choice: #289 gives the same precedence to class objects
  across owners. No behaviour moves. `two_owners_in_one_band_give_the_attribute_to_the_later_object`
  pins it in both orders, and the §14.7.6.1 ledger row names it.
- **Struck words.** Round 2225's seven are restated per ADR 3603: #33, #218, #358, #431 three
  times, and #759.
- **`check` (ADR 3607).** A note keeps the page's text beside its strike, and a quotation that
  puts other words around the passage is counted rather than listed. `MIN_WORDS` counts runs of
  two characters or more, and digits. Other-clause lines: 1134 before. Now 327 are counted as
  repeated phrases, and 705 are listed. A Sonnet triage of the 705, a text-overlap heuristic,
  finds 70 groups real outside the records, 218 noise and 191 in records. `doc/errata-read.md`'s
  last section lists the 70 by erratum. The real ones, by erratum: #453, #384, #47/#48, #287, #162/#288, #42, #386,
  #481, #155, #128 and about fifteen more. They are owed to a reading.
- Ledger: no status moves. §14.7.6.1 gains a test and a sentence.

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 (`pdf-archive pdf-transform pdf-retrieve
pdf-model pdf-signature pdf-colour spec-errata viewer-qt`); `pdf-sandbox --bins` 0; nextest
`--lib --bins --tests` of the same set 0, 5695 of 5695, none in the manifest, which is empty;
rustdoc `-D warnings --document-private-items` 0; fuzz `cargo check --bins` 0 (public types
changed); `cargo test -p conformance` 0. No golden or corpus check: no pixel moves.
