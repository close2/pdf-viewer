# 0971 — The second parser over the same carried files kept the silence

Status: accepted. Session 964.
Context: `crates/pdf-font/src/tounicode.rs`, `crates/pdf-font/src/loading.rs`,
`crates/pdf-model/src/content/font.rs`, `crates/pdf-model/tests/hostile_budgets.rs`,
ISO 32000-2 §9.10.2, §9.10.3, §9.7.4.2, §9.7.5.2, `doc/traps/instruments-and-reports.md`
traps 11 and 13, ADRs 0961 and 0963.

## The method this round was asked to generalise

ADR 0963's finding was not a bound. It was that **a constant can sit below what a datum this
binary already carries states**, and that the question is arithmetic over things already on
disk — no corpus, no document. This round asked it of everything in `data/`.

The census, and it is the output of the round as much as the fix is:

| the carried datum | the bounds it passes through | measured |
|---|---|---|
| `data/icc/sRGB2014.icc` | `icc.rs`'s `MAX_PROFILE`, `MAX_TAGS`, `MAX_TAG_TEXT`, `MAX_CLUT`, `MAX_INPUTS`/`MAX_OUTPUTS` | 3 024 bytes against 16 MiB and 16 tags against 1 024; the widest tag is 2 060 bytes. Clear by three orders of magnitude |
| Adobe's 240 `CMap` files | `cmap.rs`'s four | session 960's, already asserted by `no_registered_cmap_is_cut_by_these_bounds` |
| the same 240 | `predefined.rs`'s `MAX_DEPTH` on the `usecmap` chain | the deepest chain is **2** (`ETenms-B5-V` → `ETenms-B5-H` → `ETen-B5-H`) against a bound of 4 |
| the same 240 | `loading.rs`'s `MAX_ADDRESSABLE_CODES` | already asserted by `every_registered_cmap_is_inside_the_addressable_bound` |
| **the same 240, read as `/ToUnicode`** | **`tounicode.rs`'s three** | **13 291 `bfrange` entries (`UCS2-ETen-B5`) against 16 384, and 17 387 individual mappings (`Adobe-Japan1-UCS2`) against 65 536 — and nothing walked them** |
| the 14 compiled-in faces | Annex D.5's `Symbol` and D.6's `ZapfDingbats` encoding tables | every one of the 189 and 188 names those tables state has a glyph in the face this binary carries for it; the faces hold one and fourteen glyphs the tables do not name |
| `FoxitSerif` and `FoxitFixed` | `standard_metrics.rs`'s 315-name Latin tables | **86 and 84 of those names have no glyph in the carried face** — the Latin Extended-A letters and eight mathematical signs. Not a defect: `content/text.rs`'s third arm counts a code that reached no glyph, so this population is already spoken for. Written down because finding it *not* to be silent took reading the arm, and the next round to measure it should not have to |

So the arithmetic paid once more, in the row in bold — not by a bound sitting below the data,
but by finding **a second parser over the same files with none of the first one's reporting**.

## The finding

The 240 files in `data/cmaps/` go through two parsers, and session 960 built the census for one
of them. §9.10.2's third method reads five of them — `registry-ordering-UCS2`, for every composite
font whose program is absent — and §9.10.3 lets a producer's own `/ToUnicode` name **any** of the
rest in `/UseCMap`, which `read_to_unicode` honours through `predefined::unicode_cmap`. That
parser is `tounicode.rs`, and its three bounds were `if len < MAX { push }` with no `else` at all:

- `MAX_SINGLES`, on the individually-stated mappings;
- `MAX_RANGES`, on the `bfrange` entries;
- the operand buffer, `MAX_SINGLES * 4`, which drops *tokens* before a section is ever read.

Measured over every carried file: the widest is `UCS2-ETen-B5` at **13 291** `bfrange` entries
against a bound of **16 384**, which is 81% of it, and `Adobe-Japan1-UCS2` at **17 387**
individual mappings against 65 536. **Nothing is cut today.** What was wrong is that being cut
said nothing, and that the population had never been counted — the two halves of ADR 0963's
finding, and this is the half that was still outstanding.

## Why it is not only about extracted text

A `/ToUnicode` cut short is obviously a text-extraction loss. It is also a **mark not made**,
which is what makes this a pixel question rather than a readback one. §9.7.4.2, on a composite
font whose program the document did not embed:

> In this case, CIDs shall not participate in glyph selection

so `CodeMapping::Substituted` reaches the substitute's glyph through the *character* the code
stands for, and the only thing that supplies that character is one of the two `ToUnicode` tables
§9.10.2's first and third methods give it. A mapping a bound discarded is a code with no
character, and a code with no character is a glyph the page does not draw — with
`Interpretation::is_complete` answering true.

## Decision

The shape is ADR 0963's, deliberately, so that the two parsers report alike:

- `ToUnicode` carries `truncated: Option<&'static str>`, set by `cut_by` **with the discarded
  entry already in hand** rather than on reaching a count — trap 11, and the property that lets
  a file whose last entry lands exactly on a bound report nothing.
