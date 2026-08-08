# What the corpus and the oracle still name

Status: **standing** — the counts live in `doc/HANDOVER.md`'s gate table; the populations live here.
Read by: whoever is taking a page off the ambiguous ranking, reading a contradicted verdict, or
deciding what an `ambiguous` means. `doc/todo/00-ambiguous-bucket.md` is the task and the method;
this is the population and what it has produced.

`doc/HANDOVER.md` sections 3 and 3a are the pointers to this file.

**Read the oracle's 45% ambiguous with care.** 370 of those pages are two long books of dense
text at book size, where `Interpretation::glyphs` earns the page the *text* tolerance — 0.90
structural similarity, measured over 153 reference-against-reference pairs because the references
disagree with each other at worst-tile 26 to 28 on text. **This file said for many sessions that
those books are "set in fonts nobody embedded, so each renderer substitutes differently", and
`pdffonts` says otherwise**: `freeculture.pdf`'s four fonts are all embedded and nothing
substitutes on any of its pages (the two-hundred-and-twenty-ninth session, `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`).
That row means "reported nothing", not "drew it right" — and since the hundred-and-seventy-fifth session **emptying it is a task rather
than a caveat**: §3a.

**Both moving numbers move in both directions on purpose.** Contradicted pages: 174 → 65 over
sessions 6 to 61, steady at 65 until the hundred-and-forty-eighth took it to 70, the
hundred-and-fifty-sixth to **72** and the two-hundred-and-fifth back to **68** — the last two were `noembed-eucjp.pdf` and `noembed-sjis.pdf`,
recorded as drawing あいうえお "in a face the references do not have" — **and they were drawing
nothing at all**, which the hundred-and-eighty-second session found by making the silence loud
(ADR 0152). Both report now, and two widget-border pages left in the two-hundred-and-fifth session (ADR 0165), so the count was **69** until the four-hundred-and-fifth, whose one width defect took it to **68**. Five of the earlier ones were net,
argued and written down in `CONTRADICTED_SUBSTITUTED_FONT`: the standard 14 are compiled in now,
so we stopped reading the same URW faces off this machine's disk that the three C references
read, and the oracle noticed within one run (ADR 0133). Corpus documents drawing incompletely: 291 → 89 over
sessions 6 to 122, then 91 in the hundred-and-twenty-seventh, where two documents that had been
drawing the wrong font in silence started saying so, and **76** in the hundred-and-fifty-sixth.

### 3. What the corpus still names

**The oracle's 68 contradicted pages, 66 of them on documents we call complete**, grouped and
ratcheted in both directions in `oracle.rs`, where each group carries its own diagnosis and its
measurement. **The 66 counted off the groups themselves in the four-hundred-and-fifth session**,
because this paragraph's own list summed to 72 and had said "4 page rounding" for a group of 2 and
"21 substituted fonts" for a list of 18: 2 page rounding, 2 our own anti-aliasing at a shape's
edge, **21 glyph edges** whose ink matches the consensus to a fraction of a level, 7 a shared
JBIG2 decoder, 1 a visibility expression the two agreeing references share a *gap* about, 3 a link
border, 1 a sub-pixel image, 1 a `CalRGB` alternate, 1 an eight-bit mask value, **5 a `DeviceCMYK`
conversion**, 2 a reference that drew nothing, 1 a reference glyph width, 1 a negative line width,
**17 substituted fonts**, **1 a tight consensus**, **0 unexplained**. The other 2 are on documents
this tree already reports (`issue5751.pdf`, `knockout_blend_multiply.pdf`) and are held by the
incomplete list rather than by a group.

**A count beside a list is not the list**, which is `doc/todo/02` §6's rule arriving one directory
over: the numbers above are now what `oracle.rs`'s arrays hold, and the way to keep them so is to
count them rather than to adjust them.

**The unexplained list is empty**, from 14 four sessions ago and from 42 at the start, and no
session that emptied it opened a debugger — the method is in
[todo 00](todo/00-ambiguous-bucket.md), which is the same method the ambiguous work uses. The
last two went to the two-hundred-and-forty-second and -third, both on the two-ladder closed form:

- `freeculture.pdf` page 313 → `CONTRADICTED_GLYPH_EDGES`. Ours at 8× is **6.0729** against a
  limit of 6.0658 and 6.0819, so the marks are right and the difference is 0.16 of 255 of glyph
  coverage at the page's own scale.
