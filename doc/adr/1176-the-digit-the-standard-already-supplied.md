# 1176 — The digit the standard already supplied, and the two syntax rules that reach a page

Status: accepted and **built**. Session 1169.
Context: `crates/pdf-transform/src/archive/hexadecimal.rs` (new),
`crates/pdf-transform/src/archive/{decision,prepare,rewrite,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`.
Supersedes: `doc/adr/0947` decision 3's second half — a content-stream hexadecimal string with an
odd digit count is no longer refused "because correcting it would edit the producer's marks".
The first half of that decision stands: the whole-file rewrite does stop at a content stream, and
`SYNTAX_INSIDE_CONTENT` is still what every other such failure gets.
Answers: `doc/adr_revisit/0947-the-converters-middle-stage-and-the-net-under-it.md`.
Builds: `doc/adr/1120` and `doc/questions/A65` (mark provenance is the fence's test),
`doc/questions/A50` (the `F`-to-`f` respelling), `doc/adr/0816` (the fence).
Clauses: ISO 32000-2 §7.2.4, §7.3.4.3, §7.4.4, §7.7.3.3 (Table 31), §8.9.7; ISO 19005-2 section
6.1.6, ISO 19005-4 section 6.1.5.

## 1. The owner's note, claim by claim

The revisit note is an argument to test, not an instruction.

- **"ADR 0947 decision 3 refuses it because correcting it would edit the producer's marks."**
  *Held*, in those words, and the code says the same in `decision::SYNTAX_INSIDE_CONTENT`.
- **"§7.3.4.3 says the missing final digit shall be assumed to be 0, so the string's VALUE is
  already defined."** *Held, and stronger than the note puts it.* The clause states it —

  > If the final digit of a hexadecimal string is missing -that is, if there is an odd number of
  > digits -the final digit shall be assumed to be 0.

  — and its EXAMPLE 2 spells the consequence: `<901FA>` *is* a three-byte string whose last byte is
  A0. The validator's own comment on `hexadecimal_string_digits` had already written this down: a
  conforming reader "cannot see this fault in the value".
- **"ADR 1120 restates the fence as mark provenance."** *Held*; `CLAUDE.md`'s fourth amendment says
  it in the file the owner ratified.
- **"The row was re-read once — ADR 1026: *nothing moves today* — before the amendment landed."**
  *Did not hold as stated.* ADR 1026 §3 re-read something else about these two rows: whether a
  part's resource carve-out *narrows* them, and its "[n]othing moves today" is about that
  asymmetry, because "both rows report a span of the file rather than an object". The refusal
  itself has not been re-read since ADR 0947. The conclusion is unaffected and one worry is
  removed: nothing was decided twice.

## 2. The ruling

**Writing the digit is on the near side of the fence, and the line was already drawn there.**
`doc/pdf-a-conversion-limits.md` section 5.2 records the owner's `A50` admitting the `F`-to-`f`
respelling on one sentence — "[i]t changes a byte in a content stream and cannot change a mark. The
fence is drawn at **marks**" — and the same sentence is true here on better evidence: §7.3.4.3 does
not merely document an equivalence, it *states the value the byte already has*. No operator is
added, removed or respelled; no operand changes value; every glyph the page shows is the one it
showed.

**The sibling row stays refused, and the difference is the whole of why this is a ruling rather
than a licence.** `file-structure/hexadecimal-string-holds-only-digits` is failed by a byte that is
neither a hexadecimal digit nor white space, and §7.3.4.3 gives such a byte **no value at all** —
it says what a string is written as and what white space does, and stops. Our reader passes over
it; that is this program's behaviour, not the standard's ruling. Deleting it would be a guess about
what its producer meant. Refused, under `SYNTAX_INSIDE_CONTENT`, unchanged.

A50's four conditions are met and are worth restating as this remedy's: a repair the standard
itself states the result of, applied only where a target's rule requires it, reported like every
other thing this converter writes, and **no wider licence** — the `.notdef` and `q`/`Q` refusals
section 5.2 names are untouched.

## 3. The other whole-file syntax remedies, and which side each is on

`decision::WRITER_EMITS` holds the requirements the serializer satisfies by construction. Only two
of them can be failed by bytes *on a page*, because only two are judged by a lexer run over content
streams; the rest judge an object's own syntax, the cross-reference, the trailer or the file's tail,
and a content stream holds none of those.

| requirement | can it fail on a page | side |
|---|---|---|
| `hexadecimal-string-digits` | yes | **near** — §7.3.4.3 states the value, so the digit is a transcription |
| `hexadecimal-string-holds-only-digits` | yes | **far** — the standard states no value for the byte |
| `stream-keyword-line-endings` | no — `stream`/`endstream` are object syntax | n/a |
| `indirect-object-syntax` | no | n/a |
| `cross-reference-keyword-line-endings` | no | n/a |
| `stream-length-matches-the-data` | no | n/a |
| `nothing-after-the-last-end-of-file-marker` | no | n/a |
| `file-identifier` | no | n/a |
| the object-count limit | no | n/a |

So the class this opens has exactly one member today, and the test for a future one is stated
rather than left to analogy: **does the standard state the value the repair writes?**

## 4. How it is built, and the two bounds

`archive/hexadecimal.rs` lexes each content stream object with `pdf_syntax::Lexer` and records the
offset of every `>` whose string held an odd digit count. **The lexer decides which `<` opens a
string**; `HexadecimalStrings`' own comment says what happens to a second reader that re-derives
that, and this is not one — what this adds is the token's span, which the lexer's position gives on
either side of the read. §8.9.7's inline image data is stepped over with
`pdf_model::inline_image::scan`, for the reason the validator steps over it: compressed samples
hold angle brackets like any other byte.

- **Per stream object, which is the standard's own division.** §7.7.3.3's Table 31: "[t]he division
  between streams may occur only at the boundaries between lexical tokens". So a conforming
  `/Contents` array never straddles a string, and a file that straddles one anyway leaves a `<`
  with no `>` in the same object — nothing is written for it, the requirement stays failed, and ADR
  0947's third stage refuses the file rather than a wrong byte being put in it.
- **The repaired stream is re-encoded as one §7.4.4 `FlateDecode`**, because the bytes written are
  the whole decoded stream and the chain that produced them is spent. A `/DecodeParms` naming
  another object refuses instead, so that discarding the parameters cannot orphan it.

## 5. What this fixes beyond the refusal

The failure the validator records at a page had no rewrite; the *same* failure recorded at a
resource-reached stream object took `Rewrite::WholeFileRewritten`, which does not touch content
bytes either — a promise that never arrived, which is precisely what ADR 0947's own comment warned
about. Both now take the repair, because the population is every content stream object rather than
the place a finding happened to name.
