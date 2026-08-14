# ADR 0359 — The other content streams §7.8.2 names, and the sentence four of them never said

Status: accepted, 2026-08-14. Session 524. Takes the one thing ADR 0356 deliberately left —
`doc/todo/03` §10's closing paragraph — and finishes ADR 0343's sentence. Amends §7.8.2, §8.10.1,
§8.7.3.1, §9.6.4, §11.6.5.1 and §12.5.5's ledger rows.

## The clause, which had already answered this

ADR 0343 made a damaged page `/Contents` draw its prefix and say so, on §7.4.1's two halves — a
reader "shall invoke the corresponding decoding filter or filters to convert the information back
to its original form", and a damaged decode does the first and cannot finish the second. What made
the *prefix* right rather than a fabrication was §7.8.2's first sentence:

> A content stream is a PDF stream object whose data consists of a sequence of instructions
> describing the graphical elements to be painted on a page.

A prefix of a sequence of instructions is a shorter sequence of the same kind. That is the whole
argument, and it was applied to one entry of one table.

**The very next paragraph of the same clause says the argument was never about Table 31**:

> Each page of a document shall be represented by one or more content streams. Content streams
> shall also be used to package sequences of instructions as self-contained graphical elements,
> such as forms (see 8.10, "Form XObjects"), patterns (8.7, "Patterns"), certain fonts (9.6.4,
> "Type 3 fonts"), and annotation appearances (12.5.5, "Appearance streams").

Four more objects, named by the clause, each already drawn from its prefix by this tree and each
silent about it. So this is not a reading to make. It is a sentence four call sites were not
saying, and the round is mostly the work of checking that the transfer is honest for each.

## What each of them is, and whether the prefix rule survives contact with it

The test is trap 5's, in ADR 0356's sharper form: **is a prefix of this thing a smaller thing of
the same kind, and are the marks it makes additive or substitutive?** A damaged font program fails
it — a table directory whose offsets point forward yields glyphs the producer never wrote, standing
*in place of* the right ones (ADR 0343) — and a short sampled function fails it, because its missing
samples are values of a mapping rather than places on a page (ADR 0356). Five answers follow.

**A form `XObject` (§8.10.1)** is "a self-contained description of any sequence of graphics
objects", which is §7.8.2's content stream under another name. Additive: the marks the prefix
carries are in the producer's own places and the ones the damage took are absent. Drawn, reported.

**A tiling pattern's cell (§8.7.3.1)** is stated as a content stream by the clause itself — "[t]he
appearance of the pattern cell shall be defined by a content stream containing the painting
operators needed to paint one instance of the cell". The one thing that needed thinking about is
that the cell is *replicated*, so the shortfall is amplified across whatever area the fill covers.
It is still additive: the shorter cell is repeated at the file's own `/XStep` and `/YStep`, so what
appears is a subset of what the producer asked for and never a substitution. Drawn, reported.

**A Type 3 glyph description (§9.6.4)** is the one where the clause makes the prefix rule *stronger*
rather than merely permitting it. Table 110's `/CharProcs` row: the stream "shall include as its
first operator either d0 or d1 , followed by operators describing one or more graphics objects". So
any prefix carrying a mark carries the glyph's own declaration ahead of it — a truncation cannot
produce a coloured glyph where the file declared an uncoloured one — and Table 110's `/Widths`
rather than the description supplies the advance, so the damage costs marks inside one glyph and
never the position of the next. That is exactly the property a damaged font program lacks. Drawn,
reported.

**An annotation appearance (§12.5.5)** the clause makes a form outright: "[e]ach appearance stream
is a form XObject (see 8.10, "Form XObjects"): a self-contained content stream that shall be
rendered inside the annotation rectangle." Drawn, reported — with one wrinkle described below.

**A soft mask's `/G` (§11.6.5.1)** is the one where the answer had to be *checked* rather than
carried over, and it is the reason this round is not a formality. The group is a form — "[t]he group
shall be defined by a transparency group XObject (see 11.6.6, "Transparency group XObjects")
designated by the G entry in the soft-mask dictionary" — but what its marks become is a *mask value*
multiplying the alpha of other objects, which is the shape ADR 0356 refused for a sampled function.
What separates them is that **this clause states the mask's value where the group painted nothing**:
for `Alpha`, "the mask value shall be the result of applying the transfer function to the input
value 0.0"; for `Luminosity`, the value "derived by transforming the BC colour to luminosity". A
place the damage took is a place the group did not paint, and the clause already answers for one of
those — where a sampled function's missing samples are values it has no answer for and interpolates
into the ones beside them. **Places, not values.** Drawn, reported.

**And one stream in the list `doc/todo/03` §10 gave is not a content stream at all**, which is worth
recording because the list was written from file names rather than from what each site reads.
`type3.rs`'s `decoded_stream_data` is the font's **`/ToUnicode` CMap** (§9.10.3), not its glyph
description — the description is read in `content/text.rs`. A CMap is not "a sequence of
instructions describing the graphical elements to be painted", it costs no mark, and a prefix of one
is a smaller *mapping*, whose missing codes already land in
`Interpretation::codes_without_a_character` as `IncompleteToUnicode`. It gets no report, on ADR
0152's trade: a shortfall in the readback is not a shortfall in the picture, and a report would take
a judged page off the oracle for a page that draws perfectly.

## What was wired, and the one place it could not be wired where it reads

`Interpreter::content_stream` is the whole mechanism: decode through
`Document::decoded_stream_data_reported`, hand back the bytes, and where `Decoded::damage` is set,
`note` an `Unsupported::DamagedContentStream` naming which of §7.8.2's kinds it was, how it stopped
and how many bytes are on the page. Four of the five call sites take it directly —
`content/xobject.rs`, `content/pattern.rs`, `content/transparency.rs`, `content/text.rs` — and each
keeps its own sentence for the case where *nothing* decoded, because there the whole element is
missing rather than the end of it.

