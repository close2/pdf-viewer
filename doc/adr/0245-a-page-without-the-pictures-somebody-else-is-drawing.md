# ADR 0245 — A page without the pictures somebody else is drawing

Status: accepted, 2026-08-08 (session 409).

## Context

`doc/todo/37` audited six populations of interactive chrome and found that all six now cross the
`viewer-core` boundary as data. It left one decision unmade, and the four-hundred-and-eighth session
turned it from a prediction into a photograph: `crates/viewer-gtk` places **76 real GTK4 controls
over `160F-2019.pdf`'s 67 fields**, and every one of them sits on top of the picture of itself
(ADR 0244, finding 1). A page carries §12.5.5's appearance streams for its widgets; a native host
draws the widgets itself; nobody had a way to ask for one without the other.

That session also stated the cost it believed: *"a flag on the render request … `false` in every
existing caller, so that the gates' display lists are byte-identical by construction"*. This round
was told to re-derive that rather than inherit it, and the first half of it is wrong.

## Decision 1 — the request means **§12.7's widgets, and only the ones the host was handed**

Two requests wear one phrase and only one of them is a form host's.

**Why the widgets are separable at all** is §12.5.6.19's first sentence, and it is the whole
argument:

> Interactive forms (see 12.7, "Forms") use widget annotations (PDF 1.2) to represent the appearance
> of fields and to manage user interactions.

A widget annotation **is** a field's appearance. A host that puts a `GtkEntry` at the widget's
rectangle has replaced exactly that, and has replaced nothing else: §12.5.6.10's markups,
§12.5.6.4's icons, §12.5.6.12's stamps and §12.5.6.5's links have no counterpart in any toolkit and
stay on the page. §12.5.1 draws the same line from the standard's own side, in the value Table 31's
`/Tabs` reserves for one subtype:

> W (widgets order): Widget annotations shall be visited in the order in which they appear in the
> page Annots array, followed by other annotation types in row order.

So "leave out the widgets" and "leave out the annotations" are different requests, and the second is
not one a form host has any business making. `WidgetAppearances` is named for the first.

**And it is narrower than `/Subtype /Widget`.** The set that leaves the page is
`form::delegated_widgets`, built by calling `form::fields` and taking the annotations it answered
with — so *what is removed is exactly what a host was handed a control for*. Three kinds of widget
therefore keep their appearance, and each would have been a page silently losing ink:

| kept | why |
|---|---|
| a widget the field tree does not reach | §12.7.4.2 makes a dictionary with no `/T` and no path to the form "simply a Widget annotation"; `Query::Fields` never mentioned it |
| a widget of a field whose `/Parent` chain runs past this crate's bound | `form::fields` refuses that field, so no control exists |
| a widget with no `/Rect` | nothing can place a control over a rectangle that is not stated |

Building the set from `fields` rather than from a second traversal is the point rather than an
implementation detail: two traversals that agree today can stop agreeing, and the invariant is what
makes the rule safe. `only_the_widgets_a_host_was_told_about_are_delegated` asserts it, and the
fixture carries one of each of the three kept cases — replacing the rule with `/Subtype /Widget`
empties two of their rectangles and the test says so.

**§12.5.5's rules are untouched.** Nothing about placement, `/AS` selection, `/BBox` clipping or the
transparency group changes; a delegated widget is not drawn *differently*, it is not drawn. Every
other annotation on the page goes through the identical path. And the *readback* follows the
picture: a page drawn without a field's appearance reads back without the field's value, which is
right — a host that drew the control has those characters in its own control, where the platform's
selection reaches them, and a text layer naming glyphs nobody can see would put a selection over an
empty rectangle.

## Decision 2 — §6.3.2.2 is a permission, not a departure

`CLAUDE.md` ranks §6.3.2.2, so a viewer that stops drawing appearance streams owes an argument. The
clause supplies it in the same sentence as the obligation:

> A PDF processor shall also render the appropriate appearance stream for all annotations
> (12.5.5, "Appearance streams") which have appearance streams designated for this purpose as
> indicated by the annotation flags (see 12.5.3, "Annotation flags"), unless otherwise instructed.

The last three words are the clause's own carve-out, and they decide the shape of everything below.
An instruction has an author: it is something a processor is *given*, not something it decides. So

- the value can never originate in `pdf-model` or in `viewer-core` — those would be the processor
  instructing itself, which is not what the sentence describes;
- the default has to be the other one. `ViewState::of` is `WidgetAppearances::Drawn`, so every
  caller that has not been instructed draws the page §6.3.2.2 requires;
