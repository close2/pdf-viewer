# 760 — The operator category a row said we could not run

The errata selection rule's sixth use, and its second run with the fourth step in place. The full
ranking's head is §9.6.4, `implemented`, and the erratum that put a round on it states the permission
the row had spent seven hundred and fifty sessions denying.

Date: 2026-08-25.
ADR: [0681](../adr/0681-the-operator-category-a-row-said-we-could-not-run.md).

Touched: `crates/pdf-model/tests/type3.rs` (two tests and a helper), `crates/pdf-model/src/type3.rs`
(the module comment), `crates/pdf-model/src/content/text.rs` (one comment),
`doc/conformance/ledger.toml` (§9.6.3, §9.6.4), `doc/errata-read.md`, `doc/todo/01`, the ADR and this
file. **No pixel moves and no behaviour moves**: what the round removes is a false claim and what it
adds is the evidence that was missing.

## What the rule gave

307 issue numbers in `doc/ISO_32000-2_sponsored_EC3.pdf` carry a strike or a caret; **124 are named
nowhere in this tree** at this round's base. Over live rows the head is **§14.8.5.3 with seven** —
the same plateau 746 left and 750 and 755 both left standing, because neither took its row from that
list. Over **every** row the head is **§9.6.4 with eleven** under four issues, `implemented`, and
**§7.4.1 with eight** is second. 750 measured both from outside at exactly 11 and 8 before the fourth
step existed; reproducing the two figures is what said the arithmetic was right before it was
trusted, which is 755's practice repeated.

The population is 120 after this round — the four issues read, and no others, which was checked
rather than assumed: an early draft of the ADR wrote `Issue #224` and `Issue #357` in full and took
§14.8.5.3 off the ranking without a verdict. They are bare there now, which is 750's rule working on
the round that read it.

## What the issues said

`doc/errata-read.md` has all four with the rectangle that places each.

- **#111** strikes `operators` out of "In Type 3 fonts, glyphs shall be defined by streams of PDF
  graphics operators" and writes *objects*; inserts a NOTE 2 saying a Type 3 glyph description can
  use any operator from any category, subject to the clause's own restrictions; and inserts a
  paragraph requiring an implementation to avoid infinite recursion where a glyph description refers
  to itself.
- **#43** inserts *The number*, *The numbers*, *the numbers* six times ahead of Table 111's symbol
  glosses. Grammar, and the sixth family of `Caret` with no `StrikeOut` this file records.
- **#144** corrects the clause's EXAMPLE from `/LastChar 104` to `98`.
- **#553** adds Adobe Technical Note #5902 to clause 2 and a §9.6.3 caret about deriving an instance
  font's PostScript name. A writer's rule, filed under §9.6.4 by the page-straddle.

## What reading them made this round look at

**§9.6.4's row denied one of the categories NOTE 2 permits.** It said, from the tenth session:
a glyph description whose marks are an inline image "draws nothing yet and reports, which is §8.9.7's
gap rather than this one's: 10 corpus documents are in that position". `pdf_model::inline_image`
landed in the **eleventh** (ADR 0019). Measured: a `d0` description's inline image is drawn, placed
by the description's own `cm`, and nothing is reported. All three claims were false for all but one
of those sessions.

**Nothing in this project was placed to print it.** The sweeps compare a row against the *code*; this
sentence was wrong about a **sibling row's status**, in another clause family, under a status of
`implemented`. `--bin overstated` reads the inverse relation, `--bin blockers` reads a stated
blocker, and `spec-errata check` cannot see the erratum that leads to the clause because the strike
is one word.

**The restriction the permission defers to had no test either side of it.** Table 111's "the glyph
description shall not include an image; however, an image mask is acceptable" and §8.6.8's "unless
painting an image mask, all image painting operators shall be ignored" were implemented in
`content/image.rs` and asserted nowhere.
`an_inline_image_is_a_glyph_description_s_marks_like_any_other_operator` and
`a_d1_glyph_description_drops_an_image_and_keeps_an_image_mask` hold both, calibrated against three
plants: images dropped inside a description — which **no pre-existing test in the file could see** —
the font matrix lost, and the image-mask exception removed.

**Trap 13 sprang on the calibration, in its own words.** The transpose plant passed at first, because
the font matrix composed with the text rendering matrix is diagonal and a diagonal matrix agrees with
its own transpose. The fixture's description states `750 0 200 375 0 0 cm` now, whose shear makes the
placed matrix disagree with its transpose; the same plant then fails.

**And an erratum's *added* text cannot be a rustdoc blockquote.** `every_quotation_is_the_standards_own_words`
asks `doc/md/` for every blockquote under `crates/`, and an inserted sentence is in no clause of that
conversion — so quoting NOTE 2 as a blockquote failed the gate, correctly. `measurement.rs`'s
convention for Issue #534 is the answer: an erratum's replacement in *italics*, naming the issue.
`doc/errata-read.md` had that rule written down for *struck* text only.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
`tools/round.sh` says this **is** a fifth round, so §2's sequence ran whole and §5 rebuilt and
installed the seven binaries — which `round.sh` had flagged as absent from `target/`. Both workers
were built before any gate that decodes an image (trap 10).

`fmt`, `clippy -D warnings`, `nextest` (2678 tests, 18 skipped), the doctests, the fuzz `check`, the
sandbox worker, corpus, `pdfref-hayro`, oracle, text extraction, selection, accessibility, dates,
XMP, JPEG 2000, quorra, `fixed_documents` and `cargo test -p conformance` all green. The only clippy
output was `viewer-qt`'s cold-build gcc `-Wmaybe-uninitialized` lines, which §2 documents as not
lints. **The oracle says no pixel moved.**

The conformance gate failed once and the failure was this round's, and it is the finding recorded
above: two rustdoc blockquotes of an erratum's inserted NOTE. The machine's one-minute load was under
10 through the sequence, so no line that spawns a reference renderer was held.

Sixteen sweeps run before the edits and after them. The "before" run was taken by reverse-applying
this round's diff and moving the new ADR aside, since the round had started editing before it swept.
`quoted` and `unpriced` were not run: this round touches no page-list note and both take the oracle's
log as their right-hand side.

`entries`, `unread`, `blockers`, `capabilities`, `callers`, `overstated`, `owed` and `spec-errata
check` and `moved` are **byte-identical**. **Not one defect bucket moved:**

- `counts` 8237 ← 8199 sentences with 430 ← 427 attributed counts, **149 the family agrees with, 58
  "no such way" and 4 places counting one family twice, all three unchanged**; the three new counts
  are attributed to a clause with no rows below it, 223 ← 220.
- `quotations` 6388 ← 6368 document quotations over 988 ← 987 documents with **diverging unchanged at
  38**, and 1951 ← 1946 ledger quotations with **diverging unchanged at 2**.
- `tables` 6707 ← 6695 sentences with **key citations unchanged at 2477 — agreeing 2313, absent 100,
  contradicted denials 6, keyless 58**.
- `pointers` 8495 ← 8474 with **absent unchanged at 131** and symbol pointers unchanged at 140, with
  **13 undefined unchanged**.
- `inapplicable` unchanged in every bucket; two of its vocabulary counts gained one naming file each,
  `CS` and `Figure`, both this round's own words.
- `overtaken` 582 ← 581 decision records with **48 overtaken unchanged**.
- `spec-errata applied` grew to 772 ← 751 places naming an erratum over 57 158 ← 57 089 places read,
  with **the read-first list unchanged at 10, the corrections quoting retired wording at 90 and the
  places inside `errata-read.md` at 72**.
