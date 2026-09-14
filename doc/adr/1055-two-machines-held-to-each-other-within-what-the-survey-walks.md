# 1055 — Two machines held to each other, within what the survey walks

Session 1038. Status: **accepted**. Takes `doc/questions/A61`'s answer — "Cross-check test first"
— and states the one thing the answer said its round had to carry: how the comparison is scoped.
Adds `crates/pdf-model/src/content/ledger.rs` and `crates/pdf-archive/tests/cross_check.rs`;
changes `pdf-model`'s `content.rs`, `content/{resources,run,xobject,pattern,text,annotations,
transparency}.rs`, `type3.rs`, and `pdf-archive`'s `survey.rs`. A tier 3 line in
`doc/todo/02` §2, and a row in its map.

`§N` is ISO 32000-2 and nothing else.

## 1. The decision

`crate::survey` and `pdf_model`'s interpreter both answer §7.8.3's question at every operator —
which entry of which subdictionary of the resource dictionary in force does this name select — and
both stay. What holds them to each other is a corpus-scale gate that records both answers in one
shape and asserts, **within every content stream both machines ran**, that the two sets of
selections are equal: every selection the survey judged is one the interpreter made, and every one
the interpreter made is one the survey judged. A disagreement is named by document, page, the run
of nested streams it stands in, the operator's ordinal in that stream, the category, the name and
what each machine found.

## 2. The scope, which is A61's condition and not this round's convenience

**The unit of comparison is a place**: the run of frames — page content, form, tiling pattern,
Type 3 glyph, annotation appearance — that reached a content stream, each keyed by the object it
is. A place only one machine ran is counted by route and printed, never judged. That is where
A61's scoping lives, and it is structural rather than a list of exceptions: the survey does not
walk a soft mask's group, an `SMask` image or a shading's function, so no place under any of them
exists on the survey's side and nothing under them is ever compared. The soft mask's frames are
still named (`Route::SoftMask`) so that the gate can *count* what it declines to judge, which is
what makes the second calibration below legible as a number rather than as silence.

The same structure is what keeps the other direction honest without a second list: an appearance
state the annotation is not showing, a glyph no text-showing operator reached, a form under optional
content the default configuration hides, a chain nested deeper than the survey's own bound — each
is a stream one machine alone ran, and none has a second reading to disagree with. **A selection
defect is always caught in the invoking stream**, which both machines ran: a form the survey
resolved to the wrong object shows there as a `Do` whose two outcomes differ, before anything under
it is reached.

Two rules narrow the selections compared, and each is a clause:

- **§8.6.8 Table 74** lets `cs` name a colour space family directly, so a family name names no
  resource, and a lookup of one that finds nothing is not a selection. The interpreter probes its
  ICC memo with one; the survey never looks. A lookup that *finds* something is kept, because then
  a machine has selected it and the survey has not judged it.
- **§14.6.1's `DP`** marks a point. Nothing is drawn under it and nothing is enclosed by it, so the
  interpreter has no reason to read its property list and does not; the survey reads it for
  ISO 19005-2 section 6.7.4's `/Lang`. One machine asking a question the other has no use for is
  not two answers to one question.

And one check the selections alone would not make: a stream the interpreter ran through a route
the survey walks, within the survey's nesting bound, that the survey reached nowhere on that page,
is a stream the survey's walk lost, and is a disagreement. A stream this program composed
(§12.7.4.3's constructed appearance) has no object and the survey has nothing to walk.

## 3. What the interpreter grew, and why it is not the observer

A61 declined an observer: a surface exposing the resolved state at every operator, which every
future operator would have to feed. What the interpreter grew instead is
`pdf_model::content::ledger::Ledger`, a record of the *lookups* it made — category, name, the entry
found, under which frames — written at the one place every lookup already passes
(`resources.rs`'s three functions), switched on by `interpret_ledgered` alone, and costing every
other caller one `Option` test per lookup. The survey records into the same type from its own
`look_up`, so the two walks state one fact in one shape. If the observer is ever built, A61's shape
decision stands and this ledger is not it; the ledger would then be what proves the survey reads
the observer correctly.

Two small changes to the interpreter fell out of making its lookups true to the clause rather than
to the ledger: `sh` now looks its shading up before asking whether the layer is hidden, because
the operator selects its resource whether or not the mark is made (the report is still withheld
for a hidden layer); and a pattern's entry is read unresolved, as `Do`'s already was, so that a
tiling cell's run can say which stream it is.

## 4. What the gate found on the corpus, and the two calibrations (trap 13)

Over `doc/veraPDF-corpus` — 2 908 documents, 12 951 pages, 12 542 places both machines ran,
13 547 selections compared — **zero disagreements**. Every unpaired place has a reason the census
names: a Type 3 font's unshown glyphs, an appearance state not shown, a chain of forms nested past
the survey's bound, a constructed appearance.

Both calibrations were run, each a one-line plant reversed by its exact inverse:

1. **A selection defect in one machine is named.** With the survey reading every form against its
   invoker's resources instead of its own, the gate reported 269 disagreements and exit 101, each
   naming the document, page, stream, operator ordinal, category, name and both outcomes — the
   first of them `isartor-6-3-4-t01-fail-e.pdf page 1, page content 15 0 R > form 10 0 R:
   /Font /F0`, missing on one side and `14 0 R` on the other. A second plant — the survey walking
   no form at all — was named by the reach check, 83 disagreements.
2. **A defect in a construct the survey does not walk is silent.** With the interpreter reading
   every soft mask's group against an empty dictionary, all thirteen selections under a soft mask
   in the corpus flipped to "nothing", the census line said so, and the gate reported zero
   disagreements and exit 0.

## 5. What the gate does not decide

Whether either machine reads §7.8.3's fallbacks as the clause states them. Both fall back on the
*invoking stream's* resources where a form or a Type 3 font states none, and NOTE 3 of §7.8.3 and
Table 110 name the *page's*; a tiling pattern stating no `/Resources` — which Table 74 requires — is
read against nothing by the interpreter and against its invoker's by the survey. Neither difference
has a witness in the corpus, so the gate is silent on both; they are questions for the clause, and
this ADR records them so that the next witness is read as one.
