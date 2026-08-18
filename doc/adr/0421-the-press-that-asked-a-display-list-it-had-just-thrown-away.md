# ADR 0421 — The press that asked a display list it had just thrown away

Status: accepted, 2026-08-18. Builds the rest of ADR 0323's instrument 1 — the composed half at
corpus scale, and the geometry half's first ratchet — and records what the instrument found on
its first run, in the loop rather than in the geometry. The design is ADR 0323's and is not
re-argued here; ADR 0333 built the geometry half.

## Context

`doc/todo/05`'s first build item was left with two halves owed: **the ratchet**, once the
geometry verdict's numbers had held across rounds, and **the two self-inverse properties** ADR
0323 puts beside the drag half, which no round had written. Both are about the same gap, and it
is the one trap 12a is about: the geometry half judges where this tree's text layer *says* the
words are, in the page's own points, and nothing judged the journey from there — device pixels
in, a `Command::Pointer`, an `Answer::Selected` out. `user_space_at`'s doc comment claimed a
coordinate space it did not have for seventy-five sessions and every click followed that sentence
into the mirror of the point it meant, because **no gate clicks** (ADR 0118).

One test has clicked since ADR 0333: one committed document, three words, with the viewport
resized to the page's own point size — where the magnification is 1 and the origin is 0, so two
thirds of the mapping a host composes are the identity.

## Decision

### 1. The composed half becomes a corpus-scale instrument

`crates/viewer-core/tests/selection_census.rs`, three properties over page one of all 974 pdf.js
documents, everything driven through `Command`/`Query` and nothing through an internal:

| | property | judged against |
|---|---|---|
| the drag | a drag across poppler's word box selects that word | `pdftotext -bbox -cropbox` |
| the readback | `Selection::All`'s text is `Interpretation::text`, byte for byte | the interpreter, read beside the boundary |
| the caret | `Query::Offset` of `Query::Caret`'s own point is that offset again | itself — the pair is documented as inverse |

Four decisions were taken here rather than inherited.

**The viewport is 800×1000 and the page is fitted into it**, deliberately not resized to the
page's own size: the origin and the magnification are then the ones a window actually has, and
they are two thirds of the arithmetic ADR 0118 is about.

**A word is dragged across only where poppler states it exactly once and this tree's own readback
states it exactly once.** Uniqueness on our side is not circularity — the *point* is still
poppler's — it is Finding 5's convention: a word we read differently from poppler is a
disagreement about characters, which the two text gates already measure, and counting it here
would measure it twice.

**A word whose reference box is taller than it is wide is not dragged across**, because a
horizontal drag is then the wrong gesture rather than a wrong answer — trap 3 in miniature. It
took `alphatrans.pdf`, `issue2074.pdf` and `bug1473809.pdf`'s vertically-set `TCPDF` out of the
misses, where they were a fact about the instrument.

**`/Rotate` and `/UserUnit` are refused by name** instead of normalised. The geometry half's
`Frame` normalises both and its findings are recorded there (ADR 0333); a second copy of that
arithmetic in a second crate is exactly the shape that comes to disagree with the first.

**The two exact properties are asserted; the drag fraction is printed.** `doc/todo/05`'s standing
rule governs the fraction — numbers enter `doc/todo/02` §2 only once they have held across rounds
— but a property that holds over the whole population on its first run is stronger stated as a
property than counted, and both of these do.

### 2. `Query::Caret` is not injective, and the property says so

The first run failed the caret property in 26 places, all on `issue15053.pdf`, where five
consecutive offsets answer with **one** point and that point names the last of them. No
arithmetic can fix it: where a glyph's advance is zero, several offsets are the same place on the
line, and a point cannot name all of them. So the property asserted is not
`offset(caret(o)) == o` but `caret(offset(caret(o))) == caret(o)` — the round trip lands on the
same *place*, which is the whole of what a host needs to put the cursor where the click was.
Offsets that share a point are counted apart rather than tolerated in silence.

### 3. What the first run found: a press that asked an interpretation it had just dropped

**78 of 1017 dragged words, over 44 corpus documents, selected nothing at all.** The witnesses
are documents with widgets — `annotation-tx.pdf`, `js-buttons.pdf`, `listbox_actions.pdf`,
`issue15818.pdf` — and the cause is in `Viewer::pointer`, four lines above where the anchor is
taken:

- §12.5.5's appearance state is changed for the annotation under the pointer;
- changing it calls `Open::stale`, which sets `interpreted = None` — correctly, because the page
  now draws differently;
- the `Pressed` arm then asked `open.position_at`, which reads `self.interpreted.as_ref()?` and
  answers `None` — so the press set **no anchor**, and every drag from it selected nothing.

A person pressing on text that happens to lie over an annotation with a down appearance got no
selection. The fix is one line moved: **where the press landed is decided against the display
list the person is looking at**, before the appearance state is touched, and both the `Pressed`
and the `Dragged` arms use that one answer. The drag census moved 92.33% → **98.91%**.

