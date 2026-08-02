# ADR 0141 — The container where a font program was stated

Status: accepted, 2026-08-02. Session 157. Small, and it is here because the *choice* inside it
is the kind this project writes down.

## What the clause says and what the files do

Table 127 states what `/FontFile2` holds:

> ( PDF 1.1 ) TrueType font program, as described in the TrueType Reference Manual .

A `TrueType` Collection is not one. It is a container of several faces sharing their tables,
introduced by a `ttcf` header where an `sfnt` has a table directory. Two of the pdf.js corpus's
first pages embed one, and both drew no text at all and reported `Invalid sfnt version
0x74746366` — which is `ttcf` in hexadecimal, and is exactly the right report for a reader that
has decided to do nothing.

## The decision, and why it is a derivation

Reading *a* face from the container is a decision the file has not made — unless it has. Table
122 gives the descriptor a `/FontName`, §9.6.2.1 makes that "the PostScript name of the font",
and every face in a collection carries its own PostScript name in its `name` table. Matching the
two is reading the document. Taking face zero is the fallback and is recorded as one, because a
collection whose faces the descriptor names none of has told us nothing.

**Both documents were opened and their collections listed, and each pays for one half of the
match:**

| document | `/FontName` | the collection holds | chosen |
|---|---|---|---|
| `issue9262_reduced.pdf` | `MSMincho` | `MS-Mincho`, `MS-PMincho` | face 0 |
| `issue13193.pdf` | `DCWGQU+CambriaMath` | `Cambria`, `CambriaMath` | **face 1** |

The first needs the hyphen normalised away *and* needs `MS-PMincho` not to match; the second
needs §9.6.4's six-letter subset prefix removed, and **face zero would be the wrong face**. That
second row is what makes the match load-bearing rather than a nicety, and it is why the test for
it uses that document and distinguishes the two faces by a table — `CambriaMath` carries `MATH`
and `Cambria` does not — rather than by comparing pictures.

## Why the face is copied out rather than referred to

`read_fonts::FontRef::from_index` opens a face in place, and taking that route would put an index
into all eight of `pdf-font`'s `FontRef::new` sites, zero at every one of them but these.
`collection::rebuild` writes the chosen face's tables into a standalone `sfnt` instead, so the
container is a fact about *loading* and nothing downstream knows it existed. It costs one copy of
one face per font load, which is the same order as the decompression that produced the bytes.

The rebuilt directory's checksums are written as zero rather than recomputed. Nothing in this
tree verifies one, and **a wrong value is a claim where a zero is visibly not one** — the same
argument that keeps this project from inventing a `/ToUnicode`.

## What it moved

Corpus documents drawing incompletely 76 → **74**; oracle pages we call complete 1681 → **1683**,
agreeing 845 → **846**, contradicted unchanged at 72. `issue13193.pdf`'s side-by-side has the same
line of mathematical symbols in all four panels.

## The habit

**A dependency's error message can name the fix.** `Invalid sfnt version 0x74746366` sat in the
corpus output for as long as the corpus gate has existed; the four bytes are `ttcf` and say what
the file is. **Read a refusal's own words before deciding what it would take to remove it** — this
one took ninety lines and an afternoon, and the reason it waited is that nobody had converted the
number.
