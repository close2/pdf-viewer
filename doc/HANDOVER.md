# Handover

Written 2026-07-26, updated 2026-07-30 at the end of the **twentieth** working session. Read
`/CLAUDE.md` first — it holds the five non-negotiable principles, what *done* means, and the
closed list of exclusions. **Principle 5 is the one that changes how to work**: the
specification is the only source of truth, and agreement with poppler, mupdf or pdf.js is
evidence that we read it right, never the definition of right. `doc/PLAN.md` holds the phases
and the conformance ledger's design; `doc/adr/` holds every decision's argument. **This file
is only the state of play, the traps, and what to do next** — when something here is also
written there, it is a pointer.

## What the twentieth session changed

**A composite font is two mappings, and for nineteen sessions this tree only worked where both
were the identity.** §9.7 was the largest gap of any kind on the demand list — 14 corpus fonts
named an embedded `CMap` stream, 41 more carried a `/CIDToGIDMap` that was not the name
`Identity`, and between them **31 documents drew no text at all** on page one. Both are built
(ADR 0029), and the demand item and the spec item were the same clause family, which is the
ninth session's ideal shape and the second time it has been available.

**Reading §9.7 as a family is what explains why the Identity case had been enough.** The clause
describes two independent mappings — §9.7.6.2's code to a CID and §9.7.4.2's CID to a glyph
index — and under `Identity-H` with `/CIDToGIDMap /Identity` *both* are the identity, so neither
has to be read and a code can go straight to a glyph index. That is not a simplification of the
model; it is the one configuration in which the model is invisible, and it is what almost every
modern producer emits.

Three of the decisions were the clause's rather than the code's, and each was settled by a
sentence rather than by a reference renderer:

- **A code's length is part of its identity.** §9.7.6.2 looks a code up "in the character code
  mappings for codes of that length", so the one-byte code `20` and the two-byte `0020` are
  different codes with the same value, and a `CMap` may define both. `issue2931.pdf`'s whole
  codespace is one *one-byte* range `<20> <76>`; `issue18117.pdf` mixes one, two, three and four
  bytes inside a string. So `Code` carries its length — which is also what §9.3.3's word spacing
  needs, and `has_single_byte_codes` is gone: the rule was answered per *font*, which was exact
  only because every mapping this crate built was wholly one or wholly two bytes.
- **A codespace range is matched byte by byte**, not by comparing the whole code: `<C280>
  <DFBF>` admits `C2 80` and not `C2 C0`. Four corpus documents write UTF-8-shaped `CMap`s where
  that is the difference — and **no corpus document can tell the two readings apart.** Swapping
  in the numeric comparison and running the whole oracle leaves all 1794 verdicts identical,
  which was checked rather than assumed, so a synthetic test is the only thing in the tree
  holding the clause to its words.
- **`bfchar` in an `Encoding` `CMap` is forbidden by §9.7.5.4 c) and named by §9.7.6.2**, whose
  own account of the decoding algorithm lists "the mappings defined by `beginbfchar` … and
  corresponding operators for ranges". `bug920426.pdf` writes an `Encoding` `CMap` whose only
  mappings are `bfchar` lines. Two subclauses disagree; the one describing what a *processor*
  does is followed, the destination is read as the character selector §9.7.5.1 says a `CMap`
  yields, and the page draws "Checkliste Service" instead of nothing.

**Then the page overturned the first reading of Table 115, which is the fourteenth session's
lesson arriving from the other side.** Table 115 conditions `/CIDToGIDMap`'s *presence* on Type 2
CIDFonts — "Required for Type 2 CIDFonts with embedded font programs" — and the first
implementation here read that as a restriction on its *meaning*, ignoring the stream for a Type 0
CIDFont because §9.7.4.2 gives a CFF its own route. `issue7901.pdf` is a `CIDFontType0` whose
`/FontFile3` is an `OpenType` wrapper around a **name**-keyed CFF, carrying a `/CIDToGIDMap`
stream of 230 entries: under that reading it drew `üãÍ†Ë œÍ†ÿ¨ Ì{«` where four renderers draw "The
Free Software Definition". Table 115's definition of the entry — "A specification of the mapping
from CIDs to glyph indices" — is unconditional, and a reading that discards it makes the file's
own statement mean nothing. **A presence condition is not a restriction on meaning.**

**And §9.7.6.3's recovery is where we now depart from two references, deliberately.**
`issue11768_reduced.pdf`'s `CMap` is `UniJIS-UTF8-H`, whose codespace admits no one-byte code
above `7F` — and it writes `1 begincidchar <e0> 151`, a one-byte mapping for a code its own
codespace excludes. §9.7.6.2 makes each `e0` invalid; §9.7.6.3's modified algorithm then consumes
the three bytes its partial match implies. `mupdf` and `ghostscript` take the mapping's own
length and draw three hyphens; `poppler` and this tree draw one `.notdef`. The standard would not
have written a two-rule recovery algorithm for invalid codes if a mapping's length were meant to
override the codespace.

| | was | is |
|---|---|---|
| **an embedded `CMap` stream** | refused and reported, 14 fonts | parsed: codespace, `cidrange`, `cidchar`, notdef, `usecmap` |
| **a code's length** | two bytes, always, for a composite font | whatever the codespace ranges say, 1 to 4 |
| **§9.3.3's word spacing** | a property of the font | a property of the code (`Code::takes_word_spacing`) |
| **`/W`'s widths** | looked up by character code | looked up by CID, which is the same only under Identity |
| **a `/CIDToGIDMap` stream** | refused and reported, 41 fonts | Table 115's two big-endian bytes at 2c |
| **a `/CIDToGIDMap` on a `CIDFontType0`** | ignored, and one page drew garbage | applied; the presence condition is not a meaning condition |
| **an `OpenType`-wrapped CID-keyed CFF** | CIDs taken as glyph indices | through the CFF's charset, per §9.7.4.2 |
| **a `notdefrange`** | — | one CID for the whole range, not consecutive ones |
| **a `usecmap` with no `/UseCMap`** | — | refused: what it inherits cannot be found (§9.7.5.4 a)) |
| **§9.7** | `unreviewed`, seventeen rows | reviewed: 7 implemented, 8 partial, 1 reported, 2 inapplicable |
| **Table 58** | cited six times for `/SA`, `/OP`, `/op`, `/OPM` | **Table 57** — the checker printed the title and the pairing was wrong |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 735 | **766** |
| corpus documents reporting something | 220 | **189** |
| of those, reporting a *font* | 100 | **67** |
| pages we call complete, in the oracle | 1525 | **1557** |
| of those, agreeing with the reference consensus | 688 | **706** |
| of those, contradicted | 96 | **98** |
| ledger subclauses nobody has read | 609 | **594** |
| `§` citations the checker verified | 725 | **827** |
| tests | 417 | **440** |

**The reported count fell by 31 and that is the largest single movement since
`CCITTFaxDecode`.** Two rows of the corpus gate's breakdown *rose* by one each — annotations 66
to 67 and transparency groups 17 to 18 — and neither is a regression: the breakdown counts each
document's *first* report, so a document whose font report vanished now reports its annotation
instead. Nothing left on the font row is a `CMap` question: 27 fonts have no `/ToUnicode` so a
substitute cannot be addressed, 21 have a substitute that draws none of their declared codes, 15
name one of Table 116's predefined `CMap`s — registered data files, so a licensing decision — and
4 ask for vertical writing.

**32 pages joined the oracle's judged set, 18 of them agreeing, and the two contradictions are
diagnosed.** Neither is a `CMap` defect:

- `issue7901.pdf` draws its sentence correctly. It is 200×40 pixels of eight-pixel text and meets
  every *absolute* bound with room — mean 3.95 against 5.00, worst tile 9.95 against 40.00, SSIM
  0.9683 against a 0.9900 floor. What fails is the differing-fraction bound at 9.89%, which on a
  page that is nothing but glyph edges is the anti-aliasing of every letter; the heatmap is the
  outline of each word and nothing else.
- `issue20232.pdf` is missing one glyph and the clause is **§9.6.5.4**, not §9.7: a simple
  `TrueType` font named `Symbol_A` whose `/Differences` calls code 71 `/Ccedilla` while its subset
  holds the *diameter* sign there, with a descriptor whose `/Flags` is 36 — the Symbolic and
  Nonsymbolic bits at once. §9.6.5.4 says the `/Encoding` "is ignored" when Symbolic is set, which
  is the route that would find the glyph; a font claiming both flags leaves it unreachable. Listed
  as `CONTRADICTED_SYMBOLIC_FONT_FLAGS` rather than chased, because ADR 0015's fifteen pages are
  what is at stake in changing that route.

**Interpretation costs +0.44%** — 1.9259 G instructions to 1.9344 G by callgrind on
`examples/callgrind_interpret` — and the baseline was measured on this machine, in a worktree at
the previous commit, rather than taken from the fifteenth session's note. That buys a codespace
scan and two map lookups per code where there had been a fixed two-byte chunk. The corpus gate is
unchanged at 1.9 s.

What it taught:

- **The configuration where a model is invisible is the one everybody ships.** `Identity-H` with
  an identity `/CIDToGIDMap` collapses both of §9.7's mappings to nothing, so nineteen sessions of
  real documents never asked what either one *is*. The tree was not missing an edge case; it was
  missing the clause, and it could not tell because the clause's degenerate case is the common
  case. **Ask what a feature looks like when its parameters are not the defaults** — that is the
  same question the fourteenth session asked about `Tk`'s initial value, from the opposite end.
- **A presence condition is not a restriction on meaning.** "Required for Type 2 CIDFonts" says
  when the key must appear, and Table 115's next sentence says what it means, unconditionally. The
  first implementation read the first sentence as bounding the second and drew one page as
  garbage. Nothing but the picture could have said so: the code was right about the clause it
  cited, and it had cited the wrong half of the table.
- **A rule the corpus cannot exercise still has to be right, and the only way to know is to break
  it and run everything.** §9.7.6.2's per-byte codespace test is load-bearing for any valid PDF
  and *invisible* across all 974 documents: the numeric reading leaves 1794 oracle verdicts
  unchanged. Establishing that took one edit and two gate runs, and it turned "this is the clause"
  into "this is the clause, and here is why no gate defends it". Trap 8 in its most literal form.
- **The citation checker found a wrong table again, three sessions after it was built for exactly
  that.** `/SA`, `/OP`, `/op` and `/OPM` are Table 57's entries in a graphics state parameter
  dictionary; six comments called them Table 58, which is *path construction operators*. The
  thirteenth session's decision to have the gate **print every cited table's title** rather than
  assert a pairing is what surfaced it — the wrong line is obvious in the output and no exception
  list was needed. A gate that prints is a gate that keeps working after its author stops looking.
- **A `notdefrange` is not a `cidrange`, and a test caught it where reading had not.** ISO 32000-2
  does not state the difference; §9.7.5.3 hands the file's syntax to Adobe Technical Note #5014,
  and §9.7.6.3's purpose for the mapping — "to obtain a substitute character selector" — implies
  it. The first draft numbered notdef CIDs upward like a `cidrange`, which is a run of substitutes
  the `CIDFont` has no reason to carry. **Where the standard defers to another document, the
  deferral is a citation too.**

## What the nineteenth session changed

**The largest gap on the demand list turned out not to be a gap, and the clause says so
twice.** Overprinting was the eighteenth session's leftover and the ledger's newest silence —
six `silent` rows, `/OP`, `/op` and `/OPM` read nowhere, 63 corpus first pages enabling it.
Reading §8.6.6 and §8.6.7 as a family (because overprinting is stated in terms of the
colourants a `Separation` names, and §8.6.6 is where those live) settled it in the opposite
direction from the one the list predicted:

- **§8.6.7 says what a device without separations does.** NOTE 1: "Not all devices support
  overprinting. … If overprinting is not supported, the value of the overprint parameter shall
  be ignored." And the overprint *mode* is settled by a `shall not` in the body — "It also
  shall not apply if the native colour space of the output device does not include CMYK device
  colourants; in that case, source colours shall be converted to the device's native colour
  space, and all components participate in the conversion, whatever their values."
- **§11.7.4's Table 146 collapses, and that is the interesting half** because it is derived
  rather than asserted. The table is indexed by the source space and by which component *of the
  group space* is affected; this group space is device RGB, three process components and no
  spot colourants. Every `spot colourant` row — the only ones whose `OP true` cells give the
  backdrop — has no component to affect. Row 1's `OPM 1` cell, the whole of what `/OPM 1` asks
  for, needs the group space to be `DeviceCMYK`, which §11.7.4.3 states in words. And the
  `Separation`/`DeviceN` rows are unreachable because §8.6.6.4 makes an additive device revert
  to the alternate space *always* and §11.7.4.3's NOTE 2 says the reverted space is the current
  one. `B(C_b, C_s) = C_s` everywhere, which is Normal, which is what we composite through.
- **The honest limit is one configuration, and it is already reported.** A group whose blending
  space *is* `DeviceCMYK` would reach row 1 — and §11.6.6 has reported exactly that on 4
  documents since the seventeenth session. Overprinting's one visible case here is a gap with a
  name already.
- **One requirement of §11.7.4 is a real gap and has nothing to do with overprinting.**
  §11.7.4.4's second bullet applies whenever overprinting is *disabled*, which here is always:
  `B` and its relatives "shall" become a non-isolated knockout group. That is §11.6.2's gap
  seen from clause 11, reported on the same condition.

**Reading that family for the overprinting question found two colourant names this tree had
never read, and both were being answered by a function the clause says to ignore.**
`/Separation /None` "shall not produce any visible output" — it is how a producer puts die
lines and technical marks in a file — and we painted them in whatever colour the tint transform
made. `/Separation /All` marks every colourant at once, and on an additive device "the
subtractive tint values … shall be complemented by subtracting from 1", so a tint of `t` is the
grey `1 − t`; we ran the transform instead. Both are now decided *before* the alternate space
and the tint transform are parsed, because §8.6.6.4 requires those to be ignored "although
valid values shall still be provided" — so a file that fails to provide them still gets the
colourant a processor "shall support … on all devices".

**Then `/SA` — the next silence — put one device pixel in `pdf-render`, and that is where a
fifteen-session GPU defect was waiting.** §10.7.5 requires a stroke under half a device pixel
to be drawn as a single-pixel line when stroke adjustment is enabled, and §8.4.3.2 requires the
same width of a zero-width stroke; §10.7.5's NOTE says the two are equivalent, so they are one
function both backends call. `tiny-skia` answers §8.4.3.2 by itself, since a width of `0.0`
selects its hairline mode — and **`kurbo` expands a zero-width stroke into an empty outline, so
every `0 w` line in every document was invisible on the GPU backend.** `zerowidthline.pdf` is
in the corpus, is named for this, and says on its own face "second should be 1 device pixel,
third should also be 1 device pixel (but scaled 2x)": the GPU drew neither, and drew none of
the page's stroked text either, because text rendering mode 1 at zero width is the same case.

| | was | is |
|---|---|---|
| **`/Separation /None`** | painted, through its tint transform | marks nothing, per §8.6.6.4 |
| **`/Separation /All`** | painted, through its tint transform | the tint complemented, in all three colourants |
| **a `DeviceN` of only `/None`** | painted | discarded without reverting (§8.6.6.5) |
| **`/All` or `/None` with an unreadable transform** | the space refused and reported | the colourant honoured; the transform is ignored anyway |
| **`0 w` on the GPU backend** | drew nothing at all | one device pixel, as on the CPU |
| **`/SA`** | read nowhere, nothing said | read, and a sub-half-pixel stroke is one pixel |
| **one device pixel** | `tiny-skia`'s convention, in a backend | `Stroke::device_width`, in `pdf-render` |
| **a stroke's bound under a shear** | `sqrt(\|det\|)`, called "the safe way round" | `Transform::max_stretch`, which is |
| **§11.7.4** | six `silent` rows, 63 documents | four subclauses satisfied by derivation, one `reported` |
| **§8.6.6, §8.6.7** | `unreviewed` and one `partial` | reviewed: three implemented, three inapplicable |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 735 | 735 |
| corpus documents reporting something | 220 | 220 |
| pages we call complete, in the oracle | 1525 | 1525 |
| of those, agreeing with the reference consensus | 688 | 688 |
| of those, contradicted | 96 | 96 |
| ledger subclauses nobody has read | 615 | **609** |
| ledger rows that are `silent` | 8 | **1** |
| `§` citations the checker verified | 674 | **725** |
| tests | 401 | **417** |

**Neither gate moved, and the session's own instrument says why.** Of the corpus's first pages,
17 paint a stroke while `/SA` is in force and only **4** have one thin enough to adjust at 72
dpi; of those 4, two agree with the references either way and two are pages the references
cannot agree about among themselves. On those two the change moves us *closer* —
`bug1721218_reduced.pdf` from mean 0.28 to 0.27, worst tile 18.58 to 18.41 — which is the right
direction and far too small for the gate to see. The GPU defect is not on either gate at all,
because both gates render on the CPU.

What it taught:

- **A demand count can count a key rather than a difference, and the narrow condition can be
  empty.** 63 documents enable overprinting; zero of them can show it on this device. The
  eighteenth session's own note said "presence of the key is not the condition" and trap 11 says
  to instrument before believing a count — what neither anticipated is that the honest condition
  would have no members. The instrument that settled it was not a corpus run but Table 146 read
  against a list of this device's colourants. **A gap sized by a corpus is a hypothesis about a
  clause.**
- **A rasteriser's convention is not a reading of the clause, and the one that agrees with PDF
  is the dangerous one.** `tiny-skia`'s hairline made §8.4.3.2 free on the CPU, so nobody wrote
  the rule down, so the GPU never got it. Fifteen sessions of cross-backend scenes could not
  see it because every scene stroked a width the document stated — trap 2 again, and the second
  consecutive session to find it in the place this project trusts most. **Where two backends
  are the oracle, a decision either of them can make alone is a decision neither has made.**
- **The ledger was wrong for the second time, and in the same shape as the first.** §8.4.3.2's
  row said a zero width "reaches the rasteriser as the thinnest line it draws" — true of
  `tiny-skia`, false of Vello. A row that names a rasteriser's behaviour has recorded that
  rasteriser rather than the clause, exactly as §8.9.5.3's row had recorded a belief about
  §10.7.4 without opening it.
- **A comment can assert the safe direction and have it backwards.** `command_bounds` derived a
  stroke's margin from `sqrt(|det|)`, "an over-estimate for a sheared one, which is the safe way
  round". The determinant is the *geometric mean* of the two singular values, never the larger,
  so a shear left the margin too small and §11.4.6's overlap report could miss an overlap. The
  claim had been sitting beside the arithmetic that refutes it.
- **A page can be the oracle when it was written to be one.** `zerowidthline.pdf` states the
  expected result in a caption. Two of the corpus's 974 documents are like this and both have
  now paid for themselves; opening the six documents that stroke a zero width took four minutes
  and one of them answered the question outright.

## What the eighteenth session changed

**A soft mask in an `/ExtGState` is applied, which was the largest reported rendering gap
left.** 28 corpus documents said `SMask in /GSn` and drew the object opaque; the whole
`Shading` row of the corpus gate's breakdown was that one report. §11.5's mask is a
transparency group evaluated for its *opacity* rather than its colour — its alpha (§11.5.2)
or the luminosity of its colour over a chosen backdrop (§11.5.3), through a transfer function
— so the seventeenth session's groups are what made it buildable, exactly as this file
predicted. ADR 0027.

**The design question was not what a mask says but where it is evaluated**, and the answer
shapes three crates:

- **The display list carries a mask's commands, not its pixels.** A mask is a coverage per
  *device* pixel, and `pdf-model` does not know a resolution — the same list is drawn at every
  zoom and by both backends. So `DisplayList` grows a table of `SoftMask`s beside its clips and
  every command grows `mask: Option<SoftMaskId>` beside its `clip`. Per command, because
  §11.6.4.3's NOTE 2 says a mask applied to two overlapping objects "multiplies with itself in
  the area of overlap" — applying it once to a run of them would be the other picture, the one
  the NOTE tells producers to get by grouping.
- **What the pixels mean is decided once, for both backends.** `SoftMask::value` turns one
  rendered pixel into one mask value, and this is load-bearing rather than tidy: §11.5.3's
  device formula is `Y = 0.30 R + 0.59 G + 0.11 B`, and **both rasterisers offer a luminance
  mask of their own that is not it** — `tiny_skia::MaskType::Luminance` is Rec. 709, Vello's
  `push_luminance_mask_layer` is the SVG formula. On grey artwork every formula agrees, which
  is 64 of the corpus's 134 mask dictionaries and how this would have shipped unnoticed. On
  green they are a fifth of the mask's range apart.
- **The GPU renders each mask to a texture and reads it back.** Vello expresses half of this
  natively — `Compose::DestIn` is §11.5.2 exactly — and no blend mode is §11.6.5.1's `/TR`,
  which 11 of those 134 dictionaries carry. A round trip per mask buys the two backends *the
  same mask*, which is the premise the cross-backend comparison rests on, and the corpus says
  what it costs: the heaviest first page of the 974 registers 27 masks, and the corpus gate's
  wall clock did not move.

**Then reading §11.7 as a family — because `/BC` is stated in a group's blending colour space
and §11.7.2 is the clause that says what one is — found the session's silence.** §11.7.4,
overprinting: `/OP`, `/op` and `/OPM` are read nowhere, and **63 of the corpus's first-page
`/ExtGState` dictionaries set one of the two booleans true**. Under §11.7.4.2 an object painted
with overprinting enabled composites through a special blend mode that leaves the backdrop's
value in every component the source does not paint; here it composites through Normal with
nothing said. Six `silent` rows for one gap, recorded where a reader of any of them would look.
The family also produced a row *satisfied by a decision taken for another reason*: §11.7.3
requires that "spot colours shall not be available in a transparency group XObject that is used
to define a soft mask; the alternate colour space shall always be substituted in that case",
and this tree converts every `Separation` and `DeviceN` colour through its tint transform at
the moment it is read — so the sentence is true here by construction.

**And a report that had been hidden behind another report appeared.** `knockout_smask.pdf`
paints an opaque blue over an opaque red inside a knockout group, *under a mask*. §11.4.6's
report fires where an element that composites overlaps one painted before it, and an opaque
fill under a soft mask composites — `command_composites` had not known that, because until
this session no command could carry a mask. The page reported the mask instead, so the
knockout gap was invisible underneath it.

| | was | is |
|---|---|---|
| **`/SMask` in a `gs`** | reported, the object drawn opaque | evaluated, per §11.5.2 or §11.5.3, with `/BC` and `/TR` |
| **the mask's coordinate system** | — | `/Matrix` × the transform at the `gs`, not at the painting |
| **outside a mask group's `/BBox`** | — | the transfer function of 0.0, or of the backdrop's luminosity |
| **an image with its own `/SMask` under a `gs` mask** | would have applied both | the image's wins, per §11.6.4.3 |
| **a mask group's `/CS`** | — | read for `/BC`'s components, and reported where it is not the device's |
| **an opaque fill under a mask** | did not count as compositing | does, which is what §11.4.6's report needed |
| **§11.7** | `unreviewed`, fourteen rows | reviewed: one satisfied, two inapplicable, six `silent` |
| **the 14 specification PDFs in `doc/`** | three reported a soft mask | **all fourteen draw page one with nothing reported** |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 718 | **735** |
| corpus documents reporting something | 237 | **220** |
| pages we call complete, in the oracle | 1505 | **1525** |
| of those, agreeing with the reference consensus | 676 | **688** |
| of those, contradicted | 93 | **96** |
| ledger subclauses nobody has read | 629 | **615** |
| ledger rows that are `silent` | 2 | **8**, and they are three gaps |
| `§` citations the checker verified | 557 | **674** |
| tests | 385 | **401** |

The contradicted count rose by three and none of the three is a masking defect. `issue21346.pdf`
and `issue7891_bc1.pdf` are pages that became *comparable*: the first is 178 pixels square where
`poppler` and `mupdf` produce 179 and its colour matches three references exactly, the second
differs only in the edge coverage of six-pixel glyphs at a mean error of 0.22. The third is the
interesting one and it is now `CONTRADICTED_MASK_QUANTISATION`.

What it taught:

- **A mask value is eight bits, and on a flat page that is a whole level.**
  `smask_luminosity_oob_transfer.pdf` paints one rectangle over the whole page through a mask
  of 0.75. The closed form is `(223, 99, 80)`; we give `(223, 100, 81)`, `mupdf` `(222, 98, 79)`,
  `ghostscript` `(223, 99, 79)`. Everybody is within a level of the arithmetic — but `mupdf` and
  `ghostscript` are within a level of *each other*, so the bound derived from them is a mean of
  1.11 and ours is 2.02. **A tolerance derived from two references that agree closely is tighter
  than the arithmetic anyone is doing**, and that is a property of the gate worth knowing before
  reading a small contradiction as a defect.
- **Reading the family found the gap the feature could not.** Overprinting has nothing to do
  with soft masks; §11.7 is where it lives, and §11.7 is where it lives *because* it is about
  colour spaces for transparency, which is what `/BC` needed. That is the seventh consecutive
  session where the family review produced something the demand item could not have reached.
- **A report can hide another report.** The knockout gap on `knockout_smask.pdf` was covered by
  the soft-mask report for four sessions. Every gap that reports takes a page out of the
  comparison, and a page out of the comparison is a page whose *other* gaps nobody is measuring
  — which is an argument for closing reports rather than accumulating them, and the reverse of
  the usual worry about over-reporting.
- **A library's luminance is not the clause's luminance, and grey artwork hides it.** Both
  rasterisers have a luminance mask primitive and both would have been wrong by a fifth of the
  range on coloured mask artwork while agreeing exactly on the 64 grey masks the corpus is full
  of. Trap 2's shape again: the natural test data cannot fail in the axis the defect moves.

## What the seventeenth session changed

**A `/Group` is a transparency group, and a form XObject's `/BBox` clips it.** Transparency
groups were the largest rendering gap the corpus sizes and owned **three of the ledger's five
`silent` rows** — §11.4.6, §11.6.6 and §11.3.7.3, one gap recorded three times because a reader
of any of those clauses should find it. They are built: `Command::Group` carries a nested
command list, `tiny-skia` composites it into a buffer of its own and a Vello layer *is* the same
construction, so both backends express it natively. §11.6.6's initialisation is the half that is
easy to leave out and the whole visible difference on an ordinary page — the blend mode and both
alpha constants are reset *inside* the group, because they belong to the group and applying them
to each element as well applies them twice. ADR 0026.

**Then reading §11.4 as a family found two defects that have nothing to do with groups, and the
second is the session.**

