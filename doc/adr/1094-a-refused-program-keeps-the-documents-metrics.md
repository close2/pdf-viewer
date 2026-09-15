# 1094 — A refused program keeps the document's metrics

Session 1080. Status: **accepted**. One decision about what a font this reader cannot draw still
owes the page: ISO 32000-2 §9.4.4's displacement, taken from the font dictionary rather than from
the program that was refused.

`§N` is ISO 32000-2 and nothing else.

## 1. The defect, and why it was not about the font it was about

`pdf_font::LoadedFont::load` refuses a font when nothing can draw its glyphs, and every one of its
refusals is a statement about the **glyph program**: §9.7.5.2 forbids the file outright ("[t]he
Identity-H and Identity-V CMaps shall not be used with a non-embedded font"), or Table 120's three
keys supplied no program and no installed face can stand in, or the bytes a `/FontFile` supplied
will not parse. `Interpreter::show_text` treated that refusal as a refusal of the whole show
string: with no font in the text state it counted the string as text the page could not draw and
returned — before §9.4.4's combined displacement was computed for a single code.

So the pen did not move, and the *next* font's glyphs began where the last `Td` left it.
`issue6127.pdf` page one is the witness. Inside one text object it states

```
/C2_14 1 Tf  5.737 0 Td  <0003>Tj  /C2_2 1 Tf  [<0003>187<000b>-32<0055>…]TJ
```

where `/C2_14` is `/TimesNewRoman`, `/Type0`, `/Encoding /Identity-H` over a `/CIDFontType2` with
no embedded program — the combination §9.7.5.2 forbids, so the refusal is right — and `/C2_2` is a
font this tree loads and draws. The words `(réf.` and `S3182).` were therefore drawn 3.0158 pt
before where both references put them (`pdftotext` xMin 165.117, `mutool` 165.11672, ours 162.101),
and a page drawn wrong for a fault reported elsewhere is the worst shape a refusal can take.

## 2. The displacement was never the program's to withhold

§9.2.4 states a glyph's width twice over —

> The width information for each glyph shall be stored both in the font dictionary and in the font
> program itself.

— and its NOTE 2 says exactly what the second copy is for:

> Storing this information in the font dictionary, although redundant, enables a PDF processor to
> determine glyph positioning without having to look inside the font program.

That is this case, named by the standard. The entries are Table 109's `/Widths` and Table 120's
`/MissingWidth` for a simple font and, for a `CIDFont`, the two §9.7.4.3 names: "[w]idths for a
CIDFont are defined using the DW and W entries in the CIDFont dictionary". `/C2_14`'s descendant
states `/W` 250 for CID 3, so §9.4.4's `t x` is (0.25 + `Tc` 0.0013) × 12.0008 = 3.0158 pt whatever
the program does — `Tw` staying out of it because §9.3.3 applies word spacing "only to the ASCII
SPACE character (20h)" and an `Identity-H` code is two bytes. And §9.4.4 asks for the update
unconditionally: "[a]fter the glyph is painted, the text matrix shall be updated according to the
glyph displacement and any spacing parameters that apply."

## 3. The decision

**A font whose program is refused is kept as a metrics-only font, and `show_text` advances the
text matrix across it.** `LoadedFont::metrics_only` builds one from the dictionary alone — a
`CodeMapping::MetricsOnly` carrying the composite font's `CMap` (or nothing, for a simple font's
single-byte codes), the widths, the default width, Table 120's extent and §9.7.4.3's second set of
metrics for writing mode 1. It answers `None` only where the document states nothing that could
delimit a code, which is a composite font whose `/Encoding` names no `CMap` this crate reads.

Three things it deliberately does **not** do, and they are one sentence — *this font marked no part
of the page*:

- **No glyph.** Every glyph route answers `None`, so nothing is painted and nothing is substituted.
  The metrics are not a recovery, and they may not read as one.
- **No readback.** §9.10.2's methods are not consulted and the font carries no `/ToUnicode`, so its
  codes contribute nothing to `Interpretation::text` and no quadrilateral to the text layer.
  Naming them would put selectable text under blank paper.
- **No quieter report.** The string is still counted as text the page could not draw, so
  `Unsupported::Text` and the `Unsupported::Font` refusal both stand and the page stays incomplete.
  The font is kept in the page-scoped cache and **not** in the cache that outlives the page, whose
  hits report nothing: a metrics-only font kept there would let page two draw nothing in silence.

## 4. What a later round must not re-litigate

- **The refusal and the metrics are separable, and the standard is what separates them.** A round
  that finds a font drawn in the wrong place may not close it by refusing the widths as well.
- **Metrics only means metrics only.** Reading a refused font's `/ToUnicode` back into the page's
  text is a different decision with a different argument — it would claim marks the page does not
  carry — and belongs in its own ADR if it is ever wanted.
- **The boundary is the program, not the font dictionary.** A Type 3 font is outside this: §9.6.4
  makes its glyphs content streams this tree runs, ADR 0866 governs a `/CharProcs` it cannot read,
  and Table 110 puts its widths in the glyph space `/FontMatrix` defines rather than in the
  thousandths §9.2.4 states.

## 5. What it cost and what it bought

`raster_golden` moved exactly one page of 974 — `issue6127.pdf` p1, raster and display list, its
report digest unchanged — and the page was looked at: the line reads
`qualité d'ayant droit" (réf.S3182).` with the run translated 3.0158 pt and nothing else altered.
`text_extraction`'s geometry verdict rose from 11094/11131 to 11096/11131, the two words named
above, with the judged set and the cross-axis population unmoved; `issue6127.pdf` left
`SELECTION_BELOW_FLOOR`, which is the last entry on that list whose mechanism was this tree's own
rather than a box convention or a substituted face. `pdf-model`'s corpus ratchets and the oracle's
by-name lists were unchanged.

`crates/pdf-model/tests/refused_font_metrics.rs` holds both halves of the rule: the corpus witness
for the composite case, and a fixture for the simple one — which has no corpus witness to be had,
because a simple font with no program is substituted rather than refused. The fixture carries its
own control (trap 13): an arm stating no width at all must put the second font back where the `Td`
left it, or the test is measuring something other than Table 109's array.
