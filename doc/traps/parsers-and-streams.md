# Traps: parsers, streams and what a refusal is worth

Status: **standing** — each is a mistake somebody actually made in this tree.
Read by: a round that touches `pdf-syntax`, a filter, a font program, an image codec, a colour
space, or anything that decides what to do with input it cannot fully handle.

`doc/HANDOVER.md` is the index and names which group holds which trap. **Every trap keeps its
number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and dozens of ADRs cite them
by number and an ADR is not edited to follow a file that moved underneath it (ADR 0232 §2).

## Traps

### 4. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them was "the
caller's responsibility" and then did not, so every modern PDF failed with a misleading `/Root is
not a dictionary`. `pdf-syntax/tests/real_documents.rs` and `pdf-model/tests/render_real_pdf.rs`
run over everything in `doc/`. The converse is trap 8.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle: `Unsupported`, `FontError`, `ImageError`,
`CpuRasterError::UnsupportedCommand`. Do not "helpfully" fall back to a default that renders
something plausible. **A rise in the incomplete count is not a regression when it is a new
report.**

The rule is easiest to lose *inside* a partly-implemented feature, because the operator is handled
and the code path exists: `Tr` was parsed with four of its eight modes silently absent; Table 57's
`/LC`, `/LJ` and `/ML` read nothing while `J`, `j` and `M` set the same parameters. **Where a
clause gives a parameter two routes, implementing one of them is the failure mode that reports
nothing.** The sharpest instance is §7.3.8.2's `/Length`, which Table 5 lets be an indirect
reference: a parser cannot follow one, so it took a scan's answer instead and dropped the *data's*
last byte on every stream whose producer wrote no end-of-line before `endstream`. Such a stream then
read as damaged while being whole — and three rounds of damaged-stream census had been counting it
(ADR 0366).

**And every resource lookup that can cost a mark says so, which is a statement rather than a list.**
Since the four-hundred-and-nineteenth session all six of Table 34's categories the interpreter looks
a name up in report a name the current resource dictionary does not define: `Font` and `Shading`
always did, `ColorSpace` reports through `ColourSpace::parse`, and `XObject`, `ExtGState` and
`Pattern` were silent until then (ADR 0255). `Properties` is deliberately outside it and the reason
is trap 11's — a missing property list costs no mark, so a report there would take a page off the
oracle's judged set for nothing.

**And a report that is loud can still be *wrong about the file*, which is this trap's own failure
one step along.** "The resource dictionary does not define this name" and "it defines it, and what
it names is not there" are two clauses — §7.8.3 and §7.3.10's null — and `Font` said the first of
the two for both conditions from the day the interpreter had fonts, while `XObject` has told them
apart since ADR 0255 and `Shading` always did. It took a corpus outside the 974 to show it: a
cairo page in an evince bug report names six fonts and the reduction that made the report small
removed their objects, so the reader printed a false sentence about the document six times. **A
refusal is owed the right clause as well as a voice** (ADR 0779), and the way to tell is to look at
the entry *before* it is resolved, which is the only moment the two conditions still differ. **The condition on all three is `is_hidden()`**: §8.11.3.1 skips
hidden content "as if there were no `Do` operator to invoke it", and a `Do` that was never invoked
cannot have failed.

**Fourteen places report *while* drawing, each deliberate**, and the test for adding a fifteenth
is that suppressing either statement loses information: `/NeedAppearances` (stale appearances drawn
because they are all the file offers); §11.6.5.2's `/Matte` where pre-blending cannot be undone
(refusing would draw a rectangle of pure matte colour); a constructed appearance drawing what its
clause states while naming what it does not (ADR 0030); §8.11.4.4's `/User` and `/Language`
(switching a layer off would answer a question about this machine that nobody asked, ADR 0044);
§12.5.6.7's `/LE`, which decorates a line the clause makes *required* — **so ask whether
the entry a refusal refuses is additive or substitutive**, and a cloudy `/BE` stays a whole
refusal because a different border is not an extra mark (ADR 0106); a `/DA` font `/DR` lacks,
laid out in a stand-in **that declines where it cannot draw the whole value** (ADR 0112); a
`DCTDecode` frame that contradicts its dictionary, drawn on the codestream's own grid because
§7.4.8 puts the dimensions there (ADR 0340); a `/Contents` part that decoded only as far as
its damage, whose prefix is the producer's own bytes and whose shortfall would otherwise make a
page cut short look like a page meant to be sparse (ADR 0343); and an image whose decoded samples
stop short of the grid §7.3.8.2 infers from its own dictionary, where the samples that arrived are
the producer's own and the rest of the grid is left unpainted rather than read as zero — 99.3% of
`178360.pdf`'s stencil used to be marked in the fill colour for want of this (ADR 0356); and the
*other four* content streams §7.8.2 names beside a page's `/Contents` — a form `XObject`, a tiling
pattern's cell, a Type 3 glyph description and an annotation appearance — whose prefixes this tree
had always drawn and never mentioned, one sentence of one clause away from the row above them
(ADR 0359); and a page whose ancestry states no `/MediaBox`, drawn on a default size this project
chose and nothing in ISO 32000-2 states, where the substituted thing is **not a mark but the frame
every mark is measured in** — so the additive-or-substitutive question above does not decide it and
a different one does: refusing throws away a whole document that draws, and saying nothing makes a
guessed sheet look like a measured one (ADR 0389); and a `CCITTFaxDecode` whose `/EndOfBlock false`
lets Table 11's `/Rows` stop the filter above the image's `/Height`, where the shortfall is **the
clause obeyed rather than damage** — the producer told the filter to stop — so the scan lines that
arrived are drawn, the ones between `/Rows` and `/Height` are blank because ISO 32000-2 states
nothing about them, and the report is what keeps an invented lower half distinguishable from a
white one (ADR 0434); and §8.5.2.1's path segment issued with **no current point**, which the
additive-or-substitutive test settles the same way a font program's prefix is settled: the clause
gives the segment no first endpoint, so drawing one means substituting a place the file never wrote
— `tiny-skia` substitutes the origin of user space and `kurbo` fires a `debug_assert!` — and the
report is what keeps a page short of a segment distinguishable from a page whose producer drew less
(ADR 0563); and a page built out of the entries §7.3.7's dictionary states *readably*, where the
page object's own bytes stop before its closing `>>`, because every entry the producer wrote after
the damage is then read as one of Table 31's defaults — no `/Contents` is a page with no marks, no
`/Group` is a page composited without one — and only the report separates that from a producer who
wrote exactly those entries (ADR 0784). **Its `h` twin is deliberately *not* the fifteenth**, on trap 11's side of the same
test: `h` with nothing to close adds no segment either way, so the page is complete and a report
would only cost it the oracle's judgement.

**The additive-or-substitutive test is what decides the other direction too**, and ADR 0343 is
where it drew a line through one clause: the *same* damaged-prefix rule that is right for a
content stream is wrong for a font program, because §7.8.2 makes the first "a sequence of
instructions" — a prefix of which is a shorter sequence of the same kind — while a font program is
a table directory whose offsets point forward, so its prefix yields glyphs the producer never
wrote, standing in place of the right ones. **Ask what a prefix of the thing *is* before deciding
whether to draw one.** And ask it of *each consumer separately*: ADR 0359 carried the content-stream
half of that answer to the four objects §7.8.2 names in the paragraph after the sentence ADR 0343
quoted, and found the argument stronger for a Type 3 glyph description than for a page — Table 110
makes `d0`/`d1` the first operator, so a prefix cannot lose the glyph's own declaration, and
`/Widths` rather than the description carries the advance.

**And §7.3.7's dictionary — the clause every one of those objects is described by — answers it a
third way, which is neither *draw the prefix* nor *refuse it*.** The clause states no extent for a
dictionary beyond its closing `>>`, and it states outright that the written order is not
information: "[t]he entries in a dictionary represent an associative table and as such shall be
unordered even though an arbitrary order may be imposed upon them when written in a file. That
ordering shall be ignored." So the entries read whole before the damage are **not the dictionary** —
two files stating one dictionary in two orders, damaged at the same byte, yield different sets —
but they *are* a subset of it, every member the producer's own. Those are two different sentences,
and the whole design is keeping them apart: `Document::get` still refuses the object outright, so
nothing in the document graph reads less of the file than it did; `Parser::parse_damaged_dictionary`
is a second door a caller opens **by name** and which hands back the offset the reading stopped at,
so no consumer can hold a prefix while believing it has a dictionary; and both readings are one
function, so the null rule, the duplicate key and the length bound cannot disagree between them.
The one consumer is `pdf_model::Pages`' recovery, and it takes a prefix by **two doors**: where the
entries that were whole *themselves* state Table 31's `/Type /Page`, and — since ADR 0786 — where
§7.7.3.2's `/Kids` names the object and the entries hold one only a page object may carry. **The
general form is worth more than the clause**: where the standard says a thing's parts are unordered,
the ordering that produced a prefix is not evidence about the thing, so the prefix is answerable as a
*subset* and never as the thing. ADR 0784.

**And which evidence a prefix can carry depends on where the candidate came from, which is the
second door's whole argument.** ADR 0784's consumer finds its candidates by scanning the file, where
an object that says nothing about itself could be anything, so it needs the object's own
declaration; the second is *handed* its candidate by the page tree, and §7.7.3.2 has then already
said the object is a page object or a page tree node — "[t]he children shall only be page objects or
other page tree nodes" — while the sentence after Table 30 closes what the second of those may hold:
a node "may contain further entries defining inherited attributes for the page objects that are its
descendants", which §7.7.3.4 enumerates as four. So a prefix holding `/Contents` or `/Annots` was
written by a producer describing a page. **The evidence has to be a Table 31 entry being *present*,
never a node's entry being absent**, and §7.3.7 is why: a subset says what the producer wrote and
nothing about what it did not, so "this dictionary states no `/Kids`" is not knowable from one. The
file that makes the difference concrete is `poppler-355-0.pdf`, whose prefix is `/Parent`, `/CropBox`
and a key in neither table — under a rule of *not a node's entry* it would be taken, and under the
positive list it is refused, which is the substitutive direction this trap forbids. ADR 0786.

**And the answer can differ between two *routes* over one clause, which is not the drift it looks
like.** ISO 32000-2 §7.4.3 makes a character outside base-85's alphabet one that "shall cause an
error", and this crate answers it twice: `filter::ascii85`, which every consumer but one reaches,
refuses the whole stream; `filter::Ascii85`, the same clause driven a window at a time, reports
`Damage::Corrupt` over the groups it has already handed to a lexer. The seven-hundred-and-fourteenth
session made them agree — on the reasoning above, that the groups before the character are the
producer's own — and `display_list_digest` moved exactly one corpus document:
`PDFBOX-3148-2-fuzzed.pdf` states its **cross-reference stream** as `/Filter [/ASCII85Decode]` with
a bad byte eight bytes in, and eight bytes handed back as a decode are a cross-reference *section*
with almost every entry missing, so the file's only page disappears in silence where refusing sends
the parser to its header scan and finds it. The test above is what decides it, applied to the route
rather than to the filter: **a window is only ever run over §7.8.2's "sequence of instructions",
where a prefix is a shorter one of the same kind, and the buffered route serves the tables, the
font programs and the profiles, where it is not.** Two answers, one clause, two populations. ADR
0587.

**ADR 0356 found a better question one clause along, and a sharper form of the test.** Ask first
whether the standard states the thing's *extent*: §7.3.8.2 infers an image's length from its own
dictionary and §7.10.2 states a sample array's outright, so both are decidable without knowing
whether a filter failed — a short stream is short however it got that way, and a producer's own
arithmetic being wrong is the same defect as a truncation. Then ask whether the prefix's marks are
**places** or **values**. An image's missing samples are places on the page, and a place with no
sample can be left unpainted; a function's are values of a mapping evaluated over its whole domain,
where a missing value is read, decoded and interpolated into the ones beside it. So one is drawn
and reported and the other is refused, from one clause family. An ICC profile needed neither,
because Table 65 states the whole recovery for a profile a reader cannot use — what it needed was
not to be *parsed*.

### 8. A corpus finds what documents contain, not what the specification says

The mirror of trap 4. The ICC evaluator agreed with two other readers on every real profile; a
profile assembled *by hand* turned white into pure green. `calrgb.pdf` page 14 states
`BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`, which Table 63 permits and no sane
producer writes.

**Two rules have been measured to be unreachable by all 974 documents, and the method is worth
as much as the finding.** §9.7.6.2's per-byte codespace test and §12.5.2's rule that a stored
appearance ignores `/CA` were each measured by breaking the rule and running both gates: all 1794
verdicts identical. **That turns "the corpus does not cover this" from a suspicion into a fact.**

**A third stood here for a hundred and eighty sessions and was wrong, and the way it was wrong is
the lesson.** It said §7.6.2's signature exception was unreachable because "eight documents carry a
signature dictionary, twenty-six an `/Encrypt`, and the two sets are disjoint, which is one `grep`".
The grep was right about the two sets it counted and **the sets were not what it thought**:
`issue17069.pdf` is in both, and the reason it was not counted is that the code being justified
could not see it — its signature dictionary states no `/Type`, which Table 255 permits, so
`is_signature_dictionary` said no and the 33 680-byte signature value went through AES and came back
empty (ADR 0215). **A measurement taken with the instrument under test is not independent of it.**
The two rules above were measured by *breaking* the rule and watching a gate move, which is the
method that does not have this failure; a census whose predicate is the thing being checked does.

**A fourth shape: a rule the corpus *does* exercise and cannot show you.** Three documents delete
an object in an incremental update and none still references what it deleted, so a reader that
resurrects a deleted object renders all three byte-identically.
`pdf-syntax/tests/cross_references.rs` pins §7.5's rules by hand for that reason, each as a *pair*
of files differing only in the rule.

### 28. A recovery's guard is a claim, and the comment above it is a *different* claim

`Pages::new`'s recovery scan — the one that finds a page by Table 31's `/Type /Page` declaration
when the tree cannot be walked — carried this comment for seven hundred and fifty sessions:

> It runs only where the tree produced nothing, so no document that opens normally pays for it.

The guard beside it was `count == 0`, and `count` is §7.7.3.2's `/Count` — the file's *claim* about
its descendants. Those are not the same condition. A root stating `/Count 5` over five `/Kids` that
are not in the file produces no page at all, and no scan either, because the claim is not zero: the
document opened, reported nothing, showed nothing, and answered `len() == 5`. The comment was never
true of the code; it was written as the *intent* and read ever after as the behaviour, which is
worse than a comment that went stale, because nothing about the tree moving underneath it would
ever have made it wrong.

**The test that would have found it is the fixture where the two conditions differ**, and it is
cheap: `crates/pdf-model/tests/page_tree_nodes.rs` is nine pairs of hand-built files, every one of
them built to separate a different pair of conditions, and not one of them separated *these* two —
a `/Count` that is positive from a tree that reaches a page. That is the general form, and it
transfers to every fallback in this tree: **a recovery's guard states when the recovery is needed,
its comment states when the recovery is right, and the round that writes one owes the file where
those two disagree.** ADR 0782, session 858. The habit's other half is trap 5's, and the same
session paid for that too: the first fix made `len()` follow the recovery even where the recovery
found nothing, which turned *this reader could not read the pages* into *the file says it has none*
and left a §7.5.7 refusal with no page to be reported on.

## Things worth knowing

- **A recovery searches for something, and *where that thing can be* is a claim the standard
  settles.** A rebuild of a damaged cross-reference table looked for its trailer by searching the
  bytes for the `trailer` keyword — reasonable, and blind to the file §7.5.8.1 describes, in which
  "the keywords xref and trailer shall no longer be used" and §7.5.8.2 puts Table 15's entries in
  the cross-reference *stream*'s dictionary instead. So the recovery found `/Root` by scanning for a
  catalogue and silently lost everything else the trailer said. **`/Encrypt` is what made that a
  correctness defect rather than a lost convenience**: two documents opened as though they were not
  encrypted and handed back every string and stream as ciphertext, reporting nothing — trap 5's
  failure reached through a *fallback* rather than through a feature, which is why no report could
  have caught it. The general form: when a fallback looks for a token, read the clause that says
  which files write that token, because the files a fallback runs on are exactly the ones least
  likely to. ADR 0781.

- **A font is reported as a whole, and that is not fine-grained enough.** `FontError` is the only
  channel a font has, so a font that maps *some* of its document's codes draws those and says
  nothing about the rest. `LoadedFont` distinguishes "this code has no glyph" from "this code's
  glyph is blank", which a space legitimately is, and the corpus gate prints both counts — **and a
  third, which is the one nobody had**: a code no method of §9.10.2 could name, which is the
  *reading* band rather than the drawing one and is two orders of magnitude wider (ADR 0311).
  `examples/unnamed_code_census` splits it by which method the font could have answered with (ADR
  0318). **None of the three is a report**, deliberately, on ADR 0152's arithmetic: a report takes
  a page off the oracle's judged set, and these are shortfalls in the readback of pages that mostly
  draw perfectly. **What to do with them is settled and is neither of the two things that sentence
  weighed** (ADR 0422): the three are one value on `Interpretation::shortfall` and they *cross* —
  `Query::Readback` beside `Query::Reports` for a host, §14.7's status group for a screen reader,
  `pdf-retrieve`'s `readback` object for a program — worded apart from a refusal, because a code
  §9.10.2 ends at is the standard's own answer and not something this program failed to do. The
  reading that settled it is on §9.10.2's ledger row: all three methods are walked, per code, in
  the clause's order, and the second and third cannot be tried out of order because no font can
  satisfy both conditions.

- **A standard-library parser is a grammar you did not write, and the difference is yours to
  enumerate.** `str::parse::<f64>` accepts an exponent, `inf`, `infinity` and `NaN`; ISO 32000-2
  §7.3.3 has none of the four, and its two forms are closed — Errata Collection 3's Issue #327 adds
  a railroad diagram of each above its EXAMPLE, with no production either figure lacks. Reaching
  for the library parser under a comment quoting the clause is how a departure gets written down as
  a conformance. **The rule is not "do not use it"** — three of those four spellings never arrive
  here, because both of §7.3.3's forms are "one or more decimal digits" and the condition ADR 0303
  added returns a run holding none as the keyword it lexically is, so the digit test is what keeps
  a library grammar inside a clause's. The rule is that the *difference* is enumerated by
  experiment, decided per item, and written beside the parse. `pdf_syntax::lexer::read_number` and
  `pdf_model::function::compile_token` are the two places in this tree where a Rust parse stands
  for a §7 grammar at all; every other one — §7.9.4's dates, `startxref`, §12.3.5.2's folder key,
  `Hn`, Annex O's fragment parameters — gates the digits itself first, which is why the sweep for
  this pattern is short. ADR 0733, session 800.

- **And what the parser does at the *edge* of its range is a second question the clause answers.**
  An overflow to infinity was mapped to `Integer(0)`, which is the smallest magnitude in place of
  the largest and — unlike a refusal — draws: a coordinate at the origin, a font size of nought.
  §7.3.3 says the range "may be limited by the internal representations used in the computer on
  which the PDF processor is running", so a limit is the clause's permission and the *value* of the
  limit is what a reader owes; it is now the largest finite double with the file's own sign. The
  run reaching it is often a perfectly conforming number — four hundred decimal digits — so this
  was not a malformed-file tolerance but a conforming number silently replaced, and no corpus
  document exercises it, which is exactly why it survived.
