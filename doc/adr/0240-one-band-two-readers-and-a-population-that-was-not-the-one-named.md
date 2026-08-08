# ADR 0240 — One band, two readers, and a population that was not the one named

Status: accepted, 2026-08-08 (session 403).

## Context

`doc/todo/22` held three of §12.7.4.3's remaining edges and asked this round to weigh them against
each other before choosing. Two of the three came with a claim about a population and neither claim
had a number behind it; the third had no witness at all and says so.

- **A field's baseline** reads Table 120's `/Ascent` and `/Descent` under a guard of
  `variable_text`'s own, `ascent > 0 && descent < 0`, where `pdf_font::measured_extent` asks
  whether the pair could be a measurement of a face (ADR 0216). ADR 0216 named the divergence and
  left it deliberately: this one *draws*, so sharing the band moves pixels, and that round had
  measured none.
- **§12.7.5.4's list box** is refused, because the clause states which items are selected and
  states nothing whatever about how a selection looks. Session 398 left that noted as the thing
  that makes a page light.
- **A composite `/DA` font** needs §9.7.6.2's codespace ranges inverted. Zero corpus documents, and
  it stays that way below.

`doc/todo/13`'s rule is that the population is measured before the code is written, and it is what
made §10.5 a small change rather than a brave one. It decided all three of these.

## The census, first

`crates/pdf-model/examples/variable_text_census.rs`, over the same 974 pdf.js documents every other
gate uses. It takes the field types, their flags and their widgets from `pdf_model::form::fields` —
the program's own reading — so the only rule spelled a second time in it is the guard being
replaced. **964 documents open; 114 carry a widget or an annotation §12.7.4.3 reaches; 11 of those
set Table 224's `/NeedAppearances`.**

| | count |
|---|---|
| widgets of a text field or a combo box | 622 |
| of those, with no `/AP` `/N` stream — the constructing path | 305 |
| §12.5.6.6 free text annotations, sent to the same layout by their own clause | 73 |
| **of all 695: draw in a font with no descriptor** | 59 |
| draw in a font whose descriptor states neither entry | 382 |
| **state a pair both rules read to the same two numbers** | **254** |
| state a pair the old guard believed and the band refuses | **0** |
| state a pair the old guard refused and the band believes | **0** |
| state a pair neither rule believes | 0 |

**Sharing the band moves nothing on this corpus**, and the reason is the finding rather than the
count: `doc/todo/22` said the change was owed to "the 42 font dictionaries that write a `/Descent`
without its sign", and **those 42 are a different population**. They are `font_metric_census`'s —
font dictionaries a *page's* content streams draw with, which reach `vertical_extent` and a
selection highlight. `Metrics::read` only ever sees a font Table 224's `/DR` defines and a `/DA`
names, and no such font in the corpus states a pair the two rules disagree about. A todo file's
number is a claim about this tree like any other, and this one had been carried from one census to
a question that census did not answer.

That is what makes the change worth making rather than not: it is a **coverage** change, on the
specification's denominator, with a measured robustness cost of zero — and it can be made without
the page-by-page reading a moving baseline would have owed.

### The list box, and the count that closed it

**10 list-box widgets over 8 documents. Every one of them states an `/AP` `/N` stream, and not one
of the 8 sets `/NeedAppearances`** — so the refusal never takes a mark off any of the 974, and the
"page it makes light" does not exist here. Where the refusal *does* fire, `appearance::regenerate`
returns the stored stream with the shortfall named, so even a regeneration leaves the file's own
artwork standing. `annotation-choice-widget.pdf` draws all four of its choice widgets, selection
highlight included, from the file's own streams, with `unsupported []`.

The census also corrected the ledger row's own sentence, which is the second thing it was for:
§12.7.5.4 said "[n]o corpus document reaches either path with no appearance stream", and **one
combo-box widget of 26 does** — the read-only one in the same file. It states no value, so the
layout returns nothing to draw and owes nothing. A sentence that was nearly right for four hundred
sessions is now a number.

## Decision

**Share ADR 0216's band. `variable_text::Metrics::read` calls `pdf_font::measured_extent` and keeps
`DEFAULT_ASCENT`/`DEFAULT_DESCENT` for a pair it refuses.**

**Leave §12.7.5.4's list box refused, and record why as a choice rather than as a gap.**

**Leave the composite `/DA` font, and say what it is waiting for.**

### Why one rule rather than two

The two functions were reading the same two entries of the same table and disagreeing about what
those entries can say. §9.8.1 puts every dimension of Table 120 in glyph space and §9.2.4 fixes
what one of those is, so both readers are reading multiples of the font size — the same quantity,
measured for two different purposes. Two guards on one quantity is two chances to be wrong about
it, and it showed: `Metrics::read`'s guard believed `/Ascent 4000 /Descent -1140`, which is a line
five ems tall and puts a 24-point field's baseline *below its own rectangle*, where the clause's
own clip then takes most of the value away; and it refused `/Ascent 905 /Descent 211`, which is
Arial's real metrics with Table 120's sign convention broken rather than its measurement withheld.

Neither of those is a new argument — both are ADR 0216's, and the only thing that had kept them
from this function is that this one draws. The census is what removes that objection.

### What the fallback stays

