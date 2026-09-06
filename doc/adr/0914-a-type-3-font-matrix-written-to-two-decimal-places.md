# ADR 0914: a Type 3 font matrix written to two decimal places

**Status**: accepted, in the nine-hundred-and-thirty-seventh session.
**Clauses**: ISO 32000-2 §9.2.4, §9.6.4 (Table 110), and §8.3.4 NOTE 3 at the far end of it.

## What the file says

`corpus-cache/tika-issue-tracker/batch5/pdfcpu/pdfcpu-52-2.pdf` states thirty-six Type 3 fonts,
and every one of them states

```
/FontMatrix [0.00 0 0 -0.00 0 0]
```

— the bytes, read out of the object stream rather than out of a pretty-printer, which matters
because `qpdf --qdf` renders the same array and a round could take the zeros for its own. Two
neighbouring documents in that directory state the same array sixteen and five times. The
`/Widths` beside it run to 2048 and the `/FontBBox` to −1951, so the glyph space these fonts draw
in is a 2048-unit em and the matrix that states it is `0.00048828…`. **The producer wrote the
matrix to two decimal places.** §8.3.4 NOTE 3's own example — "if a matrix contains a, b, c, and d
elements that are all zero" — reached by rounding rather than by anybody scaling by zero.

## What this tree did with it

It ran the glyph descriptions. §9.6.4 says the CTM a description executes under "shall be the
concatenation of the font matrix … and the text space", so every glyph of every string landed
under a rank-zero matrix: the whole of glyph space carried onto the single point the text position
put it at. `pdf_render` builds those marks, no backend paints one — a mark with no area covers no
device pixel — and `pdf_model` reported, once for the page:

```
NoninvertibleMatrix { commands: 966 }
```

of 1358 commands. That report is true and it is about the wrong thing. It tells a reader that some
matrix on the page had no inverse; it does not tell them that the page's **text is not drawn**,
and `Unsupported::Text`, which is this tree's word for exactly that, was not raised, because a
font *had* been found. The reader gets a page with its body text missing and a sentence about
matrices.

## The decision

**A `/FontMatrix` with no inverse refuses the font, in `Type3Font::read`, as
`Type3Error::UninvertibleFontMatrix`.**

The warrant is §9.2.4, which says what the entry is for and says it with a `shall`:

> for a Type 3 font, the transformation from glyph space to text space shall be defined by a font
> matrix specified in an explicit FontMatrix entry in the font

A matrix with no inverse defines no such transformation — it is not a mapping onto text space at
all, it is a mapping onto one point of it — and Table 110 requires the entry "mapping glyph space
to text space". So the entry is present in form and absent in substance, which is the case
`Type3Error::NoFontMatrix` already covers, and the argument written on that variant carries over
word for word: the common `[0.001 0 0 0.001 0 0]` is available and is **not** substituted, because
a font drawing on a 1-unit grid would then be a thousand times too small in silence. Nothing is
guessed here either. What changes is only which of the two true sentences the reader is told:

| | before | after |
|---|---|---|
| the report | `NoninvertibleMatrix { commands: 966 }` | `Font { detail: "font /F1 is a Type 3 font whose /FontMatrix [0 0 0 -0 0 0] has no inverse, so it states no transformation from glyph space to text space" }`, one per font, and `Text { operations: N }` |
| the display list | 966 marks with no area | those marks are not built |
| the page | blank where its text was | blank where its text was |

**The pixels do not move**, and that is a claim this round measured rather than reasoned to: the
marks were unpaintable by construction, so removing them is expected to change nothing, and the
three witness documents' ink is identical before and after to the digit the instrument prints.
`doc/checks/fixed-documents.toml` carries the row that keeps it so.

**The condition is *no inverse* rather than *six zeros*, deliberately.** A matrix that flattens
glyph space onto a *line* states no glyph shape either, and `Transform::invert` is the predicate
the rest of this tree already asks of a transform — `pdf_render::paint_space` asks it of a mark,
`split_collapsed_fill` asks it of a placement, `examples/singular_transform_census` asks it of a
population. A second predicate that agreed with those only on the witness would be a second thing
to keep in step.

**And it is a refusal of the *font*, not of the page.** Every other font on the page still draws,
which is ADR 0482's rule one clause upstream: a defect in one resource costs that resource.

## What this does to ADR 0482's open residual, said plainly

`doc/todo/11` item 8 holds §10.7.4's mark for a shape its *transform* collapsed, and one of the
three things it says a round taking that owes is **a witness, which there is not**: "[m]ost
matching pages state *one* such mark among thousands, and the two whose count is large are a
garbage stream and a page whose 280 are images."

The instrument that settles that is `examples/singular_transform_census`, and this round is the
first to have run it over **every corpus on this disk** rather than over the pages somebody had
looked at. Page one alone, 89 286 documents: **465 985 such marks on 35 documents**, four of them
at 333 327, 49 715, 39 895 and 39 895 apiece. So the bullet was false, and it was false before this
round.

**What this decision takes out of that population is three documents and 1737 marks — 0.37% of
it — and none of the four largest.** The census re-run over exactly those 35 documents afterwards
reports 32 documents and 464 248 marks, with `pdfcpu-39-1`, `pdfcpu-39-6` and `pdfcpu-52-2` gone
and every other count unchanged to the mark. Item 8 keeps its witness, and it now has a real one
rather than a sentence saying there is none.

**Where the two clauses meet is worth stating anyway**, because the three that left did so for a
reason and not by accident: §9.2.4 is about an entry that fails to state a glyph space, and it
fires *before* any shape exists; §10.7.4 is about a shape that does exist and lands on a pixel
boundary. The more specific clause answers first. A page whose collapse is a `cm` rather than a
font matrix — which is what the other 32 are — is untouched by this and is still item 8's.

## What it does not decide

Whether poppler is right. On `pdfcpu-39-6.pdf` poppler paints a pixel at 218–234 of 255 at each
of the collapsed points, and `mutool draw` and this tree paint none — a live three-way split, with
§10.7.4's "no shape ever disappears" on poppler's side and §8.3.4 NOTE 3's "unpredictable" on the
other. Under this decision the question never arises for *this* file, because the glyphs are not
drawn at all; it arises for the next file whose collapse is a `cm` rather than a font matrix, and
`doc/todo/11` item 8 is still where it lives.