- **§8.10.1 step c) says `Do` "Clips according to the form dictionary's BBox entry", and only an
  annotation's appearance was clipped.** `issue11279.pdf` was contradicted by all three
  references for it. The cost is easier to see on `tracemonkey.pdf` page 6, which is not in the
  comparison because its fonts are substituted: a form painted a white background beyond its own
  box and **covered the figure above it**, so a page four renderers draw with two figures had
  one. Nothing reported, nothing measured it, and it had been true since the first form XObject.
- **§11.4.7's page group is *isolated*, and both backends were painting onto the medium.** "The
  page group shall be treated as an isolated group, whose results shall then be composited with
  a backdrop colour appropriate for the medium" — so a page's own initial backdrop is
  **transparent**, and white is applied to the finished page. Filling the raster with white and
  drawing over it is the natural implementation and a different picture for every blend mode,
  because §11.3.6 leaves an object blending against zero alpha its own colour and white is not
  zero alpha. `transparency_group.pdf` announces it on its own face: an ellipse under
  `/BM /Difference` that four references draw crimson over white, and that we drew as its
  inverse. Four pages improved by between 5.6 and **60.4** mean error.

**And that exposed a GPU defect fifteen sessions of tests could not see.** `read_back` converted
Vello's output from premultiplied to straight alpha; Vello hands back straight alpha already.
Every pixel came back with an alpha of 255 while the page was rendered onto an opaque
background, and the conversion is the identity there. The first render onto transparency showed
it in one pixel: half-covered by a 50% grey, `tiny-skia` gives `[128, 0, 0, 128]` and the GPU
gave `[255, 0, 0, 128]` — the colour divided by its own coverage.

**Three departures are reported rather than drawn wrong in silence**, each on the condition its
clause states rather than on the key's presence:

- **a non-isolated group** is drawn as an isolated one, which §11.6.7's NOTE 1 says is the same
  computation when every element blends Normal — so the report fires on a blend mode *inside*
  the group, which §11.4.4's NOTE 2 gives as the whole reason the two kinds exist. 9 documents.
- **a knockout group** (§11.4.6), where an element that composites overlaps one painted before
  it. 6 documents, against the 8 that write `/K true`.
- **a group blending colour space** that is not the device's three components. 4 documents, all
  `/DeviceCMYK`.

| | was | is |
|---|---|---|
| **a form XObject's `/BBox`** | clipped an annotation's appearance and nothing else | clips every form, per §8.10.1 step c) |
| **a `/Group` on a form** | read nowhere; drawn as an ordinary form | composited as one object, under `ca` and `/BM` once |
| **`ca` inside a group** | applied to every element *and* to nothing else | reset to 1.0 inside, applied to the group (§11.6.6) |
| **the page's backdrop** | opaque white, which every blend mode saw | transparent, imposed on the medium afterwards (§11.4.7) |
| **Vello's readback** | demultiplied, on a belief nobody had tested | taken as the straight alpha it is |
| **§8.10 and §11.4** | `unreviewed`, twelve rows | reviewed, with two defects and three departures named |
| **§11.4.6, §11.6.6, §11.3.7.3** | three `silent` rows | `reported` and two `partial` |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 724 | **718** |
| corpus documents reporting something | 231 | **237** |
| pages we call complete, in the oracle | 1513 | **1505** |
| of those, agreeing with the reference consensus | 676 | 676 |
| of those, contradicted | 100 | **93** |
| ledger subclauses nobody has read | 646 | **629** |
| ledger rows that are `silent` | 5 | **2** |
| `§` citations the checker verified | 491 | **557** |
| tests | 377 | **385** |

The reported count **rose** by six and that is the session's shape rather than a regression: six
knockout groups and one non-isolated group that blends began saying so, and one document
(`issue15372.pdf`) stopped reporting §9.3.8 because the constant alpha its glyphs carried is now
applied to their group instead. Seven pages left the contradicted list — one fixed, four by
being reported, two by the same report on another page.

What it taught:

- **A clause about the *page* was invisible until something needed the page to be a group.**
  §11.4.7 is two paragraphs and had been `unreviewed` since the ledger existed. Nothing in the
  tree had a reason to render onto transparency, so nothing could tell an opaque backdrop from a
  transparent one — and every blend mode in the corpus was composited against the wrong one. The
  family review is what found it, and the demand item is what made it findable.
- **A metric that gets worse can be the page getting better.** `tracemonkey.pdf` page 6 rose
  0.68 in mean error and gained a whole figure the references draw and we had been covering with
  a white box. The error rose because the figure's text is set in a font nobody embedded. Trap 1
  in a new direction: the number moved the wrong way for the right reason, and only the picture
  said which.
- **Fifteen sessions of cross-backend agreement proved nothing about alpha.** Every GPU test
  compared pages rendered onto opaque white, where premultiplied and straight alpha coincide.
  The oracle could not see it either, since it renders pages. A test whose input cannot exercise
  a conversion is not a test of it — trap 2, in the one place this project trusts most.
- **A report's cost is paid in the gate, and this time it bought the honest answer.**
  `knockout_*.pdf` are still contradicted; they are no longer *judged*. Four pages left the
  comparison for a gap that was already there and had simply never been said out loud. That is
  the third session running where the contradicted count fell partly because something started
  reporting, and the count means less every time it happens — which is why the table above says
  which of the seven were fixed.

## What the sixteenth session changed

**A reduced image is averaged rather than sampled — and the clause that governs it says not
to.** The last item on the short list three sessions had been working off was "a filter that
averages over the area a destination pixel covers", carried since the twelfth session because
`bug1001080.pdf` draws `pinL LesL` where four renderers draw `pint test`. It is built, both
backends share one function, and **three contradicted pages became agreeing** — the two the
item named and one it did not. But the interesting half is what reading the clause first did
to the argument.

**§10.7.4 addresses image reduction and forbids the fix.** "The position of the centre of such
a pixel … shall be mapped back into source space to determine how to colour the pixel. **There
shall not be averaging over the pixel area.** If the resolution of the source image is higher
than that of device space, some source samples might not be used." That is point sampling,
normatively, and it is the opposite of what every reference does and of what makes a page of
eleven-times-reduced fax glyphs legible. Three things had to be settled before departing:

- **This tree's comments said the standard was silent here.** `is_smoothed`'s doc comment and
  the ledger's §8.9.5.3 row both said reduction was something "the clause does not address" —
  meaning §8.9.5.3, which is about magnification. It does not; §10.7.4 does, and **nothing in
  the tree had ever cited §10.7 at all**. *"The clause says nothing" is a licence to choose;
  "the clause says the opposite" is a debt to record*, and the two had been confused for two
  sessions.
- **We already depart from the same subclause and had never said so.** §10.7.4's first rule is
  that a shape "shall be scan-converted by painting any pixel whose half-open square region
  intersects the shape, no matter how small the intersection is". Both backends **anti-alias**.
  That has been true since the first commit with no clause cited anywhere near it.
- **§10.7.1's NOTE is what licenses all three** — "the specifics of the scan conversion
  algorithm are not defined as part of PDF". §10.7.4 describes a device that quantises coverage
  to whole pixels; a display does not.

So it is taken as a *departure*, with its cost written down: a producer who relied on one
sample surviving the reduction — a one-pixel rule, a dither pattern — gets a softened version
of it instead. ADR 0025.

**The block boundaries were wrong first, and the corpus said so in one run.** Fixed multiples
of the reduction factor leave a short block at the right and bottom edge; giving that remainder
a whole output cell squeezes the image into 99.4% of the unit square. On `firefox_logo.pdf`
that moved the worst tile from 9.97 to **14.23** — *further* from three references than no
filtering at all. Proportional bands fixed it and have a test, because a sub-pixel geometry
error is invisible in every picture except the one that shows it.

**And the page that left the contradicted list unpredicted was filed under the wrong cause,
for the fourth time.** `french_diacritics.pdf` sat in `CONTRADICTED_PAGE_ROUNDING` because its
raster is 595x842 against `poppler`'s and `mupdf`'s 596. That is true, and it was not what the
references were disagreeing about: worst tile 12.60 against a bound of 5.89 before, inside the
bound after, from a change that touches nothing but the image path. Type 3 fonts, `/Rotate`,
`alphatrans.pdf`'s gradient, and now this.

**§10.7 got six ledger rows where it had none**, and the family review produced a fifth
`silent` row: **§10.7.5, automatic stroke adjustment**. `/SA` is read nowhere, 49 corpus
documents set it true, and a document that enables it gets an anti-aliased hairline rather
than the grid-snapped one the clause asks for. It is deliberately **not** reported yet, and
that is trap 11 rather than an oversight — see the table below.

| | was | is |
|---|---|---|
| **an image reduced eight-fold or more** | four taps of a bilinear filter, most samples unread | averaged over the samples that share a device pixel |
| **`bug1001080.pdf`** | `pinL LesL`, unreadable | `pint test`, agreeing with three references |
| **`firefox_logo.pdf`, `french_diacritics.pdf`** | contradicted | agreeing |
| **§10.7** | cited nowhere, six `unreviewed` rows | reviewed, with three departures named and one new `silent` row |
| **"the clause says nothing about reduction"** | in a doc comment and a ledger row | corrected: §10.7.4 says the opposite |
| **anti-aliasing** | done since the first commit, citing nothing | recorded as §10.7.4's first departure |
| **a backend's cost** | unmeasurable — `callgrind_interpret.rs` stops at the display list | `callgrind_rasterise.rs`, and four numbers |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 724 | 724 |
| corpus documents reporting something | 231 | 231 |
| pages we call complete, in the oracle | 1513 | 1513 |
| of those, agreeing with the reference consensus | 673 | **676** |
| of those, contradicted | 103 | **100** |
| ledger subclauses nobody has read | 652 | **646** |
| ledger rows that are `silent` | 4 | **5** |
| `§` citations the checker verified | 479 | **491** |
| tests | 367 | **377** |

Nothing moved on the corpus gate, and that is the shape of this session rather than an
oversight: it fixed *how* pages already drawn were drawn, so the only instrument that could see
it is the one holding us against other renderers. A session whose corpus row is flat and whose
oracle row moves by three is the reverse of the eighth session's, and both are progress.

What it taught:

- **Read the clause even when the fix is obviously right, because the clause may forbid it.**
  Every reference does area averaging, the page in question is unreadable without it, and the
  standard still says "there shall not be averaging over the pixel area". Finding that turned a
  two-hour improvement into a documented departure with three parts, one of which — anti-aliasing
  — the project had been doing silently since day one. Principle 5's direction of inference
  survives intact: the references are not why we average, they are evidence that a display is
  not the device §10.7.4 describes.
- **A benchmark that measures nothing looks exactly like a change that costs nothing.** The
  first four callgrind numbers for this change were flat to four significant figures. The
  example was passing 4096 as `for_page`'s *total pixel* budget rather than an extent, so every
  run panicked and callgrind counted the panic. A page-sized raster is half a million pixels and
  the argument is not named at the call site. **A suspiciously clean result is a reason to check
  the instrument**, and this is trap 4's shape inside a measurement.
- **A test of a filter has to put the filtered pixels where the tolerance can see them.** The
  first CPU-versus-GPU scene reduced a 64x64 image into an 8x4 corner of a 200x200 page and
  **passed with the GPU filter removed altogether** — 32 channels of 160 000 is under
  `MAX_DIFFERING_FRACTION`. Trap 2 has always said a scene must be able to fail in the *axis*
  the defect moves; it must also be able to fail at its *magnitude*.
- **Saturating arithmetic cost 8 points of the 17 this change spent on its worst page.** Plain
  arithmetic under an `#[expect]` naming the bound halved it. This is the seventh session's
  lesson arriving in a new loop, and the bound is the sort that is provable rather than assumed:
  a block holds at most as many samples as the image, and each contributes under 2^16.
- **A cosmetic entry and an unreadable page can be the same defect, and only the second gets
  built.** `firefox_logo.pdf` was 0.02 outside a bound for four sessions and was correctly sized
  "Small". Nothing about the defect changed when `bug1001080.pdf` joined it; what changed was
  the evidence available for ranking it.

## What the fifteenth session changed

**A soft mask of another size applies, which was the last raster this tree refused to
combine.** §11.6.5.2 Table 143 says a mask's dimensions are "independent of" its image's and
that both are mapped to the unit square "regardless of whether the samples coincide
individually" — the same sentence §8.9.6.3 writes for an explicit mask, so the same answer.
ADR 0023's grid choice became one function taking the mask's contribution as a closure, and
`smaskdim.pdf`'s bullets are round, `chrome-text-selection-markedContent.pdf`'s twelve masks
apply, and the corpus gate's `Image` row has **nothing left on it that is a feature**. ADR 0024.

**Then the rest of §11.6 was read — seventeen ledger rows — and it produced three defects the
demand item could not have reached.** The largest is that **a shading did not carry the alpha
constant**. §11.6.4.4 makes `ca` a property of the graphics state applied to painting, not of a
colour; a shading *replaces* the colour, so the natural implementation drops the alpha with the
colour it did not use. `alphatrans.pdf` announces `Gradient: .5` on its own face, and we painted
that gradient opaque over three objects all three references show through it. It had been
contradicted since the oracle existed, filed under `CONTRADICTED_SUBSTITUTED_FONT` because its
labels use a font nobody embedded — **the third time a group's name has turned out not to be a
diagnosis of its members**.

The other two are smaller and the same shape as each other: rules that look right until you
read the sentence.

- **A `/BM` array took the first *name*, not the first mode this reader knows.** §11.6.3:
  "shall use the first blend mode in the array that it recognizes (or Normal if it recognizes
  none of them)". `[/FooBar /Multiply]` was Normal and is Multiply. Invisible on every array
  anybody actually writes, which is §9.3.3's word spacing again.
- **§11.6.2's one-object rule is now reported.** "Portions of an object shall not be
  composited with one another … (such as a self-intersecting path, combined fill and stroke of
  a path…)" — and `B` becomes a `Fill` and a `Stroke` here, so the band they share composites
  twice. 4 documents, and the condition took instrumenting: both parts have to *mark the page*,
  and three of the six documents that fill and stroke under a `gs` set one of the two alphas to
  zero.

**Table 143 also decides two things a reader would otherwise trust.** An `/SMask` that is an
image mask carries no grey level at all — reading its first component as opacity makes the
parent image *fully transparent*, a page silently missing its picture — and an `/SMask` in some
other colour space has no clause saying which component is the opacity. Both are refused and
named. No corpus document trips either, which is the point: the failure they prevent is the
invisible kind.

**And `/Matte` is undone rather than merely reported.** Table 144's pre-blending
`c′ = m + α × (c - m)` is inverted for `DeviceGray` and `DeviceRGB` parents, where the
conversion into this crate's raster is the identity on components and the arithmetic is
therefore exact; any other space is reported, because §11.6.5.2 requires the inversion to
"precede the colour conversion" and by then it has not. `issue13931.pdf`'s red seal loses the
dark fringe its black matte left on every soft edge.

| | was | is |
|---|---|---|
| **an `/SMask` of another size** | reported, the image drawn opaque | applied, on the finer of the two grids |
| **a mask's contribution** | replaced the image's alpha | multiplies it, per §11.3.7.1's `α = f × q` |
| **`/Matte`** | read nowhere, nothing said | inverted in the two device spaces, reported elsewhere |
| **an `/SMask` that is a stencil** | would have blanked the image | refused and named |
| **a shading under `ca`** | painted opaque | carries the constant, ramp, grid and mesh alike |
| **`/BM [/Unknown /Multiply]`** | Normal | Multiply |
| **a filled *and* stroked path** | composited twice, silently | composited twice, and said so |
| **§11.3.7, §11.5, §11.6** | `unreviewed`, seventeen rows | reviewed, with three defects and two `silent` rows |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 723 | **724** |
| corpus documents reporting something | 232 | **231** |
| of those, reporting an *image* | 13 | **11** |
| pages we call complete, in the oracle | 1512 | **1513** |
| of those, agreeing with the reference consensus | 672 | **673** |
| of those, contradicted | 104 | **103** |
| ledger subclauses nobody has read | 668 | **652** |
| ledger rows that are `silent` | 2 | **4** |
| `§` citations the checker verified | 447 | **479** |
| tests | 361 | **367** |

Two of those movements are the reverse of what they look like. The `silent` count **rose**
because §11.6.6 and §11.3.7.3 were read and found to be drawn wrong with nothing said — they
are the transparency-group gap §11.4.6 already owns, recorded where a reader of *those* clauses
would look. And one page left the contradicted list without being fixed *as such*:
`alphatrans.pdf` was fixed and then left the comparison anyway, because the same session gave
it a §11.6.2 report.

What it taught:

- **A report costs gated pages, and this time the bill arrived on a page the same session had
  just fixed.** §11.6.2's report names `alphatrans.pdf`, which went from contradicted to
  agreeing an hour earlier and is now not judged at all. That is the correct trade — a page
  drawn under a model the standard does not describe should say so — but it is worth seeing
  once in this direction, because the tempting conclusion after fixing a page is that it is
  done.
- **Instrument a report before believing its count.** The first version of the §11.6.2 check
  named six documents and cost two agreeing pages. Printing the actual alphas showed that three
  of the six set one of the two to *zero*, which is one object painted once and cannot overlap
  itself. The clause's own words — "portions of an object" — say there must be two portions;
  the code had assumed the operator implied them.
- **The oldest wrong-looking page in the corpus was not about its fonts.** Four sessions of
  handovers had `alphatrans.pdf` under substituted fonts. Its actual defect was one `return`
  in `fill_paint` that dropped an alpha, and the side-by-side showed it in about four seconds:
  our gradient is a solid slab, theirs is glass. **Open the artefact before believing the
  label** is now three-for-three.
- **A bound written for one case can refuse a different case that is fine.** `MAX_MASK_GRID`
  refuses a combined grid above 2^24 samples, which is what stops a 2×2 image with a 34862×4332
  mask. Applied flatly it also refused a 12608×16806 mask on a 12608×16806 image, where
  combining costs exactly what the image already costs. The bound belongs on the *growth*, and
  the corpus said so within a minute of the first run.

## What the fourteenth session changed

**`/Mask` applies, in both the forms one dictionary key spells.** §8.9.6.3's explicit mask —
a second image saying which parts of this one are painted — and §8.9.6.4's colour key ranges
were the last two of the four masking mechanisms §8.9.6 names, and the only reason
`colorkeymask.pdf` had been this project's standing example of a silently wrong page. Both are
implemented, with tests at the pixel, and five corpus documents draw completely that did not.
ADR 0023.

Three of the decisions were the clause's rather than the code's:

- **The two rasters are combined on the finer grid.** "The base image and the image mask need
  not have the same resolution … their boundaries on the page will coincide", so the true
  composite happens at *output* resolution, which an image decoder does not know. Taking the
  finer of the two grids per axis discards nothing either carries; `issue4246.pdf` masks a
  50×40 gradient with a 1000×800 stencil spelling three words, and on the image's own grid
  those words are eight blocks. Bounded at 2^24 samples, because the grid is a product of two
  numbers a document controls.
- **A colour key is a test on samples, so it lives in the unpacker.** The ranges are compared
  against "colour values before decoding with the Decode array"; after conversion those values
  are gone, and for every space but the device ones they were never in the raster. Filtered
  images are refused and reported — which is the pair the clause's own NOTE 2 warns about.
- **A `/Mask` stream that is not an image mask is reported rather than interpreted.** Table 87
  and §8.9.6.3 both require one; `issue6621.pdf` writes a one-bit `DeviceGray` image instead.
  The lenient stencil reading was written, and the page rejected it — see below.

**Then reading clause 11's half of masking produced a rule neither §8.9.6 nor Table 87
states.** §11.6.4.3: an image's `/SMask`, or a non-zero `/SMaskInData`, "shall override any
explicit or colour key mask specified by the image dictionary's Mask entry". So an image
carrying both does not get both, and the `/Mask` is not a gap to report — it is a key the file
itself has superseded. No corpus document writes both, and nothing but reading the clause the
session's own work cited would have found it.

**And §9.3.8, text knockout, stopped being silent — but not for the reason the last handover
predicted.** That file said reporting it "costs one key lookup", meaning `/TK` in an
`/ExtGState`. It does not: `Tk`'s **initial value is true**, so the gap is the default and no
key needs to appear for a page to be drawn under the wrong model. What a report has to test is
whether the two models can differ at all — the paint has to composite (a constant alpha below
one, or a blend mode other than Normal; opaque Normal painting gives both models identical
pixels) and two glyphs of one text object have to overlap. Both are now checked, and `/TK`
itself is read, including the clause's rule that a value set between `BT` and `ET` "shall be
ignored". **2 corpus documents report.** The looser check — two glyphs under a compositing
paint, without the overlap test — reported 7 and took three *agreeing* pages out of the
oracle's gated set for a difference that could not have been on any of them.

| | was | is |
|---|---|---|
| **`/Mask` as a colour key** | reported, the masked samples painted | applied, on the raw samples, bounds inclusive |
| **`/Mask` as an image** | reported, the masked areas painted | applied, combined on the finer of the two grids |
| **a `/Mask` beside an `/SMask`** | would have applied both | superseded, per §11.6.4.3 |
| **a `/Mask` that is not an image mask** | reported as `/Mask` | reported as what it is, and drawn unmasked |
| **`/TK`** | read nowhere, nothing said | read, and the gap reported where it can show |
| **§8.9.6, §11.6.4** | `partial`/`reported`, and `unreviewed` | reviewed, with one precedence rule found |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 720 | **723** |
| corpus documents reporting something | 235 | **232** |
| of those, reporting an *image* | 18 | **13** |
| pages we call complete, in the oracle | 1508 | **1512** |
| of those, agreeing with the reference consensus | 672 | 672 |
| of those, contradicted | 103 | **104** |
| ledger subclauses nobody has read | 673 | **668** |
| ledger rows that are `silent` | 3 | **2** |
| `§` citations the checker verified | 382 | **447** |
| tests | 349 | **361** |

The one arrival on the contradicted list is `colorkeymask.pdf`, and it is not about masking:
its raster is 595 pixels wide where `poppler`'s and `mupdf`'s are 596, and on a page whose only
content is two coloured bands, three one-pixel edges are the whole difference. The heatmap
shows three vertical lines and nothing else. It joins `CONTRADICTED_PAGE_ROUNDING` as the third
page to arrive there by becoming *comparable* — the same thing happened to `bug1065245.pdf` and
`french_diacritics.pdf` when inline images landed.

What it taught:

- **The page rejected the lenient reading, and that is the result.** `issue6621.pdf`'s `/Mask`
  is a one-bit greyscale image with no `/ImageMask`. Treating it as a stencil is the only
  reading its samples admit, so it was implemented — and §8.9.6.2's "a sample value of 0 shall
  mark the page" then made the *background* of a court seal the painted part, blanking a page
  three renderers draw. The reading those three use is §11.6.5.2's, luminosity as opacity,
  which is a different clause about a different key and would invert every stencil whose author
  merely forgot `/ImageMask`. Where the standard defines nothing, refusing and saying so beats
  both readings.
- **A report is worth exactly its precision.** The first §9.3.8 check named 7 documents and
  cost three agreeing pages their place in the strongest gate this project has. The second
  names 2. Nothing about the *gap* changed between them — what changed is that the second one
  tests both of the clause's conditions instead of one. A report that fires where the output is
  provably identical is not caution; it is noise that removes pages from a comparison.
- **The estimate in this file was wrong in the interesting direction.** "One key lookup" assumed
  the gap arrives with the key. It arrives with the *default*. A parameter whose initial value
  is the unimplemented one is a gap on every page in the world, and the only reason that is
  bearable is that it is invisible almost everywhere — which is a fact about compositing, not
  about the key.
- **Two clauses in different parts of the standard describe one dictionary.** §8.9.6 says what
  `/Mask` means and §11.6.4.3 says when it loses. Reading either alone gives an implementation
  that is complete by its own lights. The session took the demand item from clause 8 and the
  family review from clause 11 for unrelated reasons, and the second corrected the first.

## What the thirteenth session changed

**All eight text rendering modes, and none of them needed a new display-list command.**
§9.3.6 Table 104's eight modes are three operations — fill, stroke, add to the clipping path
— and this tree did one of them properly. Modes 1 and 2 were drawn as a plain fill in the
*non-stroking* colour on 14 corpus documents, so a page that outlines its display type came
out solid and usually in the wrong colour. Modes 4 to 7 built no clip on 5 more, so
`text_clip_cff_cid.pdf` drew a solid blue bar where four renderers show "ABC123". Both were
*reported*, which is the only reason either was schedulable. `Command::Stroke` and `Clip` have
carried everything needed since the first display list; what was missing was the middle, which
is ADR 0018's `d` operator again. ADR 0022.

Three of the clause's own rules decided more than the modes did:

- **A stroke's width is in user space** — "shall be interpreted in user space rather than in
  text space" — and a `Command::Stroke`'s width is in its *path's* space. Scaling the width by
  the inverse of the glyph transform is only right when that transform is uniform, so the
  glyph outline is moved into user space instead and the command carries the state's stroke
  parameters unchanged. Exact for any text matrix, and the copy per stroked glyph is paid only
  by the four modes that stroke.
- **An empty accumulator sets no clip.** "If no glyphs are shown or if the only glyphs shown
  have no outlines … no clipping shall occur." The natural implementation clips to whatever
  accumulated, and an empty clip hides everything drawn after the text object — so a mode-7
  text object showing one space, which is a blank line of OCR text, would blank the page.
- **A hidden optional-content layer still accumulates the clip.** §8.11.3.1 lists clipping
  among the graphics state operations that "shall still be applied", and the clip outlives its
  `ET`. `end_path` had already read that clause the same way for a path's `W`; the two now
  agree instead of one clause being read twice, differently.

**Then the family review found what the mode did not.** §9.3 and §9.4 were read entire —
thirteen ledger rows — and produced a defect nothing could have seen from a page: **§9.3.3's
word spacing is a rule about a code's encoded length, and we had implemented it as a rule about
its value.** Word spacing applies to "the single-byte character code 32" and "shall not apply
to occurrences of the byte value 32 in multiple-byte codes"; we applied it to any code equal to
32, so an `Identity-H` string containing the bytes `00 20` was pushed right by `Tw` for every
one of them. No page of Latin text can show this, because a composite font's space is usually
some other CID entirely. It also produced **§9.3.8, text knockout**, as the ledger's third
`silent` row: `/TK` arrives only in an `/ExtGState` and nothing looks for the key.

**And the checker learned to read table numbers, after one of them turned out to be wrong.**
Four comments, two tests and a written report said "§9.3.6 Table 106" for the text rendering
modes. They are Table 104; Table 106 is the text-*positioning* operators. Every check passed,
because the clause exists and the table exists and only the pair is wrong. The strong gate —
the clause beside a reference must be one the standard discusses that table in — was built and
**rejected by its own output**: it fails fourteen of the tree's twenty-five references and all
fourteen are correct writing. So the assertion is the weaker true one and the gate *prints the
title of every distinct table the tree cites*, which is thirty-one lines in which the wrong
pairing is visible at a glance. `PLAN.md` §5a has the argument.

