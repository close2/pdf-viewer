# ADR 0763 — The form the producer chose, and the substitute that lost it

Status: accepted, 2026-08-31. Session 836. Cites ISO 32000-2 §9.7.5.1 and its NOTE, §9.7.5.2 with
Table 116, §9.7.4.2, §9.7.3 and §9.10.2; §9.5's NOTE 5 is what makes the second half of the rule a
documented choice rather than a reading. It sits beside ADR 0140 (the registered `CMap` data being
carried at all), ADR 0152 and ADR 0270 (what a substituted font is allowed to say about what it
could not draw) and ADR 0358 (the previous time a substituted face was corrected from a table the
*file* states rather than from another renderer's output).

## The page

`doc/corpora/pdf-differences/VerticalText/VerticalText.pdf` is the PDF Association's witness for
vertical writing, and its README says what it is for: "[b]ecause there is no embedded font in this
PDF, a substitute font supporting vertical writing is required to be present." It states
`/Encoding /Identity-V` over a `CIDFontType0` whose `/CIDSystemInfo` is Adobe-Japan1-5 and whose
descriptor carries no font program, so the two-byte codes in its content stream **are**
Adobe-Japan1 CIDs — and the producer has written the *vertical* ones: 7911 and 7912 for 「」, 7887
and 7888 for 、。, 7891 for ー, beside ordinary kanji and kana at their single CIDs.

This tree placed its seven columns exactly where the four references place them — §9.7.4.3's
`/DW2 [880 −1600]` has been read since the thirty-sixth session — and drew every bracket lying on
its side and every full stop in the middle of the column instead of at the top right. That is the
picture the corpus publishes as `IncorrectVertical.png`.

## Why the shapes were lost, in one step

§9.7.4.2 leaves a substituted composite font reachable only by character: "CIDs shall not
participate in glyph selection". So this tree takes the CID to a Unicode value through §9.10.2's
third method — the collection's own `Adobe-Japan1-UCS2` table — and asks the chosen face's `cmap`
for the character.

**That table is keyed to the character and not to the form.** Adobe-Japan1's CID 7911 is the
vertical LEFT CORNER BRACKET and CID 686 is the horizontal one, and the `-UCS2` file maps both to
U+300C, because Unicode has one code point for the character. Everything the producer said about
*which shape* is thrown away at that step, and there is nothing wrong with the table: it answers
§9.10.2's question, which is what a code *means*.

What says the discarded thing matters is §9.7.5.1's NOTE, which this project had read for hundreds
of sessions as being about metrics:

> Writing mode is specified as part of the CMap because, in some cases, different shapes are used
> when writing horizontally and vertically. In such cases, the horizontal and vertical variants of
> a CMap specify different CIDs for a given character code.

An embedded font is therefore correct by the CID alone, which is why no document with its font in
it has ever shown this. A substituted one is not.

## The rule, in two halves, neither of them a convention

§9.5's NOTE 5 puts the choice of a substitute outside the standard altogether, so nothing here can
be *derived*. What can be done — and is the difference between this and guessing — is to make each
half a published table read for what it says.

**Which CIDs are vertical forms is the character collection's own statement.** Table 116 publishes
each collection's Unicode `CMap` twice, and of the pair says that "those ending in V specify
vertical writing mode"; the vertical file is its horizontal twin plus the characters whose shape
changes. So the pair *is* the answer: a character whose vertical `CMap` sends it to a different CID
than its horizontal one has a vertical form, and that CID is it. Both files have been compiled into
this binary since the hundred-and-fifty-sixth session (ADR 0140), so this needed no new data at all
— `predefined::is_vertical_form` asks the two maps and compares.

Both halves of that comparison are load-bearing, and the second is the one that is easy to leave
out. Without `V(ch) == cid`, a producer that wrote the **horizontal** CID under a vertical `CMap` —
which is legal, and is a statement about that glyph — would be answered with a rotation it did not
ask for. Without `H(ch) != cid`, every kanji on the page would qualify, because a character with no
vertical form is sent to one CID by both files.

**Which glyph of the face is that form is the face's own statement**, and OpenType's registered
`vert` and `vrt2` features are where a face makes it. `pdf_font::vertical` reads them out of `GSUB`
directly: only lookup type 1, because a vertical form is one glyph for one glyph and anything else
would be a rule about a sequence. It does not shape — `doc/stack.md` rules that out and it is not
needed — and it does not select by script or language, because choosing a script means deciding
what language a run is in, which a content stream does not say, and because the feature means the
same thing under every script that registers it. `vrt2` is read after `vert` and therefore wins
where a face states both, which is a choice and is stated as one.

## What was deliberately not built

**A report.** A substituted face with no vertical form draws the producer's character in the
producer's place in a shape the substitute had, which is the same shortfall as a face with no glyph
for a character at all — and ADR 0152 priced that: a report costs the oracle a judged page, and
this is a statement about a face rather than about a file. It stays counted-by-nothing and
described here, exactly as ADR 0270 left its neighbours.

**A set of vertical CIDs per collection, built by walking every `*-V` `CMap` Adobe publishes.**
That was the first design and it is worse for a reason worth recording: it answers "is this CID
*somebody's* vertical form" where the question is "is it *this character's*", and it would have
needed a build-script change to know which of the 239 files belong to which collection. Asking one
published pair, per character, is exact and needs no new data.

**Adobe-KR and Adobe-Japan2 rows.** Table 116 publishes `UniAKR-UTF16-H` and no vertical
counterpart for Adobe-KR — one of the four collections §9.7.5.2 requires supporting — and
Adobe-Japan2 is deprecated in this edition. Neither has a pair to compare, so `is_vertical_form`
answers `false` for both. That is the table's doing rather than a gap here, and the row-less table
says so.

## What it costs

Nothing at startup and nothing on any horizontal page. `Downward::read` is reached only for a
composite font that is all four of substituted, in writing mode 1, in a collection Table 116
publishes a vertical `CMap` for, and drawn from a face with a `vert` or `vrt2` feature; a document
that opens no such font reads no byte of `GSUB` and inflates no extra `CMap`. Per glyph drawn the
route adds two binary searches over the collection's Unicode pair and one `BTreeMap` lookup, on a
path that already builds a `FontRef` and a `charmap` per call.

## What is gated, and what is this machine's

The collection half is not machine-dependent at all and is asserted from the compiled-in files:
`predefined::a_collection_says_which_of_two_cids_is_the_vertical_form` names Adobe-Japan1's two
CIDs for 「 and 。 and the two negatives — a character with one CID, and a CID belonging to another
character — and `each_collections_vertical_pair_is_carried` checks that every row of the table
names two files this binary has and that their writing modes are 0 and 1.

The face half is this machine's, because which face `substitute` finds is (§9.5 NOTE 5). So
`pdf-model/tests/vertical_forms.rs` names no glyph: it reads **one descendant twice**, once under
each identity `CMap`, and asserts which of its CIDs the writing mode changes — the four vertical
forms and nothing else, with three single-CID characters and the four horizontal twins held equal.
Two readings of one dictionary differ in the writing mode alone, which is trap 8's rule for a
property no single document states.

Its skip condition is `LoadedFont::face_states_vertical_forms`, which reads the face's `GSUB` and
**not** whether the route changed anything: a skip read off the output of the thing under test is
trap 13, and it would have made the file green with the whole route deleted. Calibrated both ways —
with `downward` forced to `None` the run fails on the first pair rather than skipping, and with the
collection test dropped from `form_of` it fails on the first *horizontal* twin.

## What did not move

Nothing outside a substituted vertical composite font can reach this code, and the corpus gates say
so: the oracle's per-page lines, the corpus gate's counts and `render-quorra`'s are what the
session's history file records either side of the change.
