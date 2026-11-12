# Q840 — May a page be laid out by a producer's own XFA template, now that page one is priced?

Source: round 2009 (ADR 2837), the ledger lane, carrying out `doc/questions/A828` group 4 and
`doc/todo/76` row 10. Follows `doc/questions/Q639`, whose answer (`A639`) deferred the amendment
until "a measurement round has priced laying out page one of the corpora's `/NeedsRendering`
documents". This is that measurement: `doc/todo/78`.

## What was measured

- **How many.** Of the 70 453 PDF files this machine's corpora hold, 10 state Table 29's
  `/NeedsRendering`; three are veraPDF fixtures built to fail, three are pdf.js's, one is
  `format-corpus`'s copy of an immigration form, and three are of the 65 968 the safedocs web crawl
  sampled — about one in 22 000. pdf.js's own XFA test set, 53 public
  forms fetched for this reading and not kept, is where the shapes were read (`doc/todo/78`
  section 1).
- **What page one needs.** Not a part of the layout engine but nearly all of it: XFA 3.3 merges the
  whole form with its data and runs its `initialize` and `ready` scripts before the layout processor
  starts at the first content area (pages 288 and 388), and a page area chosen as `only` or `last`
  is chosen for page one only after the end is known (page 308). Of 56 `/NeedsRendering`
  resources (the 53 fetched and pdf.js's three), 47 state an `initialize` or `ready` script and 38 set `presence` from one.
- **What it costs.** About 23 to 28 rounds for the whole, 21 to 26 for page one (`doc/todo/78`
  section 4), most of it a new crate, `xfa-layout`, and a template-script object model the
  JavaScript and FormCalc engines do not have.

## The question

1. **The amendment.** Do you accept the fifth near-side construction RFC 0011 section 7 words — *a
   page laid out by a producer's own XFA template*, every mark the template's, positions fixed by
   XFA 3.3's algorithm rather than by a clause of ISO 32000-2 — and so lift XFA's dynamic half
   from `CLAUDE.md`'s exclusions?
2. **If you do: scripts at `off`.** A dynamic form decides page one in its scripts, and the reader's
   level is `off` by default. Is page one at `off` laid out from the template and the data alone,
   with the window saying the form's scripts did not run (the recommendation), or is a dynamic
   form's layout held until the reader raises the level?
3. **If you do: two texts.** The barcodes 14 of the 53 fetched forms state are ISO/IEC 15438
   (PDF417), 16388 (Code 39) and 16022 (Data Matrix), none held; and RFC 0011 section 1 asks that
   `doc/specifications.zip` carry XFA 3.3 once a test derives a value from it.

## Recommendation

1. **Accept the amendment, and build it after the ledger rows reach their final states**, in the
   order `doc/todo/78` section 4 lists — the merge and the launch measurement first, so that the
   first round can still say page one costs too much before the layout is written. The case for
   accepting is the target: "every PDF that exists renders as its producer specified", and the
   producer of a `/NeedsRendering` file specified its pages in the template, not in the wrapper
   page. The case against is the price set beside the share section 1 counts; it is the owner's
   to weigh, and the recommendation is to pay it late rather than refuse it.
2. **At `off`, lay out page one without the scripts and say so**, the way ADR 1932 says the form is
   XFA: a page a reader can see is closer to the producer's than the wrapper's notice, and the
   level stays the reader's.
3. **Buy no barcode text now**: refuse a barcode by name on the generated page until a form in the
   corpora needs it drawn; add XFA 3.3 to `doc/specifications.zip` with the first test that cites it.

## What the tree does meanwhile

What A639 says: a `/NeedsRendering` document shows the page its producer wrote for processors that
decline, and every window says it is an XFA form and what is shown instead (ADR 1932); Annex K's
dynamic rows stay `out-of-scope`; the static half proceeds.