| | was | is |
|---|---|---|
| **`Tr` modes 1 and 2** | filled in the non-stroking colour, reported | stroked in the stroking colour, in user space |
| **`Tr` modes 4 to 7** | no clip built, reported | one non-zero-filled clip at `ET`, lasting until `Q` |
| **an undefined `Tr` operand** | stored, and would now have drawn nothing at all | reported, and the mode left as it was |
| **word spacing on a two-byte code 32** | applied | refused, per §9.3.3's second sentence |
| **`Table N` in a comment** | unchecked, and one was wrong | checked against the standard, and every title printed |
| **§9.3 and §9.4** | `unreviewed`, thirteen rows | reviewed, with one defect and one `silent` row |

**The numbers:**

| | before | now |
|---|---|---|
| corpus documents drawing with nothing reported | 704 | **720** |
| corpus documents reporting something | 251 | **235** |
| of those, reporting an *operator* | 33 | **15** |
| pages we call complete, in the oracle | 1492 | **1508** |
| of those, agreeing with the reference consensus | 661 | **672** |
| of those, contradicted | 103 | 103 |
| ledger subclauses nobody has read | 686 | **673** |
| cited clauses still owing a review | 25 | **23** |
| `§` citations the checker verified | 340 | **382** |
| `Table N` references the checker verified | — | **31 distinct** |
| tests | 324 | **349** |

**Sixteen documents became complete and not one of them is contradicted** — eleven agree with
the reference consensus, three are ambiguous, two are not comparable. That is unusual: every
feature since the ninth session has put at least one newly-drawable page on the contradicted
list. What it means is that these pages were not *nearly* right before; they were a solid bar
where a word belongs, and there was nothing else on them to be wrong about.

What it taught:

- **A clause's exceptions are where the failures are, and they fail in the dangerous
  direction.** Three of §9.3.6's sentences are exceptions to its own table, and the one that
  matters most — no outlines, no clipping — is the one an implementation drops silently and
  the one whose omission blanks a page. The rules are cheap; finding out they exist costs a
  careful read.
- **A rule about an encoding, implemented as a rule about a value, looks correct forever.**
  §9.3.3 has been wrong since composite fonts landed and no corpus page could show it. The
  clause states the distinction in its own second sentence. This is the same shape as
  §9.6.5.4: the code was self-consistent and right about every document anybody had opened.
- **Build the strong gate, then let it tell you it is wrong.** The table-attribution check was
  written, run, and thrown away on the evidence of its own fourteen false positives — all of
  which were correct writing. Keeping it behind an exception list would have made a checker
  that is mostly exceptions, which teaches a reader to ignore it. What replaced it asserts
  less and *prints* more, and the printing is what would have caught the original error.
- **The tree was not `clippy` clean, and this file said it was.** Eleven warnings, all from
  the twelfth session's own new files — `allow-panic-in-tests` does not reach an integration
  test's helper functions, which is not obvious and is worth knowing. Fixed. A claim in this
  file that nothing recomputes is a claim that drifts, and "clippy clean" is one somebody has
  to actually run.

## What the twelfth session changed

**The loop got three times shorter, and that was the session's first job.** The oracle gate
took 75 seconds, of which 1021 seconds of processor time were spent in `pdftoppm`, `mutool` and
`gs` against 46 in our own pipeline. Nothing about their answers changes between runs, so
`pdfref::cache` remembers them: **75 s → 25 s, with every count identical**. That claim is
checked rather than asserted — `PDFREF_CACHE=off` runs the same gate the old way, and on the
finished tree it takes 98 s and produces the same eight counts. (98 rather than the 75 measured
at the start of the session, on a busier machine and with 14 more pages in the comparison; the
number that matters here is that the two columns of verdicts match, not the two clocks.) The danger was
named in this file before it was built — "a cache key omitting one variable would compare
against stale renders in silence" — and the answer is that the key is *derived from the
invocation itself*, `Reference::build_command`'s own argument list with the two paths that vary
per page replaced, plus the renderer's version and the document's SHA-256. A flag that is not
in the key is a flag that is not passed to the renderer either. ADR 0020.

**`CCITTFaxDecode` decodes** (§7.4.6), the last image codec absent and the largest named image
gap at 12 documents. Group 3 and Group 4 through `hayro-ccitt` in the sandboxed worker, with
Table 11's eight parameters resolved on this side of the pipe so the worker holds no opinion
about PDF. Two entries are refused rather than approximated — `/DamagedRowsBeforeError`, which
is error concealment the decoder has none of, and a `/Columns` that disagrees with the image's
`/Width` — and one silence of the standard's is recorded as a choice: a stream whose data runs
out before `/Rows` is legal, and the rows never delivered are left blank. ADR 0021.

**And then `/Rotate` had been turning pages the wrong way since the first page tree.** Drawing
`issue5747.pdf`'s fax-encoded scan for the first time put it on the screen **upside down** beside
four renderers that agree with each other. §7.7.3.3 Table 31 says *clockwise*; page space here
is y-up and the flip to a raster happens later, so a clockwise turn is a negative rotation in
this space — and the 90 and 270 matrices were exchanged, which is a 180° error. Six pages were
contradicted by it, five of them filed under substituted fonts because they also carry one.

| | was | is |
|---|---|---|
| **`/Rotate 90` and `270`** | exchanged, so every rotated page was drawn 180° out | §7.7.3.3's clockwise turn, pinned by where a corner lands rather than by a matrix |
| **the oracle's reference renders** | recomputed every run, 1021 s of other programs' processor time | remembered, keyed on the invocation itself; 99.7% hits |
| **a reference timeout** | 30 s of waiting per page per run, twice over | remembered for a week, counted and printed separately |
| **`CCITTFaxDecode`** | reported on 12 documents | decoded, in the sandboxed worker |
| **§7.4.6 and §8.6.4.2** | `unreviewed` | `partial` and `implemented`, with what was refused named |

**The numbers:**

| | before | now |
|---|---|---|
| oracle gate, wall clock | 74.9 s | **25 s** |
| corpus documents drawing with nothing reported | 692 | **704** |
| corpus documents reporting something | 263 | 251 |
| pages we call complete, in the oracle | 1478 | **1492** |
| of those, agreeing with the reference consensus | 652 | **661** |
| of those, **contradicted** | 108 | **103** |
| ledger subclauses nobody has read | 688 | **686** |
| cited clauses still owing a review | 25 | 25 |
| `§` citations the checker verified | 317 | 340 |
| tests | 314 | **324** |

Six contradicted pages left by being fixed and one arrived: `bug1001080.pdf`, which is a page
of Type 3 glyphs each drawn as an inline CCITT image mask, and which we now render as
`pinL LesL` where four renderers render `pint test`. That is not a fax defect — it is
`tiny-skia` sampling four neighbours of a 39×53 bitmap drawn at five pixels — and it is the
same defect `firefox_logo.pdf` has carried for four sessions at 0.02 outside the bound. The
item was sized "Small"; it is unreadable text.

What it taught:

- **The instrument was the bottleneck, and nobody had measured it.** Eleven sessions treated
  85 seconds as the price of the oracle. It was 95% three other programs answering a question
  they had answered the day before. When a loop feels slow, measure what is in it before
  deciding what to build less of.
- **Measurement rejected the first design of the cache, and that is the interesting part.**
  Refusing to remember timeouts is obviously the safe rule; with everything else cached it
  left **two pages out of 1794 accounting for 46 of the run's 57 seconds**. The rule that
  replaced it is written down with its cost and its expiry, which is what principle 1 asks
  of a shortcut.
- **A hypothesis about a group of contradicted pages is not a diagnosis of its members.**
  `CONTRADICTED_SUBSTITUTED_FONT` held 25 pages; six of them were one line in `content.rs`
  and had nothing to do with fonts. They were only ever grouped there because they *also*
  name a font nobody embedded. The picture said so in ten seconds; six sessions of counting
  had not.
- **An upside-down page has the right ink in the right quantity.** No metric this project
  owns — not `unsupported`, not mean error, not the ink coverage a fax test could check —
  can see a 180° rotation. Only pixels somebody else produced can, which is trap 1 again in
  a form that has nothing to do with fonts.
- **A test that skips silently is worse than no test.** The first draft of `tests/ccitt.rs`
  named `bug1001080.pdf` as its fixture, whose fax data turned out to be inside a Type 3
  font's glyph descriptions rather than in an image `XObject`; both tests "passed" by doing
  nothing. A missing corpus is a skip; a present corpus that lacks what the test needs is a
  panic.

## How the project got here

Each session's argument is in its ADR; this file keeps only what is still load-bearing.

| Session | What landed | Where the reasoning is |
|---|---|---|
| 5 | The reference oracle, over every page of the corpus | ADR 0011 |
| 6 | `CalGray`/`CalRGB` through XYZ; annotation appearance streams | ADRs 0012, 0013 |
| 7 | JBIG2 and JPEG 2000, in a sandboxed worker; the first speed comparison | ADR 0014 |
| 8 | §9.6.5.4, the `TrueType` code-to-glyph algorithm, in full | ADR 0015 |
| 9 | The conformance ledger and citation checker; optional content | ADRs 0016, 0017 |
| 10 | Type 3 fonts; dashed lines, which had never been dashed | ADR 0018 |
| 11 | Inline images; `/Interpolate`; `Indexed`, `Separation` and `DeviceN` images | ADR 0019 |
| 12 | A cache for the oracle's reference renders; `CCITTFaxDecode`; `/Rotate` | ADRs 0020, 0021 |
| 13 | All eight text rendering modes; §9.3 and §9.4 reviewed; table numbers checked | ADR 0022 |
| 14 | `/Mask` in both its forms; §11.6.4 reviewed; §9.3.8 reports | ADR 0023 |
| 15 | Soft masks at any resolution and `/Matte`; §11.3.7, §11.5 and §11.6 reviewed; a shading carries `ca` | ADR 0024 |
| 16 | Area averaging for reduced images; §10.7 reviewed, and it forbids what was built | ADR 0025 |
| 17 | Transparency groups; §8.10 and §11.4 reviewed; the page group is isolated | ADR 0026 |
| 18 | Soft masks in an `/ExtGState`; §11.7 reviewed, and overprinting is silent | ADR 0027 |
| 19 | `/SA` and the device's thinnest line; §8.6.6 and §8.6.7 reviewed, and overprinting is *not* a gap | ADR 0028 |
| 20 | Embedded `CMap`s and `/CIDToGIDMap`; the whole of §9.7 reviewed, demand item and spec item at once | ADR 0029 |

The contradicted count has gone 174 → 120 → 108 → 106 → 104 → 108 → 103 → 103 → 104 → 103 →
100 → 93 → 96 → 96 → 98 across sessions 6 to 20, and the corpus's incomplete count 291 → 368 → 250 → 290 → 283 →
263 → 251 → 235 → 232 → 231 → 231 → 237 → 220 → 220 → 189 —
both move in both directions on purpose: a rise in the first can mean pages *joined* the comparison and a
rise in the second is honesty when a silence ends, and the sections below say which.

## Where we are

A PDF **renderer** that opens real files and draws pages: geometry, colour, images,
shadings, patterns, embedded text and annotation appearances, on both a CPU and a GPU
backend, with JBIG2 and JPEG 2000 images decoded in a confined worker process, and with
transparency groups and soft masks composited as clause 11 defines them. It is not yet a PDF
*viewer* in the full sense — no forms and no encryption — and the gap between those two words
is measured further down rather than guessed at.

- **440 tests**, `clippy` clean under `pedantic` + `unwrap_used`/`panic`/`arithmetic_side_effects`,
  `cargo fmt --check` clean, `cargo deny` clean on all four checks (verified, not assumed — and
  the thirteenth session found this line had been *wrong*: eleven warnings had accumulated in
  the twelfth session's own new files, because `allow-panic-in-tests` does not reach an
  integration test's helper functions).
- **The 14 specification PDFs in `doc/`** — including ISO 32000-2 itself, 1023 pages and
  101 318 objects — all parse, all render page one with **nothing reported at all** since
  §11.5's soft masks landed, and all extract **100% of the words `pdftotext` finds**.
- **The 974-document pdf.js corpus is a gate, not a survey.** All 974 open, 955 reach page
  one, **766 draw with nothing reported at all**, and everything the other 189 cannot draw
  is named. The counts are ratcheted. 1501 of 1501 PDF functions parse; **all 1793 shadings
  build**, mesh types included. The whole gate runs in **1.6 s** and has **no named slow
  document left**.
- **A second gate asks whether what we drew is *right*.** `oracle.rs` compares us against
  poppler, mupdf and ghostscript over **1794 pages** — every page of the corpus, plus page
  one of each specification PDF — **in 33 s**, because the references' renders are remembered
  between runs rather than recomputed (ADR 0020). Of the 1557 pages we claim to draw
  completely, **706 agree with the reference consensus, 98 are contradicted by it and 740 are
  pages the references cannot agree on among themselves**. The 98 are named, grouped and
  ratcheted in both directions. ADR 0011.
- **JBIG2 and JPEG 2000 decode in a sandboxed worker.** `pdf-sandbox` confines it with
  resource limits, Landlock and a seccomp-BPF allow-list; `--no-sandbox` turns it off for
  trusted documents and says what that costs. The strongest evidence the decode is right is
  not a reference renderer: the corpus encodes **one image ninety-six ways** and all ninety-six
  produce byte-identical pixels. ADR 0014.
- **Colour resolves from the document.** `ICCBased` profiles are evaluated by an A2B
  evaluator written here, `CalGray`/`CalRGB`/`Lab` are converted through XYZ, `/DefaultCMYK`
  and output intents are honoured, and there is exactly one route from XYZ to a pixel and
  exactly one `DeviceCMYK` conversion. ADRs 0009 and 0012.
- **Annotations draw.** `/AP /N` is placed by §12.5.5's algorithm and run by the same
  machinery as any other form XObject; nothing is synthesised, and an annotation with no
  appearance is reported. ADR 0013.
- **A glyph may be a content stream.** Type 3 fonts (§9.6.4) are read in `pdf-model`, since
  drawing one means running the interpreter — `/FontMatrix` for glyph space, `/Widths` read
  through it, an encoding that is the whole mapping, and `d0`/`d1` including the rule that an
  uncoloured figure takes its colour from outside. ADR 0018.
- **A composite font is a `CMap` and a `CIDFont`, and both are read.** §9.7 in full except for
  data: an embedded `CMap` stream (§9.7.5.3) decides how many bytes each code takes and which CID
  it selects, byte by byte against its codespace ranges, with §9.7.6.3's recovery for a code that
  matches none; and a CID reaches a glyph through a CID-keyed CFF's charset or a `/CIDToGIDMap`
  stream (§9.7.4.2). 31 corpus documents that drew no text now draw it. What is left is Table
  116's predefined `CMap`s — registered data files, so a licensing question — and vertical
  writing, which is §9.2.4's missing `/W2` metrics. The parser is fuzzed, on the property that
  matters most: §9.7.6.2 extracts "one or more bytes" per code, and a `CMap` that led it to
  consume zero would hang a page rather than draw it wrong. ADR 0029.
- **An image may be written into the content stream.** Inline images (§8.9.7) are scanned into
  the dictionary an image `XObject` would have had and decoded by the same route, so nothing
  downstream knows one from the other. Where the data ends is answered by `/L`, by §8.9.3's
  sample layout, or — for filtered data in a file with no `/L` — by a search. ADR 0019.
- **Every image codec a PDF may name now decodes.** `CCITTFaxDecode` was the last one
  absent (§7.4.6, ADR 0021): Group 3 and Group 4 through `hayro-ccitt` in the same sandboxed
  worker, with Table 11's parameters resolved before they cross the pipe. `LZWDecode` is the
  only standard filter of any kind still missing, and no corpus first page reaches it.
- **An image is masked every way §8.9.6 and §11.6.5.2 define.** Its own `/ImageMask` stencil,
  an explicit `/Mask` naming a second image, a colour-key `/Mask` naming ranges of sample
  values (ADR 0023), and an `/SMask` supplying per-sample opacity — the last of which no longer
  has to be the image's own size (ADR 0024). Two rasters of different sizes are combined on the
  finer grid, which is a documented choice rather than a derivation: the clause puts both on
  the unit square and leaves the sampling to the device. §11.6.4.3's precedence is honoured, so
  an `/SMask` beside a `/Mask` supersedes it, and Table 144's `/Matte` pre-blending is undone
  where the arithmetic is exact.
- **A rotated page turns the way the standard says.** §7.7.3.3 Table 31's `/Rotate` is a
  *clockwise* turn as displayed, which in this y-up space is a negative rotation; 90 and 270
  had been exchanged since the first page tree, so every rotated page in the corpus was drawn
  180° out. Six contradicted pages were this one line.
- **An image is filtered only where the document allows it, and a reduced one is averaged.**
  §8.9.5.3's `/Interpolate` decides whether a *magnified* image is smoothed, and both backends
  ask `Image::is_smoothed` so they cannot disagree. A reduced image is averaged over the
  samples that share a device pixel, by `Image::area_averaged` — which is a **documented
  departure from §10.7.4**, not a reading of it: that clause requires point sampling and says
  "there shall not be averaging over the pixel area". §10.7.1 licenses the departure, this tree
  already takes two others in the same subclause by anti-aliasing at all, and the page that
  argues for it is a Type 3 font of eleven-times-reduced fax glyphs that is otherwise
  illegible. ADR 0025.
- **A `/Group` is composited as one object.** §11.6.6's transparency group XObject: the
  elements are drawn onto a transparent backdrop and the result painted once, under the
  constant alpha and blend mode in force at the `Do` — which §11.6.6 resets *inside* the group
  so they are not applied twice. That is §11.4.5's isolated, non-knockout group, which both
  backends have natively; the three answers that ask for something else — non-isolated with a
  blend mode inside it, knockout, and a blending colour space that is not the device's — are
  reported on the condition each clause states. ADR 0026.
- **A soft mask is a group evaluated for its opacity.** §11.5's mask in an `/ExtGState`: the
  group named by `/G` is run into a command list of its own, positioned by `/Matrix` and the
  transform in force at the `gs` (§11.6.5.1), and each backend rasterises it at the target it
  is drawing to — the CPU into a `tiny_skia::Mask` multiplied into the clip, the GPU into a
  texture composited back with `Compose::DestIn`. `SoftMask::value` is the one place rendered
  pixels become mask values, so the two backends cannot differ about `/S`, `/BC` or `/TR`. An
  image carrying its own mask ignores the graphics state's, per §11.6.4.3. ADR 0027.
- **A page is a group too, and an isolated one.** §11.4.7: the page's own initial backdrop is
  transparent and the medium's white is composited with the *result*, which is what
  `impose_on_medium` does at both backends' boundary. Painting onto white instead is a
  different picture for every blend mode, and was this tree's until the seventeenth session.
- **A form XObject is clipped by its `/BBox`.** §8.10.1 step c), required of every form and
  not only of an annotation's appearance. One contradicted page was this, and one page of
  `tracemonkey.pdf` had a figure covered by another form's white background.
- **A layer the document turns off is not drawn.** §8.11 in full as far as it decides what is
  marked: the default configuration, membership dictionaries including `/VE` visibility
  expressions, intent, and `/OC` on marked-content spans, XObjects and annotations. ADR 0017.
- **One device pixel is the thinnest line, and both backends agree what that means.**
  `Stroke::device_width` is §8.4.3.2's zero-width minimum and §10.7.5's stroke adjustment in one
  function, because the clause's own NOTE makes them the same width. Before the nineteenth
  session the rule was `tiny-skia`'s hairline convention rather than a reading, so the GPU
  backend — where `kurbo` expands a zero-width stroke into an empty outline — drew nothing at
  all for a `0 w` line. `/SA` is read from Table 58; the clause's other half, grid-fitting a
  stroke's coordinates, is a documented departure of the same family as §10.7.4's three. ADR 0028.
- **Overprinting is ignored, and §8.6.7 is what says to ignore it.** `/OP`, `/op` and `/OPM` are
  named in `content.rs` as deliberately unread: overprinting decides what happens to the device
  colourants an operation does not name, this device has three additive ones and no separations,
  and both §8.6.7 and §11.7.4's Table 146 reach the same answer — the special overprinting blend
  function is the source colour for every row a three-process-component group space can reach,
  which is Normal. The one configuration that would differ is a `DeviceCMYK` group space, which
  §11.6.6 already reports. ADR 0028.
- **A colourant may be `/None` or `/All`.** §8.6.6.4's two special names, which "shall" be
  honoured "on all devices" with the alternate space and tint transform ignored: `/None` marks
  nothing, and `/All` is the tint complemented in every colourant, because an additive device
  complements a subtractive tint. Both are decided before the transform is parsed, so an
  unreadable one cannot take the colourant with it.
- **The citations are checked.** `tools/conformance` holds every `§` in the tree to a clause
  the standard has — 827 of them — every rustdoc blockquote to the standard's own words, and
  the conformance ledger's 823 rows to the standard's subclauses. It also prints the title of
  every table the tree cites, which is how the twentieth session found six comments calling
  Table 57's graphics state entries "Table 58". ADR 0016, `doc/PLAN.md` §5a.
- Both backends draw everything the display list can express, and agree on it: **eleven**
  headless GPU scenes hold `tiny-skia` and Vello to the same pixels, at more than one scale
  and along both axes — see trap 2 for why that matters. A twelfth test is not a scene but a
  single pixel: `vello_hands_back_straight_alpha`, which is what fifteen sessions of
  opaque-background comparisons could not see. The newest scene is the one that would have
  caught the zero-width stroke: `cpu_and_gpu_agree_on_the_thinnest_line_the_device_draws`.

### Run it

```sh
cargo run --release -p viewer-ui --bin pdf-viewer -- doc/PDF20_AN001-BPC.pdf
```

Arrow keys / Page Up / Down / Space turn pages, Home and End jump to the ends, Escape
quits. The title bar names anything on the page that could not be drawn.

`--no-sandbox` decodes JBIG2 and JPEG 2000 in the viewer's own process instead of in a
confined worker: faster by a process spawn and a pipe round trip, and appropriate for
documents whose origin you trust. It prints a line saying what it gave up.

