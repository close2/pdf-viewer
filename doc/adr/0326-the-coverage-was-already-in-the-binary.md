# ADR 0326 — The interface's coverage was already in the binary, and the encoding threw it away

Status: accepted, 2026-08-14 (session 491).

## Context

`doc/todo/27` had one thing left open, and it was the expensive one. Everything this program
draws for *itself* is set in §9.6.2.2's fourteen font programs, compiled in (ADR 0133), so an
interface reproduces on a machine with no fonts installed; a character those faces cannot set is
drawn as a box, 0.6 em wide, which advances and is counted (ADR 0195). What a box does not say is
*what*, and for a person reading an outline in their own language that is most of what they
wanted. The file priced three answers and none is free:

1. **fall back to a face on the machine**, which costs ADR 0133's whole argument — the interface
   stops looking the same on two computers, and every assertion about it becomes an assertion
   about which fonts this one has;
2. **compile in a face with the coverage**, which is a licence question, a megabyte question and a
   decision the project owner has not been asked for;
3. **ask the host**, which is right for `viewer-gtk` and `viewer-qt` and leaves `viewer-ui` — the
   host with no toolkit — exactly where it was.

The three were priced. **What was never measured is the demand**: which characters real documents
actually ask an interface to set. `viewer-ui --example chrome_coverage` counted how many
characters were lost and in which populations; it could not say what they *were*, and it asked
`Chrome`, which is the code under test.

## The measurement

`pdf-model --example interface_font_census` opens every corpus document through `pdf-syntax` and
`pdf-model` — no `Chrome`, no `viewer-core`, so nothing here is the instrument asking itself
(trap 8) — and takes the seven populations a program draws *from* a document: §12.3.3's outline
titles, §8.11.4.3's layer names, §7.11.4's file names and descriptions, §14.3.3's `/Info`,
§14.3.2's XMP, §12.4.2's page labels and §12.5.6.14's popup text. Each character is asked **two**
questions rather than one:

- does a character code reach it — `LoadedFont::code_for`, which is what every panel asked;
- does the face state a glyph for it at all — the same compiled-in bytes, asked by character.

The headline is the two numbers in front of the table:

> the compiled-in Helvetica: **149 characters by code, 668 by character**

A simple font's codes are one byte (§9.7.1: "each byte of a string to be shown selects one
glyph"), and `LoadedFont::standard` assembles a `/Type1` dictionary with no `/Encoding`, so the
codes are §9.6.5.2's `StandardEncoding` and the answer stops at 149 characters. The face behind
it is Liberation Sans, whose `cmap` states 668 — every accented Latin letter, the Greek alphabet,
the Cyrillic alphabet. **The interface had been drawing a box for `é` in a face that has one.**

Over the corpus: **54 documents lost a character** in a panel, **41 of them lose nothing at all**
once the face is asked by character, and 13 still do. Of the 144 characters recovered, 130 are
Latin-1 Supplement, 12 Cyrillic and 2 Latin Extended-A — so the commonest thing this interface
could not set was not a foreign script at all, it was a French or German or Spanish word.

What is left is genuinely outside the binary, and the census names it by script and by file: 213
characters of Hebrew, every one of them in `issue14046.pdf`; 81 of Thai, every one in
`issue13211.pdf`; 85 of Japanese and Chinese over six documents; and 77 of U+FFFD, which is a
*report about a malformed file* rather than a coverage gap — `bug1146106.pdf` writes its text
strings as UTF-16 little-endian, which is none of §7.9.2.2's three encodings (ADR 0195), and the
rest are fuzzed files.

## Decision

**A character with no code is looked up in the face by character, and only what the face does not
state at all is a box.** `LoadedFont::character_glyph` is that route, and three things about it
are deliberate:

- **It is not a route a document's text may take.** A document selects a glyph by code — §9.6.5's
  encoding, §9.7.6's `CMap` — and drawing a glyph the file did not select would be inventing what
  the page says. What has no code is the text a *program* draws for itself, and there is no
  encoding in that question to go round. The method's own doc comment says so, because the next
  reader's temptation is to reach for it from the interpreter.
- **The advance is the program's own**, from `hmtx`, because no `/Widths` array exists to disagree
  with; `Chrome::set` stays the one place `text` and `width` agree about a character, so elision,
  wrapping and the popup's title bar move with it.
- **The code route still comes first**, so every Latin string is measured and drawn exactly as
  before — §9.6.2.2's published metrics, not Liberation Sans's — and nothing that was already
  right could move.

**`doc/todo/27`'s three answers are not chosen between; they are made smaller.** The demand they
were priced against was 54 documents and is now 13, of which one is a malformed file and the rest
are Hebrew, Thai and CJK — four documents in 964. Compiling in a CJK face to serve four documents
is a worse trade than it was when the item was written, and it stays the owner's to make.

## Consequences

- **Nothing else in the tree changes**, and that is checkable rather than hoped: a document's
  fonts never reach this method, the corpus gate and the oracle judge pages rather than panels,
  and the corpus gate was run to say so.
- **`without_a_code` still counts exactly the boxes** — it is computed through `Chrome::set` — so
  §12.5.6.14's popup keeps saying how many characters it could not set, and now says a smaller
  and truer number.
- **The compiled-in faces are worth more than the row that carries them says.** §9.6.2.2's ledger
  row recorded the fourteen as a *reproducibility* decision, which is what they were bought for;
  what this session found is that their character sets had never been asked about. That is
  `doc/habits.md`'s "a capability that arrived and announced nothing", one directory over from the
  ledger: the bytes arrived in the hundred-and-forty-eighth session and the interface asked them
  the wrong question for three hundred and forty-three sessions.
- **Two censuses now exist for one subject and both are kept.** `chrome_coverage` asks what *this
  host's* `Chrome` can set, which is the question a change to `Chrome` has to answer;
  `interface_font_census` asks what the *binary* can set, which is the question a change to the
  compiled-in data has to answer. Merging them would put the instrument inside the thing it
  measures.
