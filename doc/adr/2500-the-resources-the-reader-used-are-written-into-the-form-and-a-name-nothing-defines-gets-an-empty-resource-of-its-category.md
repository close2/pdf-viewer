# 2500 — The resources the reader used are written into the form, and a name nothing defines gets an empty resource of its category

Status: accepted. Session 1833, batch one hundred and twenty-four, the archive slot for ADR 2490's
R04. Context: ADR 2490 (the remedy and its place in the build order), ADR 0255 (a form with no
`/Resources` is drawn with the page's), ADR 0816 (no remedy edits a byte of a content stream), ADR
0947 (a shape a remedy cannot carry out is a refusal naming it, never a partial fix), ADR 1014 and
ADR 2394 (the appended page and the photographed page, where a loss is stated instead),
`doc/questions/A681` item 1, `doc/questions/A694` item 3, `doc/pdf-a-mitigations.md`
(`graphics/named-resources-are-defined`, `graphics/content-streams-have-an-explicit-resources-dictionary`).

## What is built

ISO 19005-2 and ISO 19005-4 section 6.2.2 ask a content stream's resource dictionary to be explicit
and, on `TechNote 0010` A002's reading for part 2, to define every name the stream uses. Two
rewrites answer them without changing a content-stream byte.

1. **`assign`** (`Rewrite::PageResources`, widened). A form XObject, a tiling pattern or an
   annotation appearance that names resources and states none is given the dictionary of the page it
   is drawn on: the reference where the page's is an object, a copy where it is direct. ISO 32000-2
   §7.8.3's last bullet ends the search for such a stream at the page's resources, so no name
   resolves differently. A page that inherits a direct dictionary from an ancestor gets its own copy
   first.
2. **`supply`** (`Rewrite::EmptyResourceDefined`, new). A name a stream used and its resources did
   not define is defined as an empty resource of its category, which draws the nothing this reader
   drew: an `ExtGState` as `<< >>` (§8.4.5, Table 57 lists every entry optional); an `XObject` as
   one shared form whose `/BBox` has no area (§8.10.2, Table 93: the form is clipped to it); a
   `Font` as one shared Type 3 font whose one glyph procedure is `0 0 d0` (§9.6.4, Tables 110 and
   111) and whose 256 widths are zero. The two empty objects are written once and take the spare
   object numbers. Both classes are `Mechanical`, loss none, under both profiles and both targets.

The survey records the plan's population: `Survey::missing_in_streams` is one record per stream and
name (the existing `missing_resources` is one per page and name and is unchanged), and
`Survey::moving_shows` is each text-showing operator that ran under a missing font name and did more
than show glyphs.

## Where the empty font is refused, and why

ISO 32000-2 §9.3.1's Table 103 says only that `Tf` has no initial value and has to be given before
any text is shown; what a show with no font in force does is this reader's choice (ADR 0255): it
draws nothing and reads none of the string. A name no dictionary defines and that is one of the
standard fourteen (§9.6.2.2) is the exception: this reader draws it from its own faces, so it gets
a plain `/Type1` font of that `/BaseFont`, never the empty one. The file that results does not
embed that font, and the embedding remedy is chosen from the requirements the source failed, which
this one was not, so such a file is refused by the output check (`fonts/font-programs-embedded`)
and not written without its text. What is owed is a second pass of planning over the definitions
this remedy added: the `/Type1` definition raises `fonts/font-programs-embedded` itself, and the
existing standard-fourteen embedding route then embeds the shipped face as it does for a font the
source declared. A Type 3 font of width zero reproduces the choice for a plain show. It does not where the show moves the position by
more than the widths (§9.4.4: the displacement includes `Tc`, `Tw` and a `TJ` adjustment), nor
where a clipping render mode adds an empty glyph set to the clip. The survey therefore tracks `Tc`
and `Tw` through `q`/`Q` and the forms that inherit them, and records a show as moving where `Tc` or
`Tw` is nonzero, a `TJ` number is nonzero, the operator is `'` or `"`, or `Tr` is not 0 to 3. Such a
font stays refused by name. The refusals are conservative: the reader does move the position by a
`TJ` adjustment and by the line a `'` starts with no font in force, and an empty font would move it
by a different amount, so the case is refused and not lifted. `"` sets both spacings from its
operands, and the survey sets both flags from them.

Refused by name, each with its sentence in the report:

| shape | why |
|---|---|
| a colour space, a pattern or a shading | the standard has no empty value of them; a pattern that paints nothing and a shading that covers nothing are inventions of what the producer meant |
| a font whose text moved more than its widths | above |
| a Type 3 glyph procedure's names and its missing `/Resources` | the search ends at the font dictionary's, shared by pages and procedures; the construction follows a glyph procedure to its font and is owed |
| a form reached from pages whose resources differ | each page resolves the names to other objects; the elegant remedy is a copy of the form per page, owed |
| a name looked up by a soft mask's group | held resolved, with no dictionary of its own to address |
| a survey that stopped at a bound | the names and streams it found are not all of them |

Under `2u` and `2a` the empty font has no `/ToUnicode`, so the output check holds the file to the
target and refuses it by name; that is the check working and nothing here relaxes it.

## The premise, measured

ADR 2490 counted 22 documents under R04 at both targets as an upper bound. The CLI over those
22 at 2b and 4, under both profiles, writes 11 and refuses 11. Of the 11 written, 7 needed no font
(`meeram_1`, `operator_list_cycle`, `issue6541`, `DefaultRGBColourSpaces-inherit`,
`PDFBOX-4372`, `bug1873345`, `issue1055r`) and 4 the empty font, one of them through a page that
inherits a direct dictionary. The 11 refused each carry a second reason the remedy does not touch: the operators and implementation limits of ADR 0816's fence (6),
the glyph, route and `/Differences` rules of ISO 19005-2 section 6.2.11 (4), or a Type 3 glyph
procedure (1); 4 of the 6 behind the fence are also refused for a font that moved. The 22 is a
ceiling and 11 is what the remedy reaches today.

Against HEAD's census with the CJK pack, each of the eight census lines moves by the same documents
and no document is newly refused: as-if-printed and as-shown, 2b and 4, 5 of pdf.js's and 6 of the
corpora's documents convert that were refused (the no-pack pair was not taken; the 22-document CLI
runs above are without the pack and agree).

