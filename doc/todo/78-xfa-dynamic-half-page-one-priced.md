# 78 — XFA's dynamic half: page one of the `/NeedsRendering` documents, priced

Status: **measured** — the measurement `doc/scope-ratifications.md` names before XFA's dynamic half
is revisited (`doc/todo/76` row 10, `doc/questions/A828`). Nothing of the dynamic half is built: it
stays a scope exclusion until the owner answers `doc/questions/Q840`. ADR 2837 is the method.
Priority: the owner's, 2026-11-11, A828: "keep the scope exclusions … while running XFA's
`/NeedsRendering` page-one measurement".
Instrument: `crates/pdf-model/examples/xfa_dynamic_census.rs` (this tree's own reader: Table 29's
flag by `pdf_model::xfa::stated`, page 333's grammar by `Template::dynamic`, the resource dumped)
and `tools/xfa-census.py` (the layout grammar counted in each dump; its `--self-test` plants every
count and `tools/conformance/tests/xfa_census.rs` runs it). XFA 3.3 is `doc/XFA-3_3.pdf`, read as
`doc/md/XFA-3_3.md`; a page number below is its printed folio.

## 1. The population

Every `*.pdf` under the corpora this machine holds, on 2026-11-12: `doc/pdf.js` (976), `doc/corpora`
(503), `doc/veraPDF-corpus` (2 908), `~/scratch/gwg` (98) and the safedocs cache under
`~/scratch/corpus-cache` (65 968) — 70 453 paths. The tika-issue-tracker and openpreserve caches
are not restored on this machine and are not in it. A second population was fetched for this
measurement: the 55 documents `doc/pdf.js/test/pdfs/xfa_*.link` name (53 arrived; two Bugzilla
attachments did not), held under `~/scratch/agent/r2009/linked/`, never in the repository. They are
pdf.js's own XFA test set, chosen for being XFA, so they say what a dynamic form *contains* and
nothing about how common one is.

Run on 2026-11-12: 70 453 paths, 70 202 opened, 251 refused at the open. "Dynamic by grammar" is
page 333's test; "XFAF" is a resource with neither test.

| corpus | files | state an XFA resource | `/NeedsRendering` true | of those, dynamic by grammar too | dynamic by grammar, flag false | XFAF |
|---|---|---|---|---|---|---|
| safedocs (Common Crawl, `cc-main-2021-31`) | 65 968 | 37 | 3 | 1 | 4 | 30 |
| `doc/pdf.js` | 976 | 7 | 3 | 0 (one has a varying `occur` the reader stops before) | 0 | 4 |
| `doc/veraPDF-corpus` | 2 908 | 4 | 3 | 0 | 0 | 2 |
| `doc/corpora` | 503 | 1 | 1 | 0 | 0 | 0 |
| `~/scratch/gwg` | 98 | 0 | 0 | 0 | 0 | 0 |
| **held, all** | **70 453** | **49** | **10** | **1** | **4** | **36** |
| fetched, pdf.js's `xfa_*.link` | 53 | 53 | 53 | 22 | 0 | 0 |

The instrument found what was known: the four `/NeedsRendering` documents RFC 0011 section 4 named
with `mutool` are among the ten. Two of pdf.js's stop at the reader (`issue14130`, `xfa_issue14315`
state no `datasets` packet, which `Resource::read` requires), so their grammar is
`tools/xfa-census.py`'s count; `issue14130`'s `occur` runs from 0 to unbounded. One veraPDF
fixture states the flag with no resource at all. Re-run:

```sh
cargo build --release -p pdf-model --example xfa_dynamic_census
find <corpus> -name '*.pdf' | <target>/release/examples/xfa_dynamic_census --dump <dir> > census.tsv
python3 -I tools/xfa-census.py <dir> --census census.tsv
```

