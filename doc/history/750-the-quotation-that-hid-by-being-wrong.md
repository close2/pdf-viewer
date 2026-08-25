# 750 — The quotation that hid by being wrong

The successor selection rule's fourth use. Its head was the plateau the third use left standing, and
following the rename at that head reached two quotations no instrument in this project could see —
one of a sentence an erratum struck, one of words ISO 32000-2 has never printed.

Date: 2026-08-25.
ADR: [0660](../adr/0660-the-quotation-that-hid-by-being-wrong.md).

Touched: `crates/pdf-syntax/src/tree.rs` (the module comment, one doc comment, one new test),
`crates/pdf-model/src/named_page.rs`, `crates/pdf-model/src/attachment.rs`,
`crates/pdf-model/src/view.rs`, `crates/pdf-model/src/form.rs`,
`crates/pdf-model/tests/saving.rs`, `crates/viewer-core/src/command.rs`,
`crates/viewer-core/tests/headless.rs`, `crates/viewer-host/src/panel.rs`,
`crates/viewer-host/tests/host_mappings.rs`, `crates/viewer-gtk/src/controls.rs`,
`crates/viewer-gtk/src/host.rs`, `crates/viewer-qt/src/bridge.rs`,
`doc/conformance/ledger.toml` (§7.7.2, §7.7.4, §7.9.6, §7.9.7, §12.7.5.4), `doc/errata-read.md`,
`doc/todo/01`, `doc/habits.md`, the ADR and this file. **No pixel moves and no behaviour moves**:
every code change is a comment, and the one addition is a test of behaviour that was already there.

## What the rule gave

`spec-errata emit` over `doc/ISO_32000-2_sponsored_EC3.pdf`, the issue numbers this tree names by
`doc/todo/01`'s two greps unioned, the attribution to the nearest live ledger row — the recipe run
rather than read. 307 issue numbers carry a strike or a caret and **130 are named nowhere**.

**The head is the plateau 746 left behind.** §12.10.2 is off the ranking, which is the decay working
for the second use running; §7.7.4 and §14.8.5.3 are back at the top with seven annotations apiece,
which are the two rows 746's tie-break ranked *below* the row it took. §7.7.4 wins the rerun of that
same tie-break — #672 changes a requirement level in a cell, §14.8.5.3's four carets swap *version*
for *level* in a referenced specification's name.

## What the issues said

`doc/errata-read.md` has all five with the rectangle that places each, taken from the annotation's
own `/Rect` against `pdftotext -bbox`.

- **#672 deprecates the catalogue's whole Web Capture surface with one hand.** Three bare `Caret`s,
  each on the `1.3)` ending an `(Optional; PDF 1.3)` cell, each writing *; deprecated in PDF 2.0*:
  Table 32's `/IDS` and `/URLS`, and Table 29's `/SpiderInfo` one clause up. §7.7.4's row had the
  first two as *owed to a feature*; a deprecated entry is a different silence, and it is the reason
  that row already gives `/AlternatePresentations`.
- **#214 is the standard's rename and not a table's.** One `Text` note on page 10 — *all occurrences
  of the term "name string" are replaced by just "string" throughout ISO 32000-2:2020* — with ten
  illustrative strikes on Table 32 and two more, four clauses along, on §7.9.6. The term is not one
  §7.9.2 defines: that clause states a text string, a PDFDocEncoded string and a byte string, so the
  erratum withdraws a type ISO 32000-2 never had.
- **#307 came with them and was on nobody's list.** Two `Caret`s with nothing struck beneath them,
  adding *Keys shall not be the null object.* to §7.9.6's Table 36 `/Names` row and to §7.9.7's
  Table 37 `/Nums` row.

## What reading them made this round look at

**A quotation of §7.9.6 for words ISO 32000-2 prints nowhere.** Following #214 to §7.9.6 — where it
strikes *lexically* and *in lexical order* — found `pdf_syntax::tree::name_pairs` citing that clause
for the phrase **"by unsigned character code"**. It is in no clause, no annex and none of the
technical specifications in `doc/md/`. Three instruments could each have caught it and none was
placed to: `spec-errata check` matches a quotation against text an erratum *struck*, and nothing is
struck under an invented phrase; `--bin quotations` reads `doc/` and `ledger.toml` rather than
`crates/`; and the conformance gate verifies rustdoc **blockquotes**, where this was a quotation
inside a sentence of prose.

**A third copy of the sentence Issue #481 struck, hidden by being misquoted.**
`viewer_core::command` put *the tree shall map name strings to file specifications* in quotation
marks against §7.11.4.1. The clause opens that sentence with *the associated name tree*, so the
quoted words were never the standard's — **and because they were not, `check`'s comparison matched
nothing**. `pdf_model::attachment` was corrected for that erratum in the four-hundred-and-eighteenth
session and `viewer_host::panel` in the four-hundred-and-twenty-ninth, each because the tool landed
on their *accurate* quotation. This one outlived both by being wrong. `doc/habits.md`'s *Reading the
specification* carries the general lesson.

**And the rename's real landing is a sentence no annotation touches.** #214's scope is stated in
prose, so an instrument built on strikes sees the illustration and never the rule. Six places in
this tree quoted §12.7.5.4's *the name string is the second of the two array elements* — **one of
them a rustdoc blockquote in the population the conformance gate checks** — and two more quoted
§7.11.4.1's NOTE. All are prose now, naming the erratum; nothing behaves differently, because the
term the erratum withdraws was never a type.

