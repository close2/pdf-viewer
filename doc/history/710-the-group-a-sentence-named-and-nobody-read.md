# 710 — The group a sentence named and nobody read, and the default a value struck out

§12.5's `partial` rows read as a family, on ADR 0538's method for the fifth block running, with the
family chosen by ADR 0567's search rather than by eye. The pair the search pointed at disagreed with
itself about an erratum, the disagreement led to a citation that was the argument and was the wrong
note, and reading the right note's conditions against the code found a `shall` in the same paragraph
that nothing read and nothing reported.

Date: 2026-08-24.
ADR: [0579](../adr/0579-the-group-a-sentence-named-and-nobody-read.md).

Touched: `doc/conformance/ledger.toml` (§12.5.2 and §12.5.5),
`crates/pdf-model/src/content/transparency.rs` (`note_appearance_group`, new),
`crates/pdf-model/src/content/annotations.rs` (its caller),
`crates/pdf-model/tests/annotations.rs` (one test, three fixtures),
`crates/pdf-model/examples/appearance_transparency_census.rs` (new),
`doc/errata-read.md`, `doc/adr/0030` (two corrections to one paragraph), `doc/todo/01`, the ADR and
this file. No status moves and no pixel moves; **one report is added and it fires on nothing in the
corpus.**

## Why §12.5, and two rules for reading the ranking

The search was run rather than read out of a document: for every parent whose subtree holds two or
more `partial` rows, count the rare five-word sequences the notes share pairwise, rank by the total.
**§12.5 heads it**, with §12.8 second, §8.11 third, §14.8 fourth and §12.8.3 fifth — 705's family is
no longer the head, which is the ranking doing its job one round after that round rewrote fifteen of
its rows.

Two things about the output cost this round time and are now in `doc/todo/01`:

- **The clause-level parents have to come out.** §12, §11, §8, §14, §7 and §10 sort above every real
  family on the tail of thousands of pairs. 0567's first run did not say so only because §12.8
  happened to beat them.
- **The total ranks the family; the pairs choose the reading.** §12.5's subtree is larger than a
  round can read properly. Its three strongest pairs are each a *quotation* one round wrote into two
  rows — §12.5.4 ~ §12.5.6.8 at 24 shared rare sequences, §12.5.2 ~ §12.5.5 at 16, §12.5.3 ~
  §12.5.6.4 at 15. **The one to take is where the two rows disagree about what their shared sentence
  leaves standing**, which was the second.

## The three findings

- **§12.5.5 said §12.5.2 lists `/BM`, `/ca` and `/CA` among the keys a reader shall ignore.** It
  lists two: Errata Collection 3's Issues #23 and #34 strike `BM` out, which §12.5.2's own row has
  recorded since the four-hundred-and-seventeenth session and which `annotation::blend_mode` applies
  on both paths. So one third of the sentence §12.5.5 calls unfollowed has been followed for nearly
  three hundred sessions. 697's rule with the direction reversed: the correction landed in the row
  that states the mechanism and never reached the row that depends on it.
- **The note that paragraph cites is the wrong note, and the citation is the argument.** §12.5.5's
  row justified building no transparency group with "§11.6.7's NOTE 1 makes it identical to painting
  the elements directly". §11.6.7 is *Patterns and transparency* and its NOTE 1 says a different
  thing — that a non-isolated group of Normal-blending elements may be treated as isolated, which is
  what `note_group_structure` cites it for, correctly. The reduction is **§11.4.4's NOTE 5**, and it
  *states its conditions*: non-isolated with the parent's knockout attribute, and "the Normal blend
  mode is used, and the shape and opacity inputs are always 1.0" at the composite with the backdrop.
  With `/BM` no longer ignored, the second condition is the annotation's to break — and the cost is
  confined to where the appearance's own marks overlap. No sweep in this tree could have printed it:
  a clause number beside a paraphrase is neither a table citation nor a quoted span.
- **The clause's other transparency sentence had no reader and no report.** "Otherwise, the isolated
  and knockout values specified in the group dictionary … shall be used" — `draw_appearance` ran the
  stream directly and `transparency_group`, the one function that reads Table 145's `/I` and `/K`,
  was called from `draw_xobject` and the soft-mask path and nowhere else. Not one comment in either
  annotation module mentioned `/Group`.

## What was added, and what was deliberately not