A pair the band refuses gets `DEFAULT_ASCENT` and `DEFAULT_DESCENT` — 0.75 and −0.25 of an em —
and **not** `vertical_extent`'s em box of 1.0 and 0.0. The two are answers to different questions
and it is worth stating rather than assuming: `vertical_extent` answers *how tall is this line*,
for a highlight laid over glyphs somebody else positioned, and §9.2.2's nominal line is the defined
quantity there; `Metrics::read` answers *where in its box does this field's text sit*, which the
standard hands back — "positioning values [the processor] determines to be appropriate" — and
splitting the em three-to-one is the choice those two constants have recorded since the
sixty-second session. Answering a refused pair with the em box would put every such field's
baseline on the bottom edge of its own rectangle.

### Why the list box stays refused

The clause is not silent about a list box; it is silent about the one thing an appearance needs.
Table 234's `/Opt` is "[a]n array of options that shall be presented to the user" and Table 233's
`Sort` row says "PDF readers shall display the options in the order in which they occur in the Opt
array" — so *which* items and *in what order* are both stated. What is stated nowhere is how a
selected item differs from an unselected one: no highlight colour, no rule, no inset, nothing. So
drawing the options would draw all of a list and none of its selection, which is this tree's own
asymmetry from ADR 0112 — a stand-in draws where it can draw the whole value and declines where it
can draw part — pointed at a different clause. A list box drawn as eight lines of unmarked text
says the field has no selection, which is the opposite answer rather than a near miss.

**And the `shall` is carried out, away from the page.** Since ADR 0235 the items, their export
values, `/TI` and the clause's own rule for which are selected cross to a host as
`pdf_model::form::ChoiceControl`, in the array's order. A list a renderer may not invent is exactly
a list a host should draw, and the requirement that the options be presented is met by the
component that has a widget to present them in.

This is recorded as **a choice** in the sense `CLAUDE.md` asks for, and with the honest limit on
it: if a later round finds a document whose list box has no appearance stream, the choice is worth
revisiting with that document in front of it. Today there is none, and the number is 0 of 974.

### Why the composite `/DA` font is not this round's

§9.7.6.2's codespace ranges would have to be inverted to turn a character back into the code a
composite font wants, and `variable_text::set_in` refuses such a font by name today. Zero corpus
documents state one, and trap 8's rule cuts the other way as well: a corpus cannot rank a
requirement no document exercises, so the *report* is what keeps it honest until a witness or a
spec-track round reaches it. It stays in `doc/todo/22` with that stated, rather than being closed
by attrition.

## The gate, and that it would have failed before

Four tests in `crates/pdf-model/tests/variable_text.rs`, built on a fixture whose `/DR` font states
a descriptor the test chooses — which needed a builder, because every other fixture in that file
names one of the standard 14 and those have no descriptor at all. The arithmetic each checks is
one arithmetic: §12.5.4's default border insets `/Rect [20 20 180 80]` to a box 58 points tall, a
single line is centred in it, and the baseline is therefore `50 − 12(A + D)` for a pair `(A, D)` in
ems. The value is `HI`, with no descender, so a difference in the topmost inked row between two
fixtures drawn in the same substituted face is exactly the difference between their baselines.

Checked by putting the old guard back and running them:

| test | before | after |
|---|---|---|
| `a_fields_baseline_ignores_a_descriptor_that_cannot_be_a_measurement` | **fails**: `/Ascent 4000 /Descent -1140` draws 28 rows from where a silent descriptor does | passes |
| `a_fields_baseline_reads_a_positive_descent_as_the_depth_it_states` | **fails**: `/Descent 211` and `/Descent -211` are 2 rows apart | passes |
| `a_fields_baseline_reads_a_zero_descent_as_a_face_with_no_descenders` | **fails**: `/Ascent 1000 /Descent 0` moves the baseline 0 points, not 6 | passes |
| `a_fields_baseline_follows_a_descriptor_the_band_believes` | passes | passes |

The fourth is the control and it is what makes the other three mean something: a band that threw
away true statements would fail it, and 254 of the corpus's `/DR` descriptors are in exactly that
case.

**And the page was looked at, because a count is not a picture** (trap 1). The same three
descriptors rendered at 4× under the old guard and under the band: `/Ascent 4000 /Descent -1140`
draws its value **cut in half along its own baseline** before the change, because
`open_marked_content` writes the clause's `re W n` around the field's box and the text has been
placed below it; after the change it draws exactly where a descriptor stating nothing draws. That
is the defect the arithmetic predicts, and it is the one thing about this round that no gate could
ever have shown, because no corpus document states such a pair.

## Consequences

- One rule decides what Table 120's two entries can say, for both of the things in this tree that
  read them. A descriptor cannot be a measurement to one and not to the other.
- **No gate moved**, which was the prediction the census made and not a hope: the corpus, the
  oracle's 1794 verdicts, quorra's 957 pages and the text gate all reproduced line for line, and
  the only figures that changed are the ones this round's own four tests, its citations and its
  quotations moved.
- `doc/todo/22` loses two of its three entries and keeps the composite font with its zero.
- A census example exists for §12.7's drawing population, which is the thing this round found
  nothing had: `field_flag_census` counts flags, `font_metric_census` counts the fonts a *page*
  draws with, and neither could say which fonts a **field** is set in.