### Verify it

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets     # must be silent of lints
cargo test --workspace
# The conformance gate is part of that run and needs nothing but the tree. Its summary is
# worth reading rather than only passing:
cargo test -p conformance -- --nocapture   # 827 citations, 63 quotations, 42 tables, 823 rows
cargo run -p conformance --bin ledger      # regenerates the rows, keeps every status
# Both gates decode images in a separate program, and -p pdf-model does not rebuild
# another package's binaries. Build it first or the numbers below are somebody else's.
cargo build --release -p pdf-sandbox --bins
cargo test --release -p pdf-model --test corpus -- --ignored --nocapture   # 974 docs, ~2 s
cargo test --release -p pdf-model --test oracle -- --ignored --nocapture   # 1794 pages vs 3 voting renderers, ~33 s
# The first run of that on a fresh build directory is ~95 s and writes 319 MB of remembered
# reference renders; every run after it is the 34 s above — it was 25 s when the cache
# landed and the fifteenth session measured 33-34 s on a busier machine at a 99.7% hit rate,
# so read the hit rate rather than the clock. Two environment variables matter:
#   PDFREF_CACHE=off              ask the three renderers again, which is how "the cache
#                                 changes no verdict" is re-checked over the whole corpus
#   PDFVIEWER_ORACLE_ONLY=a,b     compare only pages whose names contain a or b — 0.2 s for a
#                                 handful of documents. A filtered run refuses to check the
#                                 ratchets and says so.
# The oracle draws a fourth panel with hayro on every page worth looking at, and the same
# binary answers "are we fast?". Neither is needed for the gate to pass.
cargo build --release -p hayro-compare --bins
cargo run --release -p hayro-compare --bin hayro-speed -- doc/pdf.js/test/pdfs/*.pdf
cargo bench -p pdf-model                   # interpretation, the time-to-first-page path
# Two callgrind examples, and they measure different halves. The first stops at the display
# list, so a backend change measures as exactly zero there; the second rasterises.
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_interpret
valgrind --tool=callgrind --callgrind-out-file=/dev/null \
  target/release/examples/callgrind_rasterise [file.pdf] [page]
cargo deny check
cargo +nightly fuzz run lexer -- -runs=50000     # from fuzz/, needs nightly
cargo +nightly fuzz run cmap  -- -runs=50000     # §9.7's CMap parser and its decoder
```

Cargo prints one line about `proc-macro-error2` being rejected by a future compiler. It is
not a lint and not ours: it arrives through `iai-callgrind`, a dev-dependency that reaches no
shipped binary, and `deny.toml` records the same exception with the reasoning. Nothing to
chase.

## Crate map

| Crate | Does | Notes |
|---|---|---|
| `pdf-spec` | Object-model validation tables | Generated from Arlington by `build.rs` |
| `pdf-syntax` | Lexer, objects, xref, filters, `Document` | Touches untrusted bytes first |
| `pdf-model` | Page tree, content interpreter, annotations, optional content, Type 3 fonts, image decode | Where PDF semantics live. `soft_mask.rs` reads Table 142 and nothing else: what a `/SMask` *means*, with its group left for `content.rs` to run, because running one needs the interpreter.  `optional_content.rs` answers "is this layer on"; the interpreter asks it in three places (§8.11.3.2 and §8.11.3.3). `type3.rs` reads a font whose glyphs are content streams, because running one needs the interpreter (§9.6.4, ADR 0018). `inline_image.rs` turns a `BI` … `EI` sequence into the stream an image `XObject` would have been, so `image.rs` stays the only decoder (§8.9.7, ADR 0019). `image.rs` also owns §8.9.6's and §11.6.5.2's masking: `mask_entry` and `soft_mask_entry` each read one key once and decide what it means, so a report cannot outlive its gap, and `combine_on_the_finer_grid` is the one place two rasters of different sizes are combined rather than refused (ADRs 0023, 0024) |
| `pdf-font` | Glyph outlines via `skrifa` | Owns both encoding algorithms: §9.6.5.2 for CFF, §9.6.5.4 for `TrueType` (ADR 0015). `cff.rs` adapts `read-fonts`; `encoding.rs` is Annex D and Table 113 data; `substitute.rs` is the only machine-dependent code in the tree. `cmap.rs` is §9.7's composite encoding: `CMap` answers both of §9.7.6.2's questions — how many bytes the next code takes and which CID it selects — and `Code` carries a value *and* a length, because the clause looks a code up "in the character code mappings for codes of that length" and §9.3.3's word spacing is stated the same way (ADR 0029). It is deliberately not `tounicode.rs`: both are `CMap` files read with the same lexer, and a `bfchar` destination is UTF-16BE text there and a character selector here. A Type 3 font is refused here — its glyphs are content streams, so it belongs in `pdf-model` |
| `pdf-render` | Display list + `Rasterizer` trait | No PDF semantics, no rasteriser. `soft_mask.rs` is where rendered pixels become §11.5's mask values — one function both backends call, because §11.5.3's coefficients are not the luminance either rasteriser offers (ADR 0027). `Command::Group` is the one nested command: a transparency group's elements, drawn onto transparency and painted once (§11.4.1, ADR 0026), and `impose_on_medium` is §11.4.7's composite of the finished page onto its medium — in `pdf-render` rather than in a backend for the same reason the resampling decisions are. `Path::extend_transformed` is the one place geometry is moved rather than travelling with a transform, and both callers are §9.3.6's text (ADR 0022). `Shading::with_alpha` is how §11.6.4.4's constant reaches a paint that has no single colour to carry it (ADR 0024). `Image::is_smoothed` and `Image::area_averaged` are two of the three device decisions, here rather than in a backend so the CPU oracle and the GPU backend cannot make them differently — the second is a documented departure from §10.7.4 (ADR 0025). The third is `Stroke::device_width`, which is §8.4.3.2's one-device-pixel minimum and §10.7.5's stroke adjustment in one function because the clause makes them the same width; it is here because it was `tiny-skia`'s hairline convention instead, and Vello has no such convention, so a `0 w` line was invisible on the GPU for fifteen sessions (ADR 0028). `Transform::max_stretch` is the length scale those decisions need, and is not `determinant().abs().sqrt()`: a shear separates the two singular values without changing the determinant |
| `render-cpu` | `tiny-skia` backend | Correctness oracle **and** startup path |
| `render-gpu` | Vello/wgpu backend | Headless by construction. `soft_mask.rs` renders each mask's group into a texture of its own and reads it back, because Vello's own luminance mask is the SVG formula and no blend mode is a `/TR` |
| `raster-compare` | Tolerant image metrics | Worst-tile error is the load-bearing one |
| `test-scenes` | Shared fixtures | Holds the same page as a display list *and* as PDF bytes |
| `tools/pdfref` | Reference-comparison harness | Triangulation rule lives here. `cache.rs` remembers what each reference renderer produced, keyed on the invocation itself so a changed flag cannot be answered from a render made under the old one (ADR 0020); `digest.rs` is the SHA-256 that key is built from, and is the one file in the tree citing a standard other than ISO 32000-2 |
| `viewer-ui` | The application | `src/bin/pdf-viewer.rs` |
| `pdf-sandbox` | Confined worker + the three image filters | Its `decode.rs` is the only place a JBIG2, JPX or CCITT codestream is looked at. `CCITTFaxDecode` joined in the twelfth session (§7.4.6, ADR 0021), through the same protocol with Table 11's parameters resolved before they cross the pipe |
| `tools/hayro-compare` | Drives `hayro` for the oracle's fourth panel and for speed | Nothing ships it; it is where `hayro`'s forty dependencies live |
| `tools/conformance` | Citation checker and the conformance ledger | Depends on nothing but `thiserror`. The one crate the citation scan skips — its own comments cite clauses that do not exist, deliberately. Since the thirteenth session it also reads `Table N` references and prints every cited table's title, which is the check a wrong table number needs |
| `viewer-core` | Empty | Documented responsibility only |

Architecture decisions are in `doc/adr/`. `doc/PLAN.md` tracks phases and measured results.

## Traps — read these before writing code

### 1. The metrics lie about fonts. Look at the page.

This is the most important thing in this file.

Wiring bare-CFF support in made every affected document report `unsupported: []` — and
render **almost no text**. The font loaded, nothing was reported, the wrong glyphs were
drawn. It was caught only by rendering a page and looking at it.

`Interpretation::is_complete()` tells you what the interpreter *knows* it skipped. It
cannot tell you that a font loaded and produced garbage. For any font or colour work,
render the corpus pages and **look at them**. There is a test that writes the PNGs:
`cargo test -p pdf-model --test render_real_pdf -- --nocapture writes_inspectable`.
It covers both CFF routes, because no metric distinguishes them.

**Both halves of that were live again in the eighth session, and the second half is the one
to carry forward.** `issue5501.pdf` drew `v 0' ' W` where poppler reads
`What's an interval?`: its font's `cmap` was not being read the way §9.6.5.4 says to read it,
and the fall-through drew glyph number `code`. `unsupported: []`, plausible-looking output,
wrong text. And `issue918.pdf` drew 388 text operations of letter fragments because a **Type
3** font — which has no font program at all — was being given a Latin substitute, so the
procedure names in its `/Differences` array resolved to whatever they happened to match.

Both are fixed, and both were found by the oracle. What is worth keeping is the shape: in
each case the font *loaded*, so every metric this crate owns said the page was fine. A
loading font is not a working font, and the only thing that can tell them apart is pixels
someone else produced.

Two automated checks *do* catch a wrong mapping, both in `crates/pdf-font/src/lib.rs`:

- `the_pdf_widths_agree_with_the_font_programs_own_advances` — the document's `/Widths`
  and the CFF charstring's own advance are independent statements of the same fact, so
  they agree only if the code reached the glyph the producer meant. This is the strongest
  check in the tree: it verifies the mapping without consulting the mapping.
- `an_uncovered_code_has_no_glyph_rather_than_a_guessed_one` — pins the absence of the
  code-as-glyph-index fall-through.

Both were confirmed to fail when the defects they describe are deliberately reintroduced.
They are complementary: an off-by-one charset trips only the first, a reinstated
fall-through only the second. Neither replaces looking at the page, and neither caught
`issue20504.pdf`.

The third check is the first that looks at *pixels*: the corpus oracle compares our page
against three renderers that share no code with us. It cannot be fooled by a font that loads
and draws the wrong glyphs. What it cannot do is tell you *which* of a page's differences
matters, so it still does not replace looking.

**And the rule is not really about fonts.** The tenth session put Type 3 fonts in, opened ten
pages beside `poppler`'s, and the difference on the first one was that our squares were solid
where three renderers drew them dashed — a defect in the `d` operator, which has nothing to do
with fonts, text, or the feature being written. It was on the screen in the first minute.
Whatever you have just built, render a page that uses it and look at the whole page.

**The eleventh session is the same lesson with a bigger yield.** Inline images landed, five
pages appeared in the oracle's newly-contradicted list, and opening them one at a time gave
`/Interpolate` (a four-sample image drawn as a blur where three renderers draw four squares), a
`Lab` colour table scaled as though its components ran 0 to 1 (a black bar where four
renderers draw pale grey), and a soft mask silently dropped for being a different size (black
bars across a page of text). None of the three is an inline-image defect. **Every page a new
feature makes drawable is a page nobody has ever looked at.**

**The twelfth session found the largest one yet, and it was not in the newly-contradicted
list at all.** `CCITTFaxDecode` landed; one of the twelve documents it made drawable was a
scan on a `/Rotate 270` page, and opening the side-by-side showed our panel **upside down**
beside four that agree. 90 and 270 had been exchanged since the first page tree. Two things
about that are worth carrying:

- **No metric this project owns can see a rotation.** An upside-down page reports nothing, has
  the right ink in the right quantity, the right page size, and a mean error that looks like a
  font difference. Only pixels somebody else produced can tell you, and only if you look.
- **It was hiding inside a group of contradicted pages that named the wrong cause.** Five of
  the six affected pages sat in `CONTRADICTED_SUBSTITUTED_FONT` for six sessions, because they
  also happen to use a font nobody embedded. Open the artefact before believing the label.

**The thirteenth session is the rule's happier form and still worth the hour.** Sixteen
documents became drawable when the text rendering modes landed, and looking at all of them
found nothing wrong with the new code — but `recursiveCompositGlyf.pdf` came out a solid red
page and it took a side-by-side to learn why: its font is a deliberately malformed TrueType
whose composite glyph refers to itself, `skrifa` produces no outline, and §9.3.6's "if the only
glyphs shown have no outlines … no clipping shall occur" then leaves the page unclipped. Two
of the four references do the same. The picture was not a defect and it *was* the fastest way
to understand what the code now does with a broken font — which is the other reason to look.

**The fifteenth session found the oldest one yet, and it had been mislabelled for four
sessions.** `alphatrans.pdf` had been contradicted by all three references since the oracle
existed, sitting in `CONTRADICTED_SUBSTITUTED_FONT` because its labels use a font nobody
embedded. Opening the side-by-side takes four seconds and shows it: our gradient is a solid
slab of red-to-blue, theirs is glass with three objects visible through it. The page announces
`Gradient: .5` on its own face. The defect was one `return` in `fill_paint` that handed back a
shading and dropped §11.6.4.4's alpha constant with the colour it did not use — nothing to do
with fonts, and nothing any metric here could see, because an opaque gradient reports nothing
and has the right shape in the right place. **Three times now a contradicted page's group has
named the wrong cause**: Type 3 fonts, `/Rotate`, and this.

**The fourteenth session is the rule inverted, and that is the version worth having.** The
picture did not find a defect in new code — it *rejected a reading of the specification*.
`issue6621.pdf` writes a `/Mask` that is a one-bit greyscale image rather than an image mask,
which the standard does not define; the only reading its samples admit is §8.9.6.2's stencil,
where a zero sample marks the page. That was implemented, and the side-by-side showed our
panel blank beside three renderers drawing a court seal, because the seal's *background* is
what a zero sample turns out to be there. No test could have said so and no metric could:
the code was right about the clause it cited, and the clause does not apply. **When the
standard defines nothing, the page is the only thing that can tell you your reading is
untenable — and "untenable" is a stronger result than "wrong", because it is what justifies
refusing rather than guessing.**

**The eighteenth session's instance took four seconds and found a report, not a defect.**
Soft masks landed, four pages arrived on the newly-contradicted list, and the first
side-by-side opened — `knockout_smask.pdf` — showed our purple overlap where two references
draw pure blue. Nothing about the mask was wrong; the page is a *knockout* group, whose report
had been suppressed by the very report the session had just removed. **Every page a new
feature makes drawable is a page nobody has ever looked at**, and on this one the artefact was
the only thing that could distinguish "we drew it wrong" from "we stopped saying we could not
draw it".

**The nineteenth session's instance is the cheap version and it took four minutes.** Six corpus
documents stroke a zero width on page one; opening them on the GPU backend showed one of them,
`zerowidthline.pdf`, drawing a red bar and a caption and nothing else — no lines, and no stroked
text. The page states the expected result in words on its own face: "second should be 1 device
pixel, third should also be 1 device pixel (but scaled 2x)". Two of the corpus's 974 documents
are written to be their own oracle and both have now paid for themselves. **When a fix touches a
named feature, list the documents that use it and open them**; the list took one throwaway
example over the corpus and the answer was in the first file.

**The twentieth session's instance is the rule inverted again, and it rejected a reading of a
table.** Composite fonts landed, three pages arrived on the newly-contradicted list, and the first
side-by-side — `issue7901.pdf` — showed our panel reading `üãÍ†Ë œÍ†ÿ¨ Ì{«` beside four that read
"The Free Software Definition". Nothing about the `CMap` was wrong. The font is a `CIDFontType0`
carrying a `/CIDToGIDMap` stream, and Table 115's "Required for Type 2 CIDFonts" had been read as
saying the entry *means* nothing on a Type 0 one — a reading no test could refute, because the code
was right about the clause it cited and had cited the wrong half of the table. **Two of the three
newly-contradicted pages were diagnosed in ten minutes and neither was about the feature just
built**: the second is a simple `TrueType` font whose `/Differences` misnames a glyph its own
subset holds (§9.6.5.4). Every page a new feature makes drawable is a page nobody has ever looked
at, six sessions running.

**The sixteenth session is the same rule pointed at a page nobody would have called suspect.**
`french_diacritics.pdf` had been contradicted for two sessions under `CONTRADICTED_PAGE_ROUNDING`,
whose whole story — its raster is 595x842 where `poppler`'s and `mupdf`'s is 596 — is *true* and
was not the disagreement. A change that touches nothing but reduced images took it from worst
tile 12.60 against a bound of 5.89 to agreeing. **Four for four**: Type 3 fonts, `/Rotate`,
`alphatrans.pdf`'s gradient, and this. The habit that keeps failing is not writing the group
down — the groups are useful — it is *believing* one without opening the artefact, and the
groups whose stories are verifiably true about the page are the most convincing wrong ones.

### 2. A paint is positioned in the *path's* space, not the device's

Both `tiny-skia` and Vello apply the drawing transform to a paint as well as to the shape:
`Pixmap::fill_path` and `stroke_path` post-concatenate it onto the shader, and Vello
encodes a brush transform as `shape transform * brush transform`. So the transform you hand
a gradient, a pattern or an image is read **in the space the path is stated in**, and
composing the page-to-device transform into it yourself applies that transform twice.

Both backends did exactly that, and it shipped:

- Every gradient was **mirrored about the page's horizontal centre line**. At a scale of
  1.0 the page-to-device transform is a y-flip about the page centre and so is its own
  inverse: the second application cancels the geometry and leaves the flip. At any other
  scale it leaves a scale-dependent displacement instead.
- Every image was sampled through a doubled transform. `issue19971.pdf` draws a 2500×1364
  photograph and we drew one flat dark-blue rectangle.

Three things about how this survived are worth carrying forward:

1. **No metric saw it.** `unsupported: []`, the right shape, colours from the right ramp.
   Trap 1's warning about fonts is the same warning: look at the page.
2. **The CPU-versus-GPU comparison could not see it**, because both backends had it and
   therefore agreed. Two implementations agreeing is evidence *only where they can fail
   independently*.
3. **Every scene that compared them used a gradient running along x**, where a y mirror is
   invisible. A test that cannot fail in the axis the defect moves is not a test of it.

The guards now are `render-cpu/tests/shading_placement.rs` and `image_placement.rs`, which
pin values against ISO 32000-2 §8.7.4.5.3 and §8.9.5.2 **at three scales**, and
`headless_gpu.rs`'s vertical-gradient and image scenes. One scale cannot see this class of
defect; that is why every case runs at more than one. All were confirmed to fail when the
defects are reintroduced.

**The nineteenth session found the sharpest form of this yet, and it is about a convention
rather than an axis.** `tiny-skia` treats a stroke width of `0.0` as a hairline, which is
exactly what §8.4.3.2 requires — "1 device pixel wide" — so the CPU backend got the clause right
without anybody writing the rule down, and a comment said the two semantics "line up in our
favour". Vello has no hairline mode: `kurbo` expands a zero-width stroke into an empty outline,
and **every `0 w` line in every document was invisible on the GPU backend**, including a whole
line of stroked text on `zerowidthline.pdf`, for fifteen sessions. Eleven cross-backend scenes
could not see it because every one of them stroked a width the document stated.

The rule to carry is stronger than "test more axes": **where two backends are the oracle, a
decision either of them can make alone is a decision neither has made.** A convention that
happens to agree with the specification is the dangerous kind, because it removes the reason to
state the rule. `Image::is_smoothed`, `Image::area_averaged` and now `Stroke::device_width` are
in `pdf-render` for this reason, and the test that pins the CPU's substituted width against
`tiny-skia`'s own hairline is what makes moving the decision out of the backend safe.

**And a scene must be able to fail at the defect's *magnitude* as well as in its axis.** The
sixteenth session added `cpu_and_gpu_agree_on_a_deeply_reduced_image` for ADR 0025, in the
right axis — reduction, where the two samplers read different neighbourhoods — and its first
draft **passed with the GPU's filter removed altogether**: it shrank a 64x64 image into an 8x4
corner of a 200x200 page, and 32 differing channels of 160 000 is under `MAX_DIFFERING_FRACTION`.
The scene now draws an 800x800 image across most of the page, and removing the GPU call site
fails it at mean error 6.50 against a bound of 0.5. Deleting the code a scene guards is one
command and it is the only thing that establishes the scene guards it.

The same reasoning shaped the sixth session's annotation tests: §12.5.5's placement algorithm is
correct for any axis-aligned `/Matrix` even if you measure the *untransformed* `/BBox`, so
the fixtures use a rotation and a non-square `/Rect`. A square rectangle cannot tell the two
axes' scales apart.

### 3. An oracle is only as good as how it invokes the other renderers

The corpus oracle's first run reported 54 documents whose page *size* we disagreed about,
which looked like a `MediaBox` defect in our page tree. It was not. `pdftoppm` and `gs`
default to the **media box**; `mutool` and we use the **crop box**, which §7.7.3.3 defines
as "the region to which the contents of the page shall be clipped (cropped) when displayed
or printed" and is what a viewer shows. The harness had been asking two of the three
references for a different page.

Two things to carry from that. It cost 54 documents of coverage outright — they could not
be compared at all — and it would have been *worse* than useless on a page whose crop box
has the same size as its media box but a different origin, where it would have compared a
correct render against a displaced one and called us wrong. And the fix was decided by the
clause, not by the fact that `mutool` happened to agree with us: agreement was evidence we
had read §7.7.3.3 the same way, which is the only thing agreement is ever evidence of.

Every reference invocation is now explicit about the page box, including `mutool`'s, whose
default was already right — a default that silently changes is a comparison that silently
changes.

### 4. Test against real documents, not hand-written fragments

Cross-reference streams are compressed *and* PNG-predicted. The code said decoding them
was "the caller's responsibility" and then did not do it, so every modern PDF failed with
a misleading `/Root is not a dictionary`. Unit tests on fragments would never have caught
it; the corpus caught it on the first run.

`crates/pdf-syntax/tests/real_documents.rs` and
`crates/pdf-model/tests/render_real_pdf.rs` run over everything in `doc/`. Keep them
passing.

The converse is trap 8: a corpus cannot find what no document in it happens to contain.

### 5. Unsupported input must stay loud

Every layer reports what it could not handle rather than skipping it: `Unsupported` in the
interpreter, `FontError`, `ImageError`, `CpuRasterError::UnsupportedCommand`. This is not
politeness — it is what makes the comparison harness trustworthy and what caught trap 1.
Do not "helpfully" fall back to a default that renders something plausible.

The oracle found three places where this rule had been broken by omission rather than by
intent, and all three were drawing something visibly wrong in silence: text render modes 4
to 7 add the glyphs to the clipping path and we built no clip; an image's `/Mask` was
ignored, so a band the document masks out was painted; and a page's annotations were absent
entirely. All three are implemented now, the last of them in the fourteenth session. **A rise in the incomplete count is not a regression when it is a new report** —
it is this rule being applied to somewhere it had not been.

The lesson generalises: a feature that is *partly* implemented is the easiest place to lose
this rule, because the operator is handled and the code path exists. `Tr` was parsed, the
mode was stored, three of its eight values were reported, and the four that change the clip
were not. All eight are implemented as of the thirteenth session (ADR 0022) — and the same
review found the *next* instance one clause away, in the ninth text state parameter: `/TK`
was not read at all and nothing said so (§9.3.8). It reports as of the fourteenth session,
and trap 11 is what that cost to get right.

There are now two places where a report accompanies drawing rather than replacing it, and both
are deliberate. The second is §11.6.5.2's `/Matte` in a colour space whose pre-blending cannot
be undone after conversion: the mask is applied, because it is fully specified and refusing it
would draw an opaque rectangle whose edges are entirely the matte colour, and the un-inverted
colours are named. The first: An `/AcroForm` setting `/NeedAppearances` is the document saying its stored
appearance streams are not the ones to draw (§12.7.4.3); we draw them anyway, because they
are all the file offers, and report that they may be stale. Two different true statements,
and suppressing either loses information. Do not generalise it further without the same
argument.

### 6. Colour: one conversion, and the specification often does not have an answer

Three separate `DeviceCMYK` → RGB conversions used to live in this tree and they disagreed.
`0.5 0 0 0.5 k` gave a red channel of 0.25; the same colour through `scn` gave 0.0; a CMYK
image gave a third answer. Nothing about a rendered page reveals that — each looks like a
plausible colour. `crates/pdf-model/tests/colour_paths.rs` now drives one value through all
three routes and demands they agree; it was verified to fail when the old code is restored.

Add no fourth path. `ColourSpace::to_rgb` is the only place a colour becomes RGB, and — since
the sixth session — `colour::xyz_d50_to_srgb` is the only place an XYZ becomes a pixel. That
second rule was added because the *same* defect had quietly recurred one level down: `lab()`
and `icc::xyz_to_rgb` each held their own copy of the nine-constant D50-to-sRGB matrix.
Nothing had gone wrong yet. It is one edit away from going wrong invisibly.

The other half is harder to hold onto: **ISO 32000-2 defines no `DeviceCMYK` conversion at
all**. §8.6.4.4 says "concentrations of process colourants" and stops; §8.6.5.7 NOTE 3 says
nothing in PDF describes the device. What the specification *does* say is where to ask —
`/DefaultCMYK` (§8.6.5.6, normative), an output intent's `/DestOutputProfile` (§14.11.5),
and an `ICCBased` profile — and all three are implemented and all three outrank the
fallback table. When you touch that table, do not reach for what another renderer produces:
read ADR 0009, and if you change it, change it as a documented choice.

The same shape recurs for a Cal space's `/BlackPoint`: §8.6.5.9 leaves black point
compensation to the processor whenever `/UseBlackPtComp` is `Default`, which is every real
document. It is read and deliberately not applied, and ADR 0012 has the argument — including
the part that decided it, which is that a stretch built from the entry is *undefined* on
input Table 63 permits.

### 7. `#[expect]`, never `#[allow]`

Every lint exception in the tree is `#[expect(..., reason = "...")]`. It errors when it
stops being necessary, which has already removed several stale ones. A bare `allow` hides
that forever.

### 8. A corpus finds what documents contain, not what the specification says

Added in the sixth session, because it is the mirror of trap 4 and the two are easy to
confuse.

The ICC evaluator agreed with two other readers on every real profile in the corpus. A test
that assembled a profile *by hand*, to check one clause of the ICC encoding, produced a
profile whose darkest colour equalled its white point — and black point compensation divided
by a span of floating-point noise and turned white into pure green. No real profile is shaped
that way.

The same thing happened again in the sixth session, from the other direction: `calrgb.pdf` page 14
states `BlackPoint [0.2 1.0 1.7]` against `WhitePoint [1 1 1]`, which Table 63 permits and
which no sane producer writes. It is what proved that the black point stretch has no
well-defined answer at all. **The corpus is not a specification, and a clause nothing in it
exercises is still a clause.** Synthetic fixtures and real corpora catch different things.

**The twentieth session found the version with no members at all, and measured it.** §9.7.6.2
matches a code against a codespace range *byte by byte* — `<C280> <DFBF>` admits `C2 80` and not
`C2 C0` — which is not the same as comparing the whole code against `0xC280..=0xDFBF`. Replacing
the per-byte test with the numeric one and running the whole oracle leaves **all 1794 verdicts
identical**: every mapping any corpus `CMap` states falls inside its own ranges byte-wise, so no
code exists in these 974 files that the two readings disagree about. The rule is still required of
any valid PDF, and the only thing in the tree defending it is a synthetic `CMap` in
`cmap.rs`. Worth doing for the *method* as much as the result: breaking a rule deliberately and
running both gates is what turns "the corpus does not cover this" from a suspicion into a fact.

This trap was stated in the sixth session and acted on in the ninth: it is the reason
`CLAUDE.md` principle 5 now defines *done* as every PDF rendering as its producer specified —
scope stated as a closed exclusion list rather than inferred from what the corpus asks for —
and the reason the conformance ledger (`PLAN.md` §5a) exists at all. A caution that changes
no plan changes nothing.

### 9. Two references can agree because they share code — or because they share a *gap*

The oracle's whole authority rests on a premise stated in ADR 0005: two implementations
sharing no code agreeing about a page is evidence. There are two ways for that premise to
fail, and the ninth session found the second.

**The second way is the common one, and it needs no shared code at all.** An unimplemented
feature almost always falls through to *draw it*, so two unrelated programs that have both
skipped the same clause produce the same picture — and the gate reads that as agreement.
`visibility_expressions.pdf` is the case: two of its five `/VE` visibility expressions are
false, so two lines stay pale. We draw them pale, and so does `poppler`; `mupdf` and
`ghostscript` draw all five dark. Their source says why, and it was read rather than inferred
— `mupdf`'s `pdf-layer.c` carries `/* FIXME: Calculate visibility from array */ return 0;`
and `ghostscript`'s `pdf_optcontent.c` carries `WARNING: OCMD contains VE, which is not
supported (ignoring)`. Meanwhile `poppler` exports `OCGs::evalOCVisibilityExpr` and pdf.js
implements it in `src/core/evaluator_utils.js`, in this repository under `doc/pdf.js`.

Three implementations against two, and §8.11.2.2 is not ambiguous: "If the VE key is present
it shall be used in preference to the OCGs and P keys." So the page stays contradicted, listed
under `CONTRADICTED_VISIBILITY_EXPRESSION` with the source citations beside it.

**What to do when a contradiction looks like this.** Read the clause first, then go and look
at what the disagreeing renderers actually *do* — their source is a search away, and "does it
implement this at all" is a much cheaper question than "is its answer right". A `FIXME` in
another project's source is stronger evidence than any number of agreeing pixels.

The first way is narrower and was found in the seventh session, on JBIG2.

**`mupdf` and `ghostscript` both link `jbig2dec`**, Artifex's library, and on seven corpus
pages it decodes nothing and renders blank, or renders the drawing strewn with noise blocks,
or prints `segment marks bitmap coding context as retained (NYI)` and gives up. Two renderers
then "agree" and the gate reports us contradicted.

It took a side-by-side to see it and a log to prove it: both renderers emit the *same warning
text*, because it is the same code emitting it. What settles those pages is not poppler's
agreement — that would only be evidence that we read ISO/IEC 14492 the same way — but
`tests/jbig2.rs`, where the corpus encodes one image ninety-six ways and all ninety-six decode
to byte-identical pixels here. The seven are `CONTRADICTED_SHARED_JBIG2_DECODER` in
`oracle.rs`, listed rather than excused.

**So ask what a reference is made of, not only what it produced.** `poppler`, `mupdf` and
`ghostscript` look like three implementations and are three *renderers*; underneath they share
libraries per format, and separately they share whichever clauses none of them has got round
to. Both are worth checking wherever two of them agree suspiciously often.

**The general form is now in the type.** `Reference::independence` says whether a renderer's
agreement is evidence, and `Reference::voting` is what the gate iterates, so a reference that
cannot supply evidence cannot silently be counted as supplying it. `hayro` is the first
entry marked `Shared`: it is a fourth renderer, rendered into the artefacts of every page
that is not agreement — a fourth panel in the side-by-side, which is the first thing to open
— and it never votes, because we share its font rasteriser, its deflate, its JPEG decoder and
both new image codecs. `mupdf` and `ghostscript` are deliberately *not* marked `Shared`: they
share only `jbig2dec`, and on every page without a JBIG2 image they are two implementations
of everything that matters, so recording the sharing where it applies keeps the evidence of a
thousand pages that marking them wholesale would throw away.

### 10a. A cached reference render is a fourth thing that can be stale

Added in the twelfth session, when the oracle stopped asking `pdftoppm`, `mutool` and `gs`
every run and started remembering what they said (ADR 0020). It went from 75 seconds to 25,
and it introduced exactly one new way to be wrong: comparing against a render that is no
longer what the renderer would produce.

The key is built from the invocation itself — `Reference::build_command`'s own argument list,
plus the renderer's version and the document's SHA-256 — so **a flag that is not in the key is
a flag that is not passed to the renderer either**, and the class of mistake this file warned
about before the cache existed cannot happen by omission. What it cannot see is a renderer
whose output changes while its version string does not, which is a distribution's problem.

Three things to know:

- `PDFREF_CACHE=off` runs the gate the old way. Doing that and comparing the counts is how
  "the cache changes no verdict" is checked over the whole corpus rather than on one fixture.
  **The variable names a *directory*, and only the literal `off` disables it** — so
  `PDFREF_CACHE=on`, which reads like the opposite of `off`, silently starts a fresh 319 MB cache
  in a directory called `on` beside whatever the working directory is. The twentieth session did
  that; the run was correct, because a cold cache asks the renderers and produces the same
  verdicts, but the useful part is that nothing said so. If a run takes 95 s and reports a 0% hit
  rate, look at the variable before looking at the corpus.
- **The hit rate is printed, and it is the tell.** A run over an unchanged tree that reports
  less than 99% is telling you the corpus or a renderer moved.
- **A remembered *timeout* is the one entry whose truth decays**, and it is counted on its own
  line for that reason. It expires after a week; the argument for remembering it at all — two
  decompression bombs were 46 of a 57-second run — is in `pdfref::cache`.

### 10. The sandbox worker is a separate binary, and Cargo will not rebuild it for you

`cargo test -p pdf-model` builds pdf-model's targets and pdf-sandbox's *library*. It does not
build pdf-sandbox's `pdf-sandbox-worker` binary, because Cargo never builds another package's
binaries. So the tests run against whatever worker was last compiled.

This is not hypothetical. While verifying that `tests/jbig2.rs` can fail, the seventh session
deliberately inverted the black-and-white sense of every JBIG2 sample — and the test passed,
because the stale worker was still decoding correctly. The defect was real, the test was
right, and the two never met.

`cargo test --workspace` builds it. `cargo build -p pdf-sandbox --bins` builds it. Both gates
call `require_the_sandbox()` first, which fails loudly if the worker is *missing* — but a
missing worker and a stale one look nothing alike, and nothing detects the second.

`pdfref-hayro`, which draws the oracle's fourth panel, is found the same way and carries the
same caveat. It is less dangerous there: a stale one produces a stale *picture* next to three
fresh ones, and it never votes, so the worst case is a confusing artefact rather than a wrong
number.

### 11. A report is only as good as the condition it fires on

Added in the fourteenth session, and it is trap 5's other edge. Principle 3 says unsupported
input must stay loud, and the reflex it produces is to report whenever the unimplemented thing
*could* be involved. §9.3.8 is the case: `Tk`'s initial value is true, so every text object in
every document is composited under a model we do not implement, and a report on that fact
alone would name several hundred documents and mean nothing.

The first draft asked one of the clause's two conditions — glyphs drawn under a paint that
composites — and named 7 documents. It also took **three pages that agreed with the reference
consensus out of the oracle's gated set**, because a page that reports is a page the oracle
stops judging (see the eighth session's 43). Those three could not have shown the difference:
their glyphs do not overlap, and knockout changes nothing where nothing overlaps.

The second draft asks both conditions — the paint composites *and* two glyphs of the object
overlap — and names 2. The gap did not change. What changed is that the report now marks pages
where the two models can actually produce different pixels.

Two things to carry:

- **Costing a report in gated pages is part of designing it.** A report is not free: it moves
  a page from "judged against three renderers" to "expected to differ". Over-reporting buys
  honesty about a page that was already right and pays for it in coverage.
- **The condition belongs in the clause, not in intuition.** Both halves of this one are
  written in §9.3.8 and §11.3.7 — knockout is about *the area of overlap*, and alpha is shape
  times opacity, so opaque Normal painting composites identically either way. Reading them is
  what made the narrow test writable at all.

**The fifteenth session did the same thing for §11.6.2 and had to instrument to get it right.**
"Portions of an object shall not be composited with one another" covers `B`, which becomes a
fill and a stroke here. The first check asked only whether the paint composites and named six
documents, two of which had been *agreeing*. Printing the actual alphas showed why: three of
the six set `ca` or `CA` to **zero**, so one of the two parts paints nothing and there are no
two portions to composite. The clause says "portions", plural; the code had taken the operator
as proof of them. The narrowed check names four documents and costs one page — `alphatrans.pdf`,
which the same session had just moved from contradicted to agreeing.

So the third thing to carry is: **print what the condition matched before trusting the count**.
Both of the times this trap has been exercised, the first draft was defensible from the clause
and wrong about the corpus, and a `eprintln!` in the branch settled it in one run.

**The nineteenth session found the condition that turned out to be empty, and that is the
trap's furthest end.** Overprinting was 63 corpus documents, six `silent` rows and the top of
the demand list. The condition trap 11 asks for — where can the special overprinting blend mode
change a pixel? — has *no members* on this device: Table 146's blend function is the source
colour for every row a group space of three process components with no spot colourants can
reach. So the third thing above generalises: **print what the condition matched before trusting
the count, and derive the condition from the clause before writing the `eprintln!`** — because
the derivation can tell you there is nothing to print. A document count sizes a *key*. Only the
clause sizes a difference.

**And the eighteenth session found the trap's other end: a report can hide another report.**
`knockout_smask.pdf` paints an opaque blue over an opaque red inside a *knockout* group, under
a soft mask. §11.4.6's report has fired since the seventeenth session on exactly that
condition — an element that composites overlapping one painted before it — and it stayed quiet
here, because `command_composites` knew about alphas, blend modes and image transparency and
nothing yet could carry a soft mask. The page reported the mask instead, so nobody looked. A
page that reports one thing is a page whose *other* gaps are unmeasured, which is an argument
for closing reports rather than accumulating them and the exact reverse of the usual worry
about over-reporting.

### 12. A bound derived from two agreeing references is tighter than the arithmetic

Added in the eighteenth session, and it is about the *gate* rather than the renderer.

`oracle.rs` judges us relative to how far the consensus references sit from one another: the
tolerance is theirs, widened by a factor. That is the right rule — it stops a page where every
renderer differs from being called our defect — and it has a consequence worth knowing.
**Where two references agree very closely, the bound they generate can be tighter than eight-bit
arithmetic.**

`smask_luminosity_oob_transfer.pdf` is the case. Its whole page is one flat composite through a
mask of 0.75; the closed form is `(223, 99, 80)`, `mupdf` gives `(222, 98, 79)`, `ghostscript`
`(223, 99, 79)` and we give `(223, 100, 81)`. Everybody is within a level of the arithmetic.
But `mupdf` and `ghostscript` are within a level of *each other*, so the bound is a mean of
1.11 and ours is 2.02 — contradicted, by one level of mask quantisation on a page with nothing
else on it.

What to do with such an entry is not to chase it. It is to check the *closed form* — write the
clause's arithmetic down and see whether we are within a level of it, which
`render-cpu/tests/soft_mask.rs` now does — and then list the page with the calculation beside
it. What would be wrong is the reflex the number invites: tightening our own rounding until a
reference's rounding is matched, which is curve-fitting with extra steps.

## Environment

The agent runs as user `AI` via `sudo -u AI`, reaching `/home/cl/projects/pdf-viewer`
through the `coders` group. This causes recurring friction:

- **Launch with a login shell** so `umask 002` applies, or every file the agent creates is
  unwritable by `cl`:
  `sudo -u AI bash -lc 'cd /home/cl/projects/pdf-viewer && claude'`
- **`AI` has no X authority cookie.** Anything needing a window fails at
  `XOpenDisplayFailed`. The GPU backend is headless by construction precisely so it can
  still be tested; the viewer binary cannot be run by the agent past event-loop creation.
- **Build directory**: `AI` builds into `/home/AI/cargo-target/pdf-viewer` via
  `~/.cargo/config.toml`, so the two users never fight over `target/`. Do not "fix" this
  by sharing it again.
- **`pdfref` needs `--work-dir`** for the same reason; its default is `./target/pdfref`.
- **`cargo-fuzz` needs `+nightly`** explicitly, because `rust-toolchain.toml` pins stable
  1.97.1. That pin is deliberate.
- The Arlington model is a **submodule** pinned at `ba7d4d61`; `pdf-spec` will not build
  without `git submodule update --init`.

## What is not implemented

Every one of these is *reported* at runtime rather than silently skipped — that is what
makes the corpus numbers below trustworthy, and it is principle 3's requirement, not a
nicety. Sized by the corpus rather than by intuition: the count is how many of the 974
documents' first pages it affects.

| Missing | Corpus | Size | Notes |
|---|---|---|---|
| Predefined `CMap`s (§9.7.5.2) | 12 | Medium | **What is left of the text row after ADR 0029**, and it is data rather than code: 15 fonts name one of Table 116's registered `CMap` files (`90ms-RKSJ-H`, `UniJIS-UTF16-H`, …), which are not in the tree. Vendoring them is a licensing decision; guessing at one draws plausible text that says something else, so each is refused and named. The clause's requirement that a processor "shall support Adobe-CNS1-7, Adobe-GB1-5, Adobe-Japan1-7 and Adobe-KR-9 character collections" cannot be met without them. The `CMap` machinery they would plug into exists. |
| Text: a substitute that cannot be addressed | 42 | Medium | The rest of the gate's `Text` row, counting *fonts*: 27 composite fonts with no `/ToUnicode`, so a CID — which means nothing outside the font that defined it — cannot be taken to a character a substitute could draw; and 23 fonts whose substitute draws none of the codes the document declares. Both are honest refusals rather than gaps in a clause, and closing them means better substitution rather than more of §9.7. |
| Synthesised annotation appearances | 63 | Medium–large | An annotation with **no** `/AP` must be drawn from `/IC`, `/C`, `/BS`, `/Border` and its subtype's own rules — a different routine per subtype. 26 `Widget`, 18 `Link`, and the rest markup annotations. Reported, never guessed. ADR 0013. |
| Encryption | 20 | Medium | RC4/AES, `/Encrypt`. 11 documents cannot reach page one at all and 9 more draw a blank page. |
| Form field appearance construction | 7 | Medium | `/NeedAppearances` (§12.7.4.3). The field's value is known only at viewing time, so its appearance has to be built from `/V`, `/DA` and `/Q`. The stored appearance is drawn and the staleness reported. |
| Optional content: the interactive half | — | Medium | §8.11 is honoured wherever it decides what is *drawn* (ADR 0017). What is missing is a layer panel and what feeds it: `/Usage` and the `/AS` usage application dictionaries (§8.11.4.4), which switch groups by zoom, language or print state, and `/Order`, `/ListMode`, `/RBGroups`, `/Locked` and the alternate `/Configs`. §8.11.4.4 is the ledger's second `silent` row: this viewer has a window, so those requirements do apply to it, and a layer that should switch itself off is drawn with nothing said. |
| `LZWDecode` | 0 | Small | **The last standard filter absent of any kind**, now that `CCITTFaxDecode` decodes. **This row said 3 and the three were miscounted**: `bug864847.pdf`, `XiaoBiaoSong.pdf` and `SimFang-variant.pdf` contain the string `LZWDecode` and all three draw page one completely, so nothing in the corpus exercises it on a first page. `colour_paths.rs` pins the report on a synthetic file and will fail when the filter lands — which is the only instrument that covers it, and trap 8 in one line. |
| Text knockout (`Tk`, §9.3.8) | 2 | Medium | Table 102's ninth text state parameter, and the only one absent. Its initial value is `true`, which makes a whole text object a non-isolated knockout group so a later glyph overwrites an earlier one where they overlap; we composite each glyph separately, which is the `Tk` false model — indistinguishable while glyphs are opaque under the Normal blend mode, and wrong otherwise. **Reported since the fourteenth session**, on the two documents where both of the clause's conditions hold: the paint composites, and two glyphs of one object overlap. `/TK` is read, including the rule that a value set between `BT` and `ET` is ignored. Implementing it is §11.4.6's knockout groups seen from clause 9, and belongs with them. |
| Compositing an object in parts (§11.6.2) | 1 | Medium | "Portions of an object shall not be composited with one another", and `B` and its three relatives paint one object as a `Fill` and a `Stroke` — so the band a centred stroke shares with the fill composites twice under a paint that composites at all. **Reported since the fifteenth session** (ADR 0024), on the pages where it can show: the paint composites, and both parts mark the page. 4 documents reach the report, `alphatrans.pdf` is the one visibly wrong, and the fix is the same one as `Tk`'s — composite the parts as one element, which is §11.4.6's groups. |
| Image `/Mask` on a filtered image, `/Matte` outside the device spaces | 0 | Small | What is left of §8.9.6 and §11.6.5.2 after ADRs 0023 and 0024, and no corpus document writes any of it. A colour key is a test on the samples a filter delivers, and a `DCTDecode` or `JPXDecode` image has become RGBA before the unpacker sees it — the clause's own NOTE 2 names that pair as the one lossy coding makes unreliable; JBIG2 and CCITT are refused with them rather than special-cased. A `/Mask` stream that is not an image mask is here too, which Table 87 excludes and 1 document writes (see trap 11). So is a `/Matte` on an image whose colour space is not `DeviceGray` or `DeviceRGB`: §11.6.5.2 requires the pre-blending to be undone *before* colour conversion, and this crate holds one RGBA raster per image, so the inversion is exact only where that conversion was the identity on components. |
| Transparency group and mask departures (§11.4, §11.5.3) | 24 | Medium | Three answers a `/Group` may give that are drawn as the isolated, non-knockout group instead, each reported where it can change a pixel (ADR 0026). **Knockout** (§11.4.6): 6 documents, where an element that composites overlaps one painted before it; the implementation is written down in the ledger row, because for an *isolated* knockout group it is a Porter-Duff Source composite modulated by coverage and nothing more. **Non-isolated with a blend mode inside it** (§11.4.4): 9 documents; without one the two computations are provably identical and nothing is reported. **A blending colour space that is not the device's three components** (§11.6.6): 4 documents, all `/DeviceCMYK`, and honouring it means a second raster format rather than a colour conversion. **A soft mask's group with such a space** (§11.5.3, ADR 0027): 7 documents, where neither the compositing nor the luminosity is the clause's — `/DeviceGray` is exempt and it is exact, since a grey converts to `R = G = B` and the three coefficients sum to 1. |
| Grid-fitting a stroke's coordinates (`/SA`, §10.7.5) | — | Small | What is left of §10.7.5 after ADR 0028. The clause's second rule is implemented — a stroke under half a device pixel under `/SA` is drawn as a single-pixel line, which is 4 of the corpus's first pages at 72 dpi, out of the 17 that paint a stroke while the parameter is in force. The first rule, adjusting "the line width and the coordinates of a stroke … to produce lines of uniform thickness", is a **documented departure** rather than a gap: the non-uniformity it removes is an artefact of the binary scan conversion §10.7.4 requires and this tree already departs from by anti-aliasing, and an anti-aliased stroke's coverage-weighted thickness is the requested width at every position. §10.7.1's NOTE licenses it as it licenses the other three. Nothing reports it because there is no page on which this device could do better. |
| Smoothness tolerance (`/SM`, §10.7.3) | 23 | Small | Read nowhere, and mostly harmless: this renderer has one fixed internal bound — a 256-sample `Ramp`, and `Triangle::is_subpixel` — where the clause asks for a per-document one, and "each output device may have internal limits on the maximum and minimum tolerances attainable" contemplates precisely that. A document asking for a *coarser* shading than we draw is given a finer one, which cannot be a fidelity error; one asking for finer than 1/256 of a component is not honoured and nothing says so. That silence hides inside a `partial` row, which is the same shape as §8.9.5.2's `/Decode`. |
| `/UserUnit` | 2 | Small | §7.7.3.3: the size of a default user-space unit in multiples of 1/72 inch. `mutool` and `gs` scale the page by it, we and `poppler` do not — `bug1947248_*.pdf` come out at 612x792 where they produce 1836x2376. Neither applied nor reported; the oracle lists them under `GEOMETRY`. |
| Annotation `NoZoom`, `NoRotate` | — | Small | Table 167 bits 4 and 5 make an appearance's size or orientation depend on the *view*, which a resolution-independent display list cannot express. Rare. |
| Type1 fonts (`/FontFile`) | 0 | Medium | No corpus page one reaches it, so this is smaller than it looks. `read_fonts::ps::type1` exists — check before writing any. |
| Soft masks and `/Mask` at a grid the bound refuses | 1 | Small | **Closed in the fifteenth session except for one file** (§11.6.5.2 Table 143, ADR 0024): a mask of any size is combined with its image on the finer of the two grids, bounded on how much bigger the combined grid may be than the image itself. `issue16263.pdf` gives a 2x2 image a 34862x4332 mask — 151 million samples, 604 MB — and that pair is refused and named, which is the whole of this row. The answer the clause actually describes is compositing at *device* resolution, which means the display list carrying an image and its mask separately and both backends sampling them; that is a `pdf-render` change and belongs to whoever takes it. A graphics-state soft mask is now the one raster that *is* combined at device resolution (ADR 0027), which is the shape the answer here wants. |
| Bit depths 2, 4 and 16 | 3 | Small | §8.9.3 permits five component widths and the unpacker reads two. Refused and reported, which is honest, and is now the largest *codec-shaped* image gap left — though the `/Mask` row above it affects more documents. |
| Vertical writing (`Identity-V`, `/W2`) | 4 | Medium | §9.2.4 gives a glyph in writing mode 1 a second set of metrics — a displacement vector `w1` and a position vector `v`, from the CIDFont's `/W2` and `/DW2` (§9.7.4.3). None of it is read. `Identity-V` was accepted beside `Identity-H` until the tenth session, because the two map codes identically, and `vertical.pdf` came out as one overlapping line across the top of a page where two columns belong down the right edge. Now refused and reported — and since ADR 0029 the refusal covers an *embedded* `CMap` declaring `/WMode 1` as well, checked against Table 118's `/WMode` too, because the clause requires the two to agree and being drawn horizontally is not a near miss. |
| Sampled shadings on the GPU | 2 | Small | Type 1 only; the CPU backend draws them. |
| Rendering intents beyond `AbsoluteColorimetric` | — | Small | Read and recorded; `A2B0` is not yet selected for `Perceptual`. |
| Forms, actions, the rest of clause 12 | — | Large | Interactivity: field values, calculation order, navigation. In scope wherever it *displays* — field appearance construction, outlines, destinations, page labels. **JavaScript and script-driven field behaviour are excluded** by principle 5's closed list; field appearance is not. Not needed to *draw* an annotation, which is why drawing landed without any of it. |
| Tagged PDF, metadata | — | Large | Clause 14 beyond output intents. In scope as far as accessibility needs it. |
| Sandboxing the *rest* of the renderer | — | Large | Spike D is done for the image codecs (ADR 0014). Interpreting and rasterising still happen in the main process. |
| JPEG 2000 at reduced resolution | 1 | Small | `issue19517.pdf` is a 12608x16806 scan whose full decode wants gigabytes for a page drawn at four megapixels. JPEG 2000's answer is to decode a lower resolution level, which needs the intended scale to reach the decoder. Refused with a clear report today. |

## How much of the specification is implemented

Four answers, because the honest one depends on what you are counting — and they are in
ascending order of how much they should worry you. The first counts what we *report*, the
second what an implementation that shares no code with us *sees*, the third what the
standard contains, and the fourth is what a person has actually read.

The first two are measured. **The third is a self-assessment and has been wrong twice**: it
called clause 9's encoding algorithms "implemented in full" while §9.6.5.4 was one line
covering about one and a half of its five routes, and the feature table below said Type 3
fonts were reported for two sessions in which they were not. Both errors were found by
pixels, not by reading. Read the "By clause" table as what the code's authors believe.

**The fourth now exists**, and it is the conformance ledger. Its headline is not a percentage
implemented but a count of unasked questions: **594 of 823 subclauses are `unreviewed`**, and
229 have been read against this code — 81 of those being clause 13, which principle 5
excludes by name. So the honest summary of clause coverage is that the project has begun
measuring it and has measured 18% of it. That number is meant to look bad; the alternative was
not knowing.

**And the ledger has now been wrong twice**, which is worth knowing before trusting a row:
§8.9.5.3's note said reduction was something the standard does not address, for two sessions,
and §10.7.4 addresses it in the opposite direction; and §8.4.3.2's row said a zero width
"reaches the rasteriser as the thinnest line it draws", which was true of `tiny-skia` and false
of Vello for fifteen sessions. A row states what its author found in the clause it names; it
cannot state what is in a clause nobody opened, and **a row that names a rasteriser's behaviour
has recorded that rasteriser rather than the clause**. The defence is the one the ninth session
built for citations — read the *family*, not the row — and it is why the review unit is a clause
family rather than a subclause.

### By what real documents need

Over the 974-document pdf.js corpus, page one:

| | count | share |
|---|---|---|
| opens | 974 | 100% |
| reaches page one | 955 | 98% |
| **draws with nothing reported** | **766** | **79%** |
| draws, with something reported | 189 | 19% |

That 79% is the number to quote for *reporting*. It **rose by thirty-one documents** in the
twentieth session, when embedded `CMap`s and `/CIDToGIDMap` streams landed (§9.7, ADR 0029) —
the largest single movement the number has ever had, and the whole of it is text that had been
refused rather than drawn wrongly. It did not move in the nineteenth session, and
that is the shape of a session whose two items were a silence that closed by derivation and a
defect on the backend neither gate renders with. It **rose by seventeen documents** in the
eighteenth session, when soft masks in an `/ExtGState` stopped being reported and started being
drawn (§11.5, ADR 0027) — the largest single movement since `CCITTFaxDecode`, and the first in
four sessions that is a feature rather than a report. It **fell by six** in the seventeenth session,
when seven documents began saying that their `/Group` is a knockout group or is non-isolated with
an element that blends, and one stopped reporting §9.3.8 because §11.6.6 resets the constant
alpha its glyphs had carried. Nothing stopped drawing correctly; two silences ended. It moved by
one document in the fifteenth
session and by three in the fourteenth, and in both the arithmetic went both ways at once: two
documents left the reported list when an `/SMask` of another size began to apply (§11.6.5.2)
and one joined it when §11.6.2's fill-and-stroke began to report. In the fourteenth, five left
when `/Mask` began to apply (§8.9.6.3 and §8.9.6.4) and two joined when §9.3.8's text knockout
began to report. It **rose** by two points in the thirteenth
session, by one in the twelfth, by two in the eleventh and by one in the tenth: the eight text
rendering modes account for 16 documents, `CCITTFaxDecode` for 12, inline images and the four
colour space families the image unpacker refused for 23, and three began saying that their
soft mask is not the size of its image. Both halves are the point. It **fell** from 72% in the eighth session,
when 24 documents began saying they carry a Type 3 font and 19 that their substitute draws
none of the codes the document declares — nothing had stopped drawing correctly, what stopped
was drawing *incorrectly in silence*, and `issue918.pdf` was emitting 388 text operations of
letter fragments while reporting nothing.

It went down in the sixth session too, from 68% to 60%, for the same kind of reason, and up
in the seventh when JBIG2 and JPEG 2000 landed. **This number measures honesty, and honesty
can fall as capability rises** — so a rise is only good news when you can say which
capability caused it, and a fall is only bad news when you cannot say which silence ended.

### By what an independent renderer sees

This is the number to worry about. Over all 1794 pages compared, of the 1557 we claim to
draw completely:

| | count | share of the 1557 |
|---|---|---|
| agree with the reference consensus | 706 | 45% |
| **contradicted by it** | **98** | **6%** |
| the references cannot agree among themselves | 740 | 48% |
| not comparable (geometry, or fewer than two renderers) | 13 | 1% |

**One page in sixteen that we say we drew completely, two independent implementations say we
did not.** The 98 are named in `oracle.rs` and grouped by what the page carries: 16 use a
font nobody embeds so every renderer substitutes differently, **8 are pages where the two
references that agree are wrong and we are right** — 7 where they are the same JBIG2 decoder
and 1 where neither implements `/VE` (trap 9 has both) — 8 are a one-pixel page-rounding
difference, 1 is an image half a device pixel tall, 1 is a
`CalRGB` alternate space two references do not convert, 2 are pages of glyphs being judged
with the tolerance for flat fills, 1 is one level of mask quantisation on a flat page (trap 12),
1 is a symbolic font claiming two contradictory descriptor flags, and **60 have nothing on them
to explain it**. That last
group is the most valuable list in the repository. 21 of them are pages beyond the first,
which a page-one comparison would never have seen.

**96 → 98 in the twentieth session, with thirty-two pages added to the denominator and
eighteen of them agreeing outright.** That is the pattern this section keeps describing arriving
at its largest scale: a feature that makes pages drawable adds them to the set being judged, and
two of the thirty-two are contradicted. Neither is a `CMap` defect — `issue7901.pdf` draws its
sentence and fails only the differing-fraction bound on a page that is nothing but eight-pixel
glyphs, and `issue20232.pdf` is missing one glyph of a simple font whose descriptor claims to be
symbolic and nonsymbolic at once (§9.6.5.4). Both are named with their arithmetic beside them.

**96 → 96 in the nineteenth session, with nothing added to the denominator.** The two items it
took cannot move this count and it was possible to say so in advance: overprinting changes no
pixel this device produces (ADR 0028), and the zero-width stroke defect is on the GPU backend,
which neither gate renders with. `/SA`'s single-pixel rule *can* move a page, and the instrument
says which: 17 corpus first pages paint a stroke while the parameter is in force, 4 have one thin
enough to adjust at 72 dpi, and of those 4 two already agree and two are pages the references
cannot agree about. On the second two the change moves us closer — `bug1721218_reduced.pdf` from
mean 0.28 to 0.27, worst tile 18.58 to 18.41 — by less than the gate can resolve. **Measuring
what a change could reach before running the gate is what turns a flat row from a
disappointment into a prediction.**

**100 → 93 in the seventeenth session, and only one of the seven was fixed as such.**
`issue11279.pdf` draws a form XObject beyond its own `/BBox`, which §8.10.1 step c) says shall
be clipped — a defect found by reading §8.10 because §11.6.6 sent a reader there, and one that
had been true since the first form XObject. The other six left by *reporting*: four
`knockout_*.pdf` and two pages of `knockout_groups_test.pdf`, which are §11.4.6's knockout
groups and were among the four pages this file's own list had already diagnosed. They are still
contradicted; they are no longer judged. Read the fall as one fix and six honest withdrawals.

**103 → 100 in the sixteenth session, and one of the three departures was in the wrong
group.** Area averaging for reduced images (ADR 0025) fixed the two pages
`CONTRADICTED_IMAGE_RESAMPLING` named — emptying the first group in `oracle.rs` ever to
empty — and a third, `french_diacritics.pdf`, which was filed under page rounding. Its raster
really is 595x842 against `poppler`'s and `mupdf`'s 596; that was true and was not the
disagreement. **Four for four now** on a group's name failing to diagnose one of its members.
All three departures are fixes, and all three stayed in the comparison, which is the cleanest
shape this count comes in.

**104 → 103 in the fifteenth session, and the departure was fixed and then left anyway.**
`alphatrans.pdf` had been contradicted by all three references since the oracle existed, filed
under substituted fonts because its labels use a font nobody embedded; what differed was its
gradient, painted opaque because a shading dropped §11.6.4.4's alpha constant. It agrees now —
and it is no longer in the comparison either, because the same session gave it a §11.6.2
report. Both halves of that are worth carrying: the group a page is filed under is a hypothesis
(three for three now), and a report is paid for in gated pages even when the page is right.

**103 → 104 in the fourteenth session, and the one arrival is the page-rounding group's
third.** `colorkeymask.pdf` became comparable when colour key masking landed; its raster is
595 wide against `poppler`'s and `mupdf`'s 596, and the diff heatmap is three vertical lines
one pixel wide at the three band edges. The masking itself agrees. Read that as the pattern
this file keeps describing rather than as a regression: a feature that makes a page drawable
adds it to the set being judged, and a page holding two coloured bands has nothing else on it
for a one-pixel edge to be averaged against.

**The session before it was 103 → 103, with sixteen pages added to the denominator and none of
them contradicted.**
That is the thirteenth session, and it is unusual enough to be worth stating: every feature
since the ninth has put at least one newly-drawable page on this list. Eleven of the sixteen
agree with the reference consensus outright. What it means is that these pages were not
*nearly* right before — they were a solid bar where a word belongs, with nothing else on them
to be wrong about.

**The session before it went 108 → 103, and the arithmetic was six out and one in.** Six left
by being *fixed*, and all six were `/Rotate 90` pages drawn 180° out — five of
`hello_world_rotated.pdf` and `issue6019.pdf`, every one of them filed under substituted fonts
because they also carry one. That is the caution above arriving in quantity: **which group a
page is in is a hypothesis, and six of the twenty-five in the largest group had nothing to do
with fonts.** The one that arrived is `bug1001080.pdf`, which joined the comparison when
`CCITTFaxDecode` landed and is about image reduction rather than about fax.

**Read the 48% ambiguous with care.** It is not "half the corpus is unsettled": 372 of those
720 pages are two long books, `freeculture.pdf` (320 pages) and `pdkids.pdf`, whose text uses
fonts nobody embedded, so each renderer substitutes a different one and the structural bound
separates them. Ambiguity concentrated in a handful of documents says more about those
documents than about the gate.

**So read the 720 as "reported nothing", not "drew it right".**

### By clause

ISO 32000-2 carries 823 subclauses under its eight technical clauses, and counting them is a
poor proxy for work: clause 12 is 166 of annotation subtypes a viewer adds one at a time,
while clause 8's 128 decide whether any page looks right at all.

**Every entry in the table below is a judgement about state, not a measurement**, and the
thing that turns one into a measurement is the ledger's `status` column. 229 of the 823
subclauses now carry one, 81 of those being clause 13's exclusion — so read this table as
belief, and the ledger as what has been checked. Where the two disagree, the ledger is the
one that had to name a code site.

| Clause | Subclauses | State |
|---|---|---|
| 7 Syntax | 138 | **Nearly complete**, and 4 of its 138 rows are now reviewed. Objects, **every standard filter but `LZWDecode`** — JBIG2 and JPEG 2000 in the seventh session, `CCITTFaxDecode` in the twelfth (§7.4.6, ADR 0021) — classic and stream xrefs, object streams, incremental updates, recovery by scanning. **Encryption is absent** and is the largest hole here. |
| 8 Graphics | 128 | **Nearly complete**, and the clause with the most ledger coverage: 52 of its 128 rows are reviewed, §8.9, §8.10 and — since the nineteenth session — §8.6.6 with §8.6.7 as families. Paths, clipping, all eleven colour space families, all seven shading types, both pattern types, form and image XObjects, inline images (§8.9.7, eleventh session), `/Interpolate`, an image's `/Mask` in both forms (§8.9.6, fourteenth session), ICC colour management, and — since the ninth session — optional content (§8.11) wherever it decides what is drawn. A form is clipped by its `/BBox` as of the seventeenth session (§8.10.1), which is the one requirement of that family this tree had missed. A general `/Decode` array is still not applied and not reported, and 2, 4 and 16 bits per component are refused. §8.6.6.4's `/All` and `/None` colourants landed in the nineteenth session, and §8.6.7's overprint control is `inapplicable` on a device with no separations, which the clause states rather than excuses (ADR 0028). |
| 9 Text | 65 | **Partial**, and 38 of its 65 rows are reviewed — §9.3 and §9.4 as two whole families in the thirteenth session, the whole of §9.7 in the twentieth. Simple and composite fonts through embedded TrueType, CFF and OpenType programs; the standard 14 by substitution; `/ToUnicode`; Type 3 fonts, whose glyphs are content streams (§9.6.4, ADR 0018); and all eight text rendering modes (§9.3.6, ADR 0022). §9.6.5.2's CFF encoding algorithm and §9.6.5.4's `TrueType` one are both implemented in full, the second as of the eighth session (ADR 0015). **§9.7's composite fonts are two mappings and both are read as of the twentieth session** (ADR 0029): an embedded `CMap` stream decides a code's length and its CID, byte by byte against the codespace ranges, with §9.7.6.3's recovery for an invalid code; a CID reaches a glyph through a CID-keyed CFF's charset or a `/CIDToGIDMap` stream. Missing: bare Type1 (`/FontFile`), Table 116's predefined `CMap`s — which are data with a licence rather than an algorithm — vertical writing mode, and text knockout (§9.3.8), which since the fourteenth session is `reported` rather than `silent`. |
| 10 Rendering | 36 | **Partial**, and 6 of its 36 rows are reviewed — the whole of §10.7 in the sixteenth session, which is the first time this tree cited the clause at all. Colour management and rendering intents are done. Halftones and transfer functions describe a marking device rather than a screen. **Flatness turned out not to belong on that list**: §10.7.2 makes ignoring it an explicit permission, which is a different and better answer than "inapplicable". §10.7.4 is `partial` with three deliberate departures named — anti-aliasing twice over, and ADR 0025's area averaging — and §10.7.5 is `partial` since the nineteenth session: its single-pixel rule is implemented and its grid-fitting rule is a fourth departure in the same family (ADR 0028). |
| 11 Transparency | 58 | **Partial**, and 46 of its 58 rows are reviewed — everything from §11.4 onwards, leaving only §11.1 to §11.3.5 and §11.3.8, which are the model rather than its PDF representation. §11.6.4 in the fourteenth session, §11.3.7, §11.5 and the rest of §11.6 in the fifteenth, the whole of §11.4 in the seventeenth, §11.7 in the eighteenth. All sixteen blend modes are implemented and reach both backends, including §11.6.3's rule for choosing among an array of names; `ca` and `CA` are two constants that reach a shading as well as a colour; an image's `/SMask` supplies its alpha at any resolution, with `/Matte` undone (§11.6.5.2, ADR 0024); a `/Group` is composited as one object, with the page itself treated as the isolated group §11.4.7 says it is (ADR 0026); and a graphics-state `/SMask` is a group evaluated for its alpha or its luminosity, with `/BC` and `/TR` (§11.5, ADR 0027). What is left is knockout (§11.4.6), a non-isolated group whose elements blend (§11.4.4) and a blending colour space that is not the device's, in a group or in a mask — all four reported. **Overprinting (§11.7.4) was six `silent` rows and is not a gap**: Table 146's blend function is the source colour for every row a three-process-component group space with no spot colourants can reach, which is Normal (ADR 0028); its one unmet requirement, §11.7.4.4's implicit group around a combined fill and stroke, is §11.6.2's already-reported gap. `/AIS` is read nowhere and is argued for in ADR 0027: with one alpha per pixel, shape and opacity multiply to the same number. |
| 12 Interactive features | 166 | **Appearances only.** Annotations are placed and drawn from `/AP` (§12.5.5), with the visibility flags of §12.5.3 honoured. Nothing is synthesised, and no forms, actions or navigation exist. |
| 13 Multimedia | 81 | **Excluded**, by name, on principle 5's closed list: a media engine rather than a rendering question. Its rows still appear in the ledger carrying that exclusion, because an invisible exclusion is indistinguishable from an oversight. |
| 14 Document interchange | 152 | **Output intents only.** No tagged PDF, no metadata, no marked-content semantics — `BDC`/`EMC` are parsed and ignored. |

So: the parts of the standard that decide whether a page is drawn correctly are largely
done; the parts that make a document *interactive* are not started.

### Feature-by-feature, from the source

| | |
|---|---|
| Content-stream operators | **73 of 73** in Table 50 (`ID`/`EI` are consumed inside the `BI` handler rather than as arms). `d0` and `d1` landed with Type 3 fonts and were the last two. `BMC`/`EMC` maintain the optional-content stack; `MP`/`DP`/`BX`/`EX`/`i` are matched and deliberately ignored. `BI` draws its image as of the eleventh session; before it, the row was the standing example that an operator having an arm is not the same as its being implemented. |
| Filters | **9 of 10** standard filters decode: `ASCIIHex`, `ASCII85`, `Flate`, `RunLength`, `Crypt` (pass-through), plus `DCTDecode`, `JBIG2Decode`, `JPXDecode` and — since the twelfth session — **`CCITTFaxDecode`**. `LZWDecode` is the only one **absent**, and no corpus first page reaches one. Table 92's abbreviations are expanded in `inline_image.rs`, so the filter layer sees full names; `/CCF` reaches `image.rs` as `CCITTFaxDecode`. |
| Colour spaces | **11 of 11** families, and the three CIE-based ones are converted rather than approximated. |
| Function types | **4 of 4** (sampled, exponential, stitching, `PostScript` calculator). |
| Shading types | **7 of 7**, on both backends. |
| Pattern types | **2 of 2** (tiling and shading). |
| Blend modes | **16 of 16**. |
| Font programs | TrueType, CFF, CFF-in-OpenType, CID-keyed CFF, and Type 3 — whose glyphs are content streams and are run by `pdf-model` rather than read by `pdf-font`. Bare Type1 is reported. (This row claimed Type 3 was reported for two sessions in which it was not — the corpus is the only thing that checks this file.) |
| Composite fonts (§9.7) | **Both of the clause's mappings**, from the twentieth session (ADR 0029). Codes to CIDs: the two Identity `CMap`s, and any embedded `CMap` stream — codespace ranges matched byte by byte and deciding a code's length from 1 to 4, `cidrange`, `cidchar`, `notdefrange`, `notdefchar`, `bfchar`, `/WMode`, `usecmap` and Table 118's `/UseCMap`, with §9.7.6.3's recovery for a code the codespace excludes. CIDs to glyph indices: a CID-keyed CFF's charset, a `/CIDToGIDMap` stream, or the identity, chosen by what the embedded program is rather than by `/Subtype`. `/W` and `/DW` are indexed by CID. Table 116's other predefined names are refused and reported, because they are registered data files this tree does not carry. |
| Annotation appearances | Placed and drawn; not synthesised where absent. |
| Line dash patterns | §8.4.3.6, from the tenth session. Before it the `d` operator set nothing and every dashed line in every document was drawn solid. |
| Text rendering modes | **8 of 8** in §9.3.6 Table 104, from the thirteenth session (ADR 0022): fill, stroke in user space, both per glyph, invisible, and the four that add the glyphs to the clipping path at `ET`. An operand outside 0..7 is reported. |
| Text state parameters | 8 of Table 102's 9. Missing: `Tk`, text knockout (§9.3.8) — read from `/TK` and *reported* where it can show since the fourteenth session, and a corner of the transparency gap. |
| Word spacing (§9.3.3) | A property of the *code's encoded length*, not of the font, since the twentieth session: an embedded `CMap` may define codes of several lengths in one font and four of the corpus's do. Before that it was answered per font, which was exact only because every mapping this crate built was wholly one-byte or wholly two-byte. |
| Optional content | §8.11 wherever it decides what is drawn: configuration, membership, `/VE`, intent, and all three places `/OC` can appear. The interactive half — `/Usage`, `/AS`, `/Order` — is not read. |
| Inline images | §8.9.7 in full, from the eleventh session: both abbreviation tables, the resource-named colour space, and three ways of finding where the data ends. ADR 0019. |
| Image colour spaces | All eleven families unpack, `Indexed` through a table converted once per entry rather than once per sample. Bit depths 1 and 8; 2, 4 and 16 are refused and reported. |
| Image masking | All four of the mechanisms an image can carry, plus the graphics state's own: the image's `/ImageMask` stencil, an explicit `/Mask` (§8.9.6.3) and a colour-key `/Mask` (§8.9.6.4) from the fourteenth session (ADR 0023), a soft-mask image at any resolution with `/Matte` undone from the fifteenth (ADR 0024), and §11.5's soft mask in an `/ExtGState` from the eighteenth (ADR 0027). Two rasters of different sizes are combined on the finer of the two grids, bounded on the growth; a graphics-state mask is combined at *device* resolution, which is what the clause actually describes. §11.6.4.3's precedence decides which wins where an image carries both. |
| Transparency groups | §11.6.6's `/Group`, from the seventeenth session (ADR 0026): the elements composited onto a transparent backdrop and the result painted once, with §11.6.6's reset of the blend mode and both alpha constants inside. §11.4.7's page group too, which is why the page is drawn onto transparency and imposed on the medium afterwards. Knockout, a non-isolated group that blends, and a group colour space that is not the device's are reported. |
| Blend modes and constants | All sixteen names of Tables 134 and 135, and §11.6.3's rule for an array of them — the first name the reader *recognises*, which is not the first name. `ca` and `CA` reach solid colours, images and, since the fifteenth session, shadings (§11.6.4.4). |
| Page rotation | §7.7.3.3 Table 31's `/Rotate`, clockwise as displayed, from the twelfth session. Before it 90 and 270 were exchanged and every rotated page was drawn 180° out. |
| Image resampling | Magnification is §8.9.5.3's `/Interpolate`, from the eleventh session. Reduction is §10.7.4's, from the sixteenth (ADR 0025) — and is the one place this tree knowingly does what a clause forbids: blocks of samples sharing a device pixel are averaged, where the clause requires point sampling. Both decisions live in `pdf-render` so the two backends cannot make them differently. |
| Scan conversion (§10.7) | **Four** deliberate departures. Three were named in the sixteenth session: anti-aliasing violates §10.7.4's "painting any pixel whose half-open square region intersects the shape" and its area rule, and area averaging violates its image rule. The fourth is §10.7.5's grid-fitting, from the nineteenth (ADR 0028), and it is the one anti-aliasing makes unnecessary rather than merely different. §10.7.1's NOTE — "the specifics of the scan conversion algorithm are not defined as part of PDF" — licenses all four. `/FL` is ignored by the clause's own permission; `/SM` is read nowhere. `/SA` **is** read, and its single-pixel rule is implemented. |
| Line width (§8.4.3.2) | A zero width is one device pixel on both backends as of the nineteenth session. It had been `tiny-skia`'s hairline convention rather than a stated rule, so the GPU drew nothing at all for a `0 w` line — including a whole line of stroked text on `zerowidthline.pdf`. `Stroke::device_width` in `pdf-render` is where the rule lives, with §10.7.5's, because the clause's own NOTE makes them the same width. |
| Overprint control (§8.6.7, §11.7.4) | Ignored, and the clause says to: a device that does not support overprinting "shall ignore" the parameter, and the overprint mode "shall not apply if the native colour space of the output device does not include CMYK device colourants". §11.7.4's Table 146 reaches the same answer independently on a group space of three process components with no spot colourants. ADR 0028. |
| Special colourants (§8.6.6.4) | `/All` and `/None`, from the nineteenth session: `/None` marks nothing, `/All` is the tint complemented in every colourant. Both ignore the alternate space and the tint transform, as the clause requires, and are decided before either is parsed. |

## What to do next

**Two tracks now, and the discipline is to take from both in every session.**

*Demand-driven* is everything the corpus and the oracle name — 96 contradicted pages, 60 of
them unexplained, and a feature list sized by how many documents want each item. It has been
productive for thirteen sessions, it is where the low-hanging fruit is, and it stays — with the
nineteenth session's caution attached: **a count on that list sizes a key, not a difference.**
Overprinting sat at the top of it with 63 documents and turned out to change no pixel this
device can produce.

*Spec-driven* is what the ledger and §6.3.2.2's ranking name — coverage against the
specification rather than against a file set. It exists now, and it has a number: **594 of
823 subclauses are `unreviewed`**. A project running only the first track finishes when the
corpus goes quiet, which can happen with a great deal of the standard unimplemented and
nothing anywhere able to say which parts.

Taking from both is a `CLAUDE.md` principle-5 rule now, not a suggestion in this file. In
practice: **one item from each track per session**, and the spec-driven item is usually the
smaller of the two, because reviewing a clause family against code that already exists is
cheaper than writing a feature. The ninth session did §8.11 as both at once — it was first by
clause 6 and seventh by corpus count — and that is the ideal shape when it is available.

Sessions ten to fifteen took the other good shape, and it is cheaper to arrange: take the
demand item, then review the clause family the code you just wrote *cites*. Type 3 fonts made
§9.6.4, §9.6.5 and §8.6.8 the obvious families; inline images made the whole of §8.9, and
reading it produced four defects that had nothing to do with inline images and one wrong
citation that named a real clause; `CCITTFaxDecode` made §7.4.6, and reading it decided two
refusals and one documented choice before a line was written; the text rendering modes made
§9.3 and §9.4, and reading those produced a defect in word spacing that no page of Latin text
could have shown and the ledger's third `silent` row; `/Mask` made §11.6.4, and reading it
produced a precedence rule — an image's `/SMask` supersedes its `/Mask` — that neither §8.9.6
nor Table 87 states and that the implementation had just got wrong; and soft-mask images made
§11.3.7, §11.5 and the rest of §11.6, whose seventeen rows produced three defects the demand
item could not have reached, one of them the four-session-old contradiction on
`alphatrans.pdf`; image resampling made §10.7, where the sixteenth session found that the
clause governing its demand item **forbids what the demand item asked for**; and transparency
groups made §8.10 and §11.4, where the seventeenth found two defects the demand item could not
have reached — a form XObject that paints outside its own `/BBox`, and a page group that is
*isolated*, so every blend mode in the corpus had been composited against a white backdrop the
standard says is not there; and soft masks made §11.7, where the eighteenth found the
overprinting silence and one clause — §11.7.3's rule that a spot colour is never available
inside a mask group — that a decision taken years earlier for another reason already
satisfies; and the nineteenth session took the *demand* item from the ledger's own silence list
and read §8.6.6 with §8.6.7 because overprinting is stated in the colourants a `Separation`
names — which found `/All` and `/None` unimplemented, and found that the demand item itself was
not a gap; and the twentieth session's two items were the *same clause family*, because the
largest thing on the demand list was composite fonts and §9.7 is where they are defined, which is
the ninth session's ideal shape and only the second time it has been available.

**The ninth pairing is the newest shape and the most useful one to know about.** The family
review did not correct the demand item; it *dissolved* it. Overprinting was 63 documents on the
demand list and six `silent` rows on the spec list, and Table 146 read against a list of this
device's colourants says the special blend function is Normal here. **A gap sized by a corpus is
a hypothesis about a clause**, and the only instrument that can test it is the clause.

**The seventh was the strongest argument yet for the pairing, and it is a new shape.** The
first six times, reading the family found something the demand item had *missed*. That one
found that the standard says the opposite of what was about to be built — which did not stop
the work, because §10.7.1 licenses the departure and the page is unreadable without it, but it
turned an obvious improvement into a documented choice with three parts and corrected two
places in the tree that claimed the standard was silent. **Read the family before writing the
feature, not only after**: the cost is an hour and the alternative is a defensible improvement
with no idea that it is a departure.

**A third thing is worth taking from the twelfth and thirteenth sessions, and it is not on
either track: the instrument.** 95% of the oracle's cost was three other programs answering a
question they had already answered, and nobody had looked because 85 seconds is not obviously
wrong. The thirteenth found the citation checker blind to table numbers, and one wrong. The
tree was also not `clippy` clean while this file said it was. **Whatever this file asserts
about the tooling, run it once before believing it.**

The one-line version of the demand track: **98 pages we claim to draw are contradicted, 60 of
them for no reason visible on the page. The largest gap of any kind is now synthesised annotation
appearances at 63 documents**, and it has been second for six sessions because text was first;
with §9.7 built (ADR 0029) it is first, and the corpus gate's annotation row is by one document
the largest row it has. The second is *encryption* at 20. What is left of text is not a clause:
12 documents name one of Table 116's predefined `CMap`s, which is data with a licence attached,
and 42 have a font nobody embedded whose substitute cannot be addressed. The one-line version of
the spec track: **22 clauses the code already cites have never been read against it**, and they
are named in `REVIEW_OWED`. Neither list has a rendering feature left that any corpus document
announces.

### 0. The ledger, and the cheapest reviews available

The machinery is built (ADR 0016). What it needs now is use, and the first rows to fill are
the ones the code already points at.

- **Work `REVIEW_OWED` down.** 22 clauses, each already cited by the code that implements it,
  so the reading is against something that exists rather than against a blank. Take them by
  family — §8.6.5 is five of them, §12.5 another five — because that is how the standard
  distributes its requirements, and because §9.6.5.4 was missed for the opposite reason:
  nobody had read §9.6.5 as a unit. **Expect findings**: fifteen families have now been
  reviewed and they have produced thirty-nine, five in §11.4 and §8.10 — including the page
  group, which changed how every blend mode in the corpus composites — five in §9.7, including
  two subclauses that contradict each other and one table whose presence condition had been read
  as a condition on its meaning, four in §8.9, four in
  §8.6.6 and §8.6.7, including the two special colourants and the finding that overprinting is
  not a gap here, three in §11.6, including the gradient defect that had made a page
  contradicted for four sessions, three in §10.7, which are *departures* rather than defects,
  three in §11.7, three in one clause (§8.6.8) that had looked like a formality, two in §7.4.6
  that turned into refusals rather than code, and two in §9.3.
- **Prefer the family belonging to whatever else the session is doing.** §7.4.6, §8.6.4.2,
  §8.6.6, §8.6.7, §8.6.8, §8.9 (all of it), §8.10, §9.3, §9.4, §9.6.4, §9.6.5, §9.7, §10.7,
  §11.3.7, §11.4, §11.5, §11.6 and §11.7 are done — **the whole of clause 11 has now been read** —
  so the families left are elsewhere: §12.5.6 if synthesised appearances are the demand item, §7.6
  if encryption is, §9.6.2 and §9.6.3 whenever a simple font's encoding is touched (which
  `issue20232.pdf` now asks for), §8.6.5 (five rows) whenever CIE-based colour is
  touched, §8.4.3 (four rows left) whenever a stroke is. Record every row, including the ones
  that turn out to be `inapplicable` — a clause read and dismissed is worth as much as one
  implemented, and costs a minute.
- **One `silent` row is left**, and it is §8.11.4.4, usage dictionaries: a layer that should
  switch itself off by zoom, language or print state is drawn with nothing said. It is last of
  the three silences on purpose — it needs a layer panel to be worth more than a report — and
  it is the only one where a report is still the cheapest honest move. The other two closed in
  the nineteenth session and closed *differently from each other*, which is the thing to carry:
  §10.7.5's `/SA` was implemented in the half a display can state and recorded as a departure in
  the half it cannot, and §11.7.4's overprinting was six rows that a reading of Table 146
  removed altogether. **A silence is not the same as a gap. It is the absence of an answer, and
  reading the clause may supply one in either direction.**

  §9.3.8, closed in the fourteenth session, is still the right thing to read before designing any
  report — the cost turned out to be not the key lookup this file predicted but the *precision*,
  because `Tk`'s initial value is the unimplemented one and a report has to name the pages where
  that can show rather than every page there is; the seventeenth session's three group reports
  were designed against that lesson and each names between 4 and 9 documents. **Two silences
  still hide *inside* `partial` rows** — §8.9.5.2's general `/Decode` array and §10.7.3's `/SM`
  — which is worth remembering when reading the ledger by status: a clause can be half
  implemented and quiet about the other half.

Five small items, listed before the big lists because they are small. The first is the
nineteenth session's leftover, the second the seventeenth's; the other three have been carried
since the seventh:

- **Give §8.11.4.4's usage dictionaries a condition, and then a report.** The last `silent` row.
  `/Usage` and the `/AS` usage application dictionaries switch an optional content group by
  zoom, language, print state or user, and none of them is read, so a layer that should be off
  is drawn. Trap 11's method applies unchanged: an `eprintln!` naming the documents that carry
  an `/AS` array *and* a group whose `/Usage` would turn it off at the resolution we draw, before
  any condition and long before any code.
- **Bound a group's buffer to the band its clip admits.** The CPU backend gives every
  transparency group a page-sized pixmap, because a group's elements resolve their clips against
  the *target* and a band-sized buffer would need every one of them shifted. No corpus page pays
  for it — the gate's timing is unchanged — but a page with hundreds of groups would, and the
  fix is one coordinate system rather than a new idea. Measure before building it:
  `callgrind_rasterise` over a group-heavy page is the instrument, and the sixteenth session's
  lesson about a benchmark that measures nothing applies.
- **Sandbox the interpreter and rasteriser too.** Spike D exists and is exercised; the rest
  of the renderer still runs in the main process, which is the half of principle 3 that is
  not yet built. The protocol would have to carry a display list rather than an image, which
  is a real design question and the reason it has not been a footnote to any session so far.
- **Profile the median page.** We are 1.66× slower than `hayro` on the typical corpus page
  and nobody has looked at why — the seventh session's two fixes were both to outliers and
  moved the median not at all. The typical page is small and text-heavy, so the candidates are
  parsing, font loading and per-page setup rather than rasterisation, but that is a guess and
  the handover's own habit says profile before believing an explanation.
  `cargo run --release -p hayro-compare --bin hayro-speed` is the measurement; `callgrind`
  over one median-sized document is the diagnosis.
- **Give the JPEG 2000 decoder a target resolution.** One corpus document is refused for
  being 212 megapixels, and the format's own answer is to decode a lower resolution level.
  It needs the scale a page is about to be drawn at to reach `image.rs`, which the display
  list deliberately does not carry — so this is a question about where decoding belongs, not
  a parameter to thread.

### 1. Work the unexplained list

`CONTRADICTED_UNEXPLAINED` in `oracle.rs`: 60 pages carrying no undrawn annotation, no hidden
optional content and no substituted font, so the difference is in something we believe we
implement. One cause is identified and live — and **read trap 9 before starting**, because
an entry may be either of its shapes: two references that are one implementation, or two that
have both skipped the same clause. The second was found in the ninth session and is the more
common one; checking it costs a web search of the other project's source.

- **`mesh_shading_empty.pdf` draws the same mesh displaced horizontally** — a placement
  question, and the class of defect trap 2 is about.

**Five entries left in the seventeenth session and only one was fixed.** The four
`knockout_*.pdf` this list had diagnosed are now *reported* (§11.4.6) rather than drawn right,
so they left the comparison rather than the defect. The fifth, `issue11279.pdf`, is the
argument for the list: it looked like one page and was §8.10.1 step c) — a form XObject's
`/BBox` clipping nothing, on every form in every document since the first one.

Three entries that used to be here are the argument for spending the hour, because none was
one page's problem. `issue20504.pdf` was worth **15 of the 81**: it looked like one page's
`/Differences` quirk and was a whole subclause (ADR 0015). `close-path-bug.pdf` looked like
one page's closed path and was **every dashed line in every document** — the `d` operator set
nothing at all, and both backends had been able to dash from the start. The only way to find
out which kind an entry is, is to open the artefact.

**One entry arrived in the eighteenth session and it is a page that became comparable rather
than a page that changed.** `issue7891_bc1.pdf` reported a soft mask until §11.5 landed; what
is left is a mean error of 0.22 with a worst tile of 10.76 against a bound of 6.04, all of it
in the edge coverage of six-pixel glyphs. It is listed rather than diagnosed, which is what
this group is for.

The other 58 are unexamined. Each is a page where two implementations sharing no code agree
and we differ by more than twice their own disagreement, with the artefacts already written:
`<target>/tmp/oracle/<stem>/p<n>/` holds our render, each reference's, a side-by-side and a
difference heatmap. **Look at the side-by-side first.**

Two cautions. A page may be contradicted for a reason other than the one its group names —
`calgray.pdf` sat under substituted fonts and differed in its colour, which is how the whole
of ADR 0012 started. And principle 5 is not suspended by a list: each entry is a question to
take to the specification, and "make it match mupdf" is exactly the failure this project
forbids.

### 2. The last silence, and what the other two taught on the way out

**This heading held soft masks for two sessions, then three silences for one, and both of the
first two closed in the nineteenth session.** What is left is **usage dictionaries
(§8.11.4.4)**, which is last on purpose: a layer that switches itself off by zoom or print state
is drawn with nothing said, and a report is worth having, but the requirement is not finished
until there is a layer panel. Its method is trap 11's, unchanged — an `eprintln!` that prints
what the condition would match, then the condition, then the report.

The reason the heading stays is the argument, not the list. A gap that reports gets scheduled and
a gap that is silent does not, so silences outlive every feature around them; §11.7.4 was found
only because somebody read a clause nothing in the tree cited. What the nineteenth session adds
is that **the two ways out are not the same, and neither is "write the report"**:

- **§10.7.5 split.** One of its two requirements was implementable exactly and is implemented;
  the other was already satisfied in effect by a departure this tree takes for another reason,
  and became a documented departure rather than work owed. A clause is not one requirement.
- **§11.7.4 evaporated.** Six rows, 63 documents, and Table 146 read against a list of this
  device's colourants says the special blend function is Normal here. The silence was real —
  nobody had read the table — and the gap was not.

So the first move on a silence is neither a report nor a feature: it is to work out what the
clause asks *of this device*. Both of these took an hour of reading and one of them saved the
week of work its corpus count implied.

### 3. Image reduction — done in the sixteenth session, and what is left of it

The item this heading held for four sessions landed as ADR 0025 and `CONTRADICTED_IMAGE_RESAMPLING`
is empty. Two pieces of it are still open, and both are the *same* piece:

- **Reduction happens at decode resolution, not at device resolution.** §10.7.4's answer is
  per-device-pixel; `Image::area_averaged` works in whole source samples and leaves a residual
  under two-to-one to the backends' own filters. A good approximation, not the thing itself.
- **A mask combined with its image is bounded rather than composited at device resolution**,
  which ADR 0024 named and `issue16263.pdf` still trips.

Both need the display list to carry an image and its sampling intent to the backends rather
than a finished raster, which is one `pdf-render` change and belongs to whoever takes it. The
same change is what "give the JPEG 2000 decoder a target resolution" needs, so three items on
this file's small list are one design question about where decoding and resampling belong.

### 4. The two gaps that draw a page visibly wrong

This list has emptied faster than it has filled. The text clipping modes headed it and landed
in the thirteenth session (§9.3.6, ADR 0022); image `/Mask` headed it after them and landed in
the fourteenth (§8.9.6, ADR 0023); a soft mask of another size headed it after *them* and
landed in the fifteenth (§11.6.5.2, ADR 0024). All three are worth noting because the estimates
written here were right about the mechanism — the `/Mask` entry said the colour-key form "must
be applied to the *source* samples before colour conversion, which is why it is not a two-line
change", and the soft-mask entry said the fourteenth session's grid choice "would close most of
this row", which it did, on 13 of 14 reports. **Writing an item down before taking it is what
makes that possible**, and the estimates have been good enough to trust.

`/Group` headed it after *them* and landed in the seventeenth (§11.6.6, ADR 0026) — and that
entry's estimate was wrong in the useful direction. It said "making it report is a key lookup
and a condition"; what reading the clause found was that the *page* is a group too, so the
work was a feature and the finding was §11.4.7. **Two below are silent** — neither reports
anything today, which is what puts them here rather than on a list of known reported gaps:

- **A general `/Decode` array** (§8.9.5.2). Only the
  fully-inverted form `[1 0]` is applied; any other linear map is ignored **and not
  reported**. The ledger records it inside a `partial` row rather than as a `silent` one,
  because the clause's defaults *are* implemented — which is a limit of a one-word status
  worth knowing about when hunting for silence. The formula is two multiplications per
  component; the reason it has not been written is that the device fast paths unpack `u8`
  without touching floating point, so applying it everywhere would cost the hot loop. A
  `Decode` that is neither the default nor the inversion is rare enough that reporting it
  would be a good first move.
- **`/UserUnit`** (§7.7.3.3), which scales the page and is neither applied nor reported.
  2 corpus documents, and the only reason it matters more than that count suggests is that
  getting a page's *size* wrong invalidates every comparison on it.

### 5. Synthesised annotation appearances, if the corpus count is the argument

63 documents carry an annotation with no `/AP`, second only to the 100 that report a font.
It is genuinely a different drawing routine per subtype and should not be started as one
task. If it is taken, take it one subtype at a time in corpus order: `Widget` (26), `Link`
(18, and its whole appearance is a border — §12.5.6.5 with §12.5.4), then the markup
annotations. Each one that lands should be measured on the oracle rather than assumed to
help, because a synthesised appearance is a *guess at what the producer meant* and the
references guess differently.

### 6. Then, by what the corpus says real documents need

**Encryption** (20 documents — 11 cannot reach page one, 9 more draw a blank page and now
say so) is the only one of these that is still code. **CID encodings are done** (100 fonts before
the twentieth session, 67 now, ADR 0029) and what is left of them is not: 15 fonts name one of
Table 116's registered `CMap` files, which is a decision about vendoring third-party data and its
licence, and 4 want vertical writing, which is §9.2.4's `/W2` metrics rather than §9.7.
**Type1 fonts** are smaller than they look: no corpus page one reaches one.

The soft masks this paragraph used to lead with are built (ADR 0027), and `doc/`'s fourteen
specification PDFs now report **nothing at all**. What is left here announces itself,
which is why it sits below the ones above: a gap that reports is a gap you can measure and
schedule, and a gap that does not is a gap that ships — which is item 2's whole argument.

### Where the time went, and where it still goes

**There is one fair thing to measure against.** Every other renderer here is C, so a timing
difference against `poppler` confounds the language, the allocator and thirty years of tuning.
`hayro` is Rust, forbids unsafe as we do, and rasterises on the CPU single-threaded as we do.
`cargo run --release -p hayro-compare --bin hayro-speed -- <files>` renders page one of each
file with both, alternating, best of N passes.

Over the corpus pages we claim to draw completely:

| | |
|---|---|
| total, ours | **5.4 s** against `hayro`'s 40.1 s, over 685 pages |
| **median page** | **1.66× slower** |
| worst page | 31× (was 34×, was 225×) |

**The totals and the median answer different questions, and only quoting both is honest.** In
aggregate we are 7.4× faster because their distribution has a long tail and ours no longer
does. On the median page we are still 1.66× slower and **that number has never moved** — it
was 1.61×, then 1.62×, and the eleventh session's image work moved it by less than the noise
between runs. The thirteenth session did not re-run `hayro-speed`; what it did measure is that
the text rendering modes cost **+0.46% of interpretation instructions** (1.912 G → 1.921 G by
callgrind on `examples/callgrind_interpret.rs`), against a `cargo bench` that claimed 8% on one
case and was wrong. The fourteenth measured the same example at **1.923 G** — the masking work
adds one `Option` check per sample to a loop that had none, and it costs +0.12%, which is below
the run-to-run noise of ordinary code motion; the corpus gate is unchanged at 1.4 s. The
fifteenth measured **1.924 G** — +0.05%, which is nothing: its per-path work is two comparisons
before a report and its per-sample work only runs on an image that carries a mask.

**The sixteenth session's change is invisible to that example, and that is the point.**
`callgrind_interpret.rs` stops at the display list, so a change to how a backend *draws* a
command measures as exactly zero there — which is why
`crates/pdf-model/examples/callgrind_rasterise.rs` now exists. Its numbers for area averaging,
twenty rasterisations apiece:

| page | before | after | |
|---|---|---|---|
| ISO 32000-2 p101, no reduced image | 14.0726 G | 14.0726 G | free |
| `bug1001080.pdf`, many 8x glyph bitmaps | 338.96 M | 330.91 M | **−2.4%** |
| `firefox_logo.pdf`, one 5x logo | 515.09 M | 540.34 M | +4.9% |
| `issue19971.pdf`, one 5x 2500x1364 photograph | 3.9264 G | 4.2793 G | +9.0% |

A page of many small reduced bitmaps got *cheaper*, because the premultiply pass and the
pattern allocation now run over the reduced grid rather than the source one; a page that is one
large photograph pays 9%; the corpus gate is unchanged at 1.6–1.8 s, so the aggregate is below
what it can measure. **Saturating arithmetic was 8 of the original 17 points** on the worst
page, and plain arithmetic under an `#[expect]` naming a provable bound halved the cost — the
seventh session's per-pixel-loop lesson arriving in a new loop.

