# 1002 — A ranking applied per font on one route and per code on the other

Session 981. Status: **accepted**. `doc/todo/21` §1's "per-character fallback", which that section
had written about a second *face* and which turned out to have a second meaning the clause states
outright — one this tree implemented for the readback and not for the page.

## The clause, read again

ISO 32000-2 §9.10.2 opens:

> A PDF processor can use these methods, in the priority given, to map a character code to a
> Unicode value.

and closes:

> If these methods fail to produce a Unicode value, there is no way to determine what the
> character code represents in which case a PDF processor may choose a character code of their
> choosing.

The first method is the font's `/ToUnicode`; the third is, for a composite font over one of the
registered collections, the collection's own `registry-ordering-UCS2` table reached through the
font's `CMap` (steps a to e). Both sentences are about *a* character code. A method that "fail[s]
to produce a Unicode value" has failed for the code it was asked about, and a `/ToUnicode` that
omits a code has failed for that code however many others it answers. The ranking is per code.

For a font whose program is embedded that is a statement about the readback. For a **substituted
composite font** it is the glyph selection algorithm, because §9.7.4.2 leaves nothing else: with
the program absent "CIDs shall not participate in glyph selection", and the processor "shall
select glyphs by translating characters" — so a character is the only address a substitute face
answers to, and which character a code is comes from §9.10.2 and nowhere else.

## What the code did

`LoadedFont::text` — the readback — walked the clause's order per code: the `/ToUnicode`, then the
collection's table through the `CMap`'s CID, then the glyph name. It had done so since session
156, when the collection tables arrived.

`CodeMapping::Substituted` — the drawing — carried a `text: Box<Meaning>`, and `Meaning` was an
enum of **one** table: `ByCode(ToUnicode)` or `ByCid(ToUnicode)`, chosen once in `load_composite`
by `if direct.is_empty() { collection } else { ByCode(direct) }`. A font whose producer wrote a
`/ToUnicode` that states some codes and not others therefore drew the stated ones and, for the
rest, asked nothing: `char_for` answered `None`, `outline` answered `None`, and the page counted a
code reaching no glyph — *in silence*, by ADR 0152's arithmetic — while the readback beside it
named the character through the table the drawing had never opened. Two routes over one clause,
disagreeing about what the file said; a page that could read back a character it did not draw.

The enum's own doc comment said why it was an enum: the two tables are keyed differently, by code
and by CID, and folding one into the other would mean enumerating every code a `CMap` defines.
That is true and it argues for keeping two tables, not for keeping one.

## What changed

- `CodeMapping::Substituted` holds no table. The font already holds both — `to_unicode` and
  `collection`, which every composite font had for its readback — and the variant's copy was the
  second reading of the pair. `Meaning` is deleted; `composite::collection_meaning` is
  `collection_table` and answers the `ToUnicode` keyed by CID that it always was.
- `LoadedFont::substituted_character` is the per-code reading: the `/ToUnicode` is asked first and
  is final wherever it **states** the code — a sequence included, because a producer's statement
  that a code is a ligature is an answer that addresses no single glyph, not a failure that hands
  the code to a table outranking nothing — and the collection is asked, through §9.10.2's step (a),
  only where the producer's table says nothing at all. `ToUnicode::states` is the question that
  distinction needs, which `char_for` could not ask: it said `None` for an omitted code and for a
  stated sequence alike.
- The refusal is unchanged in effect and moved in form: a substituted composite font with no
  `/ToUnicode` *and* no carried collection is still `FontError::NoSubstitute` with
  `collection_gap`'s four-way reason, and it is now the one case with no question left to ask
  rather than the `else` of a choice.
- `load_composite` parses the `/ToUnicode` once where it parsed it twice, and the collection table
  once where it parsed it once or twice depending on the branch.

## The population, measured before it was assumed

`crates/pdf-font/examples/partial_to_unicode_census.rs` counts the *shape* — a `Type0` font whose
descendant embeds no program, whose `/CIDSystemInfo` names a collection this binary carries a
table for, and whose `/ToUnicode` states at least one mapping — and, for each, how many codes the
font's `CMap` makes addressable that the `/ToUnicode` omits and the collection names. That is
capacity rather than incidence, deliberately: which codes a page *shows* is a content stream's
business and `pdf-font` reads none. Every object the cross-reference table names and every
dictionary nested inside one is walked, for trap 25's reason.