- and the *host* is the only party that knows, because whether the appearances are being drawn
  somewhere else is a fact about the program, not about the file.

That is the same shape as `Command::Restrict` (ADR 0212) and it is a different *kind* of policy,
which is worth saying because the two will sit next to each other in every host: `Restrict` says how
much of what a **document** asserts over its reader this program obeys; this says which half of the
page the **host** has undertaken to draw. Neither is a fact about the document.

## Decision 3 — where it lives, and why the render request could not carry it

Session 408's "a flag on the render request" is not merely a cheaper option than the one taken; it
is not available. **`Event::NeedsRender` is this crate's output.** `RenderRequest` carries an
`Arc<DisplayList>` that has already been interpreted, so a flag on it would arrive after the
decision it governs — the earliest a host could set it is the frame after the one it wanted. What
408 was reaching for is real and survives: *a value that is `false` in every existing caller*. Where
it lives is the part that had to be re-derived.

It lives in **`pdf_model::view::ViewState`**, and rule 1 of the UI boundary leaves no alternative:
interpretation is a pure function of the immutable document and the view state, so the view state is
the only channel by which anything outside the file may change what is drawn. There is a precedent
of exactly this kind already in the struct — `magnification`, which is a property of the *window*
and lives there because §12.5.3's `NoZoom` makes interpretation depend on it. That field's own
comment called itself "the one thing in this struct that is a property of the window"; it is now one
of two, and its comment says so.

The route is the one the magnification takes, end to end:

```
host ──Command::Delegate(WidgetAppearances)──▶ Viewer::delegated
                                                   │  Viewer::settle
                                   ViewState::set_widget_appearances ──▶ interpret_with
```

Three consequences, each a choice:

- **A `Command` rather than a field on `Open`.** A host may change its mind, and `viewer-gtk` does:
  the two photographs below were taken from one binary with and without a flag. It is applied in
  `settle`, so a document opened *after* the command gets it too.
- **Viewer-wide rather than per document.** A program that draws native controls draws them over
  every document it shows.
- **An enum rather than a `bool`.** Nothing in this vocabulary is `#[non_exhaustive]`, so if a third
  answer is ever wanted every consumer fails to compile until it says what it does — which is what
  ADR 0212 kept its two unbuilt levels cheap with.

The cost, stated: when the instruction is given, interpreting a page costs one extra walk of
§12.7.4.1's field tree — the same walk `Query::Fields` already makes for the same page. When it is
not given, it costs one enum comparison against `WidgetAppearances::Drawn`.

## The correctness constraint, demonstrated rather than argued

The oracle's 1794-page comparison rests on `interpret` being a pure function of the bytes, so this
round owed a demonstration that every existing caller still produces the display list it produced
before. Two summary numbers agreeing is not that demonstration — two different lists can rasterise
to the same verdict — so the artefact itself was compared.

`cargo run -p pdf-model --example display_list_digest` prints one line per document: the command
count of page one's display list, the byte length of its `Debug` rendering, and a hash of it. Run on
`89de636` in a worktree and on this tree, over all 974 corpus documents:

> `diff before.txt after.txt` → **empty; 975 identical lines**, 964 documents opened and 959 first
> pages interpreted.

**The instrument was checked by an accident, which is worth recording because it is the only
evidence that an empty diff means anything.** The first run of the pair differed on 96 documents —
`bug1815476.pdf` 1490 → 1522 commands, `issue13372.pdf` 0 → 1 — because the worktree had no
`pdf-sandbox-worker` beside its binary and every JBIG2 and JPEG 2000 image was refused. Building
trap 10's binary in both trees is what made the two runs comparable, and it proved the digest
detects a difference of exactly the kind being looked for.

The gates say the same thing from the other side, unmoved: corpus **974 documents, 65 incomplete**;
oracle **905 agree / 68 contradicted / 786 ambiguous** over 1794 pages; quorra **912 agree, 36
differ, 9 refused, 17 not comparable**; text **99.2% (24043/24243 words)**.

## Evidence, under `Xvfb`

ADR 0126's recipe: `Xvfb :78` at 1100×1200, `GSK_RENDERER=cairo`, `xwd` for the pixels.
`pdf-viewer-gtk` delegates by default now — it *does* place a control over every widget — and
`--draw-widget-appearances` restores §6.3.2.2's default, which is what makes a pair of pictures
possible from one binary.