**The appearance stream cannot report where it is drawn**, and that is a fact about §12.7.4.3 rather
than about this code. A widget whose variable text a reader has changed is regenerated by *splicing*
new marks into the stored stream's `/Tx BMC` … `EMC` region, so what reaches `draw_appearance` is a
`Content::Constructed` copy with no stream behind it. A report taken at the draw would therefore go
quiet for exactly the annotations a reader has touched. So `annotation::appearance_damage` asks at
the one point that still holds the stream, and `Appearance::damaged` carries the answer to the
drawing, where `draw_appearance` states it. The report is the same variant either way.

**Why a new variant rather than `ContentIssue::Damaged`.** The vocabulary that mattered — `Damage`,
and the argument under it — is reused unchanged; what is not reusable is `ContentIssue` itself, which
is Table 31's noun. Every one of its variants is indexed by a *part of `/Contents`*, and a form
reached through `/XObject` or an appearance reached through `/AP` has no such index. Same sentence,
different subject, so a different word for the subject. `DamagedStream` is the payload both share in
substance.

## Trap 11: the condition, and what it costs

The condition is `Decoded::damage.is_some()` at a site that then *draws* the prefix, which is derived
from §7.4.1 rather than from a guess about when damage might matter — and it fires nowhere else,
because an undamaged decode carries `None` by construction. `content/xobject.rs`'s `Do` is behind
§8.11.3.1's `is_hidden()` already, so a form inside a switched-off layer costs no report. `scn` and
`gs` are deliberately **not** guarded that way: §8.11.3.1 keeps applying graphics-state operations
inside hidden content, so a pattern or a soft mask established there can still paint after the `EMC`,
and a guard would lose a real report to save one that costs nothing.

**What it matched, before the count was trusted.** `examples/damaged_stream_census` grew a line that
interprets *every page* of every document holding a damaged stream and counts the reports naming
damage, split into ADR 0343's route and this one — the before is the total minus the split, from one
run. On the pdf.js corpus: 57 damaged streams over 20 documents, 7 of them form `XObject`s, and this
round takes the tree from naming damage on 1 document to naming it on 4. Over all 65 944 crawled
documents — 145 archives, one process each, 0 failures — the tree now makes **1182 damage reports
over 432 documents, 295 of them one of these four kinds**, so ADR 0343's route alone accounted for
887 and this round adds a third again. The census's older lines reproduce session 521's to the
digit: 2260 damaged streams in 726 documents, the role table unchanged, 54 short images in 8
documents and not one short sampled function.

**295 reports against 46 damaged form `XObject`s** is not an arithmetic error and is the more
interesting number: a report is per page, so one damaged form drawn on forty pages is forty of
them — and the rest are the tiling patterns, glyph descriptions and appearances that the census
classifies as *unclassified*, because a content stream's dictionary carries nothing that says what
reads it. The role table's largest silent bucket turns out to be substantially the thing this round
made loud, which no count in it could have said.

**What it cost, in the units trap 11 asks for.** Three pages left the oracle's judged set — 1694
complete before, 1691 after — and they are named: `comments.pdf` page 1 and `highlights.pdf` page 1,
both `ambiguous` and both listed in `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`, whose entries are deleted
with the reason written beside the group; and `issue3885.pdf` page 1, which `agrees`. The
corpus gate's incomplete list rises from 61 to 64 by exactly those three, and the pdftotext gate's
judged set loses the same three documents — 1180 words, **all of them matched**, so its 179 unmatched
words are the same 179 before and after and the rate's move from 99.3% to 99.2% is arithmetic on a
smaller denominator rather than an extraction that got worse.

**And the one thing that costs work rather than a judged page was measured rather than argued.**
`annotation::appearance_damage` decodes a stored appearance stream that `draw_appearance` then
decodes again, once per annotation per interpretation — a second `decoded_stream_data_reported`,
which is a cache hit plus the filter chain it is keyed on. Interpreting all 974 corpus first pages
(`examples/display_list_digest`, five runs each side, discarding the two that were warming the page
cache) is 47.5–54.1 s with the change against 48.4–52.1 s without: the same distribution, so the
second decode does not show. It is kept because the alternative loses the annotation's subtype from
the report or splits one kind of report across two sites.

**Nothing drawn moved**, and the artefact says so rather than a summary: `examples/display_list_digest`
over all 974 pdf.js documents is byte-identical across the change, and the three witnesses' page-one
PNGs are byte-identical too. So `doc/todo/00` step 7 needs no re-run, on ADR 0343's and 0356's
reasoning — the ink ranking reads the oracle's artefacts and its input did not move — and no quorra
lane can have moved either.

## The witnesses, looked at

`comments.pdf` and `highlights.pdf` are the same PLDI paper with a reader's markup over it. Object
694 is the form an ink annotation's appearance invokes; its flate stream inflates 851 bytes and then
ends, and the last thing in those bytes is

```text
…484.611604 678.484869 503.113673 680.392741 c
511.914028 681.300204 520.883831 684.100611 529.687863 684.004067 c
S
```

— a completed stroke. The green loop drawn round the paper's title stops where the producer's stylus
data stops, and until this round the page said nothing at all about it. `highlights.pdf`'s object 667
is the same shape one operator over, 648 bytes ending in a completed `f` for a highlight quad. That
is trap 5's own sentence in a picture: a page cut short is otherwise indistinguishable from a page
meant to be sparse.

## What is left

The census's role table still has populations nothing says anything about, and they are now the
*unclassified* 296 and the object-stream 144 rather than any content stream. `doc/todo/03` §10
carries what each would need.