| corpus | opened | `Type0` | no program | carried collection | with `/ToUnicode` | shape |
|---|---|---|---|---|---|---|
| `doc/pdf.js`, `doc/corpora`, `corpus-cache/openpreserve` | 1 503 | 886 | 58 | 26 | 0 | 0 |
| `corpus-cache/tika-issue-tracker` | 22 906 | 38 341 | 1 987 | 1 235 | 23 | 2 |
| `corpus-cache/safedocs` (CC-MAIN-2021-31) | 65 720 | 194 330 | 7 940 | 7 432 | 6 | 2 |

Four documents of 90 129 opened: `sumatrapdf-1550-0.pdf` (7 fonts), `sumatrapdf-LINK-1532-0.pdf`
(16), `2514637.pdf` (5) and `3621086.pdf` (1). Their capacities are in the hundreds of thousands of
codes because a `UniJIS-UTF16-H`-shaped `CMap` addresses the whole of a collection and the
producers' tables state a few hundred codes each.

**Incidence on those four is zero**, measured by tracing every page of each with
`PDFVIEWER_TRACE_MISSING_GLYPH=1` under the old route and the new one: identical counts on all
four (63, 171, 0, 0 lines, none of them from a font of this shape). The codes those pages show are
the codes their producers' tables state. So no silence line of the corpus gate moves, and this is a
defect no document on this disk carries — which `CLAUDE.md` is explicit about: "a count that does
*not* move is not evidence that nothing happened".

## The pin

`loading.rs::a_code_the_to_unicode_omits_is_selected_through_the_collection` is a hand-built
`Type0` over `UniJIS-UCS2-H` and a non-embedded Adobe-Japan1 descendant whose `/ToUnicode` states
exactly one mapping, `<3042>` → U+3044 — deliberately *not* what the collection says CID 843 is,
so that which table answered is visible in the glyph. Three codes in one string: `<3042>` stated,
`<3044>` and `<3046>` omitted. The test asserts relations and no glyph index, because which face
stands in is this machine's business (§9.5 NOTE 5, ADR 0133): the omitted code reaches a glyph;
the stated code's glyph equals the omitted one's, because the producer said い and outranks the
collection's あ; the third differs from both; the readback is `いいう`; and where the machine has no
face covering あ the load refuses and the test prints why rather than passing.

Calibrated both ways in the same session: with `substituted_character` reading the `/ToUnicode`
alone once non-empty — the route this ADR replaces — the omitted code reaches no glyph and the
first assertion fails; with the collection alone, the stated code draws あ and the equality fails.

## Why this is not the per-character fallback `doc/todo/21` §1 owes, and why it was mistaken for it

§1 is about a **face**: a document whose substituted composite font shows a character the chosen
face lacks, and a second face asked for it. That mechanism was built and reverted in session 256
because every assertion about it is an assertion about which faces one machine has, and it is
still owed with no witness. This ADR is about a **table**: which character a code is, before any
face is asked. The two share the words "per character" and nothing else, and §1's heading had
absorbed both; `doc/todo/21` now states them apart.

The general form is worth more than the instance, and it is trap 13's shape in a different place:
**where two routes read one clause, the test is whether they read it with the same granularity**,
not only whether each cites it. Both routes here cited §9.10.2 correctly, both quoted its ranking,
and one applied the ranking to a font while the other applied it to a code. A grep for the clause
number finds both and says nothing; what found it was reading the enum's constructor beside the
readback's loop and asking why one had an `if` where the other had a fall-through.

## What this does not settle

- **A `/ToUnicode` that states a code and the collection that names it differently** is decided
  by rank — the producer's table wins — and no corpus document exercises the disagreement. The
  fixture does, on purpose; a real one would be worth having.
- **`Identity` orderings** are untouched: with no collection table the second method has nothing
  to read and a `/ToUnicode` that omits a code leaves that code with §9.10.2's own "there is no
  way". `collection_gap` says so, and the eighteen `doc/pdf.js` documents refused on §9.7.5.2's
  prohibition are exactly as they were.
- **The remainder of `doc/todo/21` §1** — a second face — is neither closer nor further.