- `issue7891_bc1.pdf` → `CONTRADICTED_TIGHT_CONSENSUS`, the new name for what trap 12 describes.
  The two ladders agree to **0.0014 of 255** — the tightest limit in this file — and **ours at
  the page's own scale is 0.004 from it, the nearest of all five**, while `poppler` and `mupdf`
  are both 0.09 under. They vote because the bound is twice *their* spread, and they agree to
  0.009.

**Every printed metric on both pages is inside the class bound.** A verdict of contradicted can
be a statement about the consensus pair rather than about the page, and both of the last two were
that — which is the argument for the closed form: it is the one number derived from no reference
at all.

**Two cautions the contradicted list earned.** A page may be contradicted for a reason other than
the one its group names — **eight for eight, so far, on the group being wrong**, the newest being
`issue4304.pdf` in the four-hundred-and-fifth session, which sat in `CONTRADICTED_SUBSTITUTED_FONT`
for a hundred and eighty sessions while drawing *Wordsthatshouldhavespacesbetweenthem.* against
four renderers' *Words that should have spaces between them.* Its font really is substituted and
that really was not the difference: `/Differences [32 /.notdef …]` sent code 32 to a glyph whose
advance §9.6.2.1 obliges this processor to supply, and the third of `simple_widths`' three sources
read the program through an sfnt parser that refuses the bare CFF ten of the fourteen compiled-in
standard faces are. Six spaces of zero width — and "make it match
mupdf" is the failure principle 5 forbids. And a page can be contradicted by a departure this
project decided on purpose: `colors.pdf` pages 1 and 2 left the unexplained list in session 68 and
are *not* fixed, because §10.7.4 asks for the hard edge and this tree anti-aliases
(`CONTRADICTED_ANTIALIASED_EDGES`, and `doc/todo/_scan-conversion.md`).

**The 67 incomplete documents** — 70 until the three-hundred-and-ninety-seventh, which stated a knockout element's shape apart from its alpha and took `knockout_nested.pdf`, `knockout_nested_group_alpha.pdf` and `knockout_smask.pdf` off the list with nothing joining (ADR 0234); **this paragraph said 73 for four rounds after the three-hundred-and-eighty-third took it to 70, and the three-hundred-and-eighty-seventh counted them off the gate rather than off this file** — 73 until that round's second residue of §11.5.3 landed (ADR 0220), 72 until the three-hundred-and-eightieth, whose one new report is a `/DeviceN` shading inside a `/DeviceGray` luminosity mask group (ADR 0217), 74 until the three-hundred-and-fourteenth and 76 until the two-hundred-and-eighty-second, where a `Tf` naming
`/Helvetica` with an empty resource dictionary stopped meaning nothing, because §9.6.2.2 says those
fourteen names name something every processor has (ADR 0183). **The split below was counted off
the gate's own output in the three-hundred-and-eighty-seventh** and is by report kind, which is
what the gate prints: **29 fonts** (fewest since session 6 — session 156's `CMap`s took 15 off this
list — of which 4 report a font program that draws nothing, ADR 0157), **10 transparency** (8 a
group, 2 `CompositedInParts`), **10 operator soup** (`BT` without `ET`, `BDC` without `EMC`, fuzzed
streams), **7 malformed images**, **6 annotations** — Table 179's line endings took one in the
three-hundred-and-fourteenth (ADR 0192) — **3 a budget reached**, **2 an undecodable content
stream** and **1 a shading**. `doc/todo/23` says what each of the transparency populations now
owes, and its own count is the 8 documents: 19 before the three-hundred-and-eightieth,
14 after it, ADR 0220 took the three that close it to 11, and ADR 0234 took four more. Session 59's reading of
the corpus's own issue trackers says most of the font half is glyph rasterisation on files chosen
for having hard fonts, which session 68 then measured on one.

### 3a. The ambiguous bucket — watched since the hundred-and-seventy-sixth, and emptied in the three-hundred-and-seventy-ninth

**749 of the pages the oracle judges on documents we call complete come back `ambiguous` (786 of
all 1794), and until the hundred-and-seventy-sixth session no gate watched one of them.** **0** are
still undiagnosed, from 754, since the three-hundred-and-seventy-ninth session — and the instrument
is not retired by that: the gate holds the list to equality in both directions, so a page that stops
agreeing arrives in an empty file and fails the build on the arrival, which is the regression it was
built to see. Step 7 — our ink minus the lightest live reference's, over every ambiguous page — is
the half no ranking can do and stays standing.

