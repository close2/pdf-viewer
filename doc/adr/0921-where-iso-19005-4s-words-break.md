# 0921 — Where ISO 19005-4's words break, and the two repairs that were not it

Session 940. Status: **accepted as a finding, with no code change**. 1914 of the 16 450 words in
this tree's reading of ISO 19005-4 are a single letter that is not a word. The cause is understood,
two candidate fixes were tried and measured, both were wrong, and the third is forbidden.

## The measurement

`tools/pdfa-text.py` prints it, and both bought parts were read the same way:

| | words | single letters that are not words |
|---|---|---|
| ISO 19005-2:2011 | 16 310 | **63** |
| ISO 19005-4:2020 | 16 450 | **1914** |

"Conforming PDF/A-4 files shall adhere to all requirements of ISO 32000-2:— as modified by this
document" reads back as "… ISO 32000-2: — a s mo d i f ie d b y t h i s do c u ment ." `pdftotext`
over the same page produces the sentence whole.

## Why it happens, and why it is not simply a bug

`content::text`'s `separate_text` infers a word break where the gap between two show operations
exceeds `word_gap`, which is 0.6 of the font's own space. The two documents differ in their
producer, not in this program: part 2 is set a line at a time, so there are few gaps to judge;
part 4 is set a fragment at a time with tracking, so the judgement is made hundreds of times per
page and a tracked gap inside a word can be as wide as six tenths of a space.

The clause says whose problem this is. §14.8.2.6.2 requires a *tagged* producer to state its own
word breaks — "any white-space characters that would be present to separate words in a pure text
representation shall be present in the tagged PDF representation of the text" — precisely so that
"the PDF processor can determine word breaks without having to rely on heuristics based on
information such as glyph positioning on the page". Part 4 is a tagged document that did not, and
what this reader does in that case is a documented **choice**, not a clause obeyed.

## The two repairs that were measured and rejected

**Raising the threshold.** Multiplying `word_gap` by 2 gives the sentence back, cleanly, on every
page tried. It is also forbidden: `CLAUDE.md` principle 5 rules out "tuning constants until a
corpus matches", and a constant chosen because it makes one producer's output agree with
`pdftotext` is exactly that. It would also be untested against the population `word_gap` was chosen
for, where a *larger* threshold loses real word breaks — a failure that reads as prose and is
therefore harder to see than this one.

**Taking the space from the font's own code for U+0020.** `word_gap` asks for the advance of byte
32, which is the space of a *simple* font (§9.7.1 gives one 256 codes, one per byte) and is not a
code at all in a two-byte composite encoding — `Font::advance` then answers for CID 0. That reads
like the bug, and `LoadedFont::code_for(' ')` is the exact inverse of the readback, built by
running it. Substituting it makes ISO 19005-4 **much worse**: page 15 goes from one damaged
sentence to "Token cha r ac t er s u sed t o del i m it objec t s". The reverse table answers with
the *first* code that means a space, and in this file's fonts that is a thin one — so the fix
replaces one wrong width with a smaller wrong width. Reverted, and recorded here so that the next
reader does not spend the same hour on it.

## What would actually settle it

The tagged route, and it is the standard's own: for content the structure tree reaches, the
producer's word breaks are stated and no gap needs judging. That is a change to which text the
reader trusts rather than to a constant, it is derivable from §14.8.2.6.2 rather than fitted to
anybody's output, and it belongs to a round that can run
`pdf-model/tests/text_extraction.rs` over its 974 documents. This session did not have that round,
so what it did instead was **measure the damage and print the number in the file's own header**,
where anybody quoting from it will see it.

## Consequences

- The Markdown of ISO 19005-4 in `doc/pdfa/` is usable and is honest about where it is not: a
  reader quoting a sentence from it must read that sentence, which they must do anyway.
- Nothing was repaired by guesswork. Gluing single letters back into words would be editing the
  standard, which is the one thing a reader of it may not do.
