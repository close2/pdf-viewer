# 2294 — A placeholder nothing derives leaves the producer's CMap at part 4, and the fence's thirty-three are damaged streams

Session 1730, the archive slot of batch one hundred and twelve. Status: implemented. Context:
`doc/history/1722`'s census; ADR 1115 (the glyph-name derivation); ADR 0816 (the content fence);
ISO 19005-2 section 6.2.11.7.2, ISO 19005-4 section 6.2.10.7; ISO 32000-2 §9.10.2, §9.10.3, §7.8.2.

## 1. `fonts/to-unicode-values-are-usable` at part 4

**What the 32 pdf.js documents carry**, read off their decoded `CMap`s: two shapes, and every one is
a composite font, which is why ADR 1115's derivation refused them by name. Thirteen state a span of
128 to 65 536 codes — `<0000> <FFFF> <0000>` is the common one — whose run passes through U+0000,
U+FEFF and U+FFFE because it passes through every value; the other nineteen map single codes to
U+0000 or U+FEFF (one document 3 639 times), by a `bfchar`, a one-code span or an array element. No
destination mixes a placeholder with a character.

**What the clause lets a converter do.** Both parts' last sentence forbids the three values in any
`CMap` present. Their first paragraph differs: ISO 19005-2 section 6.2.11.7.2 requires, at Level U
and A, a `CMap` mapping the codes of every referenced glyph; ISO 19005-4 section 6.2.10.7 states the
same as a recommendation. §9.10.2's second method needs a glyph name, and a CID names none
(§9.7.4.2), so nothing can be put where the placeholder was — but nothing need be: part 2's NOTE 2
calls these values placeholders, and a statement that a code stands for no character can be taken
out without saying anything new. Part 4 then admits the unmapped code outright; part 2 admits it
only where no content stream showed the code.

**Built** (`pdf_transform::archive::to_unicode::placeholders`): where the derivation fails, the
producer's `CMap` is restated statement by statement — the codespace, `CIDSystemInfo`, `usecmap` and
every section stating no placeholder carried byte for byte; a `bfchar` to a placeholder dropped; a
span split around the value it passes through, its codes in the producer's byte width; an array
span's placeholder element dropped. A destination mixing a placeholder with text is refused, since
trimming it would decide what the producer meant. The rewrite is proved with `pdf_font`'s reader,
the validator's: no placeholder stated, every code the producer's map answered usably answered the
same. A span is kept in the producer's increment-by-scalar shape, which is the reading the validator
and the extractor share; ISO 32000-2's EXAMPLE 2 prints such spans and §9.10.3 states no bound on the
last byte (veraPDF warns on it, and passes the files).

**Census against HEAD** (both exports' binaries, the walk alone; converted / conforming / refused /
unreadable): pdf.js at part 4 723/0/233/11 → 751/0/205/11 under `as-if-printed` and 704/0/252/11 →
732/0/224/11 under `as-shown`, the seven others the row refused now stopping on their other rows
alone; `doc/corpora` at part 4 336/0/139/16 → 339/0/136/16 and 290/0/185/16 → 292/0/183/16; nothing
newly refused. The converter over the veraPDF corpus at part 4: 267 → 270 converted under
`as-if-printed`, 265 → 268 under `as-shown`, every other target unchanged. 26 of the 32 were converted one at a time and
drawn: page 1 within one level of the source at 100 dpi, `basicapi` looked at. veraPDF 1.30.3 passes
21 of the 26 and fails five on 6.2.10.4.1, a descendant `CIDFontType0` whose `FontFile3` states no
`Subtype`, which this validator passes — a disagreement of its own, owed to `doc/todo/72`.

**The hypothesis did not hold**: no document is answered from glyph names, because all 32 fonts are
composite. What answers them is the second sentence of the rule rather than its method.

## 2. The content-stream fence: thirty-three damaged streams

`graphics/only-operators-the-base-standard-defines` refuses 33 `doc/corpora` documents. Read one at
a time: 28 are the format corpus's `govdocs1-error-pdfs/error_set_2`, four are its hand-built
malformed tests (`T02-05-01_006`, `_009`, `_011`, `_014`, named for the defect each plants) and one
is a JHOVE error file. The spellings are numbers run into operators (`0.059998re`, `11522559l`),
words of text outside any string, unbalanced `)` and, in `_014`, `xref` and `trailer` read as
content. **Not one producer's shape and not unbalanced `q`/`Q`** — the brief's hypothesis did not
hold. §7.8.2 makes an unrecognised operator outside a compatibility section an error, so such a page
has no rendering its producer specified, and which bytes the damage took is nowhere in the file. The
fence stands for all 33; `doc/pdf-a-mitigations.md`'s entry names them. Nothing is built.
