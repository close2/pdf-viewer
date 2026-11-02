# 2600 — The font row's statement lists each photographed page and each face embedded with a loss

Status: implemented — session 1883, the third slot of batch 129 (an archive round). Context: `doc/rfc/0014` section 4,
`doc/questions/A706`, `doc/questions/A717` ("(and `state-every-loss`)"), ADR 2562 and ADR 2591 (the halftone rows'
statement and its hand-on), ADR 2576 (the per-page record of a photograph's groups). Code:
`crates/pdf-transform/src/archive/statement/fonts.rs`, `config/steps.rs` (`STATED_FONTS`, `statement_hands_on`),
`prepare.rs` (`stated_fonts`, `fonts_may_be_stated`, the order of `prepare_fonts`), `mod.rs` (`unstated`).

## Decision

1. `append-statement` is admitted at `fonts/font-programs-embedded`. Its row resolves to one `discard` carrying the
   font bundle, and the report already holds that bundle one record at a time — a `PhotographedPage` per photographed
   page, a `SubstitutedFont` per face embedded — so the page lists those records: per page its number, the
   resolution, its loss words in `PhotographedPage::losses`' order, the `/BaseFont` of each font not drawn, the
   content-stream requirements the photograph answered, the structure leaves taken out and the optional content
   groups fixed with the state drawn; per face with a non-empty `losses`, the `/BaseFont`, the resource name and the
   loss words. Each loss word has a line fixed in `statement/fonts.rs`, written for a reader of the archive rather
   than taken from `Loss::describe`, whose sentences address the person running a conversion.
2. A page photographed for a fault of its content stream is listed under the font row too: in the shipped profiles
   it is the font row's `discard` that authorises `page-as-image`.
3. A page that cannot be composed hands every record to `discard`, as at the halftone rows (ADR 2591), and the report
   names the composition's own sentence (`Conversion::unstated`).
4. The fonts are prepared before the preserved pages, since the page lists what that preparation did. Object numbers
   of an output holding both therefore come in the other order; nothing reads them but the file.
5. The statement's text paints in Table 51's initial `DeviceGray`, so the output intent is prepared wherever the font
   row states its loss and its preparation will run (the font row failed, or a `fenced` row failed where
   `page-as-image` is authorised) — known before the fonts are prepared — and written only where a page is appended.
   Cost, written down: where nothing is listed but another page is appended, that page's file gains an intent it did
   not need.

## Bounds

The statement's own: `MOST_STATED_OBJECTS` counts the font entries with every other (the photographs are bounded at
32 by `MAX_PHOTOGRAPHED_PAGES` before them), each name is held to an eighth of `MOST_STATEMENT_BYTES` before it is
escaped, and the text to `MOST_STATEMENT_BYTES` before each entry and after the last.

## Not decided here

The second pass (`settle.rs`) folds its `photographed` into the first report and not its statement; a page the second
pass photographs is stated on that pass's own page, if any.

## Figures

The census converts under `as-if-printed` and `as-shown` and prints no `state-every-loss` line, at HEAD and here
alike (0 and 0), so it cannot witness this admission; `--test profiles` does (no step of a shipped profile is passed
over by name), and so do the two read-back tests. Its `as-shown` lines moved — pdf.js 813 → 820 converted at 2b and
818 → 825 at 4, corpora 338 → 339 and 336 → 337, the same with and without the CJK pack — every move a sibling's (the
page-tree sources freed as `page-not-held`, the CID 0 sentence, the whole-population remedy, the `/Type /Font` row),
none this one's. veraPDF on the tests' outputs: the two-page fixture passes at 2b, the four-page and the layered at
4; each with bytes planted after `%%EOF` fails one rule, ISO 19005 6.1.3 test 3.
