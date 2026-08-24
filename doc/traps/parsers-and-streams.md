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
oracle's judged set for nothing. **The condition on all three is `is_hidden()`**: §8.11.3.1 skips
hidden content "as if there were no `Do` operator to invoke it", and a `Do` that was never invoked
cannot have failed.

**Thirteen places report *while* drawing, each deliberate**, and the test for adding a fourteenth
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
(ADR 0563). **Its `h` twin is deliberately *not* the fourteenth**, on trap 11's side of the same
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

## Things worth knowing

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
