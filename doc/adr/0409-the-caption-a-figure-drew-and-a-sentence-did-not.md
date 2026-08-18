# ADR 0409 — The caption a figure drew and a sentence did not

Status: accepted.
Session: the five-hundred-and-seventy-fourth, a general improvement round.

## 1. What this decides

§12.5.6.7's `/Cap` is drawn. Until this round a line annotation with `/Cap true` drew its line and
reported the caption, on the sentence

> §12.5.6.7's /Cap asks for /Contents as a caption, and no entry gives it a font

which had stood since the hundred-and-sixteenth session (ADRs 0075, 0106, 0192). The sentence is
true. The inference from it is not, and the clause says so in the same table cell.

Four things follow, and only the first is about this entry:

1. **Table 178's `/Cap` is a `shall` about the appearance**, so the silence about a font sits
   *inside* a requirement to make the mark. That is ADR 0109's test — the question a silence poses
   is not whether a reader may fill it but whether a sentence around it requires one to — and it is
   the same shape §12.7.5.4's list box was refused on one round earlier (ADR 0407).
2. **`/CP` and `/CO` are read with it**, and between them they state the caption's position with
   nothing left to invent. `/CO`'s own wording is what makes the construction the line's rather
   than the page's.
3. **Figure 81 rejected a reading of the clause**, which is trap 1's rule arriving from the
   standard's own pages rather than from a corpus document: the first implementation auto-sized the
   caption to the line's length, and the figure the entry cites by name draws a caption longer than
   its line at the same size as the others.
4. **`variable_text::LaidOut` gains an `advance`**, because Figure 81's inline caption sits in a
   *break* in the line and where that break goes is a question only the layout can answer.

No document in any population this project measures states `/Cap true`, so the witnesses are
hand-built. That is trap 8's case and §12.5.6.6's `/CL` precedent (ADR 0329).

## 2. What the clause states

ISO 32000-2 §12.5.6.7, Table 178, on `/Cap`:

> If true , the text specified by the Contents or RC entries shall be replicated as a caption in
> the appearance of the line, as shown in "Figure 81 - Lines with captions appearing as part of the
> line" and "Figure 82 - Line with a caption appearing as part of the offset". The text shall be
> rendered in a manner appropriate to the content, taking into account factors such as writing
> direction. Default value: false .

On `/CP`:

> ( Optional; meaningful only if Cap is true; PDF 1.7 ) A name describing the annotation's caption
> positioning. Valid values are Inline , meaning the caption shall be centred inside the line, and
> Top , meaning the caption shall be on top of the line. Default value: Inline

And on `/CO`:

> The first value shall be the horizontal offset along the annotation line from its midpoint, with a
> positive value indicating offset to the right and a negative value indicating offset to the left.
> The second value shall be the vertical offset perpendicular to the annotation line, with a positive
> value indicating a shift up and a negative value indicating a shift down. Default value: [0, 0] (no
> offset from normal positioning)

**`/CO` is what says the caption turns with the line.** An offset measured "along the annotation
line" and "perpendicular to the annotation line" is a statement in a frame whose axes are the
line's; the page's axes appear nowhere in the entry. So the caption is written under a `cm` whose
first four operands are the line's direction and its perpendicular, and `/CO` moves the origin
inside that frame. Nothing here is a choice.

**What the clause does not state is a font and a size**, and that is the whole of the silence the
old refusal was about. It is the silence §12.5.6.4's icons have — a `shall` for the mark and not a
line of artwork — and `CLAUDE.md` says what to do with one: make a deliberate choice and document
it *as a choice*.

## 3. The two choices

- **The face is §9.6.2.2's Helvetica, out of this binary.** It is not a new mechanism: the caption
  is laid out through §12.7.4.3's own routine with a `/DA` of `/Helv 12 Tf`, and `/Helv` is one of
  the fourteen abbreviations `variable_text` already resolves to a compiled-in standard face where
  a document's `/DR` defines nothing (ADR 0112). What that buys is ADR 0133's argument: the caption
  is drawn from the binary rather than from whatever face this machine has installed, so the page
  reproduces where no fonts are.
- **The size is 12 points**, and the number is not picked here. It is what §12.7.5.3's own EXAMPLE
  sets variable text at — the same worked example `variable_text::LINE_HEIGHT` takes its 13/12 line
  spacing from. The standard states one worked size for laying annotation text out and this is it.