The seventh session's two fixes were both to outliers. **The typical corpus page
is small and text-heavy and has never been profiled**, which is the next measurement rather
than the next optimisation.

Two fixes are worth carrying as patterns rather than as history. Unpacking JPEG output was
6.89 G instructions on one page — nearly twice what `zune-jpeg` spent decoding it — because a
`match`, three bounds-checked `get`s, saturating arithmetic and a re-checked `extend_from_slice`
all ran *per pixel*; two paired `chunks_exact` iterators took it to 1.25 G. **The safety
habits this project enforces everywhere are expensive in a loop that runs per pixel, and that
is exactly where the profile should be consulted rather than the habit.** And a mesh triangle
was subdivided by colour alone, so one covering a tenth of a pixel still split into 4096
filled pieces; `Triangle::is_subpixel` is a correctness statement rather than a trade — a
triangle smaller than a sample cannot display a gradient — and it took `personwithdog.pdf`
from 17.3 s to 1.06 s **while moving every mesh page closer to the references**. A change made
for speed that improves fidelity means the old code was doing work that was worse than
useless.

**Still open, and the largest items.** This profile predates both fixes and its shading half
is still live:

| on `bug1721218_reduced.pdf`, 16.1 G instructions | share |
|---|---|
| `tiny_skia::pipeline::lowp::gradient` | 29.7% |
| `pdf_model::function::Function::parse` | 23.2% |
| `pdf_model::function::Function::eval` | 13.8% |
| `ColourSpace::to_rgb_at` | 2.6% |

