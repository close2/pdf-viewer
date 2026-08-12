# 0294 — A flag on the note, and a style its table does not give

**Status.** Accepted.
**Context.** The four-hundred-and-fifty-eighth session left §12.5.6 and §12.5.6.8 pointed at: the
last two ledger rows whose notes were written in the sitting that produced §12.5.4's, where a
wrong sentence had just been found to arrive as a block. Both were read against the code and the
standard. Both rows' own claims held — `STANDARD_SUBTYPES` carries all twenty-eight of Table 171's
names, `appearance::construct` switches on the subtype only where §12.5.5 found no stream,
`Border::inset` is §12.5.6.8's "inscribed within", and `/RD` is applied in the clause's own left,
top, right, bottom order. **What the block turned out to contain was one departure per clause,
neither of them the row's subject**, and one of them is a `shall` this program failed in silence.

## The flag on the note

§12.5.6.4, first sentence:

> When closed, the annotation shall appear as an icon; when open, it shall display a popup window
> containing the text of the note in a font and size chosen by the interactive PDF processor.

and Table 175 gives the annotation the entry that decides which:

> Open boolean (Optional) A flag specifying whether the annotation shall initially be displayed
> open. Default value: false (closed).

**Nothing in this tree read it.** `grep '"Open"'` over `crates/` found exactly one reader,
`popup::read`, and what it reads is Table 186's identically named entry on the *popup* annotation.
A file saying its sticky note starts open showed the icon, no window, and no report — the
silently-wrong page trap 5 exists for, on a clause that obliges rather than recommends.

**Two reasons it survived, and the second is the interesting one.**

The comment above `appearance::text_icon` said "`/Open` is not read. §12.5.6.4 gives it a popup
window … and `crate::annotation` draws no popup for any subtype, on the ground that a window is
not part of the page." That was true when it was written and false from the three-hundred-and-twelfth
session, when `crate::popup` and `Query::Popups` gave this program windows — `doc/todo/01`'s
capability shape, in a doc comment rather than in a ledger row.

And **the ledger row said the opposite of the truth**. §12.5.6.4's note read: "The third was 'the
popup window /Open selects, which §12.5.6.14 does not draw for any subtype', and it is drawn since
the three-hundred-and-twelfth session." The *window* is drawn; the entry that selects it was never
read. A row that retires a refusal by naming the capability that arrived is a row nobody re-reads,
because it looks settled.

### Why it is a disjunction and not a precedence

Two entries named `Open` now bear on one window, and the tempting question — which wins — has no
answer in the standard because it is not asked. Table 186's says "whether the popup annotation
shall initially be displayed open"; Table 175's says whether the *text annotation* is, and
§12.5.6.4 says an open text annotation displays this window. Each defaults to `false`. So each
states a condition under which the window is open and **neither states one under which it is
closed**: a disjunction, with no conflict for Table 186's four-entry override list to have settled.

That list is worth naming because this tree already leans on it in the other direction:
`popup::popups` reads `/F` from the popup and not from its parent, on the ground that "a table that
enumerates four is a table that has said something about the fifth" (`pr7352.pdf` asserts the
opposite rule). The two readings are consistent. An *override* settles a conflict between two
statements of the same fact; two entries that can only open a window have no conflict to settle.

**Only a text annotation's `/Open` counts.** Table 175 is the only table outside Table 186 that
gives an annotation the entry at all — §12.5.6.7, §12.5.6.8, §12.5.6.9, §12.5.6.10 and §12.5.6.13
each say their annotation displays a popup window "when opened" and not one of them states an entry
that opens it. An `/Open` on a `Highlight` is a key the standard does not define, and the corpus
contains none.

### What the corpus can say, counted before it was believed

`crates/pdf-model/examples/open_annotation_census.rs`, over the 964 openable documents of the 974:

```text
  34 835 annotations, 28 of subtype /Text
       1 of those states Table 175's /Open true
       1 names a /Popup, and that popup already states Table 186's /Open true
       7 popups state Table 186's /Open on their own
       0 annotations of another subtype state an /Open no table gives them
```

The one is `pr7352.pdf`, and its popup agreeing means **the corpus cannot rank this rule**: a
reader that ignores Table 175 draws every one of the 974 identically. Trap 8 in the shape
`pdf-syntax/tests/cross_references.rs` already uses, so
`a_text_annotations_own_open_opens_its_popup` is a *pair* of fixtures differing only in the six
characters `/Open true`, and `only_a_text_annotations_open_reaches_its_popup` is the second, which
fixes the subtype condition. Each was confirmed to fail with the rule it guards removed — the
disjunction and the `Text` test respectively — because a fixture that has not been watched fail is
a fixture nobody has established guards anything.