- Three names, spelled as the bound a reader would grep for: `max_tounicode_singles`,
  `max_tounicode_ranges`, `max_tounicode_operands`.
- A map absorbs the truncation of the `CMap` it builds on, because Table 118 makes the pair one
  statement in two files and `append` already reads it as one. The child's own bound wins where
  both cut: that is the one a reader of *this* stream can act on.
- **`insert_single` asks the bound about the entry rather than about the count**, which is not
  only tidiness: §9.10.3 does not forbid a later section from restating a code, and a
  restatement discards nothing — so a full map that is handed a code it already holds overwrites
  it and stays silent. The old code returned out of the section on reaching the count and would
  have reported. `restating_a_code_at_the_bound_discards_nothing` is that case.
- Two conditions that were folded in with the bounds are now separate, because neither discards
  anything of the producer's: a `bfrange` whose `<hi>` is below its `<lo>` states no span at all,
  and an array longer than the span it belongs to has said nothing about the codes past `high`.
  Reporting either would be trap 11 in the other direction.
- `LoadedFont::to_unicode_truncated` asks all three tables a font can hold — the producer's own
  `/ToUnicode`, the collection's `registry-ordering-UCS2`, and whichever of the two
  `CodeMapping::Substituted` selects glyphs through — and `pdf-model`'s
  `note_to_unicode_truncation` raises `Unsupported::LimitReached { limit }` beside its `CMap`
  sibling, at the load and again where the cross-page cache serves the font, because a report is
  about *this page*.
- **No bound was raised.** 81% of a bound is not a defect, and raising a constant because a
  measurement came close would be speculative. What it is is a number that now has a gate under
  it: `no_carried_unicode_cmap_is_cut_by_these_bounds` fails the day a later edition of Adobe's
  files crosses it, which is exactly the event that was previously silent.

## Calibration

Per trap 13, every new assertion was run against the defect it looks for.

`no_carried_unicode_cmap_is_cut_by_these_bounds` walks all 240 carried files through
`predefined::unicode_cmap`, asserts `truncated() == None` for the 46 that state any `bfchar` or
`bfrange`, and then checks the **last entry** of each of the two widest files against the value
the file itself gives it — `<FFE5> <FFE5> <A244>` of `UCS2-ETen-B5` and `<5a13> <32ff>` of
`Adobe-Japan1-UCS2`. Calibrated twice: with `MAX_RANGES` lowered to `1 << 13` it fails naming
`/B5pc-UCS2`; with the bound lowered *and* `cut_by` planted as a no-op — a bound raised only far
enough to stop the flag firing while the parse still lost the tail — the second half fails
instead, on the character `UCS2-ETen-B5` gives U+FFE5.

`a_tounicode_past_the_range_bound_is_reported_by_name` and its three siblings assert the report;
`a_tounicode_exactly_on_the_range_bound_reports_nothing_about_it` and
`restating_a_code_at_the_bound_discards_nothing` assert its absence. Calibrated by planting
`cut_by` as a no-op: the four "past" tests fail and the two controls still pass. The restatement
control was calibrated separately, by removing `insert_single`'s `contains_key` escape: it fails.

`a_to_unicode_past_the_range_bound_is_reported_by_name` in `hostile_budgets.rs` is the same
question at the page's level — a substituted composite font over a `/ToUnicode` of 16 385
`bfrange` entries — with a 16 384-entry control beside it. Both assert the page draws, so neither
can pass by the font failing to load wearing a bound's name. Calibrated by planting
`to_unicode_truncated` as `None`: the first fails, the control passes.

## What this leaves

The census table above is the round's standing answer for `data/`, and two rows of it are
assertions rather than measurements taken once — `cmap.rs`'s and `tounicode.rs`'s. The other
four are measurements in this document, which is the weaker form: a round that adds a datum to
`data/` has nothing that fails. Turning the ICC and font rows into walks the way the two `CMap`
rows are walks is small, principled and was not done here.

## A habit worth recording, which this round could not place

`doc/traps/` and `doc/habits.md` were being rewritten by a sibling this round, so this goes here
for the orchestrator to place rather than into either file.

> **When one datum has two readers, a census built for one of them proves nothing about the
> other.** Session 960 counted Adobe's `CMap` files against `cmap.rs`'s bounds and wrote a gate
> that walks all 240 of them; the same 240 files also go through `tounicode.rs`, whose three
> bounds nothing had ever counted. The grep that finds the shape is *not* over bounds — it is
> over the **data**: for each compiled-in datum, list every parser that reads it, and check that
> the census names each one. `data/cmaps/` had two readers and one census; `data/standard-fonts/`
> has three (`sfnt`, `cff`, `type1`) and the tables that describe them are in a fourth place
> again.

It is the same lesson trap 15 teaches about binaries — *which* build produced the thing you are
measuring — asked about parsers instead: which reader produced the answer you are asserting is
whole.
