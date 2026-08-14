# ADR 0356 — The samples a stream stops short of, and the three consumers §9 left owed

Status: accepted, 2026-08-14. Session 521. Takes `doc/todo/03` §9's remaining chunk — the damaged
streams ADR 0343 did *not* make loud — and answers it per consumer, which is the shape §9 said the
answer would have to take. Amends §7.3.8.2, §7.10.2, §8.6.5.5 and §8.9.5.1's ledger rows.

## What §9 asked, and why the answer is not one rule

ADR 0343 made a damaged `/Contents` loud on §7.8.2's reading — a content stream is "a sequence of
instructions", so a prefix of one is a shorter sequence of the same kind — and refused a damaged
font program on the opposite reading, because a font program is a table directory whose offsets
point forward. §9 recorded the rest as owed and said what each would need: *is a prefix of this a
smaller one of the same kind, and are the marks it makes additive or substitutive?*

**The question turned out to have a better predicate than damage.** For two of the three consumers
the standard states the extent of the stream independently, and a stream that falls short of it is
wrong whether a filter failed or a producer simply wrote too little. §7.3.8.2:

> Finally, streams are used to represent many objects from whose attributes a length can be
> inferred. All of these constraints shall be consistent.

and its EXAMPLE is an image, with the arithmetic and the verdict both stated: a 10-row, 20-column,
8-bit, one-component image "requires exactly 200 bytes of image data", and "[a]n error occurs if
Length is too small, if an explicit EOD marker occurs too soon, or if the decoded data does not
contain 200 bytes."

## The three answers

### An image draws what it carries, and says what it does not

**The defect.** `unpack` took each row as `data.get(row_range).unwrap_or_default()`, so a row past
the end of the data was an empty slice and every sample in it read back as **zero** — black in
`DeviceGray` and `DeviceRGB`, and *marked* for an `/ImageMask`, whose §8.9.6.2 default `/Decode`
paints where the sample is 0. `178360.pdf` in `govdocs1-error-pdfs` is the witness: a 133 × 2944
stencil whose flate stream is corrupt 359 bytes into the 50 048 its grid needs, so **99.3% of that
image was marked in the fill colour** — a solid bar down the page that `pdftoppm` does not draw.

**The fix, and why it is the prefix rule rather than a refusal.** The samples the file carries are
at exactly the positions the grid gives them: rows are byte-aligned (§8.9.5.1, "[e]ach row of
samples shall begin on a byte boundary"), so a sample's place does not depend on any sample after
it. They are the producer's own bytes in the producer's own places, which is ADR 0343's condition.
What is not the producer's is everything past the end, and that is now left **unpainted** rather
than painted zero. The bound is on whole samples: a sample whose last bits are past the end was
never written, and half of one is not a colour.

**And it reports**, because trap 5's test is that suppressing either statement loses information: a
page missing the bottom nine tenths of a picture is otherwise indistinguishable from a page whose
picture is that shape. `image::short_of_its_grid` is the ninth entry in `doc/HANDOVER.md` trap 5's
list of places that report *while* drawing.

### A sampled function is refused

§7.10.2 states the same constraint for its own object, in one sentence:

> The stream data shall be long enough to contain the entire sample array, as indicated by Size ,
> Range , and BitsPerSample ; see 7.3.8.2, "Stream extent".

Here the prefix is **not** a smaller thing of the same kind, and the difference from the image is
worth stating because both are grids of samples. An image's missing samples are *places on the
page* that can be left alone; a function's missing samples are *values of a mapping* that is then
evaluated over its whole domain — the reader answered 0 for them, mapped that through `/Decode`
and interpolated it into the last real sample beside it, so a tint transform or a shading was
evaluated over a function the file never carried, and the marks it made stood in place of the
producer's. Substitutive: refused, and the caller reports it.

### A damaged ICC profile takes the fallback the standard already states

The third consumer needed no new report at all, which is the answer rather than an evasion. Table
65 states the whole recovery for a profile a reader cannot use: `/Alternate` "shall be used in
case the one specified in the stream data is not supported", and where that entry is absent "the
colour space that shall be used is DeviceGray , DeviceRGB , or DeviceCMYK , depending on whether
the value of N is 1 , 3 , or 4". Both were already in `colour.rs`.

What was wrong was one step earlier: a damaged profile was **parsed**. A profile is a tag table
whose offsets point forward — ADR 0343's font-program argument, one clause over — so a prefix of
one is a directory describing bytes that are not there, and `Profile::parse`'s failure modes are
not uniform: an `A2B1` tag past the end drops through to the curve-and-matrix branch, where a
missing `rTRC` reads as `Curve::None` and the profile is *accepted* with a tone response nobody
wrote. `ColourSpace::parse_icc_based` now asks `Decoded::damage` before it parses.

## The population, measured before any of it was decided

`examples/damaged_stream_census` gained a role for every damaged stream — read off the entry the
standard makes required of that object — and the two extent arithmetics. One process per archive,
145 archives, 0 failures; its three older lines reproduce ADR 0343's numbers exactly, which is what
says the instrument did not move under the change.

**The 2260 damaged streams of the crawl's 65 944 documents, by the consumer that reads them:**

| consumer | damaged streams | what happens today |
|---|---|---|
| a page's `/Contents` | **841** | drawn and reported (ADR 0343) |
| an image | **529** | drawn; reported where it is short of its grid |
| a font program | **371** | refused and reported (ADR 0343) |
| unclassified | 296 | a Type 3 glyph, an `Indexed` palette, an appearance |
| an object stream | 144 | objects that parse are read; §7.5.7 |
| a form `XObject` | 46 | drawn, and **still silent** — see below |
| an ICC profile | 19 | Table 65's alternate, deliberately |
| a cross-reference stream | 10 | the recovery scan; §7.5.8 |
| a metadata stream / a function | 2 / 2 | — |

**§9's own arithmetic was pessimistic and this is the correction**: it read the loud route as 90 of
2260, "about 4%", because 90 is the count of *page-one* `/Contents`. The route is loud for all 841
whenever the page holding one is drawn, which is 37%.

**Short of the extent §7.3.8.2 infers, which is the other population:** **54 images in 8 of the
65 944**, 51 in 2 of `format-corpus`'s 167, **0 of the 974**; and **not one** short sampled
function anywhere on this disk. The two populations overlap and neither contains the other — a
damaged image whose data still covers its grid loses nothing that shows, and a short image whose
stream decoded cleanly is a producer's own arithmetic being wrong.

## What it cost

**Nothing drawn moved in the gate corpus**, and the artefact says so rather than a summary number:
`examples/display_list_digest` over all 974 pdf.js documents is **byte-identical** before and
after, 958 interpreted first pages. That is the expected result and it is worth having in writing,
because it is the census's prediction — no document of the 974 is short of a grid, carries a
damaged ICC profile or states a short sample table. The corpus gate's incomplete count is 61 before
and 61 after, measured both ways.

`doc/todo/00` step 7 needs no re-run for the same reason ADR 0343 gave: the ink ranking reads the
oracle's artefacts and its input did not move.

## The one thing this deliberately does not take

**A damaged form `XObject`, tiling pattern, appearance stream or Type 3 glyph description is still
silent**, and §7.8.2's argument for the page's `/Contents` applies to every one of them word for
word — they are content streams, and this tree already draws their prefixes. 46 of the crawl's
damaged streams are form `XObject`s and 7 of the pdf.js corpus's 57 are. It is left because it is a
*report* to place in five call sites rather than a reading to make, and mixing it into a round that
changed what gets drawn would have made the digest above prove less. `doc/todo/03` §10 carries it
with the count and the clause.
