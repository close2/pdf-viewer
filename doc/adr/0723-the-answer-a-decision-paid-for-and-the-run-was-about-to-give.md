# ADR 0723 — The answer a decision paid for, and the run was about to give

Status: accepted, 2026-08-28. Session 787. Takes the last line `doc/todo/41` still carried —
"whoever takes it should check first whether §12.5.5's route still needs the answer before the
run" — and answers it. Amends the ledger's §12.5.5 row.

## What changed, in one line

**Deciding an annotation no longer decodes a windowed appearance stream it is about to draw.**
`annotation::decided` asked `NestedContent::damage` of every stored appearance before the draw;
for a stream the decoded-stream memo declines that answer is one full pump of the decode — up to
`Limits::max_stream_len`, a gibibyte on a bomb — paid once per decision, per interpretation, in
front of a run that decodes the same stream and reports the same damage itself. The decision now
asks the question at the cost its route is worth: `stated_damage` (free) for a stream the draw
will read, the full pump only where §12.7.4.3's regeneration replaces the stored bytes and
nothing later reads them.

## The mechanism, and why the pre-pass existed

ADR 0359 put the damage question where the stream is *resolved* rather than where it is drawn,
and its reason is real and stays: §12.7.4.3's regeneration hands the draw a spliced *copy* of
the stored bytes, so a report taken at the draw goes quiet for exactly the fields whose variable
text a reader has changed. ADR 0427 then made the appearance a *source*, so that the memo's two
shapes serve both questions — and its own doc comment recorded the cost as acceptable because
"asking a bomb whether it is damaged may not cost what reading it costs": the pump allocates
nothing past the window.

What nobody re-derived is that the pump's *time* is the whole decode, and that the decision asks
it of every stored appearance while the regeneration it exists for is one narrow route
(`appearance::regenerates`: a widget, a text or choice field, under Table 224's
`/NeedAppearances` or a value this program replaced). For every other annotation the very next
thing that happens is `draw_appearance` reading the same stream:

- **held whole** (the memo keeps it — every corpus appearance): the damage is a field beside the
  bytes, and `stated_damage` is the same answer at the same price. Nothing changes for this
  population, including the report's wording, which names the subtype.
- **windowed** (the memo declines it — every bomb, every decode above the ~4 MiB allowance): the
  run pumps the stream anyway, meets the damage mid-stream, and reports it through
  `Interpreter::run`'s issue translation in the run's own words. The pre-pass was a second full
  decode for a sentence the page then carried **twice**, in two spellings — the decision's named
  one and the run's generic one.

So the fix is one `match` in `annotation::appearance_damage`, taking the decided `Content` as an
argument: `Content::Stored` asks `stated_damage`, `Content::Constructed` — the regenerated
route, where the argument of ADR 0359 actually binds — still asks `damage()`.

## What it buys, measured

Both witnesses are one-page documents with one `Square` annotation whose `/AP /N` the memo
declines, built by the generator in `doc/history/787-the-answer-a-decision-paid-for.md` (the
appearance-stream variant of ADR 0586's);
both arms built in one sitting from adjacent commits, callgrind counts under
`--tool=callgrind`, wall clock three runs an arm, alternating, on a machine at load ~30–42 (the
sibling rounds'), which is why the wall figures are ranges and the instruction counts are the
authority.

| witness | before | after | |
|---|---|---|---|
| hex-armoured zlib bomb (ADR 0586's, 4 174 537 B encoded, decode bounded at 1 GiB) | 23 617 148 028 instr; 1.14–1.38 s to interpret | 22 052 215 589 instr; 1.06–1.14 s | **−6.63 %** |
| benign 5.24 MiB appearance (10 244 B flate, draws marks) | 233 371 525 instr | 167 667 365 instr | **−28.2 %** |

The two rows fail differently on purpose. The bomb's decode is zeros, which inflate at about
1.5 instructions a byte while the run's lexer still walks the gibibyte, so removing the duplicate
decode is a sixth of the instructions — and the bomb is also self-limiting across
interpretations, because the run that draws nothing records `window_found_nothing` (ADR 0646)
and every later decision meets `Refused`. The benign appearance is the sustained case: it draws
marks, so nothing is ever memoised, and the duplicated decode was a quarter of *every*
interpretation of that page for as long as the document is open — a zoom of such a page
re-interprets it (§12.5.3), so this was a per-gesture cost, not a per-open one.

Over ISO 32000-2's 1023 pages (`callgrind_pages`, `fresh`): 35 430 168 416 → 35 431 060 484
instructions, **+0.0025 %**, at identical display-list command totals (2 197 097) — codegen
noise on a population that is entirely the held-whole shape, whose two spellings of the
question are the same read of the same memo field.

## What a page shows, and what it reports

No pixel moves anywhere: the marks were never a function of the pre-pass. One report class
moves, on a population no corpus document is in: a **damaged, windowed, drawn** appearance used
to be reported twice — "a Square annotation's appearance stream (§12.5.5)" from the decision and
"an annotation's appearance stream (§12.5.5)" from the run — and is now reported once, by the
run. A damaged appearance held whole keeps its named report from the decision, byte for byte;
a damaged regenerated one keeps its named report too, from the route that still owes it.

## What remains, deliberately

The regenerated route still pays one full pump per decision for a windowed stored stream —
beside the whole buffered decode `regenerate` itself performs through `decoded_stream_data`.
That population is a widget appearance above the memo's allowance carrying variable text, which
no document on this disk states; the answer there is genuinely owed before the run, and folding
it into `regenerate`'s own decode would thread damage through `appearance::Regenerated` for a
population of none. Priced as: one decode where today there are two, on the rarest shape this
mechanism has. If such a document ever surfaces, the fold is the fix.

## Tests, and their calibration

`damaged_content_streams.rs` gains the two shapes the pinned test (`a_damaged_appearance_stream_
draws_its_prefix_and_reports_the_shortfall`, held whole, unchanged) could not tell apart:

- `a_windowed_appearance_streams_damage_is_reported_once_by_the_run` — a 5.25 MiB truncated
  `RunLengthDecode` appearance: the damage is reported exactly once, in the run's words, and the
  prefix's marks are on the page.
- `a_regenerated_widgets_stored_stream_still_reports_its_damage` — the same stream behind a
  `/NeedAppearances` text field: the splice replaces the bytes and the decision still names the
  damage, which is ADR 0359's requirement pinned against this very change.

Trap 13, above the calibration commit: planting the old shape (always `damage()`) fails the
first test on a doubled report and nothing else; planting the over-correction (always
`stated_damage()`) fails the second on a lost report and nothing else.