The count in this file used to be 72, which was `wc -l` of a file with a twelve-line header and was
corrected in the three-hundred-and-seventeenth by counting what the gate counts. The twenty rounds
from the two-hundred-and-fifty-first took three populations at once and then worked the tail a page
at a time. The verdict means "nobody's difference is large
enough to call anybody wrong", which is the right thing for the *ratchet* to do and is not the
same as "right". `issue7406.pdf` drew a JPEG cyan-on-black inside an `ambiguous` verdict for as
long as anybody looked, and it is correct now, and **nothing announced either event**.

The project owner's judgement, in the hundred-and-seventy-fifth session, is that the tree is far
enough along for this to be the work rather than a caveat. It is the last large population where
a defect can live without a name, and **the task, the instrument, the method and the next names
are [todo 00](todo/00-ambiguous-bucket.md)**.

**What it has produced, because that is the argument for keeping at it.** Forty-five sessions,
**fourteen defects found and thirteen of them fixed** — the newest being a page this tree drew
*nothing* on, which the ranking rated 0.73 and the step-7 sweep found at −1.783 (§12.5.6.4's text
annotation attached to a point) — — a page one that was page two (ADR 0148), a
photograph rendered black (0149), a shading painted as a square (0150), a stencil that drew
nothing (0151), a whole grid that disappeared (0154), a sentence drawn as one Greek letter
because the font's name ends in the word "Symbol" (0158), a stamp's gradient painted flat
(0160), a widget's border losing a fifth of its ink to a clip on its own edge and a comb field's
separators losing theirs to a miter bound (0165), a `loca` whose offsets descend so that 36 of
one font's 71 glyphs were refused in silence (0170), **§8.7.4.5.4's greatest *admissible*
root** — found in the two-hundred-and-sixth session, fixed in the two-hundred-and-thirty-second
on all three backends at once (0171), and the longest-standing of them because every gradient
library gets it wrong the same way — **a blurred word nobody drew** (0173): §8.6.8's
uncoloured restriction was still in force inside a soft mask's own group, so a `d1` glyph
procedure that set a `/Luminosity` mask had its mask evaluated to zero and painted nothing, with
every command present and nothing reported — and **a space that was a bar** (0174), where the
`loca` repair of sixteen sessions earlier read a glyph's length from its own bytes even where
the table said, in the standard's own spelling, that the glyph was empty.

Beside them: a pattern cell's clip worth 15% of a page's ink (0155), ten documents whose
substituted font drew none of its characters in silence (0152), the coverage rule that made
eight of them draw (0153), and a font program that draws nothing now saying so (0157).

**The eleventh is found and not fixed**, from the two-hundred-and-fifteenth: a stroke under a
pixel wide loses the half of `tiny-skia`'s hairline smear that falls outside the raster's top
edge, so `vertical.pdf`'s two hairlines carry 55% of their area at the page's top and 98%
everywhere else ([todo 11](todo/11-shapes-that-still-disappear.md) item 3). The bucket itself
went 754 → **0** undiagnosed and all 786 pages carry a diagnosis; *eleven defects nobody could see* is
the number to watch.

**And the three-hundred-and-seventy-ninth took the last five, none of them a defect either, and
each by a different mechanism** — two on §10.7.4's glyph edges where `issue4665.pdf` is the first
page in the bucket on which *all four* references converge on one number (four ladders within 0.044
of 255, three within 0.009); one on §9.7.4.2's own closing sentence, with the half that clause does
**not** leave open checked at 8× to the pixel; one where 111 of a Type 3 font's 114 glyph
descriptions paint themselves white and §9.6.4 Table 111 takes the colour away, so the two readings
differ by a blank page; and one where `ghostscript` prints *An embedded font is invalid* and
substitutes, with the corrupt part of that CFF measured to be the Private DICT's hinting operands,
which carry no outline. **The instrument it added is for a ladder that does not converge**: a
reference's excess divided by the ink a one-pixel erosion removes is an outward offset, and
`ghostscript`'s triples in device pixels while holding at 0.040 ± 0.004 *points* — user space, so a
different shape rather than a different sampling.

