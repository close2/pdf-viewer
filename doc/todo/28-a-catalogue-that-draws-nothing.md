# A 92-page catalogue whose pages draw nothing, and the four things it asks for

Status: **the image is decoded and the catalogue draws, since the three-hundred-and-fifty-fourth
session** (ADR 0203). **Item 2 is finished**: its clause was taken in the three-hundred-and-eightieth
(ADR 0217) and both of the residues that left behind were paid in the three-hundred-and-eighty-third
(ADR 0220), so a mask group in this file now composites in the quantity §11.5.3 states whatever it
is painted with. **Half of item 3 is finished too**: the two §11.4.4 NOTE 5 groups this file prints
are still owed, but §11.4.6's shape closed in the three-hundred-and-ninety-seventh (ADR 0234), so a
knockout group here whose element is a nested group or carries a soft mask now draws. What is left of
item 3 is `doc/todo/23`'s, with this document as its witness. Measured in the
three-hundred-and-thirty-sixth, from a document the project owner opened and the notes it printed.
Priority: 28
Corpus: **0** — and that is the point. This is a real document from outside the pdf.js corpus, and
the gates cannot see it.
Clauses: §7.4.8 (`DCTDecode`), §8.9.5.1, §8.6.4.4, §11.4.7, §11.6.6, §7.7.2
Code: `crates/pdf-model/src/image.rs` (`convert_channels`, `decode_jpeg`),
`crates/pdf-model/src/content.rs`, `crates/pdf-model/src/soft_mask.rs`
Witness: `/tmp/katalogs_2023_web.pdf` (19.9 MB, 92 pages) — **not in this repository**; the owner
has it.

## 1. ~~The page draws *nothing*~~ — **closed in the three-hundred-and-fifty-fourth session**

`decode_jpeg` asked `zune-jpeg` for four components out only where the input space was `CMYK`, so
a `YCCK` codestream fell through to the default three and `convert_channels` refused. It now asks
for whichever four-component space the codestream states, and `ycck_to_cmyk` does Adobe's
transform 2 here — because `zune-jpeg` has no YCCK → CMYK conversion at all, only two YCCK → RGB
arms that composite the black channel away. The inversion is deliberately **not** undone: an Adobe
four-component JPEG stores CMYK inverted whichever transform it uses, and the file's own
`/Decode [1 0 1 0 1 0 1 0]` is what undoes it one step later. ADR 0203.

On page one at 72 dpi: ours 222.689, `poppler` 223.742, `mupdf` 224.255, and **ours is 0.0113 from
`poppler` where `mupdf` is 0.0384**. Pages 1, 2, 5, 10 and 40 all report `unsupported []`.

## What it looked like before

The viewer prints one line about an image:

> an image (Im0: colour space a 4-component space on a JPEG of 3 components is not supported) was
> not drawn

What that line does not say is that **the image is the whole page**. `open_one` on pages 1, 2, 3, 5
and 10 reports **0 commands** and that one refusal apiece, and `render_at` on page 1 produces a
raster with an ink of **0.000** — a blank sheet. The reader sees an empty window and a note about
an image.

**What the image is**, from the file itself:

```text
pdfimages -list   1757 × 2489, cmyk, 4 components, 8 bits, jpeg, 300 ppi, with an /SMask
the codestream    SOF0: 4 components, 1757 × 2489
                  APP14 Adobe, transform = 2
```

Adobe's transform 2 is **YCCK** — a four-component JPEG whose first three channels are a
luminance-chrominance transform of C, M and Y with K carried alongside. So the file's own
codestream states four components, and this tree's decoder handed `convert_channels` **three**,
which is why the space and the codestream "contradict each other" and the image is refused by
name.

**The first question is therefore about the decoder rather than about the clause**: what does
`decode_jpeg` do with an Adobe transform of 2, and does the three it reports come from the frame
header or from a colour transform applied on the way out? §7.4.8 sends the codestream's syntax to
ISO/IEC 10918, and Table 13's `/ColorTransform` is the entry that talks about this — the same entry
this project has *one* corpus witness for, and whose witness contradicts the clause (the handover's
closed-by-decision list). A four-component YCCK JPEG is the case that list does not cover.