**#307 is a requirement and now has a test.** It is a writer's `shall not`, so what a reader owes is
a defined behaviour on a file that breaks it — and the part that matters is not that a null key
answers nothing but that **every pair after it still holds the value the file put beside it**. Both
walks chunk the pairs array in twos, so a null in a key position costs its own pair;
`a_null_key_yields_nothing_and_leaves_its_neighbours_paired` asserts both trees, since the erratum
states the sentence once per clause. Calibrated per trap 13, two plants, both removed: dropping the
null *element* before pairing, which re-pairs the remainder and loses the key after the fault; and
admitting a null key as an empty key. The fixture gives every key a different value so that a shift
by one cannot pass, and it is hand-built and says so (trap 8) — a file that states one is, by this
erratum's own words, a file that should not exist.

## What this use found about the instrument, which is three of four

- **A round that reads an issue without recording it in `doc/errata-read.md` leaves it at the head.**
  746 read #214 and #672 far enough to break its tie and wrote both as bolded bare numbers —
  `**#214**` — into its ADR and into `doc/todo/01`, a form *neither* of step 2's greps can see. That
  is 739's finding in a new costume, and a third grep is not the repair, because the bare-number
  search collides with `doc/HAYRO_ISSUES.md`. The repair is the rule now in `doc/todo/01`: an
  erratum read to a verdict is recorded in `doc/errata-read.md`, and one read only far enough to
  rank a row is deliberately left in the population.
- **The ranking drops `implemented` rows, and the true head is inside them.** #307 landed on two
  `implemented` rows and the rule could not see either. Ranked at this base with `implemented`
  admitted, **§9.6.4 carries 11 unread annotations and §7.4.1 carries 8**, both above the live
  head's seven. `doc/todo/01`'s recipe gains a fourth step: rank twice, and say which list the row
  came from.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this **is** a fifth round, so §2's sequence ran whole and §5 rebuilt and
installed the binaries — which `round.sh` had flagged as absent from `target/` — before any of it.
Both workers were built before any gate that decodes an image (trap 10).

`fmt`, `clippy -D warnings`, `nextest` (2657 tests, 18 skipped), the doctests, the fuzz `check`, the
sandbox worker, corpus, `pdfref-hayro`, oracle, text extraction, selection, accessibility, dates,
XMP, JPEG 2000, quorra, `fixed_documents` and `cargo test -p conformance` all green, the last of them
re-run after the final document edit. The only clippy output was `viewer-qt`'s cold-build gcc
`-Wmaybe-uninitialized` lines, which §2 documents as not lints. **The oracle says no pixel moved**,
which is what a round whose whole diff is comments and one test should look like.

`cargo test -p conformance` failed once and the failure was this round's:
`every_quotation_is_the_standards_own_words` attributed the module comment's new blockquote to
§14.7.5.4, the last clause the comment had named before it. The blockquote's own sentence names
§7.9.6 now.

**The machine was loud and the sequence was split around it.** The one-minute load average passed 65
on 24 cores while three sibling rounds ran, so the lines that spawn a reference renderer — the
oracle, the text extraction and quorra — were held until it fell below 12 and the rest ran first.

Thirteen sweeps run before the edits and after them, with the three errata commands beside them.
`quoted` and `unpriced` were not run: this round touches no page-list note and both take the
oracle's log as their right-hand side.

`entries`, `unread`, `callers` and `overstated` are **byte-identical**; `blockers`, `capabilities`,
`inapplicable` and `spec-errata moved` differ only in line numbers and citation counts this round's
insertions shifted. **Not one defect bucket moved**, and one instrument count fell for the right
reason:

- `owed` **182 unnamed terms over 113 rows, unchanged** — but it went to 186 first, and the four were
  this round's own prose rather than a debt: two occurrences of *Capture* under §7.7.4, which no
  source carries and which that row already names as *web capture*, and `/IDS` and `/URLS` written
  into §7.7.2's note where they are §7.7.4's entries and not that row's. Both sentences say the same
  thing in the row's own vocabulary now, which is `doc/todo/01`'s rule about writing a claim in the
  form its sweep reads rather than a rewording to dodge it. `unread` came back byte-identical with
  them, from 43 confirmed keys to **41**.
- `counts` 8068 ← 8032 sentences with 425 ← 415 attributed counts, **149 the family agrees with, 58
  "no such way" and 4 places counting one family twice, all three unchanged**; `quotations` 6296 ←
  6279 document quotations over 968 ← 967 documents with **diverging unchanged at 38**, and 1944 ←
  1943 ledger quotations with **diverging unchanged at 2**; `tables` 6634 ← 6610 sentences and 2468
  ← 2457 key citations with **absent unchanged at 100, contradicted denials at 6 and keyless at 58**;
  `pointers` 8348 ← 8329 with **absent unchanged at 131** and 137 symbol pointers with **13
  undefined**, both unchanged; `overtaken` 572 ← 571 decision records with **48 overtaken
  unchanged**; `inapplicable` unchanged at 55 / 233 / 224.
- `spec-errata check` is **unchanged at 105 quotations of struck text, blockquote 6, document 73,
  ledger 17, prose 9** — the same breakdown, the same rows, only line numbers moved. Its comparison
  population fell 1710 → **1706**, which is the four quotations this round turned into prose, and the
  four were not of struck text: **the misquotation this round's ADR is about was never in that
  population at all.**
- `spec-errata applied` grew to 733 ← 699 places naming an erratum over 56 407 ← 56 352 places read,
  with **the read-first list unchanged at 10, the corrections quoting retired wording at 90 and the
  quotations of struck text at 172**.