## The style Table 180 does not give a square

§12.5.4, on what a border style dictionary is for:

> Such dictionaries may also be used to specify the width and dash pattern for the lines drawn by
> line, square, circle, and ink annotations.

and Table 180 says the same thing from the other end, giving a square or circle a `/BS`
"specifying the line width and dash pattern that shall be used in drawing the rectangle or
ellipse". Two of Table 168's entries, named twice, for four subtypes.

`square_or_circle` obeyed that where it *draws* — the mark is the annotation's own rectangle or
ellipse, never `Border::outline`'s styled path — and then ended `Ok(border.simulated())`, which
reports "Table 168's beveled and inset borders state no highlight or shadow colour" whenever `/S`
is `B` or `I`. So a square whose `/BS` states a bevel was named as an appearance this crate could
not derive, for a bevel the clause does not ask it to draw. That is trap 11's condition — a report
firing where the clause asks for nothing — and it is the exact mirror of the departure the previous
round found one sentence away: an entry consulted where its own table says it supplies nothing.
§12.5.6.9's polygon, under the identically worded Table 181, never reported it.

`border_precedence_census.rs` finds no `B` and no `I` among the 33 781 corpus annotations that
state no `/AP`, so this too is hand-built, and as a pair: the same `/BS` on a **link**, whose `/BS`
*is* §12.5.4's border and where Table 168's `/S` applies in full, still reports. A reader that asked
the border dictionary one question for both subtypes would report twice or not at all.

## What the other readers do, as evidence and not as truth — and here they disagree

pdf.js reads `/Open` in exactly one place, `PopupAnnotation`'s constructor: `this.data.open =
!!dict.get("Open")`, where `dict` is the **popup's**. Its `TextAnnotation` reads `/Name`, `/State`
and `/StateModel` and not `/Open` at all (`src/core/annotation.js`, grepped rather than
remembered). So on the fixture above pdf.js shows no window either.

**That is a disagreement, and principle 5 says what to do with one: find the clause.** The clause
is not ambiguous — Table 175 states the entry, §12.5.6.4 states what an open text annotation does,
and both are `shall`s addressed to the processor. Nothing here is a case of the standard being
silent and a convention filling it. This tree follows the sentence and records that a widely-used
reader does not, which is the honest form of the finding rather than a claim to have been agreed
with.

`pr7352.pdf` is the corpus's only witness and it cannot separate the two readings, which is
consistent with the disagreement having survived: a rule no file exercises visibly is a rule
nobody's test suite protects.

## Consequences

- **No page pixel moves.** A popup is a window a host draws and not part of the page's rendering,
  and the second change alters only a report on a condition no corpus document meets. Both corpus
  gates and the oracle are unchanged, and `doc/todo/00` step 7's ink sweep over all 786 ambiguous
  pages reproduces the four-hundred-and-fifty-eighth's run **to the thousandth**: twenty names at
  or past −1 of 255, sixteen of them documents this tree already calls incomplete, and the four
  complete ones the four diagnosed names — `issue16038.pdf` −5.734, `issue12295.pdf` −2.823,
  `issue14297.pdf` −1.145, `issue7821.pdf` −1.000.
- What *did* move is visible only by running the program, which is where it was checked: a
  hand-built note with `/Open true` and a popup stating nothing now opens its window with the page,
  title bar in Table 166's `/C` and §12.5.6.4's icon beside it.
- §12.5.6.4 stays `partial`, for Table 175's `/State` and `/StateModel`. §12.5.6.8 stays `partial`,
  for Table 169's cloudy `/BE`. §12.5.6 and §12.5.6.14 are unchanged in status and carry the
  reading.
- **The lesson is about how a refusal is retired.** Both of the last two rounds' silent departures
  hid behind a sentence that was *true about the thing it named*: "a stroke is centred on its path"
  said nothing about where the path goes, and "the popup window is drawn since the
  three-hundred-and-twelfth session" said nothing about whether the entry selecting it is read.
  When a row retires a blocker by naming a capability that arrived, **check the entry, not the
  capability** — the sweep that finds an expired blocker will not find an entry nobody wired to the
  capability once it existed.