Until it is answered, every page of a 92-page catalogue is blank, which is the strongest single
piece of demand this project has ever had from outside its corpus.

## 2. §11.4's transparency departures, with a witness the owner can see

The same document prints the four populations `doc/todo/23` names, on one page:

- a soft mask's group composited in device RGB and its luminosity taken there, rather than in the
  blending colour space its `/CS` names (§11.6.6) — **twelve times**;
- a blending colour space of `/DeviceCMYK` (§11.4.7) — **four times**;
- non-isolated, and an element blends with the backdrop it excludes (§11.4.4's NOTE 5) — **twice**.

`doc/todo/23` counted these over the pdf.js corpus at 19 documents and said the first question is
"what would a backend have to be handed". This document is what that work is *for*: a commercial
catalogue in CMYK, which is the case where compositing in the wrong space is visible rather than
theoretical.

**The first of the three is finished, in two rounds, and the third is finished in one more.** The three-hundred-and-eightieth paints a
`/Luminosity` mask group whose blending space is subtractive in the ink §10.4.2.3 weighs and
composites it there (ADR 0217); the three-hundred-and-eighty-third scales that channel so the
clause's `min` can wait for the compositing, and carries an image's samples and a shading's ramp
into the same quantity (ADR 0220). All three sentences the twelve reports were worded in have been
deleted from the tree, so **the expected result of a run over this file is that the twelve are
gone** — and that is now a check rather than a question. Two things would still be printed and
neither has a corpus member: a `Lab` mask group, and a blend mode inside a `/DeviceCMYK` one.

**And the two §11.4.4 NOTE 5 lines are expected to remain while the knockout ones are not.** ADR
0234 states a knockout element's shape apart from its alpha, so a group here whose element is a
nested group or carries a soft mask draws rather than reporting — which is a *check* rather than a
question, exactly as the twelve above became one. Two things would still be printed and both are
this file's own: §11.4.4's non-isolated pair, and §11.6.4.3's `/AIS` if this producer sets it, which
now refuses a knockout group by name. The catalogue is CMYK commercial work, which is where a
producer is most likely to set it.

**The measurement nobody in this tree can take is still worth taking**, because the file is the
owner's and the pdf.js corpus's mask groups turned out to be grey artwork over ordinary backdrops
almost without exception. A commercial catalogue in CMYK is where a registration-black backdrop and
a CMYK image inside a mask are most likely to be, and it is the only witness this project has for
whether the fixtures measured the real case. The other one, §11.4.7's `/DeviceCMYK` blending space,
is untouched and stands exactly as below; §11.4.4's NOTE 5 is untouched too and is now the only
structural departure this file's groups are expected to print.

## 3. §7.7.2's `TwoColumnRight`, said once and correctly

> this document asks for the TwoColumnRight page layout (§7.7.2); this window shows one page at a
> time

That is the right sentence and it is not a defect: Table 29's `/PageLayout` is handed to the host
and this host has one page. It is here because a catalogue is exactly the kind of document that
means it — a spread is how the pages were designed — and a host that showed two pages would need
the layout, the scroll and the page-turn arithmetic to change together.

## What "showing this PDF correctly" means, in order

1. **The image.** Decode a four-component JPEG whose Adobe marker says YCCK, and hand
   `convert_channels` four components. Until then the document is blank.
2. ~~**The soft masks.** §11.6.6's blending space inside a mask~~ — **done**, in the
   three-hundred-and-eightieth session (ADR 0217) and the three-hundred-and-eighty-third, which paid
   both of the residues the first left behind (ADR 0220).
3. **The groups.** §11.4.7's `/DeviceCMYK` blending space, which is the same change one level out.
4. Nothing else. The layout note is a statement about this host, and the file is otherwise read.
