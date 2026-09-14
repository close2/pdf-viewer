# 1074 — A glyph class is decided by the character, and every class a character cannot decide is refused by name

Session 1060. Status: **accepted**. Settles how ISO 32000-2 §9.8.3.3's `/FD` reaches a glyph without
the character collection's CID tables, and what a reader does with the classes it cannot reach, so
that a later round reading Table 123 beside the code does not re-open either half.

Context: `crates/pdf-font/src/glyph_class.rs`, `crates/pdf-font/src/substitute.rs` (`overridden`,
`glyph_classes`), `crates/pdf-font/src/loading.rs` (`class_faces`, `Placed`, `ClassFace`,
`refused_glyph_classes`), `crates/pdf-model/examples/glyph_class_census.rs`; ADRs 0007, 0152, 0270;
`doc/traps/parsers-and-streams.md` trap 8, `doc/traps/instruments-and-reports.md` trap 13.

## 1. Why the CID boundary is not the boundary it was taken for

Table 122 makes `/FD` a dictionary "whose keys identify a class of glyphs in a CIDFont", whose values
"shall override the corresponding values in the main font descriptor dictionary for that class of
glyphs". §9.8.3.3 then says where the keys come from:

> The names of the glyph classes depend on the character collection, as identified by the Registry ,
> Ordering , and Supplement entries in the CIDSystemInfo dictionary.

For five hundred and fifty-nine sessions this tree read the entry and applied it to nothing, on the
argument that knowing which *CIDs* a class holds means holding the collection, which is registered
data published outside this standard — Table 116's boundary, and ADR 0007's licensing decision.

That argument is sound and is unchanged: nothing in `glyph_class.rs` reads a CID. What it overlooked
is that the question does not have to be asked of a CID. The only route on which a font descriptor
decides a glyph here is a **substituted** CIDFont, and §9.7.4.2 leaves that route reachable by
nothing but a character — "In this case, CIDs shall not participate in glyph selection" — so every
glyph the entry could reach arrives with §9.10.2's Unicode value already resolved. And Table 123
describes each class by the characters its glyphs stand for: "Proportional Latin glyphs",
"Full-width hanzi (Chinese) glyphs", "Hangul and jamo glyphs", "Numeric glyphs". Those descriptions
are the standard's own. A character can therefore be *shown* to lie outside a class, and often inside
exactly one of the classes a given file names — which is all an override needs.

## 2. The rule

A character is sorted into one of eight kinds, each of which some cell of Table 123 names
(`glyph_class::Kind`); each class the file states holds a set of kinds, read off that class's own
cell. A kind held by **exactly one** stated class assigns it; anything else refuses. So a file
stating only `Proportional` gives its Latin letters and digits to `Proportional` and leaves its kanji
on the main descriptor, and a file stating `Proportional` *and* `HRoman` gets neither, because
"Proportional Latin glyphs" and "Half-width Latin glyphs" differ by a width no character carries.

**A class stated alone is not that ambiguity, and this is the one judgement in the rule.** The
substitute this tree picks for a Latin class is a proportional Latin face whatever the class is
called; it has no half-width Latin glyphs to be confused with. What is refused is a file that asks
the reader to choose between two classes, not a file that names one.

## 3. What is refused, and why a refusal rather than a guess

`LoadedFont::refused_glyph_classes` names every declined class and the sentence that declined it.
Five kinds of refusal, each from Table 123 or the clause:

- a class distinguished from another by the glyph's **form** rather than its character — every
  `…Rot` class, "Same as HRoman but rotated for use in vertical writing", and `Ruby`, "Glyphs used
  for setting ruby (small glyphs that serve to annotate other glyphs with meanings or readings)";
- a class described by something no Unicode value states — `Dingbats`, "Special symbols", and
  `Generic`, "Typeface-independent glyphs, such as line-drawing";
- a class Table 123 does not list for the collection the descendant states;
- an ordering Table 123 does not tabulate at all, which is every `Identity` one and the two
  `Adobe-Identity` descriptors the census finds;
- two stated classes that hold one kind of character.

A refusal keeps the main descriptor, so it can change no mark the font already made. Guessing would:
assigning every unclassifiable character to a lone `/Dingbats` would hand a mathematical symbol the
metrics of a dingbat, which is the confident wrong answer the rule exists to refuse.

**A list and not a report**, on ADR 0152's arithmetic and the same reading `unsupplied_vertical_form`
took: what it says is about this machine's catalogue and this reader's reading of Table 123 rather
than about the file, and a report would take a page off the oracle's judged set for it.

## 4. A class that states no exception is not an exception

§9.8.3.3: "The FD entry in the font descriptor shall contain exceptions to these defaults." A class
descriptor whose merged result derives the request the main descriptor already derives has stated no
exception this tree can act on, and `class_faces` looks no face up for it. Without that rule such a
class would still get a *second search*, because the characters a class's face must cover are the
class's rather than the collection's — a `Proportional` face need not draw 的 — and the search could
land on another face for nothing the file said. That is not a corner: all 129 `/FD` dictionaries on
this disk restate `/Flags` and `/ItalicAngle` unchanged and none states a `/FontWeight`.

## 5. What the census settles, and what it cannot

`pdf-model/examples/glyph_class_census` over 91 800 documents: 129 font descriptors of 614 068 state
`/FD`, in 32 documents; each names exactly one class; 127 embed their program. What they override is
`/StemV`, `/StemH`, `/CapHeight`, `/XHeight` and the `/Ascent`-`/Descent` pair — none of which this
tree reads as an input to anything — and never one of the three entries a face is chosen by.

So **no document on this disk can move a glyph through this route**, and `raster_golden` holds 974 of
974. That is trap 8 stated rather than hidden: the rule above is derived from the clause and the
corpus is what says no page exercises it. The fixture in `loading::glyph_class_tests` is the route's
only witness, and it is calibrated (trap 13) by deleting the `class_face` arm — its first assertion
fails and its second passes.

The corpus cannot settle one thing and a later round should not read it as having done so: whether
`/StemV` and the rest ought to be inputs. `doc/todo/21` item 4 carries that question for the *main*
descriptor, and the moment it is answered, `/FD` already carries the answer per class.
