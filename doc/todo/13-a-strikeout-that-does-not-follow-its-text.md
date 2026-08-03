# A strike-out that does not follow the text it strikes out

Status: **reported by the project owner, two-hundred-and-thirty-second session; diagnosed to a
clause, not decided.**
Priority: 13 — a defect a person can see, and the clause it comes from is one this tree
implemented on purpose three sessions earlier
Corpus: `doc/ISO_32000-2_sponsored_EC3.pdf` page 285 (fourteen annotations, ten of them markup),
and every other errata page of the same document
Clauses: §12.5.3 (Table 167's `NoZoom`), §12.5.6.10, Table 182
Code: `crates/pdf-model/src/annotation.rs` (`ViewGeometry::adjustment`),
`crates/viewer-core/src/open.rs`

## What was seen

> on page 285 of the `ISO_32000-2_sponsored_EC3.pdf` is some strikethrough text. When zooming
> the lines stay in place. I am pretty sure they should move with the text.

## What the file says, checked rather than guessed

Page 285's `/Annots` holds fourteen annotations. The struck-out text is object `11702`:

```text
/Subtype /StrikeOut  /IT /StrikeOutTextEdit  /Subj (Issue #19)
/F 220
/Rect [ 86.155 714.216 487.863 738.456 ]
/QuadPoints [ 143.287 737.796 484.714 737.796 143.287 727.236 484.714 727.236
              89.304 725.436 281.827 725.436 89.304 714.876 281.827 714.876 ]
```

`/F 220` is `128 + 64 + 16 + 8 + 4`: Locked, ReadOnly, **NoRotate**, **NoZoom**, Print. So the
file *asks* for what is being seen, and this tree started obeying it in the
two-hundred-and-seventeenth session (ADR 0168, §12.5.3's `NoZoom` and `NoRotate`). Before that
the annotation scaled with the page because nothing read the flag.

**This is therefore not a rendering accident. It is one clause obeyed, and the question is
whether a second clause says it should not have been.**

## The tension, and it is real

§12.5.3 is unconditional about the flag:

> If the NoZoom flag is set, the annotation shall always maintain the same fixed size on the
> screen and shall be unaffected by the magnification level at which the page itself is
> displayed.

and it names the fixed point: "the coordinates of the upper-left corner of its annotation
rectangle". Our `ViewGeometry::adjustment` does exactly that — a similarity about that corner,
in default user space, composed before the page's transform. At 200% the strike-out is drawn
half-size about `(86.155, 738.456)`, so its left end stays on the text and its right end falls
short of it by half the line's length. That is "the lines stay in place" seen from the other
side: they *are* anchored, and they no longer span the words.

§12.5.6.10 says what a text markup annotation *is*, and it is defined by the text underneath:

> Text markup annotations shall appear as highlights, underlines, strikeouts (all PDF 1.3), or
> jagged ("squiggly") underlines ( PDF 1.4 ) in the text of a document.

and Table 182 says what fixes them to it:

> An array of 8×n numbers specifying the coordinates of n quadrilaterals in default user space.
> Each quadrilateral shall encompasses a word or group of contiguous words in the text
> underlying the annotation.

A strike-out whose quadrilaterals no longer lie over the words they were written for is not the
thing that clause defines — and "in default user space" is the second half of the evidence,
because default user space is exactly what a magnification scales.

So two clauses, and only one of them can be obeyed at 200%.

## What has to be settled

1. **Which reading wins, and on what grounds.** §12.5.3's `shall` is about a class of
   annotation whose appearance is an *object on* the page — a note icon, a stamp, a widget —
   where a fixed screen size is the whole point. §12.5.6.10's is about an annotation that is a
   *property of* the page's text. If the second is right, the rule is that `NoZoom` does not
   reach a text markup annotation, and that has to be written as a reading of the two clauses
   rather than as an exception bolted on because a page looked wrong.
   §12.5.6.4's sentence is the precedent to weigh it against: it makes `Text` annotations
   "behave as if [`NoZoom` and `NoRotate`] were always set" — the standard *does* legislate the
   flag per subtype when it means to, and it says nothing of the kind for §12.5.6.10.
2. **Whether the producer meant it.** `/F 220` on every markup annotation of an errata document
   looks like one generator's habit rather than fourteen decisions. Count the corpus: how many
   `StrikeOut`, `Highlight`, `Underline` and `Squiggly` annotations carry `NoZoom`, and in how
   many documents. A flag set by one producer on everything it emits is weaker evidence of
   intent than a flag set on one annotation in a file.
3. **What the other readers do, as evidence and not as the answer** (principle 5). Whether
   `poppler`, `mupdf` and `pdf.js` scale a `NoZoom` strike-out is a question about their reading
   of the same two clauses, and disagreement among them would itself be informative.
4. **Whether the same argument reaches `/IRT` and `/Popup`.** Object `11701` is this
   annotation's popup, also `/F 220`, and a popup genuinely *is* a fixed-size object on the
   screen. Any rule written here must not sweep those in.

## Why it is not fixed in the session that found it

The fix is one predicate — which subtypes `ViewGeometry::adjustment` applies `NoZoom` to — and
writing it before item 1 is settled would put a subtype list in the code with no clause behind
it, which is the shape `CLAUDE.md` principle 5 forbids. What is *done* is the diagnosis: the
annotation, its flags, the two clauses, and the three things to count before choosing.

**And there is a gate-shaped observation in it.** None of the six gates can see this: the oracle
renders every page at its own scale, where `ViewGeometry::magnification` is `None` and `NoZoom`
changes nothing by construction (`annotation.rs` says so in as many words). The defect exists
only at a magnification a person chose, which is the population `viewer-core/tests/headless.rs`
watches and where this belongs.