| what | what the run said |
|---|---|
| the form, both ways | `160F-2019.pdf`: `67 field(s) on the page, 76 control(s) placed`, first frame 107–118 ms |
| what changed on the screen | **3257 of 1 320 000 window pixels (0.247%)**, every one of them inside the page's own area (x 398–892, y 150–834) — and the difference image is the outline of each widget's rectangle, which is the part of its appearance the control did not cover |
| a document where the appearance is text | `annotation-text-widget.pdf`: 7 fields, 7 controls, and the page's display list goes from **687 to 161 commands** — the 526 that leave are the glyphs of the seven values and the comb field's cell divisions |
| the page itself, without the window | `render_at … --delegate` on that document: the document's own frames stay (they are page content), and the values inside them go. Side by side, that is the duplication with one copy removed |
| the corpus | `delegated_census`: of **964 openable documents, 102 have a delegable widget on page one**, **637 widgets** in all, **2723 display-list commands** removed between them; most from one page is `annotation-text-widget.pdf` at 526 |

**And the honest half of the picture.** On `160F-2019.pdf` the change is only 0.247% of the window,
and the reason is ADR 0244's *second* finding rather than a defect in this one: a GTK control has a
theme-decided minimum size and `set_size_request` is a floor, so a control that overflows its
rectangle was already covering most of the picture it duplicated. The overflow is what *hid* the
duplication, which is a sharper statement than "they are two consequences of one decision", and it
is measured now that it can be seen alone. `ControlFit` in `viewer-gtk` asks each control what it
would take:

> `160F-2019.pdf`: **11 of 76 control(s) wider than their /Rect (worst +85 on 120 px), 76 taller
> (worst +22 on 12 px)**
> `annotation-text-widget.pdf`: **0 of 7 wider, 6 taller (worst +12 on 22 px)**

Every control on that form is taller than the rectangle the document states for it, and the worst is
nearly three times its rectangle. That is not `viewer-core`'s to fix and not a control to shrink —
a platform control sized by its theme is the entire point of using one. What it says is that a real
native form host must also choose the **scale** the page is drawn at, which is a third decision and
belongs to `doc/todo/30`.

## What the round also found, in the clauses

**A sentence in quotation marks that ISO 32000-2 does not contain.** Three source comments and one
todo attributed to §12.5.2 the words *"the annotations shall be drawn in the order in which they
appear in the array"*. The standard has no such sentence anywhere — the nearest are §12.5.1's
`/Tabs` values, which are about visiting order for the tab key. Where ISO 32000-2 states painting
order is **§12.5.5**: an appearance's transparency group "shall be composited with a backdrop
consisting of the page content along with any previously painted annotations". The claim was right
and its citation was invented; both are corrected, and §12.5.5's ledger row records it. This is
principle 5's "quotation marks mean verbatim" failing in the one place the conformance checker
cannot see — a `//` comment rather than a rustdoc blockquote.

**A table number that names the wrong table.** `viewer-gtk`, ADR 0244 and `doc/todo/37` cite "Table
189's `/R`" for a widget's rotation. Table 189 is the *movie* annotation's; a widget's `/R` is Table
192's, which §12.5.6.19's own ledger row has had right since the hundred-and-fifth session. The code
is corrected. And in the ledger, §12.5.3's row attributed `/FixedPrint` to Table 167 — §12.5.3 does
not mention the entry at all, and it is Table 193's, §12.5.6.22's watermark annotation. Both found
by `doc/todo/02` §4's ninth sweep applied to the three rows this round read.

## Consequences

- **`viewer-core` gained one `Command` and no `Event` or `Query`.** Eleven messages in eleven rounds
  of hosts, each because a clause needed a channel. This one is §6.3.2.2's three words.
- **The confined transport carries it** (`command_kind::DELEGATE = 21`), because the confined process
  is the one that interprets, so the party drawing the widgets has to be able to tell it.
- **`doc/todo/37` is closed.** Its remaining item was this decision, and its two smaller notes —
  Table 229 bit 26's `RadiosInUnison`, and §12.7.5.4's list box drawing nothing — move to
  `doc/todo/30` with the scale question, which is now the thing standing between this and a *good*
  native form host.
- **What is still not delegable, deliberately**: everything else on the page. A host that wants
  §12.5.6.14's popup in an `NSPopover` needs nothing from this decision, and the reason is a nicer
  one — `annotation::decide` already answers `Nothing` for that subtype, on the clause's own terms
  ("a popup is the window belonging to some *other* annotation"), so the page never drew one and
  `Query::Popups` hands over the window's contents instead. A widget is the case where the standard
  puts the control's picture *on the page*, which is why it needed a switch and the popup did not.
