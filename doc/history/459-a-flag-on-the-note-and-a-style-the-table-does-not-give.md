# 459 — A flag on the note, and a style its table does not give

**Finding.** The four-hundred-and-fifty-eighth left §12.5.6 and §12.5.6.8 as the last two `partial`
rows whose notes predate commit 110, written in the same sitting as §12.5.4's two silent
departures, on the assumption that a wrong sentence arrives as a block. Both rows' own claims held.
**The block held anyway, one departure per clause and neither of them the row's subject.**

§12.5.6.4's first sentence — "when open, it shall display a popup window containing the text of the
note" — and Table 175's `/Open`, "[a] flag specifying whether the annotation shall initially be
displayed open", were **read nowhere in the tree**. `grep '"Open"'` over `crates/` finds one
reader, and what it reads is Table 186's identically named entry on the *popup*. A file saying its
sticky note starts open showed the icon, no window and no report. Two things kept it: a doc comment
above `appearance::text_icon` saying `/Open` was not read "on the ground that this program draws no
popup for any subtype", true until the three-hundred-and-twelfth session and false after it, and
**§12.5.6.4's ledger row asserting the opposite of the truth** — "the popup window /Open selects …
is drawn since the three-hundred-and-twelfth session", which is right about the window and silent
about the entry.

And §12.5.6.8: Table 180 gives a square or circle a `/BS` "specifying the line width and dash
pattern", and §12.5.4 says the same for four subtypes at once — "[s]uch dictionaries may also be
used to specify the width and dash pattern for the lines drawn by line, square, circle, and ink
annotations". `square_or_circle` obeyed that where it draws and then reported a `/S` of `B` or `I`
as an appearance it could not derive, naming a gap the clause does not have. Trap 11, and the
mirror of the previous round's second departure one sentence over.

**Date.** 2026-08-13.
**ADR.** [0294](../adr/0294-a-flag-on-the-note-and-a-style-the-table-does-not-give.md).
**Touched.** `crates/pdf-model/src/popup.rs` (`opens_with_the_page`, two tests),
`crates/pdf-model/src/appearance.rs` (`square_or_circle`, `Border::simulated`, `text_icon`'s
comment), `crates/pdf-model/tests/annotations.rs` (one test),
`crates/pdf-model/examples/open_annotation_census.rs` (new),
`doc/conformance/ledger.toml` (§12.5.4, §12.5.6, §12.5.6.4, §12.5.6.8, §12.5.6.14),
`doc/habits.md` (a sixth refusal shape), `doc/HANDOVER.md` (the sentence that counted five),
`doc/todo/01-ledger-partial-rows.md` (the blame list's last two, and the grep that finds the new
shape), `doc/adr/0294-*`, this file.

## What the round verified rather than assumed

- **The two rows it was sent to read were checked and held.** `STANDARD_SUBTYPES` carries all
  twenty-eight of Table 171's names; `appearance::construct` switches on the subtype only where
  §12.5.5 found no stream; `Border::inset` is §12.5.6.8's "inscribed within the annotation
  rectangle"; `differences` applies `/RD` in the clause's left, top, right, bottom order. The
  ledger now records that they were read rather than leaving the next round to re-derive it.
- **Each fixture was watched fail with the rule it guards removed** — the `/Open` disjunction, the
  `Text` subtype test, and the square-versus-link report pair — because an invocation that has not
  been seen to fail establishes nothing.
- **The condition was counted before it was believed.** `open_annotation_census` over the 964
  openable documents: 34 835 annotations, 28 of subtype `/Text`, **one** stating `/Open true`
  (`pr7352.pdf`), and that one's popup already states Table 186's own — so the corpus cannot rank
  this rule at all, and no annotation of any other subtype states an `/Open` no table gives it.
  `border_precedence_census` had already found no `B` and no `I` among the 33 781 annotations
  stating no `/AP`, so the second departure has no witness either. Both fixtures are therefore
  hand-built pairs (trap 8).
- **The other readers disagree, and the clause decides.** pdf.js reads `/Open` only in
  `PopupAnnotation`'s constructor; its `TextAnnotation` reads `/Name`, `/State` and `/StateModel`
  and not `/Open`. Grepped, not remembered. Principle 5: the sentence is a `shall` and is not
  ambiguous, so this tree follows it and records the disagreement.
- **The change was run rather than only tested.** A hand-built note with `/Open true` and a popup
  stating nothing, opened under `Xvfb` with the release binary: the window is on the page with the
  page, `Ada` in Table 166's `/C` on its title bar and §12.5.6.4's icon beside it.
- **`doc/todo/00` step 7's ink sweep**, over all 786 ambiguous pages: twenty at or past −1 of 255,
  sixteen of them documents this tree calls incomplete, and the four complete ones the four
  diagnosed names — `issue16038.pdf` −5.734, `issue12295.pdf` −2.823, `issue14297.pdf` −1.145,
  `issue7821.pdf` −1.000. **Identical to the previous round's to the thousandth**, which is what a
  round that moves no page pixel should produce. The instrument itself had to be got right first:
  without `-alpha off` the same loop puts `calrgb` page 15 at the head of the list and reproduces
  nothing.

## Gates

`fmt`, `clippy --workspace` (silent of lints; the `viewer-qt@0.1.0:` lines are gcc's on a cold
build), `nextest --workspace` (1639 passed, 11 skipped), doctests, `pdf-sandbox`'s gates binaries,
the pdf-model corpus gate, `pdfref-hayro`, the oracle, both text gates, dates, xmp, jpeg2000, the
quorra corpus gate, and `conformance`. Every ratchet held and no oracle verdict moved.

## What the next round should know

- **Nothing above commit 110 of 607 is unread now.** §12.5.4, §12.5.6 and §12.5.6.8 were the three
  genuinely unread rows the previous round's `git blame` run found; the last two are read. What still
  blames above that fold is the seventeen the four-hundred-and-forty-second read and *kept*, because
  keeping a row edits nothing — which is the flaw in using blame as a reading list.
- **What replaces it is a shape rather than an order: check the entry, not the capability.** A row
  that retires a refusal by naming a capability that arrived — "drawn since the Nth session", "this
  program now has a window" — names no blocker, no missing vocabulary and no absent architecture, so
  every one of `doc/todo/01`'s fourteen sweeps passes it, and nothing asks whether the *entry* that
  turns the capability on was ever wired to it. It is `doc/habits.md`'s sixth refusal shape now, and
  the grep that found this one was `grep -rn '"Open"' crates/`.
- **A `shall` about what a *host* shows is not visible to any gate here.** The corpus gate
  rasterises a page, the oracle compares one, and both were byte-identical across this change while
  a window appeared that had never appeared before. §12.5.6.14, §12.5.6.2's `/RC`, §12.3.3's
  outline and §12.2's preferences are all in that class; the only instrument is `Xvfb` plus a
  fixture, which `doc/environment.md` has the recipe for.