**The gradient stage** is the largest single item because a `Ramp` carries 256 samples, so a
shading becomes a 256-stop gradient and `tiny-skia` scans its stops per pixel batch; handing
the *rasteriser* fewer stops would fix it, while coarsening the `Ramp` in the display list
would lose fidelity and is not the same thing. **Roughly 40% of that run is building the
shadings** rather than drawing them: a function is parsed and then sampled 256 times per
shading, and that page has 3576 of them. Whether that is 3576 *distinct* functions or one
re-parsed 3576 times has never been checked, and it decides whether the fix is memoisation by
object reference or something harder — check before designing.

One caution: `to_rgb_at` was 2.6% when `CalGray` was a pass-through. It now runs a Bradford
adaptation and a matrix per colour, and per *sample* for a Cal-space image.

### Reproducing the numbers above

The oracle survey is `oracle.rs` and the corpus counts are `corpus.rs`; both print their
evidence per document. The ledger's counts come from `cargo test -p conformance -- --nocapture`.

Two classification counts are still throwaway, and deliberately so — scratch-quality
diagnostics do not belong in a repository held to `clippy::pedantic`. **Whether a page's fonts
are embedded** walks each `/Font` resource and its `/DescendantFonts` for `/FontFile`,
`/FontFile2` or `/FontFile3`. **The annotation subtype breakdown** comes free from the corpus
gate's own output: `grep -o '"[A-Za-z]*: no appearance stream"' | sort | uniq -c`.