Two things that look like choices and are not. The **colour** is Table 166's `/C`, which is what
the line itself is stroked in: the clause makes the caption part of "the appearance of the line",
and Table 178 reserves `/IC` for "the annotation's line endings". Figure 81 draws its captions in
the line's colour. And **which way along the line the text reads** is settled by taking the sense
that leaves it the right way up on the page, because the clause fixes the axis and says nothing
about the sense; a line drawn right to left would otherwise carry its caption upside down.

## 4. What Figure 81 settled, and it is the round's own lesson

The first implementation auto-sized the caption to fit the line's own length, on an argument that
reads well: both of `/CP`'s values put the caption *on* the line — "centred inside the line", "on
top of the line" — so the line's length is the only extent the clause gives the caption, and
§12.7.4.3's auto-sizing is the standard's own name for computing a size from a box. It was wrong,
and the thing that said so is the figure the entry cites by name.

Figure 81 draws three lines. The third is captioned *This is a caption that is longer than the
line* and is set at the same size as the other two, overhanging both ends. So the size does not
depend on the line at all, and the sentence that would have derived it was deriving something the
standard does not do.

The same figure settles two more things the sentence leaves open:

- **An inline caption sits in a break in the line**, not over it. The figure's first example draws
  the line in two pieces either side of the words. Drawing the full line under an inline caption —
  which is what the first implementation did — puts a rule through the middle of the text.
- **The caption is the line's colour**, which is the reading `/C` gives above and which the figure
  confirms rather than establishes.

**The transferable half is about where a picture comes from.** Trap 1 says the metrics lie and to
look at the page; every instance of it in this tree so far has been a page *this program drew*.
Here the page that rejected the reading was **the standard's own**, sitting inside `doc/md/` as a
base64 image nobody had extracted, two lines below the sentence three rounds have quoted. A clause
that cites a figure by name is a clause whose figure is part of what it states, and this project
reads `doc/md/` constantly and had never once looked at one.

## 5. The construction

`appearance::caption` is the whole of it, and it runs *before* the line is stroked — which is the
only structural change to `appearance::line`, whose three exits become one.

1. `/Cap` must be `true`. `/Contents` is taken through §12.5.6.2's group source, falling back to
   `/RC` exactly as §12.5.6.6's free text does. A `/Cap true` stating neither owes nothing: the
   entry replicates text this annotation does not have.
2. `/CP` and `/CO` are read. A `/CP` outside the table's two names and a `/CO` that is not two
   numbers are **reported**, and the line is still drawn — Table 178 tolerates no third placement
   the way Table 168 tolerates a third border style, so a name outside the pair states no position
   and drawing at the default would put a mark where the file did not ask for one.
3. The frame is built from the line **proper** — with `/LL` present that is not `/L`, for the same
   reason ADR 0192 puts the endings there — and `/CO` moves its origin.
4. **The layout runs twice, and the first pass draws nothing.** §12.7.4.3's layout clips to the box
   it is given, and the box that does not clip this caption is the caption's own width, which is
   not known until it has been laid out. So the first pass measures and the second is kept.
   `LaidOut::advance` is what the first pass reads: the sum of the advances the glyphs are
   positioned by, rather than a second opinion about where they are.
5. The line is stroked in the pieces the break leaves, then the leaders, then Table 179's endings,
   then the caption.

Two rules bound the break, and both are ADR 0106's:

- **A break that would leave no line is not taken.** §12.5.6.7 makes the line required, so a
  caption wider than the line it sits on gets the whole line under it. That is the honest outcome
  rather than a good-looking one: the file has asked for a caption that does not fit inside its
  line, and the alternative is deleting a mark the clause requires.
- **A `/CO` that lifts the caption clear of the line takes the break with it**, which is Figure
  82's case. The entry offsets the caption "from its normal position", and the break exists because
  the words occupy that stretch of the line; where they no longer do, nothing is in the line's way.

## 6. What moved

Nothing in any gate, and that is expected rather than disappointing: `examples/witness_census`
finds **two** documents on this disk stating the key at all, both of them pdf.js line-annotation
fixtures, and both write `/Cap false`. The corpus, oracle, text, quorra, date and XMP gates are
unchanged. Two tests are added and one is rewritten — the one that asserted the refusal, which is
`doc/habits.md`'s rule that a test pinning a refusal must be rewritten when the refusal ends.

## 7. What this leaves

- **`/IT` and `/Measure` are still unread** on this subtype, and §12.5.6.7's row says so.
- **A figure is evidence and this tree has never read one.** `doc/md/` carries them inline as
  base64; extracting one is four lines of Python. Every clause that says "as shown in Figure N" is
  a clause whose reading is incomplete without it, and this round found one such reading wrong on
  its first try. Whether that is worth a sweep is a question for a later round; what is worth
  recording now is that the instrument exists and costs nothing.
