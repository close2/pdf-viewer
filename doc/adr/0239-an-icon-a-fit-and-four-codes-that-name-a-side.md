# ADR 0239 — An icon, a fit, and four codes that name a side and not a size

Status: accepted, 2026-08-08 (session 402).

## Context

§12.5.6.19's ledger row has been `partial` for seven entries of Table 192 since the
three-hundred-and-eighty-seventh session, when the first sweep found the *false* claim that had been
sitting above them for three hundred and sixty-four sessions and the seven were written down for the
first time. They are the push-button half of the appearance characteristics dictionary: `/I`, `/RI`
and `/IX`, a normal, rollover and down icon; `/IF`, how each is fitted; `/RC` and `/AC`, the
rollover and down captions beside the `/CA` that is already drawn; and `/TP`, where the caption sits
relative to the icon.

The three-hundred-and-ninety-fourth session re-checked that all seven were genuinely unread rather
than believing the row, and they were. This round is the one with room for them.

**The count came before the work**, which is `doc/todo/01`'s rule for a demand-track item and is
what `doc/todo/13` did for the transfer function. `crates/pdf-model/examples/push_button_census.rs`
reads every widget in the corpus:

```
964 document(s) opened, 833 widget(s), 42 push-button(s), 42 with an /MK, 33 with an /AP /N
  /I    1 widget(s) (0 of them constructing) in  1 document(s): evaljs.pdf
  /IF  12 widget(s) (9 of them constructing) in  3 document(s): 160F-2019.pdf, form_two_pages.pdf, listbox_actions.pdf
  /TP   2 widget(s) (0 of them constructing) in  2 document(s): evaljs.pdf, issue15096.pdf
  /RC, /AC, /RI, /IX: 0 widget(s) in 0 document(s)
```

Two numbers in that decide the shape of what follows.

**33 of the 42 state their own `/AP /N`.** Table 191 makes `/MK` an appearance characteristics
dictionary "that shall be used in constructing a dynamic appearance stream specifying the
annotation's visual presentation on the page" — so a widget with a stored appearance is drawn from
it by §12.5.5 and never asks Table 192 anything. The "constructing" column is therefore the only one
that can change a pixel, and it is zero for `/I` and `/TP` and nine for `/IF` — nine `/IF`s in one
document, `160F-2019.pdf`, which states no `/I` at all. **An icon fit with no icon fits nothing.**

So: not one of the seven entries can change a mark on any of the 974 documents here. That is trap
8's situation exactly — a corpus finds what documents contain, not what the standard says — and it
is a reason to be careful about *what* is built rather than a reason to build nothing. Three of the
seven are stated by the standard completely enough to implement without inventing anything, and four
are not.

## Decision

### `/I` and Table 250's fit are implemented, because both are stated whole

Table 192 gives the normal icon as "[a] form XObject … which shall be displayed when it is not
interacting with the user", and the entry "shall be an indirect reference". That last phrase is what
makes the implementation cheap and is worth naming as the reason: the reference goes into the
constructed appearance's `/XObject` resource *unresolved*, and the interpreter resolves it against
the same document when it runs the `Do`. Nothing is copied and no second decode path appears.

The icon's own extent is §8.10.2's, because a form XObject states its size nowhere else: the `/BBox`
"in the form coordinate system", transformed by the `/Matrix` that maps that system into the space
the `Do` runs in. Table 250 then fits *that* rectangle into the annotation's.

Table 250 has four entries and every one of them has a stated default, so a widget with no `/IF` is
fitted by the same code with `IconFit::DEFAULT`. Two readings inside it are ours rather than the
table's and both are recorded here:

- **"Bigger" and "smaller".** `/SW B` is "[s]cale only when the icon is bigger than the annotation
  rectangle" and `/SW S` is the converse; the table does not say what a rectangle wider and shorter
  than the icon counts as. Bigger is read as *exceeding on either axis* and smaller as *fitting on
  both with room to spare*, which is the only reading under which `B` and `S` are complementary
  rather than overlapping — and an icon that overflows on one axis is one a `B` file wanted scaled.
- **`/A` under anamorphic scaling.** The table says "[t]his entry shall be used only if the icon is
  scaled proportionally", in as many words, so it is not applied otherwise. There is leftover space
  to place in exactly one case that leaves — `/S A` together with `/SW N` or a `/SW B` that does not
  fire — and the icon sits at the corner `[0.0 0.0]` names, which is the array's own bottom-left.
  The file has asked for the icon's own size and said nothing about where to put it.