## What remains, per the owner's stance of 2026-10-27

For a document the viewer draws, the elegant remedy is named and, failing it, the loss is stated on
an appended page under the profile (ADR 1014; ADR 2394's photograph where `A694` item 1 allows it).
The elegant remedy is: a copy of the form per page for a form reached from differing pages; the
glyph procedure's resources written into the Type 3 font dictionary for a glyph procedure; and, for
a font that moved, nothing the file can say, because only the content stream's own spacing operands
would restate the reader's position, so the loss is stated. R03a (`A694` item 3: bake the state the
viewer shows) is owed and not started; ADR 2501 is unused.

## Proof

- 44 outputs (11 documents, 2 targets, 2 profiles) pass veraPDF at their flavour. The new shape,
  the empty Type 3 font, was planted first: the same output with `/Widths` removed fails veraPDF
  under ISO 19005-2 clause 6.2.11.2, so the pass was not vacuous.
- The page 1 raster of each of the 11 equals the source's pixel for pixel, except two documents
  whose device colours the output intent now manages (at most one level in one channel, the
  existing `device-rgb` remedy and not this one).
- Tests, each with a plant (the rewrite or the record disabled, the test fails): the form is given
  the page's resources with its bytes untouched; an `ExtGState` and a form nothing defines; the
  font under four stream shapes that moved nothing and six that moved; the three refused
  categories; a stream naming 100 100 missing names (past the survey's bound: refused, none
  defined); a stream naming 50 000 (under it: all defined); a standard-fourteen name defined as a
  `/Type1` font and not an empty one (the probe `BT /Helvetica 24 Tf (Hello) Tj ET` drew 204 dark
  pixels at 60 dpi in the source and the empty font would have drawn none); a spacing set by `"`
  refusing a later show under a missing font; sixty forms under a large direct page dictionary
  sharing one object.
- A form given the page's dictionary does not copy it: where the page's is direct, one object holds
  it for every form (§7.8.3 lets a dictionary be an indirect object any number of streams refer
  to), and the names only the forms used are defined in that object.
- The 50 000 test found a cost this remedy would have exposed to any document: the survey's
  `peek` copied a directly written category table for each lookup, so a table of n names read n
  times was quadratic (12 000 names: 4.4 s). It now reads the table in place; the conversion of 50 000
  names went from 347 s to 1.3 s.
- The stale worker: the export's `quorra-transform` printed the refusal this remedy removed while
  its own strings held none of it, because `cargo build --bin quorra-transform` leaves
  `quorra-transform-worker` stale (trap 230); `--bins` rebuilt it.