`viewer-core/tests/headless.rs::a_press_over_an_annotation_still_anchors_a_selection` is the
regression test, un-ignored, on `annotation-tx.pdf` with both endpoints read out of the file —
the widget's `/Rect` and the page's own text position. The census *found* it; that test is what
keeps it, because the census's drag fraction is not ratcheted and could not fail a build for it.

### 4. The geometry half's verdict is ratcheted

ADR 0323's rule is met: **the numbers held across eighty-eight rounds.** Session 498 measured
98.26% of matched words in bounds, 485 of 507 documents fully in bounds; this session measures
98.26% and 486 of **508** — the one document's difference being a document that entered the
judged set, not a word that moved. So `the_word_boxes_we_place_agree_with_the_references` now
carries what the text gate has always carried: a named list of the 22 documents with a word out
of bounds, checked in **both** directions, and a floor under the judged set — trap 11's
arithmetic as a ratchet, because a refusal that grew would shrink the denominator and leave the
verdict looking unmoved. The gate now prints every out-of-bounds document rather than the worst
ten, since the list is what a round maintains from the gate's own output.

### 5. §9.4.2 gained a requirement in Errata Collection 3, and this tree did not have it

`spec-errata emit` over clause 9, run before writing as `doc/todo/02` §4 asks, prints issue #368
(`/State` `Review` `Completed`) adding to §9.4.2 that within a text object the graphics state
stack operators `q` and `Q` "shall additionally push and pop Tm and Tlm as part of the graphics
state stack" — with §7.8.2 pointed at it in the same collection. §9.4.1's ledger row said the
opposite in as many words: "q/Q cannot save them".

The stack entry is now the graphics state **and** the two matrices, restored only inside a text
object because outside one Table 105's `BT` sets them anyway. It is one stack rather than two on
purpose: a stream whose `q` is inside a text object and whose `Q` is outside it would put two
stacks out of step, and "as part of the graphics state stack" is what the sentence says.

**The corpus exercises the operators and cannot exercise the rule**, which is trap 8's shape
measured rather than assumed: 13 of the 974 documents put a `q` or a `Q` inside a text object,
and not one moves `Tm` between the two — no corpus page, no oracle verdict and no word box moves
either way. So it is pinned by a **pair** of streams differing only in the `q`, in
`text_state.rs`, the construction `cross_references.rs` uses for the same reason.

The quotation is in prose rather than in a blockquote, and that is a rule rather than a slip: the
conformance checker verifies a blockquote against `doc/md/`, which is a conversion of the base
text, and a sentence the errata *add* is not in it (ADRs 0252, 0253). Presenting one as a
verifiable quotation would fail the gate with the standard blamed for it.

## Establishing that it discriminates

`doc/habits.md`'s rule — deleting the code a scene guards is the only thing that establishes the
scene guards it — applied to each instrument, one breakage at a time, each reverted:

| breakage | what it does | what caught it |
|---|---|---|
| the y flip mirrored in `Viewer::page_point` | device → page space is the mirror | drag census **98.91% → 48.86%**, caret property fails in 32 places, 14 headless tests fail |
| the text layer's quads shifted 1 pt in x | every glyph box a point out | geometry gate **98.26% → 0.13%**, the new ratchet names 484 documents newly out of bounds — and the **drag census does not move** (98.81%) |
| `Selection::All` hands back a trimmed copy | the selection path tidies the readback | readback property fails on 88 documents |
| `Query::Offset` answers one byte early | the inverse stops being one | caret property fails in 1585 places |
| the `pointer` fix reverted | the defect above, restored | the new headless test fails; the census reports **92.48%** |
| the `Q` restore removed | §9.4.2's addition unexecuted | `q_and_q_save_the_text_matrices_inside_a_text_object` fails |

**The second row is the one worth keeping**, because it is the two instruments' relationship
measured rather than asserted: a one-point shift is fatal to the geometry half and invisible to
the composed half, and the mirror is the other way about. Neither is the other's approximation.
The first row's 48.86% is also worth reading: a mirror does not take the drag to zero, because a
page with one line of text has only one line for a press to be nearest to — which is exactly why
ADR 0118's own tests could not see the defect they were written for.

## Consequences

- The loop from a press to a selection has an independent judge over 453 corpus documents, and
  `doc/HANDOVER.md`'s "It is **used**" is measured rather than asserted for the first capability
  in that paragraph.
- `doc/todo/02` §2 gains one line, six seconds warm, sharing its extraction cache with the line
  above it. The geometry gate gains a ratchet and keeps its printed spread.
- Eleven drags still miss, each named by the gate, in four classes: an end glyph whose advance we
  and poppler disagree about (`issue5039.pdf`), reversed text (`issue2391-2.pdf`,
  `issue11656.pdf`, `issue14046.pdf`), and a form-heavy page where the word poppler reads from a
  widget's appearance is not the word our layer has at that point (`issue9972-1/2/3.pdf`). They
  are a list to read rather than a number to chase.
- `doc/todo/05`'s item 1 is closed but for the drag fraction's own ratchet, which the round after
  next may take once it has held.