`transparency::note_appearance_group` reports the two stated values, each only where it can change a
pixel — isolated where an element blends (§11.4.4's NOTE 2), knockout where a later element
composites over an earlier one (§11.4.6, through `knockout_can_show`). Table 145's `/CS` gets no
report and the clause is why: §11.6.6 gives a group colour space only to an isolated group, so on
the group this path builds a `/CS` states nothing — and two of the corpus's four appearance groups
state one, so a report on the entry would have fired twice for no difference at all.

**Implementing the group instead was considered and declined in writing** (ADR 0579 §4): routing the
appearance through `run_transparency_group` would put all four corpus appearance groups, every one
non-isolated and non-knockout, through a second code path to obtain by construction what NOTE 5 says
the current path already produces.

`annotations.rs::an_appearance_group_the_file_states_is_named_and_the_default_one_is_not` is three
fixtures: an isolated group that blends, the same appearance under the default group as the control,
and a knockout group with an overlapping element. Calibrated per trap 13 four ways — each branch
disabled in turn (the matching half fails with its own message), and the isolated branch made
unconditional (the control fails). All four plants removed.

## The census, and the negative that decayed

`crates/pdf-model/examples/appearance_transparency_census.rs` is the command `doc/todo/01`'s
counted-claim rule asks for, in `border_precedence_census`'s three scopes. Over the 974 it finds four
appearance streams stating a `/Group`, all `/S /Transparency`, none isolated, none knockout, and not
one annotation anywhere stating a `/BM`. Over `CC-MAIN-2021-31`'s 65 944 it finds 143 such groups
with **95 isolated and one knockout**, and **eight annotations stating `/BM /Multiply`** beside a
stored appearance, on ink and polyline annotations in two documents. ADR 0490's shape again: the
fixture is the corpus's only witness and is not the world's.

## The erratum, and it is a struck *value*

`emit` over the two pages §12.5.2 spans files seventeen annotation objects and **one is named nowhere
in this tree**: **Issue #577**, `Review`/`Accepted`, dated 2026-05-21 — the newest erratum any round
here has met. A `StrikeOut` over `1.0 ` and a `Caret` saying `the value of CA`, at
`[263.930 285.359 278.601 297.032]`; `pdftotext -bbox` puts `Default value: 1.0` at 544.888 from the
top of an 841.92-tall page, which is 841.92 − 297.032 to three decimals, three lines under "…but not
the popup window that appears when the annotation is opened" — so it is Table 166's **`/ca`** row
and not the `/CA` two rows below it.

`annotation::construct` has read `/ca`, then `/CA`, then 1.0 since it was written, on the strength of
the `/CA` row's own sentence rather than the `/ca` row's, which said 1.0 and seemed to deny it. **No
arithmetic moves; the authority does.**

**And it is a third way `check` is blind**, beside a caret with no strikeout and a strikeout under
the four-word floor: **a strikeout whose text is a *value*.** `1.0` shares no sentence with anything,
so a tree quoting the whole `/ca` row verbatim would still not match it.

## Gates and sweeps

`PDFREF_CACHE` pointed at the shared warm cache, `/home/AI/cargo-target/pdf-viewer/tmp/pdfref-cache`.
The full sequence was run: `tools/round.sh` says this is a fifth round, and the round adds code to
`pdf-model`. The machine was quiet — a load average under 5 on 24 cores when the lines that spawn a
reference renderer ran, which is §2's rule that such a gate measures two programs and a loaded
machine is a silent third.

`fmt`, `clippy -D warnings`, `nextest`, the doctests, the fuzz `check`, the sandbox worker, corpus,
`pdfref-hayro`, oracle, text extraction, selection, accessibility, dates, XMP, JPEG 2000, quorra,
`fixed_documents` and `cargo test -p conformance` all green, the last of them after the final edit.
§5's binaries rebuilt and installed. The two clippy findings were both `doc_markdown` on prose and
both took a backtick.

Thirteen sweeps run **before** the edits — with the round's own files copied aside and the tree
checked out, because `workspace_root()` is compiled in and trap 15 makes a sweep binary a
measurement of the tree it sits in — then after them, and a third time on the tree carrying the ADR,
this file and `doc/todo/01`'s new section, which are `SOURCE_ROOTS` too. **Three levels moved into a
defect bucket on this round's own prose and all three were put back**:

- `--bin counts` gained three counts it "can be counted no such way", on three sentences of this
  round's own saying how many `partial` rows §12.5's subtree holds — a cardinal governing one of the
  ledger's words for a row, which is 691's noise shape exactly. The sentences say *more than a round
  can read properly* now and the level is back.
- `--bin quotations`' diverging documents went 34 → 35 on the ADR reproducing §12.5.5's row, whose
  own text carries an elided quotation of the standard; the ADR paraphrases the row and quotes only
  the verbatim fragments now.
- `--bin quotations`' diverging **ledger** spans went 2 → 3, and this one was a real misquotation:
  the new §12.5.5 sentence elided "(see 11.6.6, "Transparency group XObjects")" out of the middle of
  the clause's own sentence. Shortened to the span that is verbatim.

Everything else moved by what the new prose contains and nothing landed in a defect bucket. Final
levels, after → before: `counts` 7533 ← 7494 sentences with 406 ← 403 attributed counts, **58 "no
such way" and 4 places counting one family twice both times**; `quotations` 5900 ← 5880 document
spans with **diverging unchanged at 34**, and 1893 ← 1887 ledger spans with **diverging unchanged at
2**; `tables` 6214 ← 6199 sentences and 2326 ← 2313 key citations with **absent unchanged at 100 and
contradicted denials at 6**; `pointers` 7819 ← 7809 with **absent unchanged at 131 and undefined at
13**; `owed` 3747 ← 3738 terms over 223 `partial` rows with **177 unnamed over 112 rows unchanged**;
`overtaken` 532 ← 531 decision records with **44 overtaken unchanged**; `blockers`, `entries`,
`unread`, `inapplicable`, `overstated`, `capabilities` and `callers` all unmoved. `spec-errata check`
is byte-identical before and after, and `applied`'s three counts — 90 quoting a replacement, 10
matching both sides, 171 quoting what an erratum struck — are unchanged.
