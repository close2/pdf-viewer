# ADR 0407 — A refusal sized by the one thing a clause does not state

Status: accepted.
Session: the five-hundred-and-seventy-first, a general improvement round.

## 1. What this decides

**§12.7.5.4's list box draws.** Its `/Opt` options are laid out one per line, in the array's own
order, from Table 234's `/TI` down, in the `/DA` font at `/Q`'s justification — and the one thing
the clause states no appearance for, a mark saying which of them `/V` selects, is reported beside
the drawing instead of taking the drawing down with it.

Three subsidiary decisions come with it, each recorded below with its argument: how `/TI` is read
when it names an index the array does not have, why a list's lines are never rewrapped, and why
auto-sizing takes no exception for this shape after a first attempt gave it one.

## 2. What the clause states, which is most of it

The refusal this replaces was one line and one sentence of reasoning: "the clause states which
items are selected and states no highlight, so `variable_text` refuses it". Read past that
sentence and the clause states the rest of the control outright.

§12.7.5.4, of the array itself:

> The Opt array specifies the list of options in the choice field, each of which shall be
> represented by a text string that shall be displayed on the screen.

Table 234 says the same of the entry — "[a]n array of options that shall be presented to the
user" — and gives the absent case its own answer: "[i]f this entry is not present, no choices
should be presented to the user."

Table 233 bit 20 fixes the order, and it is one of the few rows in clause 12 whose `shall` names
its addressee:

> PDF readers shall display the options in the order in which they occur in the Opt array.

Table 234's `/TI` says where the display starts — "[f]or scrollable list boxes, the top index (the
index in the Opt array of the first option visible in the list)", default 0 — and §12.7.4.3 says
that all of this has to be built rather than read out of the file. Its NOTE names this control
first among its three examples:

> Examples include text fields to be filled in with text typed by the user from the keyboard,
> scrollable list boxes whose contents are determined interactively at the time the document is
> displayed and fields containing current dates or values calculated by an ECMAScript.

So a clause that "states no appearance" states: what strings are shown, in what order, starting at
which one, in what font, at what justification, inside what marked-content region, in a box derived
from `/Rect`. What it does not state is the artwork of the selection — no highlight colour, no
rule, no inversion, nothing, and a search of the whole standard for a sentence about a selected
item finds one, which is the sentence saying `/V` "identifies the item or items currently
selected".

## 3. Why that is a report and not a refusal

ADR 0106's test decides it, and it was written for §12.5.6.7's `/LE`: **ask whether the entry a
refusal refuses is additive or substitutive.** A line ending decorates a line the clause makes
required, so refusing it costs the line; a cloudy `/BE` replaces the border rather than adding to
one, so refusing it is a whole refusal.

A selection mark is additive in the strongest form of the word: it is drawn *over* an item that is
drawn either way. Refusing it costs every option on the list.

The counter-argument the old code rested on is in this tree's own test, and it is not nothing:
drawing a list with nothing saying which item is chosen "would put every option on the page with
nothing saying which is chosen, which is worse than refusing and is the plausible-looking wrong
page trap 1 is about". Two things answer it.

- **A page that draws nothing is not the safe state here.** The alternative to the constructed
  list is not a blank rectangle; it is the file's *stored* appearance stream, which under the two
  conditions that reach this construction — Table 224's `/NeedAppearances`, and a person who has
  chosen an item — is the stream the producer wrote for the *previous* value. So the refusal was
  not withholding a wrong mark, it was keeping one: the old highlight, on the old item, with a
  report beside it. Six options drawn with no highlight is strictly less wrong than six options
  drawn with the highlight on the wrong one.
- **The report is what carries the missing statement**, and the honest place for it is beside the
  drawing rather than in place of it — ADR 0030's shape, which this tree uses for every
  constructed appearance that draws what its clause states while naming what it does not.

**The mark itself is not invented, and that is a principle-5 decision rather than a shortage of
effort.** §12.5.6.4's icons are invented because a `shall` requires an appearance and the clause
draws none; there is no such `shall` here. Where this tree needs a selection colour it has an
established answer, and it is the one text selection already uses: the geometry crosses to the host
and the host draws it in its own colour. `pdf_model::form::ChoiceControl::selected` is that
geometry's equivalent for a list, and all three native hosts already build a real list control
from it — so the person using this program *does* see which item is chosen, and what cannot say so
is the page raster, which is exactly what the report is about.

## 4. Three smaller decisions, each with its argument

**A `/TI` past the end of the array is clamped, not obeyed and not ignored.** Table 234 makes the
entry optional with a default of 0, and §12.7.5.4 makes `/Opt` the thing that shall be displayed.
Obeying an out-of-range index literally would show nothing, which is an optional entry erasing what
the clause states — the failure ADR 0111 is about. Ignoring it and starting at 0 would discard a
statement the file made. Clamping to the last option is the third answer and the only one that is
about a *list*: a scrollable list scrolled past its end is showing its last item.

**A list's lines are never rewrapped**, which is the whole of `Shape::ListBox` against
`Shape::Multiline`. §12.7.5.4 makes each option "a text string that shall be displayed on the
screen" — one string, one item — so an option too wide for the box is clipped by the clip the
marked-content region already writes. Wrapping it would show the array as holding more entries than
it does.

**Auto-sizing takes no exception, and the picture is what decided that.** §12.7.4.3 makes the
function "implementation dependent", so a shape may have one of its own, and the first version of
this change gave the list box one: its rectangle is a scrolling window rather than a bound, said
the reasoning, so the size to find is the largest at which *one* line fits. The rule is defensible
and its output is not — on `listbox_actions.pdf` with the size zeroed it chose 34-point type for a
120-point-wide list of six short labels and showed two and a half of them, which is not a list.
Trap 1 applied to a rule rather than to a defect.

The window reading survives and is answered one step earlier: the options laid out are the ones
from `/TI` onward, so the run the shared auto-sizer measures already *is* the visible window, and
fitting it to the box is fitting the visible options. One auto-sizing function, and `/TI` doing the
work the exception was invented for.

## 5. Where the demand was, and where it was not

Every list box in the pdf.js corpus states an `/AP`, and not one of the documents holding one sets
`/NeedAppearances` — measured with this tree's own reader over the corpus, which is why the corpus
gate reported no list box before this round and reports none after it. That is not an argument that
the work was unnecessary; it is a statement about *which* population was owed.

The construction is reached under exactly two conditions, and this program has been able to create
the second of them since ADR 0248:

- Table 224's `/NeedAppearances`, where the writer has said the stored streams may not match.
- A value a person changed — `Entered::Chosen` — or §12.7.6.3's reset action putting `/DV` back.

So the round that gave three hosts a multi-selection list control gave this program a verb whose
result the page could not show, and `ViewState::save` wrote Table 224's flag where a stream
belonged. Both are closed here: the same construction serves the drawing and the save, as it does
for every other field type, so a saved file shows what the screen shows.

This is `doc/habits.md`'s rule about verbs, arriving from the direction it warns about: **after a
session that gives the program a new verb, re-read the clauses it already claims.** The row for
this one had been re-read twice since ADR 0248 and kept, both times on the sentence about the
highlight — which was true, and was about a mark rather than about a list.