### What the corpus gate reports today

Ratcheted in `crates/pdf-model/tests/corpus.rs`; the numbers only ever go down, except where
a rise is a new report and is written down as one.

| | count | |
|---|---|---|
| unopenable | 0 | and it should stay there |
| no page one | 19 | 11 encrypted, 8 with unrecoverable page trees |
| draws incompletely | 189 | Counted by each document's *first* report, so the column sums: 67 a font, 67 an annotation, 18 a transparency group or mask departure, 15 an operator, 11 an image, 7 an undecodable content stream, 2 an object composited in parts, 1 a text knockout, 1 a bound reached |
| slower than 30 s | 0 | `KNOWN_SLOW` is empty, and the next document to cross the budget fails the gate |

**The operator row was 33** until the thirteenth session implemented §9.3.6's eight text
rendering modes, and is 15. Nothing left on it is a feature anybody could implement: it is
`BT` without `ET`, `BDC` without `EMC`, and the byte soup a fuzzed content stream lexes as
operator names.

**The shading row is gone.** It held 28 documents and every one of them was a soft mask in an
`/ExtGState` — filed under shading because nothing else fitted. §11.5 is implemented (ADR
0027), so 17 of those documents left the list outright and the rest say something narrower: 7
that their mask group's `/CS` is not the device's three components, 1 that its group is a
knockout one.

The image row was 161 before JBIG2 and JPEG 2000 landed, 53 before inline images did, 31
before `CCITTFaxDecode` did, 19 before `/Mask` applied, 13 before an `/SMask` of another size
did, and is **11** now — one image apiece, and **nothing on it is a feature**: 4 malformed
streams, 3 bit depths the unpacker refuses, one `/Mask` that is not an image mask, one JBIG2
using a segment type ISO/IEC 14492 does not define, one 212-megapixel JPEG 2000 scan, and one
`/SMask` of 34862x4332 against a 2x2 image, whose combined grid the bound refuses. The row that
was 14 soft masks is one, and `issue16263.pdf` wrote 13 of those 14.

**The font row was 100 and is 67**, which is the largest fall any row of this table has had.
§9.7's composite fonts landed in the twentieth session (ADR 0029) and the two largest entries left
it outright: 26 fonts with a non-identity `/CIDToGIDMap` and 14 with an embedded `CMap` stream now
draw. Counted as *fonts* rather than documents because a page may name several, what is left is 27
with no `/ToUnicode` so a substitute cannot be addressed, 23 whose substitute draws none of their
declared codes, 15 naming one of Table 116's predefined `CMap`s, 4 asking for vertical writing, and
the rest malformed programs. **Nothing on it is a `CMap` question any longer**, and the row is no
longer the largest — annotations are, by one document.

### What the oracle gate reports today

Ratcheted in `crates/pdf-model/tests/oracle.rs`, by name and in both directions.

| of the 1557 pages we call complete | count | |
|---|---|---|
| agree with the reference consensus | 706 | |
| **contradicted** | **98** | 8 page rounding, 7 a shared JBIG2 decoder and 1 a shared *gap* (trap 9, both halves), 1 a sub-pixel image, 1 a `CalRGB` alternate, 1 an eight-bit mask value, 2 glyphs judged as vector, 1 a symbolic font's contradictory flags, 16 substituted fonts, **60 unexplained** |
| ambiguous | 740 | the references disagree with each other; 372 of them are two long books set in fonts nobody embedded |
| our page geometry differs | 3 | 2 are `/UserUnit`, 1 unexamined |
| not comparable | 8 | fewer than two references produced an image, or they disagree on the page size |

The 237 incomplete pages are compared and printed too, but cannot fail the gate: a page we
already say we cannot draw is expected to differ, and listing hundreds of them would drown
the signal. It **fell by 32** in the twentieth session as §9.7's composite fonts stopped
reporting, which is the largest single arrival the gated set has ever had: 32 pages moved *into*
the comparison and 18 of them agree with the reference consensus outright. It **fell by 20** in
the eighteenth session as soft masks stopped reporting — and the reverse of
the trade the rest of this paragraph describes, because a gap that closes gives the pages back.
It **rose by 8** in the seventeenth session, as seven documents began reporting a
transparency group departure and one page changed its mind about §9.3.8 — the cost, in
coverage, of two silences ending. It did not move at all in the sixteenth session, which is that session's shape:
area averaging changed *how* pages already drawn were drawn, so nothing entered or left the
gated set and the whole result is three verdicts flipping inside it. It fell by 1 in the
fifteenth session — two pages gained as soft masks of another
size began to apply, one lost as §11.6.2 began to report — and by 4 in the fourteenth — seven pages gained as `/Mask` began to
apply, three lost as §9.3.8 began to report, and trap 11 is about the second half of that
trade. It fell by 16 in the thirteenth session as the text rendering modes landed, by 14
in the twelfth as fax-encoded images started drawing, by 42
in the eleventh as inline and `Indexed` images did, and by 10 in the tenth as Type 3 pages did — every one of those pages moved *into*
the gated set, which is why the contradicted count rose while nothing got worse. In the eighth
it rose by 43 and the gated total fell by the same, which is the cost of honesty: a page that
starts reporting stops being watched by *this* gate. It is the reason a report should never be
reached for as a way of making a contradiction go away, and the reason
`CONTRADICTED_SUBSTITUTED_FONT` now records which of its departures were fixes and which were
exits.

**Where its time goes, measured and printed by the gate itself.** It used to be roughly
1000–1300 s of processor time in the three external renderers against 45–55 s in ours, for 75 s
of wall clock on 24 cores — a ratio above 20:1, so **the gate was essentially a measurement of
`pdftoppm`, `mutool` and `gs`**. That was the twelfth session's first item and ADR 0020 is the
answer: the references' renders are remembered, keyed on the invocation itself, and the run is
**25 s with 17 s left in the three renderers** when it landed, and 34 s with 24 s in them a
session later on a busier machine — the hit rate is the number that says the cache is working,
not the clock. Every verdict is unchanged, which was checked
by running the whole corpus both ways and comparing the counts.

What is left is ours. Roughly 600 s of processor time over 24 cores goes on our own render,
the comparison and the artefacts — the SSIM and heatmaps for the thousand pages that are not
agreement — so if 25 s ever becomes the constraint, that is where to look, and not at the
subprocesses. The cache prints its hit rate for the same reason the versions are printed: a
run over an unchanged tree that reports less than 99% hits means the corpus or a renderer
moved, and it is better to see that than to infer it from the clock.

**The time budget reports; it cannot enforce.** A Rust thread cannot be cancelled, so a
document that never returns hangs the suite rather than failing it. A real budget has to
live inside the interpreter and the rasteriser. `PDFVIEWER_CORPUS_TRACE=1` names each
document on stderr as it starts and finishes, which is how a hang gets identified from a
killed run.

**`doc/pdf.js` is a submodule** (Apache-2.0, pinned at v6.1.200), holding those 974 PDFs and
459 more behind link files. It is optional to clone — every test that uses it reports being
skipped rather than failing — but the ratchets only mean anything where it is present, so CI
must have it.

## Habits these sessions earned

**The configuration in which a model is invisible is the one every producer ships.** §9.7 gives a
composite font two mappings — a code to a CID, and a CID to a glyph index — and under `Identity-H`
with `/CIDToGIDMap /Identity` both collapse to nothing, so a code can go straight to a glyph index
and neither mapping has to be read. Nineteen sessions of real documents never asked what either
one *is*, and the tree could not notice, because the clause's degenerate case is the case almost
every file uses. The general form is the fourteenth session's `Tk` lesson from the other end:
**ask what a feature looks like when its parameters are not their defaults**, because the defaults
are exactly where a missing implementation is indistinguishable from a complete one.

**A presence condition is not a restriction on meaning.** Table 115 says `/CIDToGIDMap` is
"Required for Type 2 CIDFonts with embedded font programs" and then, in the next sentence, what it
means: "A specification of the mapping from CIDs to glyph indices." The first implementation here
read the first sentence as bounding the second and ignored the entry on a Type 0 `CIDFont`, which
drew one page as garbage where four renderers draw a sentence. Nothing but the picture could have
said so — the code was right about the clause it cited and had cited the wrong half of the table.
When a clause conditions something, read what the condition is *about*.

**Where the standard defers to another document, the deferral is a citation.** §9.7.5.3 hands a
`CMap` file's syntax to Adobe Technical Note #5014 and says nothing more about it, so ISO 32000-2
never states that a `notdefrange` gives its whole range one CID while a `cidrange` numbers upward.
The first draft here numbered both upward, which is a run of substitute glyphs the `CIDFont` has
no reason to carry. A test caught it; reading had not, because the sentence that would have said
so is in a document the standard points at rather than in the standard.

**A rule no document exercises still has to be right, and you find out by breaking it.** §9.7.6.2
matches a code against a codespace range byte by byte, which is not the same as comparing the whole
code, and **all 1794 oracle verdicts are identical under either reading** — measured, by swapping
the comparison and running the gate twice. That turned "the corpus does not cover this" from a
suspicion into a fact, and it says which test is load-bearing: a synthetic `CMap`, not any of 974
files. Trap 8 has said for fourteen sessions that a corpus finds what documents contain; this is
the cheap way to find out *which* of your rules it is not checking.

**Ask what the clause requires of *this* device before deciding it is a gap.** Overprinting was
the top of the demand list at 63 documents and six `silent` rows on the spec list, and both
counts were honest about what they measured: a key, present in 63 files, and a table nobody had
read. What settled it was neither — it was §8.6.7's own sentence about a device that produces no
separations and Table 146 read against a list of this device's colourants, which together say
the special overprinting blend function is Normal here. **A gap sized by a corpus is a hypothesis
about a clause.** The same reading, applied to §10.7.5, split one clause into one requirement to
implement and one already satisfied by a departure taken years earlier for another reason.

**A convention that agrees with the specification is worse than one that does not, because it
removes the reason to write the rule down.** `tiny-skia` draws a zero-width stroke as one device
pixel, which is exactly §8.4.3.2, so the CPU backend was right for free and the clause was never
stated anywhere. Vello has no such convention, and every `0 w` line in every document was
invisible on the GPU for fifteen sessions. Where two backends are the oracle, **a decision either
of them can make alone is a decision neither has made** — which is why the three device decisions
in this tree all live in `pdf-render`.

**A clause about the whole page can be invisible until one construction needs it.** §11.4.7 is
two paragraphs saying the page is an isolated group, and it decides how *every* blend mode in
*every* document composites against unpainted paper. It stayed `unreviewed` through sixteen
sessions and three reviews of clause 11's other families, because nothing in the tree had a
reason to render onto transparency and so nothing could tell a white backdrop from no backdrop.
What made it findable was building the thing one level down: a group has to composite onto
nothing, and once one does, the question "what does the page composite onto" asks itself.

**An assumption a test cannot exercise is not tested, however many tests run over it.** The GPU
backend converted Vello's output from premultiplied alpha for fifteen sessions. Vello does not
produce premultiplied alpha. Nine cross-backend scenes and 1794 oracle pages could not see it,
because every one of them was rendered onto an opaque background where the conversion is the
identity — the input never had a partial alpha in it. The fix is one line; the lesson is that
"the backends agree" is a claim about the inputs they were given, and a *constant* input
property is invisible to every test that shares it.

**"The clause says nothing" and "the clause says the opposite" are different findings, and
only one of them is a licence.** Two places in this tree recorded image reduction as
unspecified — `is_smoothed`'s doc comment and the ledger's §8.9.5.3 row — meaning §8.9.5.3,
which is about magnification and genuinely is silent. §10.7.4 is not: "there shall not be
averaging over the pixel area", in a clause nothing here had ever cited. Both sentences
produce the same code; only the second produces a *departure*, which has to be argued,
recorded and costed. When a comment says the standard is silent about something, the question
to ask is not "is that true of this clause" but "which clause would say it".

**A departure is only honest once you have looked for the others.** Finding §10.7.4's image
sentence made it necessary to read the rest of the subclause, and the first rule in it —
"painting any pixel whose half-open square region intersects the shape, no matter how small
the intersection is" — has been departed from since the first commit, by anti-aliasing, with
no clause cited anywhere near it. One departure looks like a compromise; three in one
subclause, all in the same direction, is a *reading* of what §10.7.4 describes, which is a
device that quantises coverage to whole pixels. §10.7.1's NOTE then licenses all three at once.

**A suspiciously clean measurement is a reason to check the instrument.** The first four
callgrind numbers for area averaging were flat to four significant figures across pages that
obviously do different amounts of work. The benchmark was passing 4096 as `TargetSpec::for_page`'s
**total pixel** budget rather than an extent, so every run panicked and callgrind faithfully
counted the panic. A page-sized raster is half a million pixels and the argument is not named
at the call site. This is trap 4 inside a measurement: a tool exercised on nothing reports
success.

**A test has to be able to fail at the defect's magnitude, not only in its axis.** Trap 2 has
always said a scene must vary in the direction the defect moves. The sixteenth session's first
CPU-versus-GPU scene did — it reduced an image, which is the direction — and **passed with the
GPU's filter removed altogether**, because it reduced 64x64 into an 8x4 corner of a 200x200
page and 32 differing channels of 160 000 is under the tolerance. Check both halves by
deleting the code the scene is meant to guard.

