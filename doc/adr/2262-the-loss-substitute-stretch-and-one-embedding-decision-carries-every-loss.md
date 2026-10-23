# 2262 — The loss `substitute-stretch`, and one embedding decision carries every loss it incurs

Session 1714, the archive slot of batch one hundred and ten. Status: implemented, as
`doc/questions/A566` orders. Context: ADRs 0358, 2146, 2147, 2162, 2179.

## What A566 settles, and what the round found beyond it

ADR 0358 has the viewer draw a substituted face's outlines at the horizontal scale that fits its
advances to the file's `/Widths`; an embedded program is drawn at none. ADR 2179 found eight
documents whose embedding was refused because it removed that fitting, and A566 allows the
printing profile to remove it under a loss of its own name while `as-shown` keeps refusing.

**The premise was narrower than the tree.** The described route (a standard font with no
descriptor) refused the fitting's removal; the route into a file's own descriptor that restates
nothing compared only advances and embedded the shipped face over a fitted substitute as a
statement, unasked. A fixture proves it: a `TrueType` Helvetica with a descriptor and a width of
721.9 against the face's 722 converted under no authorisation before this ADR. The restated-program
route is the same case at larger scales — a width the face is far wider than restates the advance
and leaves the glyph wide.

## Decision

1. **`Loss::SubstituteStretch`, the word `substitute-stretch`.** Every route that embeds the shipped
   face for a font the viewer drew from that face at a scale other than one incurs it: the
   described route, the restatement route and the unrestated route into the file's descriptor. An
   operator's `--font` incurs neither loss; it is the operator's statement. A machine's face on the
   unrestated route is still embedded over as a statement — `doc/todo/72`'s owed entry, which would
   refuse every TrueType name `substitute::find` matches on this machine under `as-shown` and is
   priced by a census of its own.
2. **The proof stays exact.** The pen places a point at `(x · scale) · stretch`, so the output's
   outline times the stretch is the source's bit for bit at every point a program states and in
   every vertical coordinate; a quadratic's elevated control points are held to 2⁻²⁰ em, the
   rounding of the two orders of operation. Advances stay bit for bit, and an embedded program read
   back at any stretch is still refused.
3. **`Losses`, a set, in `Decision::Authorised` and `Unauthorised`.** The font row's one decision
   carries the union of its fonts' losses and proceeds only where every member is authorised;
   `--authorise machine-face` alone leaves a document needing both stopped. The report names each
   font with its losses and the scale it lost, the JSON adds `losses` and `fitting`, and the file's
   own `xmpMM:History` names both per font. A configuration's `discard` at
   `fonts/font-programs-embedded` authorises every loss the row can incur, because the word says
   *I accept what this site costs*: `as-if-printed` takes both, and an out-of-tree configuration
   with that `discard` now accepts the fitting too, named in its report rather than silently.

## The eight, drawn

Page one of each source and its output, 100 dpi and 288 dpi, by `render_at` (largest channel
difference in levels of 255): `Simple PDF 2.0 file`, `… via incremental save`, `… with offset start`
convert at part 4 and differ in 266 / 24 / 26 pixels at 100 dpi and 4 305 / 240 / 279 at 288 dpi,
never by more than 1 level. `me450f12project10_report` converts at 2b; its body is identical but for
one pixel at 1 level, and its heading differs by up to 225 levels because that font is drawn from a
machine's Times — the `machine-face` half of the same decision, looked at. `427669` stops at the
content-stream fence; `E3_Mayhew_et_al_2009`, `PDF 2.0 image with BPC` and `… page level output
intent` now pass the font row and fail the output's second verdict (`non-symbolic-truetype-differences-are-listed-names`;
`symbolic-truetype-states-no-encoding` on a descriptor flagged symbolic), each a converter defect
the fitting refusal had hidden, refused loudly and owed.

## The census, HEAD against this ADR, by document

Converted / conforming / refused / unreadable, real corpora (pdf.js 967, `doc/corpora` 491), both
exports' binaries built from HEAD and from HEAD plus this slot's paths, with ADR 2263 beside it:

| corpus | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` | 4 `as-shown` |
|---|---|---|---|---|
| pdf.js | 756/2/198/11 → 757/2/197/11 | 722/0/234/11 → 723/0/233/11 | 752/2/202/11 → 745/2/209/11 | 718/0/238/11 → 712/0/244/11 |
| `doc/corpora` | 340/6/129/16 → 341/6/127/16 | 330/0/145/16 → 333/0/142/16 | 319/6/150/16 → 301/6/168/16 | 309/0/166/16 → 289/0/186/16 |

Under `as-if-printed` no document is newly refused (`PDF 2.0 via incremental save` leaves the 2b
column for the verb's own source refusal of a 2.0 header). Under `as-shown` 8 + 18 (2b) and 7 + 20
(4) documents are newly refused, every one by `substitute-stretch` withheld at
`fonts/font-programs-embedded`: each converted on HEAD by embedding the shipped face over a fitted
substitute and saying nothing, which is the promise `as-shown` exists not to break. The scales those
fonts were drawn at, read off the reports of 26 of the newly refused documents (88 fonts), run from
0.9999765 (44 of them) to 0.9928438
(`bug1671312_reduced.pdf`, 0.72 % — drawn: 288 dpi, at most 51 levels on glyph edges, the word
unchanged to the eye), above Q566's 0.026 %, which was the eight documents' figure and not a bound.