**And the three-hundred-and-seventy-second took three names with no defect among them, which is
the outcome worth describing anyway** — because two of the three replaced a group's *argument*
with **arithmetic**. `bug1889122.pdf` is one stroked rectangle whose ink can be written down
(`150 × 22 − 148 × 20 = 340` square points over 19 635 pixels, 4.4156 of 255), and ours is 0.05%
over it where `ghostscript` is 26.7% over and `hayro` 17% under —
`AMBIGUOUS_WIDGET_BORDER`'s sentence for the sixth time and the first time against a number rather
than a limit. `issue4379.pdf` places a stencil-masked image at an exact two-to-one reduction onto
integer device coordinates, so §10.7.4's sampled-image paragraph names one raster sample by sample:
`ghostscript` reproduces it on **all** 500 990 pixels and this tree departs on **3 927**, which is
ADR 0025's stated cost measured on a real page for the first time — invisible to any ink
measurement, since the five renderers agree to 0.023 of 255 there. `issue14953.pdf` declares
`0 0 0 0` for its Type 3 font box and for all fifteen of its glyphs, and a synthetic A/B that
differs only in `d1`'s four operands shows `ghostscript` drawing nothing above 72 dpi and `poppler`
losing the glyphs as the pixels shrink, while this tree and `mupdf` are byte-identical across the
pair — §9.6.4 Table 111's "the result is implementation-dependent" with the implementations
separated. **Its by-product is the round's spec-track item**: §9.2.4's and §9.6.4's ledger rows both
attributed to Table 111 a permission ("a processor may make no assumptions") that Table 111 does not
contain and Table 110 states only for an all-zero *font* box. Both corrected.

**Step 6's own assumption failed for the first time in the two-hundred-and-sixteenth**, on
`issue2177.pdf`: the closed form takes a reference to eight times the resolution because a
renderer's departure from the geometry shrinks with the pixels, and `poppler` on a §8.7.3 tiling
pattern goes the other way — 34.15 → 18.03 → 16.32 from 72 to 2304 dpi, its strokes thinning
rather than its edges sharpening. Ours is flat across four scales and `mupdf` at 8× agrees with
us to four significant figures. **A limit is only a limit if the thing taking it is converging,
and one ladder cannot tell convergence from drift** — take two.

**And the two-hundred-and-fifteenth session cleared the whole ranking above 1.6 in one sitting —
seven pages — which is a result about the *list* rather than about any page.** Two were a face
nobody ships and where §9.8.1 puts the answer, two were one word on a page the size of a postage
stamp, two were hairlines, one was an eight-bit ramp on a stamp fixed sixteen sessions earlier.
**The top of the ranking is populations now rather than defects**, and the one new defect in it
came from a synthetic ladder rather than from a reference.

**And the ninth was a correction rather than a finding, which is why it is here.** Two of the
eight above quoted an ink table that was **half** ours and `hayro`'s and whole for the three C
references, because the method file's own command averaged an alpha channel in — a defect
session 161 found, fixed and wrote down in two places, neither of them the file a session reads
when it goes hunting. Both ADRs carry the correction, the recipe is repaired, and there is a new
closed form beside it: the same page at eight times the resolution, which is what says *which*
renderer is measuring area. ADR 0163.

**And the eighth was a gate rather than a page.** `jp2k-resetprob.pdf` sat first on the ranking
at 5.03 and its name is a JPEG 2000 coding option; checking that hypothesis meant decoding every
`JPXDecode` stream in the corpus against ISO/IEC 15444-5's reference software, which **ruled the
codec out for that file and found thirteen of the thirty codestreams wrong** — every one of them
on the irreversible 9/7 path, by up to 87 levels of 255. Four codecs reach this tree through
dependencies and only two of them had ever been checked against anything. ADR 0161,
`doc/JPEG2000_FEEDBACK.md`.

**The seventh was the ranking's own first name.** `issue7821.pdf` sat at 5.44 bounds with a
stamp whose rounded box looked like a plausible flat green fill and is a shading pattern in four
other renderers: an annotation's appearance stream is a form XObject, and §8.7.2's rule about
where a pattern's matrix points was applied on the `Do` path and not on the appearance path, so
the axis landed off the page and `/Extend` painted one colour. **§8.7.2's ledger row has now been
wrong twice about the same sentence, once per way of becoming a parent content stream.** ADR
0160.

**And the sixth was found by a comment rather than by a number.** `issue8697.pdf` was on the
text gate's list with a paragraph explaining that its readback was a question about §9.10.2 and
that "both readbacks are defensible" — four true sentences about the readback, none of which
asked why the page was in Greek. The defect was in font *substitution* one stage upstream, and
the gate that could see the symptom had closed the question downstream of it. ADR 0158.

**Two of those are worth repeating here because they are about this file rather than about a
page.** `CONTRADICTED_SUBSTITUTED_FONT`'s comment said two documents drew "the same five kana in
the same places in all four panels" — our panel was white, and the sentence described the
*references'* half of the side-by-side. **A group's comment is a claim about a picture, the
picture is one `Read` away, and no gate can check a comment.** And `ambiguous` is not a measure of
how wrong a page is: `issue13372.pdf` sat at 26.95 bounds inside a verdict that cannot tell a
blank page from a grainy one.
