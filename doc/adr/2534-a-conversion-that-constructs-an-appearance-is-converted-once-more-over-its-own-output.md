# 2534 — A conversion that constructs an appearance is converted once more over its own output

Status: implemented. Session 1850. Q716 is not asked.
Context: ISO 19005-2 §6.3.3 and §6.2.11.4, ISO 32000-2 §12.5.5 and §12.7.4.3; `doc/pdfa-blocking-issues.md` section 8;
ADR 0947 (the second verdict).
Code: `crates/pdf-transform/src/archive/settle.rs`, `archive/mod.rs` (`run`, `run_pass`, `apply_the_decisions`),
`archive/report.rs` (`Conversion::settled`), test `a_constructed_appearance_is_converted_as_the_content_it_is`.
Documents: `doc/pdfa-blocking-issues.md` section 8.

## 1. The defect

The conversion constructs appearance streams where the source had none: a free-text annotation, a widget, a markup
(`prepare_appearances`, `prepare_field_appearances`). Those streams are built by `pdf_model::appearance` from the
annotation's `/DA` and its variable text, so they name `/Helv` (a face nothing embeds) and select `DeviceGray` or
`DeviceRGB` (a colour no default or output intent licenses). They are content that **no pass over the source saw**: the
font pass embeds from a survey of the source, and the default colour spaces were written for the source's colours.
The output then failed requirements the source *met*, the second verdict's `regressions`, or failed the font
requirement at the appearance's face on a page whose own faces the source had failed too, and the converter refused a
file for which every remedy existed. Seven documents of the corpora were refused this way
(`annotation-tx`, `bug1538111`, `bug1865341`, `bug1871353`, `freetext_no_appearance`, `issue12750`, `issue16021`).

## 2. The decision

**Where the first output would be refused and the conversion constructed appearances, the output is converted once
more, as a source of its own, under the same plan.** The constructed streams are then ordinary content in a document,
and the font pass, the colour pass and every other remedy already written reach them without a second implementation.

- The condition is all of: `settling`, the conversion has `appearances`, the first output does not stand
  (`stands_as_departed`), and the second output *does* stand. The first output's failures need not be `regressions`: a
  page that shows an unembedded face of its own and a widget whose `/DA` names another fails
  `fonts/font-programs-embedded` in the source as well, and is settled all the same. A conversion that stands is never
  converted twice; one that does not settle is refused with its own first refusal.
- `run` is `run_pass(…, true)` and the nested conversion is `run_pass(…, false)`, so there is **one extra pass and no
  more**: no recursion an input can deepen.
- `second_conversion` returns `None` on any refusal of the nested conversion, and nothing is written to the caller's
  sinks by it. The nested conversion's refusal is dropped because the first one's already says why the document stood
  refused; reporting both would name a requirement of an intermediate file nobody asked for.
- The report says what the second pass brought: `Conversion::settled` names each requirement the second conversion
  decided, `decided` carries the rows the first did not hold, and the text report has one line saying so.
- The history says it too: a face the second conversion embedded is an `xmpMM:History` entry (ISO 19005-2 section
  6.6.6, NOTE 1), and `wants_metadata` makes the packet the prepared one for any recorded entry, so a first output
  whose packet already conformed is not left without it. The faces
  embedded, the metrics restated and the profile supplied are folded in. Nothing the first pass reported is dropped.

## 3. What it costs

- **Time and memory**: a conversion that settles is two conversions. Measured on the three one-page documents, with
  the pack, one process each and a machine at a load average of 6 of 24 cores: `annotation-tx` 206 ms, `bug1538111`
  36 ms, `issue12750` 65 ms for the whole verb. A large document that constructs appearances and regresses pays a
  second conversion's time and holds the first output (as bytes) and the second document beside the first's. The path
  is taken only for a document that would otherwise be refused, so no document that was written pays it; the census
  with the pack shows no document that was written before is refused now.
- **A second reading of what the first wrote**: the nested conversion sees the first pass's output as a producer's
  file. A loss the first pass authorised (smoothing, say) is not lost twice, because the first output no longer holds
  it; a loss that is only visible in the constructed content is authorised by the same plan or refused by it.
- **Complexity**: `run`'s body moved to `run_pass`, and `apply_the_decisions` takes `settling`. The alternative is to
  teach each remedy about constructed content (the font survey would read `pdf_model::appearance`'s streams before they
  are built, the default colour spaces would be written for them). That is a second implementation of two passes, in
  the one place where they cannot be tested against the validator's reading of the bytes.

## 4. Rejected

- **Constructing the appearances before the passes** (build them first, then convert): `prepare_appearances` needs the
  output's fonts to lay the text out, so the order is the other way round, and an appearance built from a face that is
  then replaced is built twice.
- **Refusing with a sentence** (what the corpus got before): the census held seven documents the viewer draws as the
  producer's own file and the converter turned away, with no remedy missing.

## 5. Evidence

The seven documents are written and conform under veraPDF at ISO 19005-2 (2b) and ISO 19005-4, `failedRules` 0 each, and
draw the pages the source states (one each). With the plant (`settling` cleared) the test fails.
