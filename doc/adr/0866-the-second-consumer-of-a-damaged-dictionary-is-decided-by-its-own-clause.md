# ADR 0866 — The second consumer of a damaged dictionary is decided by *its own* clause, and §9.6.4's step b) is what lets `/CharProcs` through

Status: accepted. Session 912.
Clauses: ISO 32000-2 §7.3.7 (what a prefix of a dictionary is), §7.3.10 (a reference, and the null
it resolves to), §9.6.4 with Table 110 (Type 3 fonts, and step b)'s outcome for an absent key),
§9.6.5.3 (`/Differences` as the whole encoding), §9.6.5.4 and §7.4.4.1 (the refusal this does *not*
relax).
Code: `crates/pdf-model/src/type3.rs` (`char_procs`, `CharProcsDamage`,
`Type3Font::char_procs_damage`), `crates/pdf-model/src/content/font.rs`
(`note_char_procs_damage`, and the cache level the report fires at).
Tests: `crates/pdf-model/tests/damaged_char_procs.rs`, five files differing in one thing;
`doc/checks/fixed-documents.toml`'s row for `batch5/cairo/cairo-85141-0.zip-3.pdf`.
Measurement: `crates/pdf-model/examples/damaged_dictionary_consumers.rs`, over 90 535 documents.
Continues ADRs 0784, 0787, 0858. Beside ADR 0867, which is the population and the door that stays
shut for the rest.

## Context

ADR 0784 built a door for §7.3.7's dictionary that stops part-way through: `Document::get` still
answers §7.3.10's null, and `Document::damaged_dictionary` answers the entries that were whole to a
caller that asks for them **by name**. It gave the door exactly one consumer — `Pages`' recovery
scan, reached only where the page tree yields no page at all — and its closing sentence about
everything else was silence.

Round 908 stopped `read_dictionary_body` walking out of an object and taking the next one's entries
(ADR 0858). Its witness is `corpus-cache/tika-issue-tracker/batch5/cairo/cairo-85141-0.zip-3.pdf`,
whose object 76 is `/F16`'s `/CharProcs` and whose bytes stop mid-entry at `/a112 57` under another
stream's compressed data. That fix left a question in writing:

> Forty real glyph procedures stop being drawn, and that is the point rather than a cost. They were
> reachable only through an object assembled out of two, under a reader that said nothing; a prefix
> drawn deliberately, reported, and taken through the door that names it is a different change and
> a later one.

This is that change.

## The question, put the right way round

The question is **not** "may a prefix be taken", which ADR 0784 answered once and for all: it may,
it is a **choice** rather than a derivation, and §7.3.7's "[t]he entries in a dictionary represent
an associative table and as such shall be unordered even though an arbitrary order may be imposed
upon them when written in a file" is why — the subset is picked by the very order the clause says
to ignore, so it is *a subset of the producer's own entries* and never *the dictionary*.

The question is **what the consumer's own clause does with the entries that are missing**, and it
has one answer per clause. ADR 0784's consumer had the worst of it: Table 31 hands every absent
page entry a **default** — no `/MediaBox` is §7.7.3.4's inheritance and then ADR 0389's chosen
sheet — so each missing entry becomes a value this reader chose, which is why that recovery carries
a report about substitution and why it took a balance-of-harms argument (a document reporting two
pages and showing neither has nowhere to put a report at all) rather than a clause.

So the test is three conditions, and only the first is about this consumer's clause:

1. **The consumer's clause states the outcome for an absent key, and what it states is that
   nothing is drawn.** Not a default, not a fallback, not a substitute — an omission the standard
   itself defines.
2. **What was lost can be named**, from a dictionary that is whole, so a report can say which
   marks are not on the page rather than that some are missing.
3. **`Document::get` was asked first and answered §7.3.10's null.** ADR 0784 section 3's identity
   condition, unchanged: the prefix is a second answer to a caller already refused, and an object
   number bearing something readable is not this population.

## §9.6.4's step b) meets the first condition outright

Table 110 makes `/CharProcs` required and says each value "shall be a content stream that
constructs and paints the glyph for that character", and §9.6.4's four steps then say what a
processor does with a character code. Step b), verbatim:

> If the name is not present as a key in CharProcs , no glyph shall be painted.

That is a `shall` about exactly the residue. A key the damage took is a key that is not present,
and the standard has already said what that means: nothing is painted. **Nothing stands in place of
the producer's marks**, which is ADR 0106's substitutive-failure test and the one this project
refuses on.

The second condition is met by the same table and §9.6.5.3. Table 110's `/Encoding` cell requires
"an encoding dictionary whose Differences array shall specify the complete character encoding for
this font", and §9.6.5.3 makes that entry the whole of a Type 3 font's glyph selection — and it is
in the **font dictionary**, which is whole, or `Type3Font::read` would not have got this far. So the
glyph names step a) can produce are a list, intersecting it with the prefix's keys is a list, and
`CharProcsDamage::undescribed` is that list rather than an estimate.

And a third thing follows that is not one of the conditions but is what makes the outcome legible
on the page: **no advance moves.** Table 110's `/Widths` is an array of `/LastChar − /FirstChar + 1`
numbers in the font dictionary, so a code whose glyph paints nothing still advances by the number
its producer wrote. The marks that survive are in the producer's own places, and the ones that do
not are holes.

## Why this does not relax round 896's refusal

ADR 0836 refuses a damaged **font program** — a `/FontFile2` whose deflate data is whole and whose
RFC 1950 check value disagrees — with a sentence of its own, and it was implemented, measured and
declined rather than assumed. The reason it is refused and this is not is the *first condition*,
and it is the whole difference:

- A font program that loads draws **through §9.6.5.4**, whose routes run out and whose closing
  permission is that a processor "may supply a mapping of its choosing". On `issue13316_reduced.pdf`
  that tier draws **A C E F** where `pdftoppm` draws five CJK glyphs, reporting nothing. Those are
  *other marks in the producer's places* — the failure ADR 0106 named and ADR 0459 refuses.
- A `/CharProcs` prefix has no such tier and cannot acquire one, because §9.6.4's own step b) closes
  the case before any fallback could apply, and §9.6.5.3's NOTE adds that "Type 3 fonts do not
  support the concept of a default glyph name". `crate::type3`'s module comment has carried that
  sentence since the eighth session, where substituting a Type 3 font drew ZapfDingbats for
  `french_diacritics.pdf`'s `/a192`.

**So the counter-example is not overruled; it is the other side of the same test.** A round that
wanted to admit the font program would still have to answer §9.6.5.4, and nothing here does.

## The honest limit, and the one thing this does not exclude

The subset is order-dependent: a producer who wrote the same `/CharProcs` in a different order,
damaged at the same byte, would give a different set of glyphs. That is §7.3.7's sentence biting,
and it is worth being exact about what it bites. **It makes *completeness* depend on the write
order, and never *correctness*** — under any order, every glyph drawn is the producer's own
description for the name the encoding gave, and every glyph not drawn is step b)'s "no glyph shall
be painted". That is a much weaker dependence than ADR 0784's consumer has, where the order decides
whether the page's sheet is the producer's or this reader's guess.

The one thing not excluded is ADR 0787's: a key **manufactured** out of the damage. The witness has
one — the prefix's forty-first entry is `d:\xff\xf1\xc0…`, a name lexed out of another stream's
compressed bytes, with a string for a value. It costs nothing here because §9.6.4's step b) only
ever looks up a name step a) produced, and step a) reads the whole font dictionary's `/Encoding`; a
manufactured key can only be reached by colliding with an encoded name **and** carrying a value
that resolves to a stream. That is three coincidences, it is the same residual ADR 0784's own door
carries in a sharper form (there the manufactured entry is the one the recovery *discriminates* on),
and it is stated here rather than discovered later.

## What was built

`Type3Font::read` asks `char_procs`, which returns the whole dictionary where the reference
resolves to one and otherwise — and only where `Document::get` answered null — takes
`Document::damaged_dictionary`'s entries and the two facts a report needs. `CharProcsDamage` then
counts the encoding's names against the prefix, and `CharProcsDamage::detail` words the report:
the byte, the parser's reason, and how many of the encoding's names have a description here and how
many do not.

The report fires **once per page and not once per `Tf`**, which took the font cache's two levels
apart. `cached_font`'s first level is the interpretation's own map and is silent; its second is
`FontCache`, which outlives the page — so a font kept from page one and served to page two would
have drawn part of a `/CharProcs` in silence, which is trap 5's own failure. Serving from that level
now notes and inserts into the first.

## The witness

`cairo-85141-0.zip-3.pdf` page 1 is a Finnish university problem sheet. Object 76's prefix holds
**41 entries** — 39 references to glyph descriptions, `/a112` cut to a bare integer, and the
manufactured key — and the font's `/Encoding` states **49** names, so 39 draw and 10 paint nothing.
The page renders its paragraph and its numbered list with those ten letters as holes:
*L en oha joi 4, Viikko 40* where the producer wrote *Luentoharjoitus 4, Viikko 40*.

Its ink is **4.63038**, and the four references do not agree with each other:

| | ink at 72 dpi | what it does with object 76 |
|---|---|---|
| ours, before | 1.70076 | discards it; §9.6.4's `NoCharProcs` |
| **ours, after** | **4.63038** | draws the 39 descriptions the file states |
| `pdftoppm` | 1.75734 | discards it |
| `mutool draw` | 1.66218 | discards it |
| `hayro` | 1.69921 | discards it |
| `ghostscript` | 8.93729 | draws glyphs for names whose descriptions are **not in the file** |

The ten missing descriptions are physically absent — their bytes are another object's stream data,
checked by hand at offset 20 678 — so whatever ghostscript puts in their place is not the
producer's. Three readers discard an object the standard nowhere tells them to discard, and one
supplies marks the standard nowhere tells it to supply; §7.3.7 states no recovery either way, and
Annex C's one recovery sentence is about the cross-reference table and is informative. This is
therefore principle 5's ordinary case rather than a consensus to move toward: **the disagreement is
about a recovery no clause states, and what a clause does state — step b) — is what this reading
follows.**

## Population

`examples/damaged_dictionary_consumers` over 90 535 documents (`corpus-cache/tika-issue-tracker`,
`corpus-cache/openpreserve`, `doc/pdf.js/test/pdfs`, the four `doc/corpora/` submodules, and all
65 944 of `corpus-cache/safedocs/cc-main-2021-31`): 311 documents hold a damaged dictionary, 953 in
all; 99 documents hold one that a reference out of an object that *parses* names, 328 such
references over 58 distinct keys; and **exactly one of the 328 is a Type 3 font's `/CharProcs`** —
the witness. ADR 0867 has the table and what the other 327 are.

**One document is the population, and that is stated rather than dressed up.** What makes the change
worth its round is not the count: it is that the decision is the one ADR 0784 deferred, that the test
above is what the other 327 will be judged by, and that the page in question goes from unreadable to
readable with every mark on it the producer's own.

## What would reopen it

A reading of §9.6.4 under which an absent `/CharProcs` key draws *something*. There is none: step b)
is a `shall`, and §9.6.5.3's NOTE closes the only route a default could come by.
