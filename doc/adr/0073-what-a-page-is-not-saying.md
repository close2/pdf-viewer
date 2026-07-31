# ADR 0073 — What a page is not saying

Status: accepted, 2026-07-31.

## Context

§14.8.2 is what tagged PDF adds to a *page*, as against what §14.7 adds to a document, and two
of its twelve rows change what `Interpretation::text` produces:

- **§14.8.2.2's artifacts.** A page's content divides into "material intentionally introduced by
  the document's author and necessary to understand the content" and everything else — running
  heads, folios, rules, background blocks. 30 of the 953 openable corpus first pages mark at
  least one.
- **§14.8.2.5.3's `ReversedChars`.** A marked-content tag saying that the show strings inside it
  hold their characters backwards, for right-to-left text set in a font whose glyphs run
  left to right. "[T]he sequence of the characters as found in the show string operator shall be
  reversed before using them."

Both are processing rules about *extraction*, which is the one thing in this tree that reads a
page back rather than drawing it.

## Decision

**Record the artifacts; reverse the strings; drop nothing.**

`Interpretation::artifacts` is a span per `/Artifact` sequence with Table 363's property list —
`/Type`'s four kinds, `/Subtype`, `/BBox`, `/Attached` — over the same readback `text` and
`described` are over. **No text is removed.** §14.8.2.2.1 states the choice as the consumer's:
a text-to-speech engine "may decide not to speak running heads or page numbers when the page is
turned", and NOTE 3 says the purpose of tagged PDF "is not to prescribe what the PDF processor
does, but to provide sufficient declarative and descriptive information to allow it to make
appropriate choices". A crate that dropped artifacts from the text would have made that choice
for every consumer, including the one that wants to copy a page number.

`ReversedChars` reverses the readback **per show string and per code**: per string because the
clause says so ("[i]f the sequence encompasses multiple show strings, only the individual
characters within each string shall be reversed"), and per code because what it reverses are the
characters the show string states — one code may map to several, and a ligature whose
`/ToUnicode` says `fi` comes back as `if` from a reversal that works on `char`s. The inferred
word breaks are suppressed inside such a string, which is the clause's own rule: a break there is
a SPACE the file wrote "at the beginning or end", and the glyphs run against the writing
direction, where a gap means nothing.

## What implementing it found

**An inline property list's array values were being discarded.** `inline_dictionary` read an
array as far as its brackets and stored `Object::Null`, under a comment that said "no property
list entry this tree reads is an array, and keeping the *keys* aligned matters more than keeping
a value nothing looks at". That was true when it was written and stopped being true here: Table
363's `/BBox` and `/Attached` are both arrays, and both came back empty.

This is `doc/HANDOVER.md`'s own trap — "a parser that recognises a delimiter without parsing it
will be read as parsing it" — met from the inside, and the comment was *accurate* about what the
code did. The fix is `inline_array`, bounded in depth and in element count like the dictionary
beside it. No gate moved, which says no corpus page one depends on an array in an inline
dictionary today; the inline-image path uses the same parser, so `/Filter [/FlateDecode]` written
inline would have been dropped in the same way.

## A measurement that corrected a written conclusion

The text gate's list of documents below its floor says "7 are right-to-left text read back in
painting order", and this session's obvious hypothesis was that `ReversedChars` would fix them.
**No corpus document writes the tag** — measured over all 953 openable first pages before the
code was written. The seven are painted in visual order with no marked content saying so, which
is a different problem with a different answer, and it is now written down as such.

So this is the specification track with nothing behind it on the demand curve, which is exactly
the case `CLAUDE.md` principle 5 says to do anyway: the clause's own EXAMPLE is the test, and a
file that uses the tag will read back correctly the first time it arrives.

## Consequences

- `silent` falls 153 → 146.
- `Interpretation` gains `artifacts`; nothing consumes it yet, which is the same debt §14.7 and
  §14.9 carry — the data is this crate's and the consumer is not.
- No page renders differently and no gate moves: 90 incomplete, 65 contradicted, 97.8% of
  `pdftotext`'s words.