**`/FB` is why the fit takes two rectangles rather than one.** "[T]he button appearance shall be
scaled to fit fully within the bounds of the annotation without taking into consideration the line
width of the border" — so the target is `/Rect` itself where the flag is true and `/Rect` inset by
§12.5.4's border width where it is not.

### `/TP`'s codes 0, 1 and 6 are carried out; 2, 3, 4 and 5 are reported

This is the round's real decision and it is a refusal.

Codes 0, 1 and 6 each say which of the two things is drawn and give whichever is drawn the whole
rectangle: "0 No icon; caption only", "1 No caption; icon only", "6 Caption overlaid directly on the
icon". Nothing is left open, and code 0 is the table's *default* — so a widget stating an `/I` and no
`/TP` is a widget the standard says shows its caption, and drawing the icon there would put a mark
on the page the file asked not to have. That is not a nicety: `evaljs.pdf`'s push-button states `/I`
and `/TP 1`, and the *other* direction is the one that would have been wrong by accident.

Codes 2 to 5 say which **side** the caption goes on — below, above, right, left — and neither this
clause nor any other in ISO 32000-2 states how much of the rectangle the caption takes. There is no
proportion, no margin, no rule that the caption gets a line's height. Choosing one would put a layout
on the page that neither the document nor the standard asked for, which is the same refusal
§12.5.6.12's stamp legends get and for the same reason: a recommendation, or a relation, is not a
licence to invent a different kind of thing from the one that is named.

So those four draw the icon — the half the code *does* state — and report the caption by name. The
report says what is missing rather than that something is unsupported:

> no appearance stream, and Table 192's /TP 2 states which side of the icon the caption goes on and
> not how much of the rectangle it takes

### `/RI`, `/IX`, `/RC` and `/AC` are reported, and the reason is structural rather than a capability

Each of the four is defined by what the **pointer** is doing: the rollover pair "when the user rolls
the cursor into its active area without pressing the mouse button", the alternate pair "when the
mouse button is pressed within its active area". This program *has* a pointer — §12.5.6.19's `/H` has
been honoured on a press since ADR 0177 — so the refusal is not `doc/todo/01`'s capability shape.

It is that a **constructed** appearance is one stream where §12.5.5 gives a stored one three. `/AP`
has `/N`, `/R` and `/D` subdictionaries and the pointer selects among them; `appearance::construct`
returns a single `Constructed`, and giving it three would be a change to the *shape of the answer*
rather than to what any one of them contains — the lesson `doc/todo/01` records from §14.9.3's `/TU`,
where a row survived the arrival of the very capability it named because what had to change was what
the program could *say*.

That change is not taken here, and the reason is the census: no corpus document states any of the
four. A file that does is told so by name, one entry at a time.

## Consequences

- §12.5.6.19 stays `partial`, and for a shorter and better-stated list: `/TP` codes 2 to 5, and the
  four pointer-state entries. §12.7.5.2.2's row loses `/I`, `/IF` and `/TP` from its own unread list.
- **The constructed appearance can now name a resource that is not a font.** `Stream` grew an `icon`
  field held apart from `resources` because the two are filled in at opposite ends of a widget's
  construction — the icon is written first so that code 6 puts the caption over it, and the caption
  is what replaces the resource dictionary wholesale with `/DR`'s. Merging at the end is what keeps
  the drawing order from deciding which resource survives. The name is chosen against `/DR`'s own
  `/XObject` names, because a collision would draw the document's form instead of the icon.
- Six tests, and each discriminates a placement rather than the presence of ink:
  `/SW N` with `/A [0.5 0.5]` paints the centre and not the corner, `/A [0 0]` the corner and not the
  centre; `/TP 0` with an `/I` paints nothing; a check box with the same `/MK` paints nothing,
  because Table 192 marks all seven "push-button fields only" and Table 229 bit 17 is what decides.
- No corpus page changes, which the corpus, oracle, quorra, text, dates and XMP gates all confirm.
  That is the expected result and it is worth stating: this is coverage work, and the instrument that
  answers coverage is the ledger rather than the corpus.