**A constant that is a property of the state must reach every paint, including the ones that
replace the colour.** `ca` is not part of a colour; §11.6.4.4 makes it a property of the
graphics state applied to painting operations. A shading replaces the current colour rather
than tinting it, so the one line that returns it dropped the alpha along with the colour it did
not use — and the page that shows this says `Gradient: .5` on its own face. The general form:
when a paint has a *special* case, check that everything the state contributes survives it, not
only the thing the special case is about.

**A bound written for the pathological case can refuse a reasonable one.** `MAX_MASK_GRID`
exists because a 2×2 image with a 34862×4332 mask asks for 604 MB. Applied as a flat limit on
the combined grid it also refused a 12608×16806 mask on a 12608×16806 image, where combining
costs exactly what the image already costs and a different bound had already admitted it. The
bound belongs on the *growth* — how much bigger the combination is than the thing being drawn —
and the corpus said so within a minute of the first run, which is the argument for running the
whole corpus after a change that adds a limit.

**Print what a condition matched before trusting its count.** Twice now a report's first draft
has been defensible from the clause and wrong about the corpus, and both times one `eprintln!`
in the branch settled it in a single run: §9.3.8's first check named seven documents where the
models could not differ, and §11.6.2's named six of which three paint one of their two parts at
alpha zero. A count is not evidence that a condition is right; the matched cases are.

**One dictionary, two clauses, and only the second one says who wins.** §8.9.6 defines what an
image's `/Mask` means, in two forms, over two pages. What it does not say — and what Table 87
does not say either — is that an `/SMask` beside it "shall override any explicit or colour key
mask". That sentence is in §11.6.4.3, in the transparency clause, and an implementation that
reads only clause 8 is complete by its own lights and wrong on any file writing both. It was
found because the session's spec-track item was the family its *demand* item cited from the
other direction. The general form: when a key appears in more than one clause's index entry,
the clause that owns the feature is rarely the one that states the precedence.

**When the standard defines nothing, a plausible reading can still be untenable — and the page
is what says so.** `issue6621.pdf`'s `/Mask` is a one-bit greyscale image where Table 87 and
§8.9.6.2 both require an image mask. There is exactly one reading its samples admit under the
clause cited, §8.9.6.2's "a sample value of 0 shall mark the page", and applying it blanked a
court seal three renderers draw, because on that page the zero samples are the background. The
alternative reading is §11.6.5.2's luminosity-as-opacity, which is a different clause about a
different key and would invert every stencil whose author forgot `/ImageMask`. So: neither, and
the entry is named. **Refusing is a result, not a failure to decide** — it is what principle 5
asks for where the specification is silent and the choices are opposite.

**A report has a price, and it is paid in gated pages.** §9.3.8's first check named 7 documents
and moved three *agreeing* pages out of the oracle's comparison, for a difference that could
not have appeared on any of them. The second check named 2. Nothing about the gap changed —
only whether the condition matched the clause. Trap 11 has the full form; the habit is to ask,
before adding a report, "on how many pages can this actually be seen?" and to make the
condition answer that question rather than a looser one.

**Build the strong gate, then let its own output tell you it is wrong.** The thirteenth
session found the tree citing "§9.3.6 Table 106" for the text rendering modes — Table 104 —
in four comments, two tests and a written report, and wrote the obvious checker: a table
reference must name a table the clause beside it discusses. It failed fourteen of the tree's
twenty-five references and **all fourteen were correct writing**, because a comment about one
clause routinely names a table belonging to another. Behind an exception list it would have
been a gate that is mostly exceptions, which teaches a reader to ignore it. What shipped
instead asserts the weaker true thing and *prints* the title of every table the tree cites, in
which the wrong pairing is obvious at a glance. **A check whose false positives are all correct
code is measuring the wrong property, not finding a backlog.**

**A rule about how something is *encoded*, implemented as a rule about its value, is invisible
forever.** §9.3.3 applies word spacing to "the single-byte character code 32" and says in its
next sentence that it does not apply to a byte 32 inside a multiple-byte code. This tree
applied it to any code numerically equal to 32, so every `Identity-H` string containing the
bytes `00 20` had the rest of its line pushed right. No corpus page could show it, no metric
could see it, and it had been wrong since composite fonts landed. It took five minutes of
reading the clause. The same shape as §9.6.5.4: self-consistent code, right about every
document anyone had opened.

**Wall-clock lied again, in the same direction, and callgrind settled it in two runs.**
`cargo bench` reported an 8% regression on one interpretation benchmark from a change that
adds three branches per glyph; `valgrind --tool=callgrind` over
`examples/callgrind_interpret.rs` put it at 1.912 G instructions before and 1.921 G after —
**+0.46%**. This file has said "count instructions" since the seventh session and it was still
tempting to believe the 8%. Run the two callgrinds; they take a minute each.

**"Clippy clean" is a claim, and this file made it while eleven warnings sat in the tree.**
Every one came from the twelfth session's own new files, and the reason is worth knowing:
`allow-panic-in-tests` in `clippy.toml` covers `#[test]` bodies, and **not** the helper
functions of an integration test — so a `panic!` in a `fn` that a test calls is a warning
while the identical line inside the test is not. Whatever this file asserts about the tooling,
run it once before believing it.

**Measure the instrument before deciding you are slow.** Eleven sessions treated the oracle's
85 seconds as the price of having an oracle. It was 95% three other programs answering a
question they had answered the day before, and the fix took an afternoon and made every
subsequent change three times cheaper to check. The general form: when a loop is slow enough
to change *what you attempt*, that loop is a thing to measure, not a constraint to design
around. The tempting alternative here — batch ten features, then work out which files moved —
would have traded the gate's completeness for speed the gate did not actually need.

**And when the first design of a fix is the obviously safe one, still measure it.** Refusing
to cache timeouts is unarguable in principle: a timeout is a fact about the machine, not about
the document. With everything else cached it left **two pages out of 1794 accounting for 46 of
the run's 57 seconds**. The rule that replaced it remembers them for a week, prints how many
it used, and has its cost and its expiry written down — which is what principle 1 asks of a
shortcut, as against taking one silently or refusing to take one at all.

**A test that skips silently is worse than no test.** `tests/ccitt.rs` was first written
against `bug1001080.pdf`, whose fax data turned out to be inside a Type 3 font's glyph
descriptions rather than in an image `XObject`. The helper returned `None`, both tests printed
"skipped: the submodule is not checked out", and both passed while checking nothing. A missing
corpus is a skip; a present corpus that does not contain what the test needs is a **panic**.
The two are one `?` apart and they look identical in the output.

**A citation nothing checks is a citation that rots — so now something checks them.** The
tree carried 146 references to ISO 32000-2 and the first tooling ever pointed at them found
two clause numbers that name nothing and three of five sampled quotations that are
paraphrases inside quotation marks. Nothing had gone wrong yet — the same condition under
which the duplicated D50 matrix was cheap to fix. A reference decays at the rate of the
attention paid to it, and in this project the attention is spent on pixels, correctly. The
checker is `tools/conformance`, it runs with the workspace's tests, and the thing to carry is
that **it kept finding errors after the obvious ones were fixed**: the corrected `/Mask`
citations were still wrong, because §8.9.6.2 is stencil masking and `/Mask` naming another
image is §8.9.6.3, and no amount of checking numbers against an index would have caught that.
Only reading the clause did.

**A fallback that fills the page is worse than one that leaves it blank.** §9.6.5.4's
predecessor ended in "if nothing else matched, the code is the glyph index", per code, and
`issue5501.pdf` drew `v 0' ' W` for `What's an interval?` — confident, plausible, wrong and
silent. The same fallback survives, restricted to a font with no readable `cmap` at all, and
the oracle proves the restriction is load-bearing: put it back per-code and `issue17333.pdf`
is contradicted immediately. Prefer the blank that reports.

**Fixing the mask shows what the mask was hiding, so budget for it.** §9.6.5.4 had nothing to
do with Type 3 fonts or with substitutes that reach none of a document's codes, and both
became visible in the afternoon spent looking at what fonts actually did — 24 documents and
19 against the one that led there. The same shape recurred in the ninth session: reading
§8.9.6 for the ledger produced a wrong citation, a missing test and an unimplemented sentence,
none of them the thing being looked for. The fix a defect leads to is often not the fix it
names.

**A dependency is a decision, and this project's own precedent decides it.** `zune-jpeg` owns
`DCTDecode`, `skrifa` owns font parsing, `flate2` owns Flate, `tiny-skia` owns rasterisation,
and `hayro-jbig2`/`hayro-jpeg2000` own the two hardest image codecs. Writing 19 400 lines of
MQ coding and symbol dictionaries here would have been consistent with none of that. The cost
is written down rather than assumed away: two decoders we cannot fix ourselves. See ADR 0014,
and `deny.toml` for what the tree refuses outright.

**A corpus can state an invariant about itself, and that beats any reference.** 96 corpus
documents encode one image ninety-six ways, and demanding they agree needs no external
renderer, so principle 5 is not even in tension — the expectation comes from the documents.
It is also more sensitive than any tolerance, and it is the only thing that could settle a
page where two of the three references are secretly the same decoder. Look for that shape: a
corpus varying one thing while holding another fixed is stating a testable invariant.

**A clause read and dismissed is worth as much as one implemented.** The ledger's statuses
include `inapplicable` and `writer-side` for exactly that, and filling one costs a minute
against the 20 to 60 a real review costs. The trap is treating the ledger as a to-do list of
features; it is a record of questions asked, and "asked, and it does not apply to a screen"
is a complete answer.

**An unimplemented feature has a default, and the default is usually "draw it".** That is why
two unrelated renderers can agree about a page while both being wrong, and it is a much more
common failure of the oracle's premise than shared code is. When a contradiction looks like
"everyone disagrees with us", the cheap next step is not to re-read our own code: it is to
search the other projects' source for the clause. `mupdf`'s `FIXME: Calculate visibility from
array` and `ghostscript`'s `WARNING: OCMD contains VE, which is not supported (ignoring)` took
minutes to find and settled a page that had looked like three-against-one.

**Two references against two is not a tie, and not a vote — it is a question with an answer.**
`Type3WordSpacing.pdf` splits them: `poppler` and `ghostscript` paint a `d1` glyph's stroke in
the fill colour, `mupdf` in the stroking colour, and we were with `mupdf`. There is no rule
that says "go with the majority", and there did not need to be — Table 111 says "its colour"
in the singular, says the description "is executed solely to determine the glyph's shape", and
explains that an image mask is admissible inside one because a mask "merely defines a region
of the page to be painted with the current colour". One region, one colour, and the two
renderers that agreed with the clause agreed with it for that reason rather than by weight.
The habit to keep is the order: read the clause first, and let the split tell you *where to
look* rather than what to conclude.

**What you measure decides what you build, so check what the measurement cannot see.** Eight
sessions were steered by two gates that both take the pdf.js corpus as their universe. They
are good gates and the work they directed was real. But the ordering they produce is a
demand curve — features ranked by how many of 974 files want them — and a demand curve
cannot rank a requirement no file exercises, cannot notice a clause nobody implemented, and
converges on "done" the moment the last file goes green. §6.3.2.2 ranks optional content
first among what is left; the corpus ranks it seventh, tied with three other five-document
items. Neither ordering is wrong; running only one of them is.

**A gap inside a feature you have implemented does not announce itself.** Every missing
subsystem in this tree reports — `LZWDecode`, JBIG2, encryption — because somebody wrote the
report while deciding not to write the feature. The gaps that ship are the ones *inside*
something implemented: `Tr` was parsed and three of its eight modes reported, `/SMask` was
honoured while `/Mask` beside it was not, `CalGray` was resolved and then converted as
`DeviceGray`, and §9.6.5.4's whole algorithm was one line that worked on Latin text. Reading
the specification asking "what have we not built" cannot find those, because the answer is
"nothing". Comparing output against another implementation can, and has, nine times now.

**The purest instance of that is the `d` operator, and it is worth keeping as the archetype.**
Every layer of the dash feature existed: `pdf-render`'s `Stroke` carried an array and a phase,
`render-cpu` built a `tiny-skia` `StrokeDash` from them, `render-gpu` handed Vello a dash
iterator, and `set_dash` had a doc comment. The one line that mattered read only the *empty*
array, because the content lexer flattens an operator's array operand and nobody had written
the case for a non-empty one. Result: an implemented feature, two agreeing backends, four
crates that all look right — and not one dashed line in 974 documents. When a feature looks
finished, check the operand path from the content stream to the state, because that seam is
the one nothing else in this tree looks at.

**A subclause is a checklist; check the code against it, not the code against itself.**
§9.6.5.4 is two pages and names five distinct routes from a code to a glyph. The code that
stood in for it implemented roughly one and a half of them, and no reading of *that code*
would have said so, because it was self-consistent, commented, and right about the documents
anyone had opened. The cheap move that was never made is: open the clause, list its rules,
and ask of each one where it is. It took five minutes and was worth 15 contradicted pages
and two unrelated silences. Do it for §11.4 and §12.5.6 before implementing either.

**A gate's numerator moves when its denominator does, and only one of those is news.** The
oracle judges pages we claim to draw completely, so a feature that makes 42 pages drawable
adds 42 pages to the set being judged — and four of them came out contradicted, none of them
by getting worse. The eleventh session's contradicted count rose from 104 to 108 in exactly
that way. A ratchet that only ever counts failures will read this as a regression; the fix is
the one this file already uses for the corpus, which is to say *which* pages moved and why,
and to keep the denominator beside the numerator wherever the count is quoted.

**When a report replaces wrong output, the reported count is the wrong scoreboard.** Two of
the eighth session's three changes made the corpus's incomplete count *rise*, by 43 documents, and
the "draws with nothing reported" share fall by four points. Every one of those documents was
already drawing wrongly. A project that watched the percentage would have had to leave
`issue918.pdf` emitting letter fragments to keep it. The rule the ratchet comments now state
in both files: a rise is fine when you can name the silence it ended, and a *fall* in the
contradicted count is only a fix when the page still enters the comparison — seven of this
session's 22 departures left it by becoming incomplete, and saying so is part of reporting
the result honestly.

**A shortcut that is right on the common case is worse than one that is wrong on all of
them.** The Cal-space pass-through was nearly correct for `/Gamma 2.2`, which is what most
documents write, and badly wrong otherwise. Nothing distinguishes the two populations at
runtime, so nothing reported. Prefer the derivation even where the approximation looks close:
"close on the files I tried" is not a property you can test for.

**Ask the reference the same question you asked yourself.** Two of the three renderers were
being asked for the media box while we rendered the crop box, which put 54 documents beyond
comparison and would have produced false failures on any page whose two boxes differ only in
origin. A comparison harness has its own defects, they look exactly like ours, and the way to
tell them apart is to check the invocation against the clause before believing the verdict.

**Look in `read-fonts` before writing font-format code.** An earlier handover specified
~80 lines of CFF charset parsing plus two 256-entry tables, and all of it already existed
in `read_fonts::ps`, which `skrifa` re-exports as `skrifa::raw`. See ADR 0006. The same
module also holds `type1`, `charmap` and `agl` — `agl` is now enabled and carries the
Adobe Glyph List, so nothing needs transcribing.

**Profile before believing an explanation, even one whose arithmetic matches.** An earlier
handover attributed a 48-second page to page-sized clip masks and supported it with
`3576 clips × 485 kB = 1.7 GB`, which is exactly what the process held. The arithmetic was
right about the memory and silent about the time: `callgrind` put the masks at under 4% and
the gradient stage at 78.9%. Fixing what the arithmetic named would have kept nearly all 48
seconds. A number that reproduces one symptom is not a diagnosis.

**Wall-clock benchmarks lie under load; count instructions instead.** A `Command::Fill`
change measured as a 24% *regression* on `cargo bench` and as an 8.5% *improvement* twenty
minutes later, purely from background build load. `valgrind --tool=callgrind` on
`crates/pdf-model/examples/callgrind_interpret.rs` settled it deterministically: 2.065 G
instructions before, 1.951 G after. Always A/B in one sitting, and prefer the instruction
count. `iai-callgrind` wraps this into a bench harness and is the right basis for the CI
perf gates `CLAUDE.md` asks for — not yet wired up.

**Measure against something comparable, or the number means nothing.** This project compared
itself to `poppler`, `mupdf` and `ghostscript` for six sessions and never once asked whether
it was *fast*, because against C the question has no clean answer. `hayro` made it answerable
and the answer was 1.61× slower on the median page. Both causes were in our own code and
neither was where intuition pointed: not the rasteriser, but a per-pixel unpacking loop and a
subdivision criterion missing a term. A benchmark you cannot attribute is a benchmark you
will not act on.

**A premise that reads like a fact does not look like a question.** "JBIG2 and JPEG 2000 have
no memory-safe implementation" sat in `PLAN.md` as the reason two filters were unimplemented,
and it was true when written and false for months before anyone checked. It was re-*read*
constantly. Nothing in a plan marks which of its statements are about the world rather than
about the project, and those are the ones that rot. Any item deferred on an external
condition should carry the date the condition was last verified.

**Two rasterisers disagreeing is information, not noise — and two agreeing is not proof.**
The CPU-versus-GPU agreement test is what found that Vello needed the same mesh seam repair
`tiny-skia` did, after a comment here had confidently claimed otherwise. Where the backends
differ, one of them is wrong. The other half of the rule was learned the hard way: both
backends positioned paints in the wrong space, in the same way, for the same reason — the two
libraries share the convention that was misread — so they agreed with each other perfectly
while both were wrong. **Agreement is evidence only where the implementations can fail
independently.**

**Two copies of a constant is one defect waiting.** Three `DeviceCMYK` conversions disagreed
and nothing looked wrong. When that was fixed, the same shape survived one level down: the
nine-constant D50-to-sRGB matrix sat in `colour.rs` and in `icc.rs`. Nothing had gone wrong
yet, which is exactly the condition under which it is cheap to fix. It is now one function
with a test that recomputes all nine numbers from the two published matrices they were folded
from — so a folded constant, which is otherwise unreadable and unfalsifiable, has a
derivation attached.

**A test written to isolate one rule finds what a corpus cannot.** The ICC evaluator agreed
with two other readers on every real profile in the corpus. Writing a test that assembles a
profile *by hand* produced one whose darkest colour equals its white point, and black point
compensation divided by a span of floating-point noise and turned white into pure green. No
real profile is shaped that way. See trap 8, which is now the general form of this.

**Measure before optimising, and delete what does not measure.** `glyph_for` builds a
`FontRef` per character, which looks like an obvious cache. Caching it changed a dense page
by less than run-to-run noise (3587 lookups, 211 distinct codes), so the cache was removed
and the reason written where the next person will look. The same session's *real* win was
found the same way: hoisting a string allocation out of `substitute::find` took a difficult
lookup from 1.37 ms to 18 µs. `cargo bench -p pdf-model` is the baseline.

## Things worth knowing

- **The sandbox is a flag, and the default is the safe one.** `--no-sandbox` decodes JBIG2
  and JPEG 2000 in the viewer's process. It can be a flag only because both decoders are
  memory-safe either way: what it trades is panic containment and a memory ceiling, which are
  real and bounded, not memory safety, which would not be offerable. There is deliberately no
  path that falls back to in-process decoding when the worker fails to start — a fallback
  that silently removes the confinement is worse than a reported failure.
- **A font is reported as a whole, and that is not fine-grained enough.** `FontError` is the
  only channel a font has, so a font either loads or does not. A font that maps *some* of the
  codes its document declares and not others therefore draws the ones it can and says nothing
  about the rest. The eighth session narrowed this — a substitute reaching *none* of the
  declared codes is now refused, which is what caught `tracemonkey.pdf`'s missing © — but the
  general case needs a report where a glyph is *shown*, in `show_text`, rather than where a
  font is loaded. That needs `LoadedFont` to distinguish "this code has no glyph" from "this
  code's glyph is blank", which a space legitimately is. Not hard; not yet done; and worth
  measuring on the corpus before assuming the volume is manageable.
- **The oracle's artefacts are the fastest diagnostic in the tree.** Every page that is not
  agreement leaves `<target>/tmp/oracle/<stem>/p<n>/` holding our render, each reference's, a
  side-by-side strip and a difference heatmap per reference. Open the side-by-side first: it
  is one image, four panels, ours leftmost, and it has explained every page it was pointed at
  so far — a solid bar where a word should be, a band that should have been masked out, grey
  swatches at the wrong lightness, a page one pixel short. Pages that agree have theirs
  deleted, so what is on disk is exactly the set worth looking at.
- **A page's tolerance class depends on what *we* drew.** The oracle picks a text tolerance
  or a vector one from our own render's content, so a change that adds text to a page also
  loosens its bound — and can move it from "ambiguous" to "judged". Four pages crossed that
  line in the sixth session when annotations started drawing, and all four had *improved*. When a
  page appears in the newly-contradicted list, check whether its bound changed before
  concluding the render got worse.
- **Reference renderers are given 30 seconds and then killed.** A corpus holds files written
  to make a reader loop, and `Command::output` waits forever. `Reference::render_within` polls
  and kills; there is deliberately no unbounded variant.
- **`doc/md/` is the specification, in a form code can read.** It holds Markdown conversions
  of the 14 specification PDFs in `doc/`, with real tables, and it is committed — so a test
  may depend on it without a skip path, unlike the pdf.js submodule. `ISO_32000-2_sponsored_EC3.md`
  is 24 MB and its 860 `##` headings give a clause number, a title and a line range apiece,
  which is the whole basis of the citation checker and the conformance ledger (`PLAN.md`
  §5a). Two caveats for whoever builds those: it is a *conversion*, so a quotation the
  checker cannot find may be a conversion artefact rather than a bad quote — check `doc/`'s
  PDF before editing the comment — and one heading number (`14.8.4.7.3`) occurs twice.

  When you need spec data — encoding tables, operator lists, value constraints — extract it from
  there rather than writing it from memory. The `WinAnsiEncoding` and `MacRomanEncoding`
  tables in `pdf-font` came out of `doc/md/ISO_32000-2_sponsored_EC3.md` Table D.2 that
  way, and the extraction caught three things memory would have got wrong: PDF's
  `MacRomanEncoding` is not Mac OS Roman, and Table D.2's *notes* assign `space` at 160
  and 202, `hyphen` at 173, and every unused WinAnsi code above 32 to `bullet`.
  The files carry base64 images inline, so `grep -v '^!\[Image\]'` before reading a range.
- **The Arlington model is the object model, not the semantics.** It says `/BaseEncoding`
  must be one of three names; it does not say what those encodings contain. Do not expect
  glyph data, operator semantics or rendering rules from it.
- **`Interpretation::text` is a readback of what was drawn**, accumulated by the same loop
  that places the glyphs, and `crates/pdf-model/tests/text_extraction.rs` compares it
  against `pdftotext` over the 14 specification PDFs in `doc/` — not the pdf.js corpus,
  which would need a per-document expectation. It is the only check that catches a code
  reaching a *plausible* wrong glyph. It found the operand-cap defect below on its first
  run, and it is known to bite: reverting that fix scores 93.2%, and shifting every
  `/ToUnicode` entry by one code scores 58.7%. Extending it to the pdf.js corpus is a real
  opportunity — 974 documents against 14 — and would need only a tolerance rather than
  expectations, since `pdftotext` supplies the reference for each. `issue20504.pdf` is the
  argument for doing it: nothing we own noticed six scripts rendering as ASCII.
- **Silent caps are defects, not safety.** The interpreter dropped operands past the 64th,
  which truncated any `TJ` array holding a justified line — three sentences on the
  specification's own title page ended mid-word, with `unsupported: []`. Bounds against
  hostile input are right; reaching one without saying so is not. Every bound now reports.
- **A command draws into the rows its clip admits, not into the page.** `Band` in
  `crates/render-cpu/src/lib.rs`, and ADR 0010 for why rows rather than a rectangle. Two
  consequences to keep in mind when touching that backend: the device transform handed to a
  command already carries the band's row offset, so anything new that composes a transform
  must use *that* one; and the clip mask is band-tall and page-wide, because `tiny-skia`
  needs it to share the pixmap's row stride.
- **The display list is deliberately flat.** `tiny-skia` wants per-clip masks, Vello wants
  a layer stack; both translate. That neither library's model is native is the evidence the
  neutral form is right, and it is what lets the CPU backend validate the GPU one on
  byte-identical input.
- **RADV and lavapipe produce byte-identical output**, so goldens need not be per-adapter.
  A test pins this; if it fails, the assumption has broken, not the code.
- **Pixel comparison cannot police text, so there is a second kind of metric now.** The
  reference renderers disagree with each other at worst-tile 26–28 on text pages — glyph
  hinting, not error — and no threshold fixes that, because the noise floor is above the
  signal. `raster_compare::Comparison::structural_similarity` (SSIM) measures whether the
  same shapes are in the same places instead, and `Tolerance` now bounds it: 0.99 for
  vector, 0.90 for text. Both numbers were measured over 153 reference-against-reference
  pairs from the corpus, and the doc comment records that the distribution is *continuous*
  — 0.8990, 0.8993, 0.8998 and 0.9009 all occur — so 0.90 is a choice about which
  population to exclude (font substitution) and not a discovered boundary. Text
  *correctness* still belongs to the extraction metric.
- **`test-scenes` holds the same page twice**, as a display list and as PDF bytes. That
  pairing is what let the harness work before a parser existed, and it is checked by a test
  that renders both and demands identical pixels.
- **`doc/` holds more than ISO 32000-2.** `PDF20_AN001-BPC.md` is the PDF Association's
  application note on black point compensation, written by ISO 32000's own
  co-project-leader, and it settled a design question the base specification leaves to
  ISO 18619 — which black to align, and why `AbsoluteColorimetric` must not compensate. It
  had been sitting unread while the same question was being answered by looking at what
  other renderers do. Check what is already in `doc/md/` before concluding the
  specification is silent. **The sixteenth session is the sharper form of that**: image
  reduction was recorded as unspecified in two places, and the clause that specifies it —
  §10.7.4 — is in the same file, four clauses away from one the tree cites constantly. The
  cheap move is `grep -n '^## '` over `ISO_32000-2_sponsored_EC3.md` and reading the *titles*
  around the subject, which takes a minute and is how "scan conversion rules" was found at
  all.
- **Debug builds are ~15× slower here, and it changes what a test can assert.** The corpus
  gate is 1.6 s in release and minutes in debug. Any test with a timing assertion is
  meaningless at debug speed; run those in release and say so in the test. The oracle gate
  is the exception that proves it: about 95% of its processor time is three external
  renderers, whose speed does not depend on how we were built.
- `cargo-deny` is installed in the agent's `~/.cargo/bin`; run it before pushing rather
  than finding out from a red pipeline.
