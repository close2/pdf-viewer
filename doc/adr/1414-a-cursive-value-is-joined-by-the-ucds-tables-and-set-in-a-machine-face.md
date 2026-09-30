# 1414 — A cursive value is joined by the UCD's tables and set in a face from the machine

Session 1288. Status: **accepted** and built.
Context: `crates/pdf-font/src/shaping/` (`joining.rs`, `face.rs`, `mod.rs`), `crates/pdf-font/build.rs`,
`crates/pdf-model/src/variable_text.rs` (`machine_set`, `machine_font`, `encode`, `Owed::FormsNotInFont`),
`crates/pdf-model/src/appearance.rs` (`for_annotation`), `crates/pdf-model/tests/variable_text.rs`,
`data/unicode/{DerivedJoiningType,ArabicShaping,UnicodeData}.txt`.
Amends: ADR 0348's "No machine-face fallback", on ADR 1382's and ADR 1154's precedents.
Clauses: ISO 32000-2 §12.7.4.3, §12.5.6.6, §9.7.4, §9.7.4.3, Table 115, Table 116, §9.10.3, §7.3.8.1.

## 1. What the witness needed

`freetext_no_appearance.pdf` — a paragraph of Arabic under `/DA (/Helv 10 Tf 0 g)`, no `/DR` — was
the one incomplete corpus document that was this reader's. ADR 0348 found it needs a glyph source,
joining-form selection and right-to-left order together. ADR 1413 is the order; this is the other two.

## 2. Joining: the Unicode Standard's rules, reached as presentation forms

- `shaping::positions` is section 9.2's rules R1 to R7 over `DerivedJoiningType.txt` (which states
  the transparent class outright, so no General_Category table is needed); `shaping::shape` adds
  ligature rules L1 to L3 for the `LAM` and `ALEF` joining groups of `ArabicShaping.txt`.
- **A form is a code point**: `UnicodeData.txt` decomposes every Arabic Presentation Forms-A and -B
  character to its letter under an `<isolated>`, `<final>`, `<initial>` or `<medial>` tag, and the
  inverse of that is the form table; the two-letter `<isolated>`/`<final>` decompositions give the
  four lam-alef ligatures. A pair of those groups the blocks hold no ligature for is drawn as two
  joined letters — the standard makes the ligature obligatory where the font's style supports it.
- **Why not the face's own `GSUB`**: the route is exact by the UCD's own data, tested by hand-worked
  expectations, and independent of the face; both faces this machine offers for Arabic map both
  presentation blocks. A face whose only route to its forms is `GSUB` (`init`/`medi`/`fina`/`rlig`,
  lookup types 1 and 4, which `read-fonts` parses and `vertical.rs` already walks for type 1) is not
  chosen by the covering search below, and a document's own such font is reported
  (`Owed::FormsNotInFont`) with its letters drawn unjoined. Executing `GSUB` is the priced next step;
  Noto Sans Arabic's `rlig` uses contextual lookups (types 5 and 6), which is where its cost is.
- A letter with no presentation form in a joined position is marked `unformed`, and an invented font
  may not fall short: the value is then refused whole, as before.

**Joining is computed over the value, and a line never splits a joined pair at a soft wrap.** The
wrap breaks at a space, and a space is non-joining, so the letters either side of any soft break
are already in the forms a line end gives them. A break inside a word happens only where one word
is wider than the whole line; the pieces then keep the forms the whole word gave them, which is a
choice: section 9.2 states joining over adjacent characters and says nothing about a line break
inside a word, and the space-based rule makes the case rare. A letter in initial form at the end
of a line is otherwise the producer's own text — `freetext_no_appearance.pdf` stores a lam and a
tatweel before a line feed, and the tatweel (join-causing) is what keeps the lam initial.

## 3. The glyph source: the machine, asked for the shaped characters

A font this module invented may not fall short (ADR 0112), and no compiled-in face has an Arabic
glyph. So after both routes into the compiled-in face, `machine_set` asks
`substitute::installed_covering` for a face covering **every character the shaped, mirrored value
displays** (`shaping::displayed_characters`) — ADR 1382's answer for the interface, and ADR 1154's
rule that a test depending on it skips with a sentence. The font written around it is §9.7.4's
`Type0`: `/Identity-H` (Table 116), a `CIDFontType2` with `/CIDToGIDMap /Identity` (Table 115), `/W`
read from the face (§9.7.4.3 makes that array what the interpreter advances by), a descriptor with
the face's extent, and a `/ToUnicode` (§9.10.3) mapping each glyph to the stored characters.

**What this reopens, and why it is accepted**: ADR 0133's argument that a page reproduces across
machines. It holds for everything the fourteen draw, which is still asked first; what differs
between machines is a value that was a blank on all of them. `/Helv` denotes Helvetica, so the
witness reports nothing; a name that denotes nothing still reports `FontNotInResources`.

## 4. A machine face is drawn, not written

The font holds the face as a stream inside a dictionary, which §7.3.8.1 does not admit in a file,
and a file carrying it would carry this machine's font. `appearance::for_annotation` — the writer's
path, which the archive conversion takes — reports such a construction as owed, so it refuses rather
than write it. Writing one properly (an indirect, subset program, and the face's embedding licence)
is not built.

## 5. Measured

`the_arabic_free_text_is_drawn_joined_and_right_to_left`, `an_arabic_value_is_shaped_and_displayed_as_the_ucd_tables_state`
(its glyph sequence fails when the reordering is planted out) and the `shaping` unit tests, which
encode section 9.2's own R1–R6 and L1 examples.
