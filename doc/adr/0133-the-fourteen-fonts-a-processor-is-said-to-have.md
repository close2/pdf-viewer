# ADR 0133 — The fourteen fonts a processor is said to have

Status: accepted, 2026-08-02. Session 148. Executes steps 1 and 2 of the third-party-data order
in `doc/HANDOVER.md` §1, together, because step 1 exists to serve step 2.

## The sentence

ISO 32000-2 §9.6.2.2, on the fourteen fonts a PDF may name without carrying:

> These fonts, or their font metrics and suitable substitution fonts, shall be available to the
> PDF processor.

Two halves. The thirtieth session took the second — `pdf_font::standard_metrics` carries the
published advances, so a document omitting `/Widths` is laid out from the specification rather
than from whatever face this machine happens to have. The first half was never here, and
`substitute.rs`'s own module comment said what that meant: **the only machine-dependent code in
the tree.** Two computers, one file, two pages.

## Decision

`pdf_font::standard` compiles the fourteen programs into the binary: ten faces from PDFium's
Foxit set (BSD-3-Clause) for Courier, Times, Symbol and ZapfDingbats, and four Liberation Sans
faces (SIL OFL 1.1) for Helvetica. 804 KB, `include_bytes!`d as `static` data, which is zero parse
time at launch — the rule `CLAUDE.md` states for compiled-in data and the one `pdf-spec`'s
Arlington tables already follow.

**Which comes first is decided by the document, not by the family.** `Request::standard` is true
when the `/BaseFont` names one of §9.6.2.2's fourteen or a metric-compatible clone, and it is what
orders the two sources:

- **the fourteen** are answered from the binary, because a document naming `/Helvetica` is asking
  for something the standard says a processor *has*, and answering it the same way everywhere is
  the whole point;
- **everything else** is answered from the machine first, because a document naming `/Garamond`
  without embedding it is asking for something no processor is required to have, and an installed
  face may have a far wider character set than Liberation Sans does.

The compiled-in set is the fallback in the second case, so **`substitute::find` can no longer
fail**: a machine with no fonts installed at all now draws text.

## Three things this turned up

**The `.pfb` files are not Type 1 programs.** PDFium's ten faces begin `01 00 04 02`, which is a
CFF header; the extension is inherited from whatever converted them. That cost nothing —
§9.6.2.1's NOTE 1 calls a CFF "an alternative, more compact but functionally equivalent
representation of a Type 1 font program", and `pdf_font::cff` has read one since the
thirty-second session because `/FontFile3` embeds them. **A file's extension is a claim its bytes
have to support**, and four lines of a hex dump settled it.

**A name-keyed substitute is addressed by name, and that is the shorter route.** An `sfnt`
substitute is reached by *character*, so §9.6.5.2's glyph name has to go through the Adobe Glyph
List first. A CFF keys its glyphs by the same names the encoding produced, so the AGL step
disappears — and that is why `Symbol` and `ZapfDingbats` work at all. Their glyph names (`a9`,
`universal`) are in no Unicode mapping worth trusting, and going through one is how a dingbat
becomes a Latin letter.

**A composite font cannot use them, and the corpus said so within one run.** §9.7.4.2 leaves a
substituted composite font reachable only through `/ToUnicode`, so its face must answer *by
character* — which the name-keyed CFF faces cannot. Preferring them there refused five corpus
documents a machine font draws. `Composite::load` therefore asks `substitute::installed` rather
than `substitute::find`, which is exactly the old behaviour, with the clause beside it.

## What it cost, measured

**Six oracle pages became contradicted and one stopped being.** Net five, and every one of the
six was opened: `issue6069.pdf` is one sans-serif line, `issue9243.pdf` one word under a
gradient, `bug847420.pdf` one italic line, `issue11403_reduced.pdf` and `bug850854.pdf` one line
each, `issue15716.pdf` a grid of card suits. All six draw the same text in a different face.
There is no defect among them.

**This is trap 9's second shape read from the inside.** `poppler`, `mupdf` and `ghostscript` all
resolve a non-embedded standard-14 font through this machine's fontconfig — the same URW faces,
off the same disk we were reading. Part of our agreement with them was shared *data*, and the
oracle noticed the moment we stopped sharing it. `CONTRADICTED_SUBSTITUTED_FONT` grew from 14 to
19 with that argument written into it, which is what a ratchet movement has to carry.

The corpus's 91 incomplete documents and the text gate's 97.9% are unchanged.

## The attribution surface, which came first for a reason

Both licences oblige a *binary* distribution to carry their notices, and this program had nowhere
to put them. Three pieces, in the order `HANDOVER.md` §1 required:

- **`/NOTICE`** at the root: every vendored file by name, both licence texts' load-bearing
  clauses, and what is in the repository but not in the binary.
- **`pdf-viewer --licences`**, which `include_str!`s it — a notice that can go missing between the
  binary and the file system is not carried by the binary.
- **`crates/viewer-ui/tests/notices.rs`**, because **`cargo deny` reads Cargo metadata and cannot
  see vendored data**. It checks that every `.pfb` and `.ttf` under `data/standard-fonts/` is
  named in `NOTICE` by file name — not by family glob, which would let a fifth weight arrive
  under a line written about four — that both licences' required sentences are present verbatim,
  and that the fourteen files still hash to what `SHA256SUMS` records.

`data/standard-fonts/PROVENANCE.md` names the upstream and its revision, which is the precedent
`pdf-spec`'s pinned Arlington submodule set.

## What is still owed

- **The metrics and the programs are two tables and could be one.** The advances now come from
  the programs themselves for a font that states `/Widths`, and from `standard_metrics` for one
  that does not. They agree by construction for the ten Foxit faces and are metric-*compatible*
  for Liberation Sans against Helvetica. Adobe's own AFM tables would close that, and are a
  separate licence to read.
- **Step 3 of `HANDOVER.md` §1, the predefined `CMap`s**, is untouched and is now the only
  third-party data item left.
- **The About panel** the owner asked for. `--licences` is the half that works headless; the
  panel needs something to draw a panel with.
