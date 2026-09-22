# 1179 — Table 167 states two answers and the output decides which

Session 1171. Status: **accepted**. Builds on [ADR 1173](1173-what-the-output-is-for-is-an-input-a-host-supplies-and-8-11-4-4s-adjustment-sets.md),
which made what the output is for an input, and answers the caller its §5 was waiting for.
Amends the reading in [ADR 0934](0934-a-watermarks-media-is-the-page-and-the-clause-says-so.md) at
the one point that clause leaves open.

ADR 1173 built `pdf_model::optional_content::Purpose` and `ViewState::set_purpose`, and nothing
in the tree ever stated `Purpose::Print`. This ADR is what happens when something does: two more
clauses turn out to have been reading half a table.

## 1. The table has two device columns and this reader had one

§12.5.3's Table 167 states bit 6 as "do not render the annotation on the screen" and bit 3 as "if
set, print the annotation when the page is printed unless the Hidden flag is also set". Those are
two rules about the same annotation for two different devices, and which one applies cannot be
read out of the annotation dictionary at all — it is a fact about what is being produced.

`annotation::displayed` consulted bit 2 and bit 6 and nothing else, so every page this program
ever produced was decided by the screen's column. The §12.5.3 ledger row had carried the sentence
"Print is a printing decision this viewer does not yet make" for hundreds of sessions, and the
row's own census counts bit 3 on **316 383** of the crawl's annotations — by far the most-stated
flag in either corpus. Half a table, over the largest population of unimplemented annotation
semantics in the tree.

`AnnotationView` now carries the purpose, and `displayed` branches on it:

- **On paper**: not `Hidden`, and then bit 3's three sentences and nothing else. `NoView` is
  deliberately not consulted, because its own row states the consequence — "[t]he annotation may
  be printed (depending on the setting of the Print flag)" — and bit 9, `ToggleNoView`, goes with
  it, because inverting `NoView` "for annotation selection and mouse hovering" is a sentence about
  a pointer no printed page has.
- **On a screen**: exactly what it was.

`Hidden` suppresses on both, which the table says twice: its own row is unconditional, and bit 3
repeats it as an exception to itself. §12.6.4.11's hide action is the same bit on both devices —
the clause's own words are "by setting or clearing their Hidden flags" — so an action that cleared
it beats the file wherever the page is going.

## 2. The third sentence is what decides the population

> If the annotation does not contain any appearance streams this flag shall be ignored.

An annotation stating no `/F` at all has bit 3 clear, and bit 3's second sentence is "[i]f clear,
never print the annotation". Reading the first two sentences alone would take every appearance
this crate *constructs* off every printed page — a note, a square, a link border, a field with no
`/AP` — which is most of what a viewer draws over a page that was written by a careless producer.

`annotation::contains_appearance_streams` is that sentence's condition, and it asks the question
the clause asks: does the **annotation** contain appearance streams, not does the state it is
currently showing resolve to one. Table 170 makes `/N`, `/R` and `/D` each "a single appearance
stream or an appearance subdictionary", so an annotation whose `/AS` selects nothing still
contains the streams the subdictionary holds, and the flag applies to it. One level deep, because
Table 170 is one level deep.

## 3. An export takes the screen's reading, and that is a decision

Table 167 names a screen and a printed page. An export is neither, so there is no third row to
apply, and `Purpose::Export` therefore keeps the screen's reading unchanged — which is also what
`pdf_transform`'s `render` verb was already doing before this round and continues to do.

It is written down as a decision rather than left as a fallthrough because the alternative is
arguable: an exported raster is not a screen either, and a reader could take bit 3 to govern
anything that is not a display. The reason not to is that it would let a document decide what a
file this program writes contains, on the strength of a sentence whose subject is paper — and the
population is the same 316 383 annotations, pointing the other way.

## 4. §12.5.6.22's media, and the one term the print side changes

ADR 0934 carried out §12.5.6.22 for the screen, on that clause's own sentence: "[w]hen displaying
a watermark annotation on-screen, interactive PDF processors shall use the dimensions of the media
box". Table 193 states the other branch and states it as a condition rather than as a default:
"[i]f the dimensions of the target media are not known at the time of drawing, drawing shall be
done relative to the dimensions specified by the page's `MediaBox` entry".

So the media box is not a fallback this tree chose. It is what the clause requires of a screen,
and what it requires of anybody who does not know the sheet. Printing onto a sheet somebody chose
is the case where that condition stops holding, and `annotation::target_media` is the whole of the
substitution: under `Purpose::Print` with a paper stated, the paper's rectangle; otherwise the
media box.

**The paper arrives as a rectangle in default user space rather than as a width and a height**,
and that is the clause's third sentence taken seriously:

> given a matrix B that maps a scaled and rotated page into the default user space, a new matrix
> shall be computed that cancels out B and translates the origin of the media (e.g., printed page)
> to the origin of the default user space

For every caller this program has, B scales nothing and rotates nothing: a page is placed on the
sheet at its own size and its own orientation. What is left of the sentence is the translation
between two origins, and a rectangle carries it — `fixed_print` measures Table 194's percentages
from the media's own corner already, and has since ADR 0934.

**The two bullets after the EXAMPLE are where B stops being the identity**, and neither is a thing
this program offers: page tiling and n-up printing, each with its own placement rule in the clause.
Whoever adds a scale mode, an n-up composition or a tiling owes the rest of that sentence;
`target_media`'s doc comment says so by name, so the debt is at the function rather than in a list.

## 5. What was verified and what was not

- Table 167's cells are quoted from `doc/md/ISO_32000-2_sponsored_EC3.md` and the truth table in
  `pdf-model/tests/print_intent.rs` is derived from them row by row, with the screen's column
  asserted beside the paper's on every row so that neither alone can pass.
- `/FixedPrint`'s three arms — screen, paper with a sheet, paper without one — are the
  discriminating set for §12.5.6.22's two branches, and each number is one multiplication.
- The fixtures are hand-built and say so (trap 8). `/FixedPrint` is essentially absent from the
  corpora — `pdf-model/examples/fixed_print_census` finds one document — and a corpus cannot hold
  the pair of files that differ in one flag and in nothing else, which is the only shape that
  discriminates here.
- **Not verified against another renderer, and deliberately.** Every expected value above comes
  from a cell of Table 167 or a sentence of §12.5.6.22; principle 5 makes agreement evidence and
  never the target, and this is a case where the corpus offers no evidence either way.

## 6. The construction sites, and the one line in a neighbour's file

`ViewState::annotation_of` takes an `Option<ObjectId>` where `annotation` took an `ObjectId`,
because a direct annotation dictionary in a page's `/Annots` array has no identity a person could
have interacted with — and the purpose and the paper are facts about the **output**, not about the
annotation. `AnnotationView::default()` for such an annotation would have decided bit 3 by the
screen's row on exactly the annotations a producer wrote inline. `content::annotations.rs` calls
the new method; it is one line, and it is the only line outside this decision's own files.