**What the count says.** Three of the 65 968 documents a web crawl sampled need their pages
generated (one in about 22 000), and three of the ten held are fixtures veraPDF built to fail a
PDF/A or PDF/UA rule rather than forms anyone filled. The world's share is small and the held
sample of real dynamic forms is four: two Canadian immigration forms (pdf.js's `imm1344e` and
`jhove`'s `MOZILLA-1671648-0`, 811 to 1 432 scripts each) and two of the crawl's (`0100888`, 11
`draw`s and no script; `4605385`, 94 scripts). The four crawl documents with a varying `occur`
and the flag false carry their pages as PDF already — a hybrid whose template says an instance
count may change — and show today what their producer wrote; they are the static half's, not this
measurement's.

## 2. What page one needs

XFA 3.3 does not lay a page out alone. The order is the processing model's: the template is merged
with the data into the Form DOM (chapter 4, pages 171–213, and chapter 9's dynamic merge from
page 333), the `initialize` events and the Form DOM's `ready` run (page 388), and only then does
the layout processor start, at "the first contentArea of the first pageSet which is a child of the
root subform" (page 288), filling it with the root subform's content in document order until a
break, an overflow or the end of the content. So:

- **The whole merge comes before page one.** How many instances a repeated subform has is decided
  by the data (page 333), and the instances page one shows are the first of a list the merge
  builds whole.
- **The scripts come before page one.** Of the 56 `/NeedsRendering` resources (the 53 fetched and
  pdf.js's three), 47 state an `initialize` or `ready` event and 38 set a container's `presence` from a
  script — what page one shows, and whether a hidden subform takes space on it, is decided there.
  DHL's shipment form (`xfa_dhl_shipment`) starts with a `CustomerConfig` subform its scripts hide.
- **The layout can stop after page one, except where page one depends on the end.** Page 308:
  "Because termination processing relies upon reprocessing, the layout processor must be able to
  lay out the last or only page twice" — a page area with `pagePosition` `only` or `last` is chosen
  for page one only once the processor knows nothing follows it (one fetched form states all four
  positions). And a script that reads
  `xfa.layout.pageCount()` (2 of the 53 fetched, and the `imm` forms' "page 1 of N") reads the
  whole layout.
- **Every mark is set from metrics.** A `draw`'s text, a field's caption and value, rich text
  (`exData`, chapter 27), borders, fills, lines, arcs and images; fonts named by typeface, which a
  shell PDF is to carry in `/AcroForm /DR` (page 21) and which may be absent; and barcodes — 14 of
  the 53 fetched forms state one (`pdf417` 24 times, `code3Of9` 7, `dataMatrix` 4), each a
  symbology standard this tree does not hold.

Read per document (the sample is the held dynamic documents and four fetched ones of each shape):

| document | why it is dynamic | page one's layout needs |
|---|---|---|
| pdf.js `issue14130` | `/NeedsRendering`; one subform `occur` 0 to unbounded | one page area, one content area; a `tb` root flowing a repeated subform bound to the data; one `draw` |
| pdf.js `xfa_issue14315` | `/NeedsRendering` only (no varying `occur`) | one page area; nine `draw`s in five subforms, two of them `tb` |
| pdf.js `xfa_filled_imm1344e` | `/NeedsRendering` only — "traditional static" | two page areas (`Page1` at most once, then `Overflow`); 25 flowed subforms, 19 `keep`s, 4 `breakBefore`s; 269 `draw`s, 63 rich-text values; 1 432 JavaScript scripts; six barcodes; two `pageCount` scripts |
| fetched `xfa_dhl_shipment` | both: 9 varying `occur`s | three page areas chosen by each top-level subform's `breakBefore`; page one is `CustomerConfig`'s, and its scripts decide what shows; 940 `draw`s, 781 scripts |
| fetched `xfa_bug1718740` | both: 12 `subformSet`s, 68 varying `occur`s | one page area of 254 × 152.4 mm; one `tb` subform holding everything; 59 `overflow`s, 77 `keep`s; 1 099 JavaScript and 271 FormCalc scripts, 44 changing an instance count |
| fetched `xfa_issue13679` | both: one `subformSet`, 57 varying `occur`s | 234 flowed subforms, 23 overflows, 67 `keep`s; 795 scripts, 27 changing an instance count |
| fetched `xfa_bug1716047` | `/NeedsRendering` only | two page areas; eight flowed subforms; 17 `draw`s, 14 scripts — the smallest real form in the sample |

**Page one is not a smaller job than the form.** Of the fetched `/NeedsRendering` resources, 31 of 53 have no
varying `occur` and no `subformSet` at all and are not XFAF only by Table 29's flag (XFA 3.3's
"traditional static", page 29); they need the same flowed layout, `draw`s and scripts as the rest.
What page one alone saves is pagination past the first content area — leaders and trailers on
later pages, `breakAfter`, `rest` page areas — and nothing of the merge, the scripts or the drawing.

## 3. What this tree already has

The static half (RFC 0011 section 5, ADR 2380) built the parts a layout would read, and none of
the layout:

- `pdf_model::xfa`: the template's containers (`subform`, `subformSet`, `field`, `exclGroup`,
  `area`, `pageSet`, `pageArea`), `bind` (direct, `global`, `dataRef`, `none`), `occur`'s test, the
  `datasets` packet read and written, pictures and locales, `config`, calculations and validations
  — the Form DOM as far as a field's value; **no instance is created by data**, no `draw`, no
  geometry, no `para`, `font`, `border` or `margin` is read.
- `pdf-script` and `pdf-formcalc`: both script languages run, but over the AcroForm object model
  (ADR 1989), at the reader's level and `off` by default; **no XFA SOM object model** —
  `xfa.resolveNode`, `presence`, `instanceManager`, `xfa.layout` — exists for a template script.
- `pdf_model::rich_text` lays out chapter 27's rich text for §12.7.4.3 (ADR 1623) and
  `variable_text` sets a field's text from metrics; both are what a `draw`'s text would reuse.
- The window says, for every `/NeedsRendering` document, that it is an XFA form and what is shown
  instead (ADR 1932), and shows the producer's own page for processors that decline.

## 4. The price

In rounds of this lane's size — one slice, its tests and its gates — read from section 2's needs and
section 3's holdings. Nothing here is measured but the corpus; the estimate is the reading's.

| piece | XFA 3.3 | where it would live | rounds |
|---|---|---|---|
| the template read whole: geometry, `para`, `font`, `margin`, `border`, `fill`, `caption`, `value`, `ui`, every `draw`'s content | chapter 2; the template reference | `pdf_model::xfa::template` | 2 |
| the merge: instances made from data by `occur`, `subformSet` relations, `global` and `dataRef` in an instance list, the empty merge | chapter 4 (pages 171–213), chapter 9 (333–370) | a merge module beside `datasets` | 3 |
| a template script's object model: SOM resolution, `presence`, `instanceManager`, `rawValue`, `xfa.layout`; `initialize` and `ready` in order | chapters 3 and 10 (events, page 388) | `pdf-script`, `pdf-formcalc` | 3–4 |
| the layout processor: positioned and flowed (`tb`, `lr-tb`, `rl-tb`), growable containers, content-area filling and splitting, pagination by `relation`, `pagePosition` and `oddOrEven`, breaks, overflow leaders and trailers, `keep`, tables | chapter 8 (pages 274–332) | a new crate, `xfa-layout`, over `pdf-model` | 6–8 |
| text from metrics: `para`, a typeface found in `/AcroForm /DR` and then on the system, rich text, line breaking | chapter 8, chapter 27 | `rich_text` and `variable_text` reused | 2 |
| drawing: `draw`s (text, line, rectangle, arc, image), edges, corners and fills, each field's `ui` and caption; barcodes refused by name | the template reference | `xfa-layout` into `pdf-render`'s display list | 3–4 |
| the generated pages in the document's page view, fillable, saved by §7.5.6 into the `datasets` packet, and said to be generated | §K.2; ADR 1932 | `pdf_model::view`, `viewer-core` | 2–3 |
| the instruments: a test per rule derived from XFA 3.3's text, the earlier-output reference, a fuzz target, the budgets, page one's launch cost | — | tests, `fuzz/` | 2 |

**About 23 to 28 rounds, and page one alone is about 21 to 26** — what it saves is the pagination
past the first content area, one or two rounds.

**Risks**, each a reason the estimate could grow:

- **Scripts.** The reader's level is `off` by default (`CLAUDE.md`, the JavaScript exclusion), and
  most sampled forms decide page one in `initialize`. At `off`, the page laid out is the template's
  and the data's alone and differs from the producer's intent; that has to be said on the page's
  window, the way ADR 1932 says the form is XFA.
- **No reference.** A generated page has no producer's bytes, so `raster_golden` and the oracle can
  hold it only against this program's earlier output (ADR 1014) and against tests derived from XFA
  3.3's rules; principle 5 refuses Acrobat's output as the target.
- **The launch path.** The merge and the `initialize` scripts run over the whole form before page
  one, and `only`/`last` page areas and `pageCount` scripts reach past it; *nothing eager* still
  binds, so the first implementation round measures page one's cost on the sample before anything
  else.
- **Barcodes.** PDF417, Code 39 and Data Matrix are ISO/IEC 15438, 16388 and 16022, none held; a
  form that states one draws a box or a refusal until the texts are.
- **Resource budgets.** A layout loop — an overflow leader that never fits, a `keep` chain longer
  than a content area — needs a budget, and the fuzzer a new target.

## 5. The question

`doc/questions/Q840` puts the near-side construction RFC 0011 section 7 words to the owner with
this price and a recommendation. Until it is answered, the tree does what A639 says it does.
