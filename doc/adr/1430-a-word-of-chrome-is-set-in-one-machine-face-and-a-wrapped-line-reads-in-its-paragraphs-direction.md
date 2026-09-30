# 1430 — A word of chrome is set in one machine face, and a wrapped line reads in its paragraph's direction

Session 1297. Status: **accepted**.
Context: `crates/viewer-host/src/machine_faces.rs` (`Word`, `MachineFaces::ask_word`),
`crates/viewer-ui/src/chrome.rs` (`Chrome::sets`, `set_word`, `label`, `Wrapped`, `wrap`),
`crates/pdf-font/src/shaping/label.rs` (`Label::line_of`).
Builds: ADR 1382 (the machine's face for a character the compiled-in faces lack), ADR 1406 (the
search off the drawing thread), ADR 1417 (a label's direction is its own). Closes the two things
ADR 1417 section 5 left.

## 1. A word changed face in its middle

ADR 1382 asks the machine for one character at a time, and ADR 1406's `ask` answers a character
from the first face already found that states it. That is right for a character and wrong for a
word. On this machine the face that answers a lam alone — the widest `cmap` covering it — states
kaf's, alef's and noon's presentation forms and lacks heh goal's medial form U+FBA9, so Urdu
"کہانی" was drawn with four glyphs from one face and the fifth from another: two weights and two
joins meeting inside a cursive word. The Unicode Standard's section 9.2 joins letters into one
shape; a face change breaks that shape as surely as a missing form does.

**The rule**: the characters of a word that the compiled-in faces lack are asked for together
(`MachineFaces::ask_word`), and the search is the covering search over all of them, so the face
that answers states every one. **A word is a run of the displayed line with no white space in
it** — a run in display order, which for a word of one direction is the same run as in logical
order. Only where the machine has no single face for the word does the chrome fall back to asking
per character, which is ADR 1382's answer and still better than a box. While the word's search is
running its glyphs are boxes, so that no frame draws half a word in a face the whole will not be
drawn in.

**And the face is ranked by its style before its repertoire.** Driven under `Xvfb`, the first
face that stated all of "کہانی" was an extra-light condensed Noto Sans Arabic UI: the widest
repertoire is `installed_covering`'s proxy for a document's other characters and says nothing about
weight, so the word came out pale beside its neighbours — one face, and the wrong one.
`pdf_font::substitute::installed_covering_styled` is the same search with the qualifying faces
ranked by distance from the requested weight (400, or 700 for bold), then slope, then width, and
only then repertoire; `machine_faces` asks it for characters and words alike. It is a function of
its own because `installed_covering` decides a substituted composite font's face on the page,
which the raster gates hold, and this decides only chrome. Driven again, the row reads in one
regular weight.

**What is deliberately not in the word's question**: the characters §9.6.2.2's compiled-in faces
state. ADR 0133's argument is that those draw the same on every machine, and ADR 1382 kept it whole
by asking the machine only for what would otherwise be a box. A word mixing a Latin letter and a
Han character is two scripts in one run, and the compiled-in face is the right one for the Latin
letter on every machine. A word of one character is asked as a character, so that its answer is
shared with every other line that asks for it.

## 2. A wrapped line took its direction from itself

ADR 1417 made a label's direction its own first strong character's, by UAX #9's rules P2 and P3,
because a label is one paragraph drawn as one line. A popup's paragraph is not: `chrome.rs` wraps it
into several lines and drew each through `Label::new`, so a line that happened to begin with a
Hebrew word after a line break in a left-to-right paragraph became a right-to-left paragraph of its
own, and its trailing full stop moved from the right end to the left. UAX #9's rules P1 to I2
resolve a paragraph; only L1 to L4 are a line's.

**The rule**: `Label::line_of(paragraph, line)` resolves the levels over the whole paragraph and
takes the line's range of them, then L1, L2 and L4 over the line; joining is over the line's
characters, which is what is drawn together. `wrap` breaks the paragraph — white space collapsed,
as it always did — into byte ranges of it and lays out each as `line_of`, and `Chrome::label` draws
a laid-out line. The popup's body, its thread of replies, the password card's prompt, the
restriction card's reasons and choice, and the card that says the drawing stopped all wrap through
it.

## 3. Consequences

- `pdf_font::shaping::Label::line_of` is additive; `Label::new(text)` is `line_of(text, 0..len)`.
- `viewer_host::machine_faces` gains `Word` and `ask_word`, and its queue carries a character or a
  word; both are searched through `installed_covering_styled` (additive in `pdf-font`). The faces
  `quorra-qt` registers come from the same search; how Qt then sets a row among its registered
  families, choosing a family per character itself, is ADR 1418's arrangement and not this
  chrome's.
- Tests: `label::tests::a_wrapped_line_reads_in_its_paragraphs_direction` and its Arabic partner;
  `chrome::tests::a_word_is_set_in_one_machine_face` and
  `chrome::tests::a_wrapped_line_reads_in_its_paragraphs_direction`, each failing with the rule
  planted out; `machine_faces::tests::a_word_is_answered_by_one_face`.
